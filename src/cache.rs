//! Caching system for Apollo configuration data.
//!
//! This module provides the `Cache` struct and related functionality for storing and managing
//! Apollo configuration data. The cache supports both in-memory and file-based storage with
//! platform-specific optimizations.
//!
//! # Features
//!
//! - **Multi-Level Caching**: In-memory cache with optional file-based persistence
//! - **Thread Safety**: All cache operations are thread-safe and async-friendly
//! - **Event System**: Support for event listeners on configuration changes
//! - **Platform Optimization**: Different behavior for native Rust vs WebAssembly
//! - **Concurrent Access Control**: Prevents race conditions during cache operations
//!
//! # Cache Hierarchy
//!
//! 1. **Memory Cache**: Fast in-memory storage for immediate access
//! 2. **File Cache** (native only): Persistent storage to reduce network requests
//! 3. **Remote Fetch**: Retrieval from Apollo server when cache misses occur
//!
//! # Platform Differences
//!
//! - **Native Rust**: Full caching with file persistence and background refresh
//! - **WebAssembly**: Persistent caching using browser localStorage with in-memory fallback for Node.js environments
//!
//! # Examples
//!
//! The cache is typically used internally by the `Client` struct and not directly
//! by end users. However, understanding its behavior is important for debugging
//! and performance optimization.

use crate::{
    EventListener,
    client_config::ClientConfig,
    namespace::{self, get_namespace},
};
use base64::display::Base64Display;
use cfg_if::cfg_if;
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use log::{debug, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fmt::Write, sync::Arc};
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};
use url::{ParseError, Url};

#[cfg(not(target_arch = "wasm32"))]
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CacheItem {
    timestamp: i64,
    config: Value,
}

/// Comprehensive error types that can occur during cache operations.
///
/// This enum covers all possible error conditions that may arise during cache
/// operations, from I/O failures to network and parsing issues.
///
/// # Error Categories
///
/// - **I/O Errors**: File system operations, directory creation, and file writing
/// - **Namespace Errors**: Issues with namespace format detection and processing
/// - **Serialization Errors**: JSON parsing and serialization failures
/// - **Network Errors**: HTTP request failures and response parsing issues
/// - **URL Errors**: Malformed URLs and parsing failures
///
/// # Examples
///
/// ```rust
/// use apollo_rust_client::Error;
///
/// // Internal cache errors are exposed through the public client error.
/// fn report_cache_error(error: &Error) {
///     if let Error::Cache(cache_error) = error {
///         eprintln!("Cache operation failed: {cache_error}");
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O error occurred during file operations.
    ///
    /// This error occurs when there are issues with file system operations,
    /// such as reading/writing cache files, creating directories, or
    /// insufficient permissions.
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),

    /// An error occurred during namespace processing.
    ///
    /// This includes errors from format detection, parsing, or type conversion
    /// operations specific to namespace handling.
    #[error("Namespace error: {0}")]
    Namespace(namespace::Error),

    /// A serialization/deserialization error occurred.
    ///
    /// This error occurs when there are issues with JSON parsing, such as
    /// malformed JSON data, type mismatches, or encoding problems.
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),

    /// A network request error occurred.
    ///
    /// This error occurs when there are issues with HTTP requests to the
    /// Apollo server, such as connection failures, timeouts, or invalid responses.
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    /// The server returned a non-success HTTP response.
    #[error("Apollo server returned HTTP {status}: {body}")]
    HttpStatus {
        /// Numeric HTTP status code.
        status: u16,
        /// Response body, retained for diagnostics.
        body: String,
    },

    /// A request exceeded the client-side timeout.
    #[error("Apollo request timed out after {seconds} seconds")]
    Timeout {
        /// Configured timeout in seconds.
        seconds: u64,
    },

    /// A URL parsing error occurred.
    ///
    /// This error occurs when the constructed URL for the Apollo server
    /// is malformed or cannot be parsed.
    #[error("Url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// The configured URL cannot be used as a hierarchical base URL.
    #[error("URL cannot be used as a base: {0}")]
    InvalidBaseUrl(String),

    /// HMAC initialization rejected the configured key.
    #[error("Invalid Apollo signing key")]
    InvalidSigningKey,
}

/// A cache instance for managing configuration data for a specific namespace.
///
/// The `Cache` struct is responsible for storing, retrieving, and managing configuration
/// data for a single Apollo namespace. It provides multi-level caching with thread-safe
/// operations and event notification capabilities.
///
/// # Features
///
/// - **Multi-Level Storage**: Memory cache with optional file-based persistence
/// - **Thread Safety**: All operations are protected by async-aware locks
/// - **Event Listeners**: Callbacks for configuration change notifications
/// - **Concurrent Protection**: Prevents race conditions during cache operations
/// - **Platform Optimization**: Adapts behavior based on target platform
///
/// # Cache Levels
///
/// 1. **Memory Cache**: Timestamped storage using `Arc<RwLock<Option<CacheItem>>>`
/// 2. **File Cache** (native only): Persistent JSON files for offline access
/// 3. **Remote Source**: Apollo Configuration Center via HTTP/HTTPS
///
/// # Concurrency Control
///
/// The cache uses `tokio::sync::RwLock` to ensure thread-safety.
/// The `memory` field is wrapped in an `Arc<RwLock<...>>` to allow multiple
/// concurrent readers and exclusive writers. This prevents data races when
/// accessing the cached configuration from multiple async tasks. The listeners
/// are also protected by a `RwLock`.
///
/// # Platform Differences
///
/// - **Native Rust**: Full feature set with file caching and background refresh
/// - **WebAssembly**: Persistent caching using browser localStorage with in-memory fallback and single-threaded execution
#[cfg_attr(target_arch = "wasm32", allow(clippy::arc_with_non_send_sync))]
#[derive(Clone)]
pub(crate) struct Cache {
    /// Client configuration containing server details and authentication.
    client_config: ClientConfig,

    /// The namespace name this cache instance manages.
    namespace: String,

    /// In-memory storage for the parsed configuration data.
    ///
    /// Contains the JSON representation of the configuration. `None` indicates
    /// that the cache has not been populated or a fetch operation failed.
    memory: Arc<RwLock<Option<CacheItem>>>,

    /// Collection of event listeners for configuration change notifications.
    ///
    /// Listeners are called when the cache is refreshed, allowing applications
    /// to react to configuration changes during periodic refresh.
    listeners: Arc<RwLock<Vec<EventListener>>>,

    /// The isolated cache key for WebAssembly localStorage persistence (wasm32 targets only).
    #[cfg(target_arch = "wasm32")]
    wasm_cache_key: String,

    /// Cancellation-safe single-flight lock for cold/stale loads.
    load_lock: Arc<Mutex<()>>,

    /// Path to the local cache file (native targets only).
    ///
    /// On native targets, this specifies where the configuration should be
    /// cached locally. The path includes the namespace name and any grayscale
    /// targeting parameters (IP, labels) to ensure cache isolation.
    #[cfg(not(target_arch = "wasm32"))]
    file_path: std::path::PathBuf,

    /// HTTP client for making network requests.
    http_client: reqwest::Client,
}

fn cache_identity(client_config: &ClientConfig, namespace: &str) -> String {
    let mut hasher = Sha1::new();
    hash_identity_part(&mut hasher, "apollo-rust-client-cache-v2");
    hash_identity_part(
        &mut hasher,
        client_config.config_server.trim_end_matches('/'),
    );
    hash_identity_part(&mut hasher, &client_config.app_id);
    hash_identity_part(&mut hasher, &client_config.cluster);
    hash_identity_part(&mut hasher, namespace);
    hash_optional_identity_part(&mut hasher, client_config.ip.as_deref());
    hash_optional_identity_part(&mut hasher, client_config.label.as_deref());

    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn hash_identity_part(hasher: &mut Sha1, value: &str) {
    hasher.update(value.len().to_be_bytes());
    hasher.update(value.as_bytes());
}

fn hash_optional_identity_part(hasher: &mut Sha1, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.update([1]);
            hash_identity_part(hasher, value);
        }
        None => hasher.update([0]),
    }
}

impl Cache {
    /// Create a new cache.
    ///
    /// # Arguments
    ///
    /// * `client_config` - The configuration for the Apollo client.
    /// * `namespace` - The namespace to get the cache for.
    /// * `http_client` - The HTTP client to use for requests.
    ///
    /// # Returns
    ///
    /// A new cache for the given namespace.
    pub(crate) fn new(
        client_config: ClientConfig,
        namespace: &str,
        http_client: reqwest::Client,
    ) -> Self {
        let cache_identity = cache_identity(&client_config, namespace);

        #[cfg(not(target_arch = "wasm32"))]
        let file_path = client_config
            .get_cache_dir()
            .join(format!("v2-{cache_identity}.cache.json"));

        #[cfg(target_arch = "wasm32")]
        let wasm_cache_key = format!("apollo_cache_v2_{cache_identity}");

        Self {
            client_config,
            namespace: namespace.to_string(),
            memory: Arc::new(RwLock::new(None)),
            listeners: Arc::new(RwLock::new(Vec::new())),
            load_lock: Arc::new(Mutex::new(())),

            #[cfg(not(target_arch = "wasm32"))]
            file_path,
            #[cfg(target_arch = "wasm32")]
            wasm_cache_key,
            http_client,
        }
    }

    /// Get a configuration from the cache.
    ///
    /// This method retrieves the configuration for the namespace. It implements a read-through
    /// cache pattern with the following logic:
    ///
    /// 1.  It first attempts to read from the in-memory cache. If the cache is populated,
    ///     it returns the configuration immediately. This read is non-blocking for other readers.
    /// 2.  If memory is missing or stale, it acquires the cancellation-safe single-flight gate
    ///     and repeats the freshness check.
    /// 3.  If the cache is still empty, it proceeds to load it, first from the file cache (on native targets)
    ///     if it's not stale, otherwise by fetching from the remote Apollo server.
    /// 4.  Once the configuration is fetched, it updates the in-memory cache and notifies any
    ///     registered listeners.
    ///
    /// Readers remain concurrent, and only the final memory swap uses the write lock.
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The configuration value if successfully retrieved.
    /// * `Err(Error)` - An error if the configuration could not be retrieved. See the `Error`
    ///   enum for possible variants.
    ///
    /// # Errors
    ///
    /// This method can return transport, HTTP status, URL, or parsing errors when no
    /// usable stale value exists. Persistent read/write failures are logged and ignored.
    pub(crate) async fn get_value(&self) -> Result<Value, Error> {
        if let Some(value) = self.fresh_memory_value().await {
            return Ok(value);
        }

        // A Tokio mutex releases automatically when a loading task is cancelled,
        // avoiding the stuck boolean/notification state of the previous design.
        let _load_guard = self.load_lock.lock().await;
        if let Some(value) = self.fresh_memory_value().await {
            return Ok(value);
        }

        self.load_and_cache().await
    }

    /// Internal method to load configuration and update cache.
    ///
    /// This method handles persistent freshness, remote loading, and stale fallback.
    async fn load_and_cache(&self) -> Result<Value, Error> {
        let memory_fallback = self.memory.read().await.clone();
        let persistent_fallback = self.load_persistent_item().await;

        if let Some(item) = persistent_fallback.as_ref()
            && self.is_fresh(item)
        {
            self.replace_memory(item.clone()).await;
            return Ok(item.config.clone());
        }

        match self.fetch_remote_config().await {
            Ok(item) => {
                self.persist_best_effort(&item).await;
                self.replace_memory(item.clone()).await;
                Ok(item.config)
            }
            Err(error) => {
                self.notify_error(&error).await;
                if let Some(fallback) = memory_fallback.or(persistent_fallback) {
                    warn!(
                        "Using stale cached configuration for namespace {} after refresh failure: {}",
                        self.namespace, error
                    );
                    self.replace_memory(fallback.clone()).await;
                    Ok(fallback.config)
                } else {
                    Err(error)
                }
            }
        }
    }

    async fn fresh_memory_value(&self) -> Option<Value> {
        let memory = self.memory.read().await;
        memory
            .as_ref()
            .filter(|item| self.is_fresh(item))
            .map(|item| item.config.clone())
    }

    fn is_fresh(&self, item: &CacheItem) -> bool {
        let age = Utc::now().timestamp().saturating_sub(item.timestamp);
        #[allow(clippy::cast_possible_wrap)]
        {
            age <= self.client_config.effective_cache_ttl() as i64
        }
    }

    #[cfg_attr(target_arch = "wasm32", allow(clippy::unused_async))]
    async fn load_persistent_item(&self) -> Option<CacheItem> {
        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                match tokio::fs::read(&self.file_path).await {
                    Ok(content) => match serde_json::from_slice(&content) {
                        Ok(item) => Some(item),
                        Err(error) => {
                            warn!("Ignoring corrupt cache file {}: {error}", self.file_path.display());
                            None
                        }
                    },
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => {
                        warn!("Unable to read cache file {}: {error}", self.file_path.display());
                        None
                    }
                }
            } else {
                load_from_local_storage(&self.wasm_cache_key).and_then(|cached| {
                    match serde_json::from_str(&cached) {
                        Ok(item) => Some(item),
                        Err(error) => {
                            warn!("Ignoring corrupt localStorage cache entry {}: {error}", self.wasm_cache_key);
                            None
                        }
                    }
                })
            }
        }
    }

    /// Refreshes the cache by fetching the latest configuration from the Apollo server.
    ///
    /// This method unconditionally fetches the latest configuration from the remote server,
    /// updates the in-memory cache, attempts persistence on a best-effort basis, and
    /// notifies listeners only when the value changed.
    ///
    /// Network and persistence I/O occur without holding the in-memory write lock.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the cache was successfully refreshed.
    /// * `Err(Error)` - An error if the refresh operation failed.
    ///
    /// # Errors
    ///
    /// This method can return various errors, such as network errors, parsing errors,
    /// or HTTP-status errors. Persistence failures are logged and do not fail refresh.
    pub(crate) async fn refresh(&self) -> Result<(), Error> {
        match self.fetch_remote_config().await {
            Ok(item) => {
                self.persist_best_effort(&item).await;
                self.replace_memory(item).await;
                Ok(())
            }
            Err(error) => {
                self.notify_error(&error).await;
                Err(error)
            }
        }
    }

    async fn replace_memory(&self, item: CacheItem) {
        let changed = {
            let mut memory = self.memory.write().await;
            let changed = memory
                .as_ref()
                .is_none_or(|previous| previous.config != item.config);
            *memory = Some(item.clone());
            changed
        };

        if changed {
            let listeners = self.listeners.read().await.clone();
            self.notify_listeners(&item.config, &listeners);
        }
    }

    async fn notify_error(&self, error: &Error) {
        let listeners = self.listeners.read().await.clone();
        for listener in listeners {
            invoke_listener(&listener, Err(crate::Error::Refresh(error.to_string())));
        }
    }

    fn notify_listeners(&self, config: &Value, listeners: &[EventListener]) {
        for listener in listeners {
            let config = config.clone();
            let namespace = self.namespace.clone();
            invoke_listener(
                listener,
                get_namespace(&namespace, config).map_err(crate::Error::Namespace),
            );
        }
    }

    async fn fetch_remote_config(&self) -> Result<CacheItem, Error> {
        let url = self.build_request_url()?;
        let client = self.build_http_request(&url)?;

        // Add timeout to prevent long pauses
        #[cfg(target_arch = "wasm32")]
        let response = self.execute_request(client).await?;

        #[cfg(not(target_arch = "wasm32"))]
        let response = {
            let timeout_duration = std::time::Duration::from_secs(10);
            tokio::time::timeout(timeout_duration, self.execute_request(client))
                .await
                .map_err(|_| Error::Timeout { seconds: 10 })??
        };

        let config = self.parse_response(response).await?;
        Ok(CacheItem {
            timestamp: Utc::now().timestamp(),
            config,
        })
    }

    #[cfg_attr(target_arch = "wasm32", allow(clippy::unused_async))]
    async fn persist_best_effort(&self, item: &CacheItem) {
        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                if let Err(error) = self.write_to_file_cache(item).await {
                    warn!("Unable to persist cache for namespace {}: {error}", self.namespace);
                }
            } else {
                if let Ok(content) = serde_json::to_string(item)
                    && save_to_local_storage(&self.wasm_cache_key, &content).is_none() {
                    warn!("Unable to persist localStorage cache entry {}", self.wasm_cache_key);
                }
            }
        }
    }

    /// Builds the request URL for the Apollo configuration service.
    ///
    /// Constructs the URL with the base path, app ID, cluster, and namespace,
    /// and adds optional query parameters for IP and label.
    ///
    /// # Returns
    ///
    /// * `Ok(Url)` - The constructed URL
    /// * `Err(Error::UrlParse)` - If URL parsing fails
    fn build_request_url(&self) -> Result<Url, Error> {
        let mut url = Url::parse(&self.client_config.config_server)?;
        {
            let base_url = url.to_string();
            let mut segments = url
                .path_segments_mut()
                .map_err(|()| Error::InvalidBaseUrl(base_url))?;
            segments.pop_if_empty();
            segments.extend([
                "configfiles",
                "json",
                &self.client_config.app_id,
                &self.client_config.cluster,
                &self.namespace,
            ]);
        }

        if let Some(ip) = &self.client_config.ip {
            url.query_pairs_mut().append_pair("ip", ip);
        }
        if let Some(label) = &self.client_config.label {
            url.query_pairs_mut().append_pair("label", label);
        }

        Ok(url)
    }

    /// Builds the HTTP request with optional authentication headers.
    ///
    /// If a secret is configured, adds timestamp and authorization headers
    /// with HMAC-SHA1 signature.
    ///
    /// # Arguments
    ///
    /// * `url` - The request URL
    ///
    /// # Returns
    ///
    /// * `Ok(reqwest::RequestBuilder)` - The configured request builder
    /// * `Err(Error)` - If signature generation fails
    fn build_http_request(&self, url: &Url) -> Result<reqwest::RequestBuilder, Error> {
        let mut client = self.http_client.get(url.as_str());

        if let Some(secret) = &self.client_config.secret {
            let timestamp = Utc::now().timestamp_millis();
            let signature = sign(timestamp, url.as_str(), secret)?;
            client = client.header("timestamp", timestamp.to_string());
            client = client.header(
                "Authorization",
                format!("Apollo {}:{}", self.client_config.app_id, signature),
            );
        }

        Ok(client)
    }

    /// Executes the HTTP request and returns the response.
    ///
    /// # Arguments
    ///
    /// * `client` - The configured request builder
    ///
    /// # Returns
    ///
    /// * `Ok(reqwest::Response)` - The HTTP response
    /// * `Err(Error::Reqwest)` - If the request fails
    async fn execute_request(
        &self,
        client: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, Error> {
        match client.send().await {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::Reqwest(e)),
        }
    }

    /// Parses the HTTP response body as JSON configuration.
    ///
    /// # Arguments
    ///
    /// * `response` - The HTTP response
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The parsed configuration
    /// * `Err(Error::Reqwest)` - If reading the response body fails
    /// * `Err(Error::Serde)` - If JSON parsing fails
    async fn parse_response(&self, response: reqwest::Response) -> Result<Value, Error> {
        let status = response.status();
        let body: String = match response.text().await {
            Ok(b) => b,
            Err(e) => return Err(Error::Reqwest(e)),
        };

        trace!("Response body {} for namespace {}", body, self.namespace);

        if !status.is_success() {
            return Err(Error::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }

        match serde_json::from_str(&body) {
            Ok(c) => Ok(c),
            Err(e) => {
                debug!("error parsing config: {e}");
                Err(Error::Serde(e))
            }
        }
    }

    /// Writes the configuration to the file cache (native targets only).
    ///
    /// Creates parent directories if they don't exist and writes the cache item
    /// with timestamp and configuration data.
    ///
    /// # Arguments
    ///
    /// * `item` - Timestamped configuration to cache
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If caching succeeds
    /// * `Err(Error::Io)` - If file operations fail
    /// * `Err(Error::Serde)` - If serialization fails
    #[cfg(not(target_arch = "wasm32"))]
    async fn write_to_file_cache(&self, item: &CacheItem) -> Result<(), Error> {
        debug!("writing cache file {}", self.file_path.display());

        // Create parent directories if they don't exist
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let cache_content = serde_json::to_vec(item)?;

        // Each writer gets a unique temporary file. `create_new` also protects
        // against a rare PID/counter collision across concurrent processes.
        let mut attempt = 0_u8;
        let (temp_file_path, mut temp_file) = loop {
            let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let temp_file_path = self.file_path.with_extension(format!(
                "json.{}.{}.tmp",
                std::process::id(),
                counter
            ));
            match tokio::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_file_path)
                .await
            {
                Ok(file) => break (temp_file_path, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && attempt < 8 => {
                    attempt += 1;
                }
                Err(error) => return Err(Error::Io(error)),
            }
        };

        if let Err(error) = async {
            temp_file.write_all(&cache_content).await?;
            temp_file.flush().await?;
            drop(temp_file);
            tokio::fs::rename(&temp_file_path, &self.file_path).await
        }
        .await
        {
            let _ = tokio::fs::remove_file(&temp_file_path).await;
            return Err(Error::Io(error));
        }

        trace!(
            "Wrote cache file {} for namespace {}",
            self.file_path.display(),
            self.namespace
        );

        Ok(())
    }

    /// Adds an event listener to the cache.
    ///
    /// Listeners are called in registration order when configuration changes or a
    /// refresh fails. They receive `Result<Namespace, crate::Error>`.
    ///
    ///
    /// Callbacks run synchronously after internal locks are released. Panics are caught
    /// and logged, and do not prevent subsequent listeners from running.
    ///
    /// # Arguments
    ///
    /// * `listener` - The event listener to register. It must be an `Arc`-wrapped, thread-safe
    ///   closure (`Send + Sync`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use apollo_rust_client::{Client, EventListener, client_config::ClientConfig};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ClientConfig::builder("my-app", "http://localhost:8080").build()?;
    /// let client = Client::new(config)?;
    /// let listener: EventListener = Arc::new(|result| {
    ///     match result {
    ///         Ok(config) => println!("Cache refreshed, new config: {:?}", config),
    ///         Err(error) => eprintln!("Cache refresh failed: {error}"),
    ///     }
    /// });
    /// client.add_listener("application", listener).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_listener(&self, listener: EventListener) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    /// Returns the WASM cache key (wasm32 targets only).
    #[cfg(all(target_arch = "wasm32", test))]
    pub(crate) fn wasm_cache_key(&self) -> &str {
        &self.wasm_cache_key
    }
}

fn invoke_listener(
    listener: &EventListener,
    result: Result<crate::namespace::Namespace, crate::Error>,
) {
    let invocation = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| listener(result)));
    if invocation.is_err() {
        log::error!("Apollo configuration listener panicked");
    }
}

type HmacSha1 = Hmac<Sha1>;

/// Generates a signature for Apollo API authentication using HMAC-SHA1.
///
/// This function takes a timestamp, the request URL (or its path and query), and an Apollo secret key
/// to create a Base64 encoded signature. This signature is typically used in the `Authorization`
/// header when making requests to the Apollo configuration service.
///
/// # Arguments
///
/// * `timestamp` - The current timestamp in milliseconds since the Unix epoch. This is used as part of the message to be signed.
/// * `url` - The URL being requested. This can be a full URL or just the path and query string.
/// * `secret` - The Apollo secret key used for generating the HMAC-SHA1 signature.
///
/// # Returns
///
/// * `Ok(String)` - A Base64 encoded string representing the signature
/// * `Err(Error)` - If URL parsing or HMAC initialization fails
///
/// # Errors
///
/// This function will return an error if:
/// - The URL cannot be parsed as either an absolute URL or a request path
/// - The HMAC implementation rejects the signing key
///
/// # Examples
///
/// ```text
/// message = "1576478257344\n/configs/100004458/default/application?ip=10.0.0.1"
/// signature = Base64(HMAC-SHA1(secret, message))
/// ```
///
/// This helper is internal; callers configure `ClientConfig::secret` and the client
/// adds the timestamp and authorization headers automatically.
pub(crate) fn sign(timestamp: i64, url: &str, secret: &str) -> Result<String, Error> {
    let u = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => match e {
            ParseError::RelativeUrlWithoutBase => {
                let base_url = Url::parse("http://localhost:8080")?;
                base_url.join(url)?
            }
            _ => {
                return Err(Error::UrlParse(e));
            }
        },
    };
    let mut path_and_query = String::from(u.path());
    if let Some(query) = u.query() {
        let _ = write!(path_and_query, "?{query}");
    }
    let input = format!("{timestamp}\n{path_and_query}");
    trace!("input for signing: {input}");

    let mut mac =
        HmacSha1::new_from_slice(secret.as_bytes()).map_err(|_| Error::InvalidSigningKey)?;
    mac.update(input.as_bytes());
    let result: [u8; 20] = mac.finalize().into_bytes().into();

    // Convert the result to a string using base64
    let code = Base64Display::new(&result, &base64::engine::general_purpose::STANDARD);
    Ok(code.to_string())
}

#[cfg(target_arch = "wasm32")]
fn load_from_local_storage(key: &str) -> Option<String> {
    let global = js_sys::global();
    let storage =
        js_sys::Reflect::get(&global, &wasm_bindgen::JsValue::from_str("localStorage")).ok()?;
    if storage.is_undefined() || storage.is_null() {
        return None;
    }
    let get_item_fn =
        js_sys::Reflect::get(&storage, &wasm_bindgen::JsValue::from_str("getItem")).ok()?;
    if get_item_fn.is_function() {
        let args = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(key));
        let result = js_sys::Reflect::apply(&get_item_fn.into(), &storage, &args).ok()?;
        if !result.is_null() && !result.is_undefined() {
            return result.as_string();
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn save_to_local_storage(key: &str, value: &str) -> Option<()> {
    let global = js_sys::global();
    let storage =
        js_sys::Reflect::get(&global, &wasm_bindgen::JsValue::from_str("localStorage")).ok()?;
    if storage.is_undefined() || storage.is_null() {
        return None;
    }
    let set_item_fn =
        js_sys::Reflect::get(&storage, &wasm_bindgen::JsValue::from_str("setItem")).ok()?;
    if set_item_fn.is_function() {
        let args = js_sys::Array::of2(
            &wasm_bindgen::JsValue::from_str(key),
            &wasm_bindgen::JsValue::from_str(value),
        );
        let _ = js_sys::Reflect::apply(&set_item_fn.into(), &storage, &args).ok()?;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::{TempDir, client_config::ClientConfig};
    #[cfg(not(target_arch = "wasm32"))]
    use std::{
        io::{Read, Write as IoWrite},
        net::{SocketAddr, TcpListener, TcpStream},
        sync::{
            Arc, Mutex as StdMutex,
            atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering},
        },
        thread,
        time::Duration,
    };

    #[cfg(not(target_arch = "wasm32"))]
    type ResponseHandler = dyn Fn(usize, &str) -> (u16, String, Duration) + Send + Sync;

    #[cfg(not(target_arch = "wasm32"))]
    struct TestHttpServer {
        address: SocketAddr,
        requests: Arc<AtomicUsize>,
        captured: Arc<StdMutex<Vec<String>>>,
        running: Arc<AtomicBool>,
        thread: Option<thread::JoinHandle<()>>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl TestHttpServer {
        fn new(handler: Arc<ResponseHandler>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(AtomicUsize::new(0));
            let captured = Arc::new(StdMutex::new(Vec::new()));
            let running = Arc::new(AtomicBool::new(true));
            let requests_in_thread = requests.clone();
            let captured_in_thread = captured.clone();
            let running_in_thread = running.clone();
            let thread = thread::spawn(move || {
                while running_in_thread.load(AtomicOrdering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let handler = handler.clone();
                            let requests = requests_in_thread.clone();
                            let captured = captured_in_thread.clone();
                            thread::spawn(move || {
                                Self::respond(stream, handler.as_ref(), &requests, &captured);
                            });
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                address,
                requests,
                captured,
                running,
                thread: Some(thread),
            }
        }

        fn respond(
            mut stream: TcpStream,
            handler: &ResponseHandler,
            requests: &AtomicUsize,
            captured: &StdMutex<Vec<String>>,
        ) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        request.extend_from_slice(&buffer[..count]);
                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }
            let request = String::from_utf8_lossy(&request).into_owned();
            captured.lock().unwrap().push(request.clone());
            let index = requests.fetch_add(1, AtomicOrdering::AcqRel) + 1;
            let (status, body, delay) = handler(index, &request);
            thread::sleep(delay);
            let reason = if (200..300).contains(&status) {
                "OK"
            } else {
                "Error"
            };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }

        fn url(&self) -> String {
            format!("http://{}", self.address)
        }

        async fn wait_for_requests(&self, expected: usize) {
            tokio::time::timeout(Duration::from_secs(2), async {
                while self.requests.load(AtomicOrdering::Acquire) < expected {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .expect("server did not receive the expected request count");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl Drop for TestHttpServer {
        fn drop(&mut self) {
            self.running.store(false, AtomicOrdering::Release);
            let _ = TcpStream::connect(self.address);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn test_config(server: &TestHttpServer, cache_dir: &std::path::Path) -> ClientConfig {
        ClientConfig::builder("test-app", server.url())
            .cache_dir(cache_dir.to_string_lossy())
            .build()
            .unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fixed_server(status: u16, body: &'static str) -> TestHttpServer {
        TestHttpServer::new(Arc::new(move |_, _| {
            (status, body.to_string(), Duration::ZERO)
        }))
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_concurrent_get_value() {
        setup();
        let temp_dir = TempDir::new("apollo_concurrent_get_test");

        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: std::env::var("APOLLO_TEST_SERVER")
                .unwrap_or_else(|_| String::from("http://localhost:8080")),
            secret: None,
            cache_dir: Some(temp_dir.path().to_str().unwrap().to_string()),
            label: None,
            ip: None,
            allow_insecure_https: None,
            #[cfg(not(target_arch = "wasm32"))]
            cache_ttl: None,
            #[cfg(not(target_arch = "wasm32"))]
            refresh_interval: None,
            #[cfg(not(target_arch = "wasm32"))]
            http_client: None,
        };

        let cache = Arc::new(Cache::new(config, "application", reqwest::Client::new()));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let cache = cache.clone();
            let handle = tokio::spawn(async move { cache.get_value().await });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;

        let first_result = results[0].as_ref().unwrap().as_ref().unwrap();
        for result in &results {
            let result = result.as_ref().unwrap().as_ref().unwrap();
            assert_eq!(result, first_result);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn cache_identity_and_url_segments_are_isolated_and_safe() {
        let temp_dir = TempDir::new("cache_identity_and_url_segments");
        let server = fixed_server(200, "{}");
        let base = ClientConfig::builder("app/../../escape", format!("{}/apollo/", server.url()))
            .cluster("prod/us")
            .cache_dir(temp_dir.path().to_string_lossy())
            .ip("10.0.0.1")
            .label("canary/a")
            .build()
            .unwrap();
        let cache = Cache::new(base.clone(), "../public.properties", reqwest::Client::new());
        let url = cache.build_request_url().unwrap();
        assert_eq!(
            url.path(),
            "/apollo/configfiles/json/app%2F..%2F..%2Fescape/prod%2Fus/..%2Fpublic.properties"
        );
        assert_eq!(url.query(), Some("ip=10.0.0.1&label=canary%2Fa"));
        assert_eq!(cache.file_path.parent().unwrap(), base.get_cache_dir());
        assert!(
            cache
                .file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("v2-")
        );

        let mut cluster = base.clone();
        cluster.cluster = "staging".to_string();
        let mut server_config = base.clone();
        server_config.config_server = "http://example.invalid".to_string();
        let mut target = base.clone();
        target.ip = None;
        for other in [cluster, server_config, target] {
            let other = Cache::new(other, "../public.properties", reqwest::Client::new());
            assert_ne!(cache.file_path, other.file_path);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn rejects_http_errors_without_caching_them() {
        let server = fixed_server(500, r#"{"message":"internal"}"#);
        let temp_dir = TempDir::new("rejects_http_errors");
        let cache = Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            reqwest::Client::new(),
        );
        let error = cache.get_value().await.unwrap_err();
        assert!(matches!(error, Error::HttpStatus { status: 500, .. }));
        assert!(!cache.file_path.exists());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn persistence_failure_is_nonfatal() {
        let server = fixed_server(200, r#"{"value":"remote"}"#);
        let temp_dir = TempDir::new("persistence_failure_is_nonfatal");
        let blocked = temp_dir.path().join("not-a-directory");
        tokio::fs::write(&blocked, b"file").await.unwrap();
        let cache = Cache::new(
            test_config(&server, &blocked),
            "application",
            reqwest::Client::new(),
        );
        assert_eq!(cache.get_value().await.unwrap()["value"], "remote");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn stale_persistent_value_falls_back_and_reports_refresh_error() {
        let server = fixed_server(503, "unavailable");
        let temp_dir = TempDir::new("stale_persistent_fallback");
        let mut config = test_config(&server, temp_dir.path());
        config.cache_ttl = Some(1);
        let cache = Cache::new(config, "application", reqwest::Client::new());
        tokio::fs::create_dir_all(cache.file_path.parent().unwrap())
            .await
            .unwrap();
        let stale = CacheItem {
            timestamp: Utc::now().timestamp() - 60,
            config: serde_json::json!({"value": "stale"}),
        };
        tokio::fs::write(&cache.file_path, serde_json::to_vec(&stale).unwrap())
            .await
            .unwrap();
        let errors = Arc::new(AtomicUsize::new(0));
        let errors_in_listener = errors.clone();
        cache
            .add_listener(Arc::new(move |result| {
                if result.is_err() {
                    errors_in_listener.fetch_add(1, AtomicOrdering::AcqRel);
                }
            }))
            .await;

        assert_eq!(cache.get_value().await.unwrap()["value"], "stale");
        assert_eq!(errors.load(AtomicOrdering::Acquire), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn cancelled_initial_load_releases_single_flight() {
        let server = TestHttpServer::new(Arc::new(|_, _| {
            (
                200,
                r#"{"value":"loaded"}"#.to_string(),
                Duration::from_millis(250),
            )
        }));
        let temp_dir = TempDir::new("cancelled_initial_load");
        let cache = Arc::new(Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            reqwest::Client::new(),
        ));
        let first_cache = cache.clone();
        let first = tokio::spawn(async move { first_cache.get_value().await });
        server.wait_for_requests(1).await;
        first.abort();
        let value = tokio::time::timeout(Duration::from_secs(2), cache.get_value())
            .await
            .expect("second load was blocked by the cancelled loader")
            .unwrap();
        assert_eq!(value["value"], "loaded");
        assert!(server.requests.load(AtomicOrdering::Acquire) >= 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn readers_are_not_blocked_by_refresh_io_and_ttl_applies_to_memory() {
        let server = TestHttpServer::new(Arc::new(|index, _| {
            let body = if index == 1 {
                r#"{"value":"first"}"#
            } else {
                r#"{"value":"second"}"#
            };
            let delay = if index == 2 {
                Duration::from_millis(250)
            } else {
                Duration::ZERO
            };
            (200, body.to_string(), delay)
        }));
        let temp_dir = TempDir::new("readers_not_blocked_by_refresh");
        let cache = Arc::new(Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            reqwest::Client::new(),
        ));
        assert_eq!(cache.get_value().await.unwrap()["value"], "first");
        let refresh_cache = cache.clone();
        let refresh = tokio::spawn(async move { refresh_cache.refresh().await });
        server.wait_for_requests(2).await;
        let current = tokio::time::timeout(Duration::from_millis(50), cache.get_value())
            .await
            .expect("reader waited for refresh network I/O")
            .unwrap();
        assert_eq!(current["value"], "first");
        refresh.await.unwrap().unwrap();

        tokio::fs::remove_file(&cache.file_path).await.unwrap();
        cache.memory.write().await.as_mut().unwrap().timestamp -= 1_000;
        assert_eq!(cache.get_value().await.unwrap()["value"], "second");
        assert_eq!(server.requests.load(AtomicOrdering::Acquire), 3);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn listeners_receive_changes_and_errors_but_not_unchanged_values() {
        let server = TestHttpServer::new(Arc::new(|index, _| {
            if index < 3 {
                (200, r#"{"value":"same"}"#.to_string(), Duration::ZERO)
            } else {
                (429, "rate limited".to_string(), Duration::ZERO)
            }
        }));
        let temp_dir = TempDir::new("listener_change_semantics");
        let cache = Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            reqwest::Client::new(),
        );
        let changes = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let changes_in_listener = changes.clone();
        let errors_in_listener = errors.clone();
        cache
            .add_listener(Arc::new(move |result| match result {
                Ok(_) => {
                    changes_in_listener.fetch_add(1, AtomicOrdering::AcqRel);
                }
                Err(_) => {
                    errors_in_listener.fetch_add(1, AtomicOrdering::AcqRel);
                }
            }))
            .await;
        cache.get_value().await.unwrap();
        cache.refresh().await.unwrap();
        assert_eq!(changes.load(AtomicOrdering::Acquire), 1);
        assert!(matches!(
            cache.refresh().await.unwrap_err(),
            Error::HttpStatus { status: 429, .. }
        ));
        assert_eq!(errors.load(AtomicOrdering::Acquire), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn authenticated_request_contains_required_headers() {
        let server = fixed_server(200, "{}");
        let temp_dir = TempDir::new("authenticated_request_headers");
        let mut config = test_config(&server, temp_dir.path());
        config.secret = Some("secret".to_string());
        let cache = Cache::new(config, "application", reqwest::Client::new());
        cache.get_value().await.unwrap();
        let request = server
            .captured
            .lock()
            .unwrap()
            .first()
            .unwrap()
            .to_lowercase();
        assert!(request.contains("\r\ntimestamp:"));
        assert!(request.contains("\r\nauthorization: apollo test-app:"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn concurrent_atomic_writers_use_unique_temporary_files() {
        let server = fixed_server(200, "{}");
        let temp_dir = TempDir::new("concurrent_atomic_writers");
        let cache = Arc::new(Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            reqwest::Client::new(),
        ));
        let item = CacheItem {
            timestamp: Utc::now().timestamp(),
            config: serde_json::json!({"value": "valid"}),
        };
        let writers = (0..16).map(|_| {
            let cache = cache.clone();
            let item = item.clone();
            tokio::spawn(async move { cache.write_to_file_cache(&item).await })
        });
        for result in futures::future::join_all(writers).await {
            result.unwrap().unwrap();
        }
        let stored: CacheItem =
            serde_json::from_slice(&tokio::fs::read(&cache.file_path).await.unwrap()).unwrap();
        assert_eq!(stored, item);
        let mut directory = tokio::fs::read_dir(cache.file_path.parent().unwrap())
            .await
            .unwrap();
        while let Some(entry) = directory.next_entry().await.unwrap() {
            assert!(!entry.file_name().to_string_lossy().ends_with(".tmp"));
        }
    }

    #[test]
    fn test_sign_with_path() {
        let url = "/configs/100004458/default/application?ip=10.0.0.1";
        let secret = "df23df3f59884980844ff3dada30fa97";
        let signature = sign(1_576_478_257_344, url, secret).unwrap();
        assert_eq!(signature, "EoKyziXvKqzHgwx+ijDJwgVTDgE=");
    }

    #[test]
    fn test_sign_url() {
        setup();
        let url = "http://localhost:8080/configs/100004458/default/application?ip=10.0.0.1";
        let secret = "df23df3f59884980844ff3dada30fa97";
        let signature = sign(1_576_478_257_344, url, secret).unwrap();
        assert_eq!(signature, "EoKyziXvKqzHgwx+ijDJwgVTDgE=");
    }
}
