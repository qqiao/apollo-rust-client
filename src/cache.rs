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

use crate::{EventListener, client_config::ClientConfig, namespace::get_namespace};
use base64::display::Base64Display;
use cfg_if::cfg_if;
use chrono::Utc;
#[cfg(target_arch = "wasm32")]
use futures::{FutureExt, future::Either};
use hmac::{Hmac, KeyInit, Mac};
use log::{debug, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fmt::Write, sync::Arc};
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};
use url::{ParseError, Url};

#[cfg(not(target_arch = "wasm32"))]
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn cleanup_stale_temp_files(cache_dir: &std::path::Path) {
    let entries = match std::fs::read_dir(cache_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            warn!(
                "Unable to scan cache directory {} for stale temporary files: {error}",
                cache_dir.display()
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_temp_file = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("v2-"))
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tmp"));
        if is_temp_file && let Err(error) = std::fs::remove_file(&path) {
            warn!(
                "Unable to remove stale cache temporary file {}: {error}",
                path.display()
            );
        }
    }
}

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

    /// A concurrent caller observed the failure from a coalesced refresh.
    #[error("Coalesced Apollo refresh failed: {0}")]
    CoalescedRefresh(String),

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

    /// Cancellation-safe single-flight lock for cold loads.
    load_lock: Arc<Mutex<()>>,

    /// Single-flight lock shared by cold loads, manual refreshes, and polling.
    refresh_lock: Arc<Mutex<()>>,

    /// Completed refresh generation used to coalesce concurrent waiters.
    refresh_generation: Arc<AtomicU64>,

    /// Error snapshot for waiters that shared a failed refresh.
    last_refresh_error: Arc<RwLock<Option<String>>>,

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
            refresh_lock: Arc::new(Mutex::new(())),
            refresh_generation: Arc::new(AtomicU64::new(0)),
            last_refresh_error: Arc::new(RwLock::new(None)),

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
    /// 1.  It first attempts to read from memory. Any cached value is returned
    ///     immediately; an expired value schedules one background revalidation.
    /// 2.  On a cold miss it acquires the cancellation-safe single-flight gate and
    ///     repeats the memory check.
    /// 3.  A persistent native-file or browser-localStorage value is loaded and
    ///     returned immediately, scheduling revalidation when expired.
    /// 4.  Only a true cold miss waits for the remote Apollo request. Concurrent
    ///     cold loads and refreshes are coalesced per namespace.
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
        if let Some(item) = self.memory.read().await.clone() {
            return Ok(self.serve_cached_item(item));
        }

        let observed_refresh_generation = self.refresh_generation.load(Ordering::Acquire);
        // A Tokio mutex releases automatically when a loading task is cancelled,
        // avoiding the stuck boolean/notification state of the previous design.
        let _load_guard = self.load_lock.lock().await;
        if let Some(item) = self.memory.read().await.clone() {
            return Ok(self.serve_cached_item(item));
        }

        if let Some(item) = self.load_persistent_item().await {
            self.replace_memory(item.clone()).await;
            return Ok(self.serve_cached_item(item));
        }

        if self.refresh_generation.load(Ordering::Acquire) != observed_refresh_generation {
            return Err(self.coalesced_refresh_error().await);
        }

        self.coalesced_refresh(false).await?;
        self.memory
            .read()
            .await
            .as_ref()
            .map(|item| item.config.clone())
            .ok_or_else(|| Error::CoalescedRefresh("refresh produced no cache value".to_string()))
    }

    fn serve_cached_item(&self, item: CacheItem) -> Value {
        if !self.is_fresh(&item) {
            self.schedule_revalidation();
        }
        item.config
    }

    fn schedule_revalidation(&self) {
        let Ok(refresh_guard) = self.refresh_lock.clone().try_lock_owned() else {
            return;
        };
        let cache = self.clone();
        let task = async move {
            if let Err(error) = cache.perform_refresh(refresh_guard).await {
                warn!(
                    "Using stale cached configuration for namespace {} after refresh failure: {}",
                    cache.namespace, error
                );
                cache.notify_error(&error).await;
            }
        };
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                wasm_bindgen_futures::spawn_local(task);
            } else {
                tokio::spawn(task);
            }
        }
    }

    fn is_fresh(&self, item: &CacheItem) -> bool {
        let cache_ttl = self.client_config.effective_cache_ttl();
        if cache_ttl == 0 {
            return false;
        }
        let age = Utc::now().timestamp().saturating_sub(item.timestamp);
        #[allow(clippy::cast_possible_wrap)]
        {
            age <= cache_ttl as i64
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
    /// This method requests the latest configuration from the remote server,
    /// coalescing with a concurrent manual or background refresh. The shared result
    /// updates memory, attempts persistence on a best-effort basis, and notifies
    /// listeners only when the value changed.
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
        self.coalesced_refresh(true).await
    }

    async fn coalesced_refresh(&self, notify_error: bool) -> Result<(), Error> {
        let observed_generation = self.refresh_generation.load(Ordering::Acquire);
        let refresh_guard = self.refresh_lock.clone().lock_owned().await;
        if self.refresh_generation.load(Ordering::Acquire) != observed_generation {
            return self.last_refresh_result().await;
        }

        let result = self.perform_refresh(refresh_guard).await;
        if notify_error && let Err(error) = &result {
            self.notify_error(error).await;
        }
        result
    }

    async fn perform_refresh(&self, _refresh_guard: OwnedMutexGuard<()>) -> Result<(), Error> {
        let result = match self.fetch_remote_config().await {
            Ok(item) => {
                self.persist_best_effort(&item).await;
                self.replace_memory(item).await;
                Ok(())
            }
            Err(error) => Err(error),
        };
        *self.last_refresh_error.write().await = result.as_ref().err().map(ToString::to_string);
        self.refresh_generation.fetch_add(1, Ordering::Release);
        result
    }

    async fn last_refresh_result(&self) -> Result<(), Error> {
        match self.last_refresh_error.read().await.clone() {
            Some(error) => Err(Error::CoalescedRefresh(error)),
            None => Ok(()),
        }
    }

    async fn coalesced_refresh_error(&self) -> Error {
        self.last_refresh_error.read().await.clone().map_or_else(
            || Error::CoalescedRefresh("concurrent refresh failed".to_string()),
            Error::CoalescedRefresh,
        )
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
        let timeout_seconds = self.client_config.effective_request_timeout();
        let request = async {
            let response = self.execute_request(client).await?;
            self.parse_response(response).await
        };

        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                #[allow(clippy::cast_possible_truncation)]
                let timeout_millis = std::time::Duration::from_secs(timeout_seconds)
                    .as_millis()
                    .min(u128::from(u32::MAX)) as u32;
                let request = request.fuse();
                let timeout = gloo_timers::future::TimeoutFuture::new(timeout_millis).fuse();
                futures::pin_mut!(request, timeout);
                let config = match futures::future::select(request, timeout).await {
                    Either::Left((result, _)) => result?,
                    Either::Right(((), _)) => {
                        return Err(Error::Timeout { seconds: timeout_seconds });
                    }
                };
            } else {
                let config = tokio::time::timeout(
                    std::time::Duration::from_secs(timeout_seconds),
                    request,
                )
                .await
                .map_err(|_| Error::Timeout { seconds: timeout_seconds })??;
            }
        }
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
    /// * `listener` - The event listener to register. Native callbacks must be
    ///   `Send + Sync`; WASM callbacks run on the local JavaScript thread.
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
    use crate::{
        TempDir,
        client_config::ClientConfig,
        test_support::{MockHttpsServer as TestHttpServer, MockResponse},
    };
    #[cfg(not(target_arch = "wasm32"))]
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering as AtomicOrdering},
        },
        time::Duration,
    };

    #[cfg(not(target_arch = "wasm32"))]
    fn test_config(server: &TestHttpServer, cache_dir: &std::path::Path) -> ClientConfig {
        ClientConfig::builder("test-app", server.url())
            .cache_dir(cache_dir.to_string_lossy())
            .allow_insecure_https(true)
            .build()
            .unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fixed_server(status: u16, body: &'static str) -> TestHttpServer {
        TestHttpServer::new(Arc::new(move |_, _| MockResponse::json(status, body)))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn test_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_concurrent_get_value() {
        setup();
        let temp_dir = TempDir::new("apollo_concurrent_get_test");
        let server = fixed_server(200, r#"{"value":"shared"}"#);
        let cache = Arc::new(Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            test_http_client(),
        ));

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
        let cache = Cache::new(base.clone(), "../public.properties", test_http_client());
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
            let other = Cache::new(other, "../public.properties", test_http_client());
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
            test_http_client(),
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
            test_http_client(),
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
        let cache = Cache::new(config, "application", test_http_client());
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
        server.wait_for_requests(1).await;
        tokio::time::timeout(Duration::from_secs(1), async {
            while errors.load(AtomicOrdering::Acquire) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("stale revalidation error was not reported");
        assert_eq!(errors.load(AtomicOrdering::Acquire), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn cancelled_initial_load_releases_single_flight() {
        let server = TestHttpServer::new(Arc::new(|_, _| {
            let mut response = MockResponse::json(200, r#"{"value":"loaded"}"#);
            response.header_delay = Duration::from_millis(250);
            response
        }));
        let temp_dir = TempDir::new("cancelled_initial_load");
        let cache = Arc::new(Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            test_http_client(),
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
        assert!(server.request_count() >= 2);
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
            let mut response = MockResponse::json(200, body);
            response.header_delay = delay;
            response
        }));
        let temp_dir = TempDir::new("readers_not_blocked_by_refresh");
        let cache = Arc::new(Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            test_http_client(),
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
        server.wait_for_requests(3).await;
        assert_eq!(server.request_count(), 3);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn listeners_receive_changes_and_errors_but_not_unchanged_values() {
        let server = TestHttpServer::new(Arc::new(|index, _| {
            if index < 3 {
                MockResponse::json(200, r#"{"value":"same"}"#)
            } else {
                MockResponse::json(429, "rate limited")
            }
        }));
        let temp_dir = TempDir::new("listener_change_semantics");
        let cache = Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            test_http_client(),
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
        let cache = Cache::new(config, "application", test_http_client());
        cache.get_value().await.unwrap();
        let request = server.captured_requests().first().unwrap().to_lowercase();
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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn stale_reads_are_immediate_and_schedule_one_revalidation() {
        let server = TestHttpServer::new(Arc::new(|index, _| {
            if index == 1 {
                MockResponse::json(200, r#"{"value":"cached"}"#)
            } else {
                let mut response = MockResponse::json(503, "unavailable");
                response.header_delay = Duration::from_millis(250);
                response
            }
        }));
        let temp_dir = TempDir::new("stale_while_revalidate");
        let mut config = test_config(&server, temp_dir.path());
        config.cache_ttl = Some(1);
        let cache = Arc::new(Cache::new(config, "application", test_http_client()));
        assert_eq!(cache.get_value().await.unwrap()["value"], "cached");
        cache.memory.write().await.as_mut().unwrap().timestamp -= 60;

        let started = std::time::Instant::now();
        let readers = (0..32).map(|_| {
            let cache = cache.clone();
            tokio::spawn(async move { cache.get_value().await.unwrap() })
        });
        for result in futures::future::join_all(readers).await {
            assert_eq!(result.unwrap()["value"], "cached");
        }
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "stale readers waited for revalidation"
        );
        server.wait_for_requests(2).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(server.request_count(), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn concurrent_refresh_callers_share_one_request() {
        let server = TestHttpServer::new(Arc::new(|_, _| {
            let mut response = MockResponse::json(200, r#"{"value":"fresh"}"#);
            response.header_delay = Duration::from_millis(150);
            response
        }));
        let temp_dir = TempDir::new("coalesced_refresh");
        let cache = Arc::new(Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            test_http_client(),
        ));
        let callers = (0..16).map(|_| {
            let cache = cache.clone();
            tokio::spawn(async move { cache.refresh().await })
        });
        for result in futures::future::join_all(callers).await {
            result.unwrap().unwrap();
        }
        assert_eq!(server.request_count(), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn zero_ttl_serves_cached_data_and_revalidates() {
        let server = TestHttpServer::new(Arc::new(|index, _| {
            MockResponse::json(200, format!(r#"{{"request":{index}}}"#))
        }));
        let temp_dir = TempDir::new("zero_ttl");
        let mut config = test_config(&server, temp_dir.path());
        config.cache_ttl = Some(0);
        let cache = Cache::new(config, "application", test_http_client());

        assert_eq!(cache.get_value().await.unwrap()["request"], 1);
        assert_eq!(cache.get_value().await.unwrap()["request"], 1);
        server.wait_for_requests(2).await;
        tokio::time::timeout(Duration::from_secs(1), async {
            while cache.memory.read().await.as_ref().unwrap().config["request"] != 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("zero-TTL revalidation did not update memory");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn read_path_failures_do_not_notify_listeners() {
        let server = fixed_server(500, "unavailable");
        let temp_dir = TempDir::new("read_failure_listener");
        let cache = Cache::new(
            test_config(&server, temp_dir.path()),
            "application",
            test_http_client(),
        );
        let errors = Arc::new(AtomicUsize::new(0));
        let listener_errors = errors.clone();
        cache
            .add_listener(Arc::new(move |result| {
                if result.is_err() {
                    listener_errors.fetch_add(1, AtomicOrdering::AcqRel);
                }
            }))
            .await;

        assert!(cache.get_value().await.is_err());
        assert_eq!(errors.load(AtomicOrdering::Acquire), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn startup_cleanup_removes_only_cache_temp_files() {
        let temp_dir = TempDir::new("temp_cleanup");
        let stale = temp_dir.path().join("v2-entry.cache.json.1.2.tmp");
        let unrelated = temp_dir.path().join("notes.tmp");
        std::fs::write(&stale, b"stale").unwrap();
        std::fs::write(&unrelated, b"keep").unwrap();

        cleanup_stale_temp_files(temp_dir.path());

        assert!(!stale.exists());
        assert!(unrelated.exists());
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
