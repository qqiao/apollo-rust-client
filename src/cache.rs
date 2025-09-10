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
//! - **WebAssembly**: Memory-only caching optimized for browser environments
//!
//! # Examples
//!
//! The cache is typically used internally by the `Client` struct and not directly
//! by end users. However, understanding its behavior is important for debugging
//! and performance optimization.

use crate::{
    client_config::ClientConfig,
    namespace::{self, get_namespace},
    EventListener,
};
use async_std::sync::RwLock;
use base64::display::Base64Display;
use cfg_if::cfg_if;
use chrono::Utc;
use hmac::{Hmac, Mac};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::Sha1;
use std::{fmt::Write, sync::Arc};
use url::{ParseError, Url};

#[derive(Serialize, Deserialize)]
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
/// - **Concurrency Errors**: Race conditions during cache operations
///
/// # Examples
///
/// ```rust,ignore
/// use apollo_rust_client::cache::Error;
///
/// // Example of handling different cache error types
/// fn handle_cache_error(error: Error) {
///     match error {
///         Error::Io(io_error) => {
///             // Handle file system errors
///             eprintln!("I/O error: {}", io_error);
///         }
///         Error::Reqwest(reqwest_error) => {
///             // Handle network errors
///             eprintln!("Network error: {}", reqwest_error);
///         }
///         Error::Serde(serde_error) => {
///             // Handle JSON parsing errors
///             eprintln!("JSON error: {}", serde_error);
///         }
///         Error::Namespace(namespace_error) => {
///             // Handle namespace errors
///             eprintln!("Namespace error: {}", namespace_error);
///         }
///         Error::NamespaceNotFound(namespace) => {
///             // Handle namespace not found
///             eprintln!("Namespace not found: {}", namespace);
///         }
///         Error::UrlParse(url_error) => {
///             // Handle URL parsing errors
///             eprintln!("URL parse error: {}", url_error);
///         }
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

    /// The requested namespace was not found.
    ///
    /// This error occurs when attempting to access a namespace that doesn't
    /// exist in the cache or when the cache has not been properly initialized.
    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),

    /// A network request error occurred.
    ///
    /// This error occurs when there are issues with HTTP requests to the
    /// Apollo server, such as connection failures, timeouts, or invalid responses.
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    /// A URL parsing error occurred.
    ///
    /// This error occurs when the constructed URL for the Apollo server
    /// is malformed or cannot be parsed.
    #[error("Url parse error: {0}")]
    UrlParse(#[from] url::ParseError),
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
/// 1. **Memory Cache**: Fast in-memory storage using `Arc<RwLock<Option<Value>>>`
/// 2. **File Cache** (native only): Persistent JSON files for offline access
/// 3. **Remote Source**: Apollo Configuration Center via HTTP/HTTPS
///
/// # Concurrency Control
///
/// The cache uses several locks to ensure thread safety:
/// - `loading`: Prevents concurrent refresh operations
/// - `checking_cache`: Prevents concurrent cache initialization
/// - `memory_cache`: Protects in-memory configuration data
/// - `listeners`: Protects event listener collection
///
/// # Platform Differences
///
/// - **Native Rust**: Full feature set with file caching and background refresh
/// - **WebAssembly**: Memory-only caching optimized for single-threaded execution
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
    memory: Arc<RwLock<Option<Value>>>,

    /// Collection of event listeners for configuration change notifications.
    ///
    /// Listeners are called when the cache is refreshed, allowing applications
    /// to react to configuration changes in real-time.
    listeners: Arc<RwLock<Vec<EventListener>>>,

    /// Path to the local cache file (native targets only).
    ///
    /// On native targets, this specifies where the configuration should be
    /// cached locally. The path includes the namespace name and any grayscale
    /// targeting parameters (IP, labels) to ensure cache isolation.
    #[cfg(not(target_arch = "wasm32"))]
    file_path: std::path::PathBuf,
}

impl Cache {
    /// Create a new cache.
    ///
    /// # Arguments
    ///
    /// * `client_config` - The configuration for the Apollo client.
    /// * `namespace` - The namespace to get the cache for.
    ///
    /// # Returns
    ///
    /// A new cache for the given namespace.
    pub(crate) fn new(client_config: ClientConfig, namespace: &str) -> Self {
        let mut file_name = namespace.to_string();
        if let Some(ip) = &client_config.ip {
            let _ = write!(file_name, "_{ip}");
        }
        if let Some(label) = &client_config.label {
            let _ = write!(file_name, "_{label}");
        }

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let file_path = client_config
                    .get_cache_dir()
                    .join(format!("{file_name}.cache.json"));
            }
        }

        Self {
            client_config,
            namespace: namespace.to_string(),
            memory: Arc::new(RwLock::new(None)),
            listeners: Arc::new(RwLock::new(Vec::new())),

            #[cfg(not(target_arch = "wasm32"))]
            file_path,
        }
    }

    /// Get a configuration from the cache.
    ///
    /// This method attempts to retrieve a configuration value associated with the given `key`.
    /// The process involves several steps:
    ///
    /// 1.  **In-Memory Cache Check**: It first checks an in-memory cache (`self.memory_cache`).
    ///     If the value is found, it's returned immediately. `self.memory_cache` is protected by an `RwLock`
    ///     to allow concurrent reads and exclusive writes.
    ///
    /// 2.  **File-Based Cache Check (non-wasm32 targets only)**: If the value is not in the memory cache
    ///     and the target architecture is not wasm32, it attempts to load the cache from a local file.
    ///     The file path is determined by the `client_config` and namespace. If the file exists, its
    ///     contents are parsed, and the in-memory cache is updated.
    ///
    /// 3.  **Refresh Operation**: If the value is not found in either the in-memory cache or the
    ///     file-based cache (or if on wasm32 where file cache is not used), a `self.refresh()`
    ///     operation is triggered to fetch the latest configuration from the Apollo server.
    ///     The in-memory cache (and file cache on non-wasm32) will be updated by the `refresh` method.
    ///
    /// To prevent multiple concurrent attempts to initialize or check the cache from the file system
    /// or via refresh, this method uses an `RwLock` named `self.checking_cache`.
    /// If another task is already performing this check/initialization, the current task will return
    /// `Err(Error::AlreadyCheckingCache)`. This indicates that a cache lookup or population is
    /// already in progress, and the caller should typically retry shortly.
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The configuration value if successfully retrieved
    /// * `Err(Error::AlreadyCheckingCache)` - If another cache operation is in progress
    /// * `Err(Error::NamespaceNotFound)` - If the namespace cannot be found or initialized
    /// * `Err(Error::Io)` - If file system operations fail (native targets only)
    /// * `Err(Error::Serde)` - If cache file parsing fails (native targets only)
    /// * `Err(Error::Reqwest)` - If network requests fail during refresh
    /// * `Err(Error::UrlParse)` - If URL construction fails
    /// * `Err(Error::Namespace)` - If namespace processing fails
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Another cache operation is already in progress
    /// - The namespace cannot be found or initialized
    /// - File system operations fail (native targets only)
    /// - Cache file parsing fails (native targets only)
    /// - Network requests fail during refresh
    /// - URL construction fails
    /// - Namespace processing fails
    pub(crate) async fn get_value(&self) -> Result<Value, Error> {
        if let Some(value) = self.memory.read().await.as_ref() {
            return Ok(value.clone());
        }

        let (config, listeners) = {
            let mut w_lock = self.memory.write().await;
            if let Some(value) = w_lock.as_ref() {
                return Ok(value.clone());
            }

            cfg_if! {
                if #[cfg(not(target_arch = "wasm32"))] {
                    let file_path = self.file_path.clone();
                    if file_path.exists() {
                        if let Ok(file) = std::fs::File::open(&file_path) {
                            if let Ok(cache_item) = serde_json::from_reader::<_, CacheItem>(file) {
                                let mut is_stale = false;
                                if let Some(ttl) = self.client_config.cache_ttl {
                                    let age = Utc::now().timestamp() - cache_item.timestamp;
                                    #[allow(clippy::cast_possible_wrap)]
                                    if age > ttl as i64 {
                                        is_stale = true;
                                    }
                                }

                                if !is_stale {
                                    w_lock.replace(cache_item.config.clone());
                                    return Ok(cache_item.config);
                                }
                            }
                        }
                    }
                }
            }

            let config = self.fetch_remote_config().await?;
            w_lock.replace(config.clone());
            let listeners = self.listeners.read().await.clone();
            (config, listeners)
        };

        self.notify_listeners(&config, &listeners);
        Ok(config)
    }

    /// Refreshes the cache by fetching the latest configuration from the Apollo server.
    ///
    /// This method performs the following steps to update the cache for the current namespace:
    ///
    /// 1.  **Construct URL**: It constructs the request URL for the Apollo configuration service
    ///     based on `client_config` (server address, app ID, cluster) and the current namespace.
    /// 2.  **Add Query Parameters**: If `client_config.ip` or `client_config.label` are set,
    ///     they are added as query parameters (`ip` and `label` respectively) to the request URL.
    ///     This is often used for grayscale release rules.
    /// 3.  **Authentication Headers (if secret is present)**:
    ///     *   If `client_config.secret` is provided, it generates a signature using the `sign` function.
    ///     *   The current `timestamp` (in milliseconds) and an `Authorization` header
    ///         (e.g., `Apollo <app_id>:<signature>`) are added to the HTTP request.
    /// 4.  **Send HTTP GET Request**: It sends an HTTP GET request to the constructed URL.
    /// 5.  **Update Caches**:
    ///     *   On a successful response, the response body (JSON configuration) is parsed.
    ///     *   For non-wasm32 targets, the fetched configuration is written to a local file cache
    ///         (path determined by `client_config.cache_dir` and namespace details).
    ///     *   The in-memory cache (`self.memory_cache`) is updated with the new configuration.
    ///     *   Registered listeners are notified with the new configuration (`Ok(config)`)
    ///         Currently, listeners are only notified on successful refresh.
    ///
    /// To prevent multiple concurrent refresh operations for the same cache instance, this method
    /// uses an `RwLock` named `self.loading`. If another task is already refreshing this cache,
    /// the current task will return `Err(Error::AlreadyLoading)`. This indicates that a refresh
    /// is already in progress, and the caller should typically wait for the ongoing refresh to
    /// complete rather than initiating a new one.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the cache was successfully refreshed
    /// * `Err(Error::AlreadyLoading)` - If another refresh operation is already in progress
    /// * `Err(Error::UrlParse)` - If URL construction fails
    /// * `Err(Error::Reqwest)` - If the HTTP request fails
    /// * `Err(Error::Serde)` - If the response body cannot be parsed as JSON
    /// * `Err(Error::Io)` - If file cache operations fail (native targets only)
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Another refresh operation is already in progress
    /// - URL construction fails
    /// - HTTP request fails (network issues, server errors, etc.)
    /// - Response body cannot be parsed as JSON
    /// - File cache operations fail (native targets only)
    /// - Authentication signature generation fails
    pub(crate) async fn refresh(&self) -> Result<(), Error> {
        let (config, listeners) = {
            let mut w_lock = self.memory.write().await;
            let config = self.fetch_remote_config().await?;
            w_lock.replace(config.clone());
            let listeners = self.listeners.read().await.clone();
            (config, listeners)
        };
        self.notify_listeners(&config, &listeners);
        Ok(())
    }

    fn notify_listeners(&self, config: &Value, listeners: &[EventListener]) {
        for listener in listeners {
            listener(
                get_namespace(&self.namespace, config.clone()).map_err(crate::Error::Namespace),
            );
        }
    }

    async fn fetch_remote_config(&self) -> Result<Value, Error> {
        let url = self.build_request_url()?;
        let http_client = self.create_http_client();
        let client = self.build_http_request(&url, &http_client)?;
        let response = self.execute_request(client).await?;
        let config = self.parse_response(response).await?;

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                self.write_to_file_cache(&config)?;
            }
        }

        Ok(config)
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
        let url = format!(
            "{}/configfiles/json/{}/{}/{}",
            self.client_config.config_server,
            self.client_config.app_id,
            self.client_config.cluster,
            self.namespace
        );

        let mut url = match Url::parse(&url) {
            Ok(u) => u,
            Err(e) => return Err(Error::UrlParse(e)),
        };

        if let Some(ip) = &self.client_config.ip {
            url.query_pairs_mut().append_pair("ip", ip);
        }
        if let Some(label) = &self.client_config.label {
            url.query_pairs_mut().append_pair("label", label);
        }

        Ok(url)
    }

    /// Creates an HTTP client with optional insecure HTTPS support.
    ///
    /// For native targets, allows insecure HTTPS if configured.
    /// For WASM targets, always uses the default client.
    ///
    /// # Returns
    ///
    /// * `reqwest::Client` - The configured HTTP client
    fn create_http_client(&self) -> reqwest::Client {
        if self.client_config.allow_insecure_https.unwrap_or(false) {
            cfg_if! {
                if #[cfg(not(target_arch = "wasm32"))] {
                    reqwest::Client::builder()
                        .danger_accept_invalid_certs(true)
                        .danger_accept_invalid_hostnames(true)
                        .build()
                        .unwrap_or_else(|_| reqwest::Client::new())
                } else {
                    // WASM target doesn't support these methods, use default client
                    reqwest::Client::new()
                }
            }
        } else {
            reqwest::Client::new()
        }
    }

    /// Builds the HTTP request with optional authentication headers.
    ///
    /// If a secret is configured, adds timestamp and authorization headers
    /// with HMAC-SHA1 signature.
    ///
    /// # Arguments
    ///
    /// * `url` - The request URL
    /// * `http_client` - The HTTP client to use
    ///
    /// # Returns
    ///
    /// * `Ok(reqwest::RequestBuilder)` - The configured request builder
    /// * `Err(Error)` - If signature generation fails
    fn build_http_request(
        &self,
        url: &Url,
        http_client: &reqwest::Client,
    ) -> Result<reqwest::RequestBuilder, Error> {
        let mut client = http_client.get(url.as_str());

        if let Some(secret) = &self.client_config.secret {
            let timestamp = Utc::now().timestamp_millis();
            let signature = sign(timestamp, url.as_str(), secret)?;
            client = client.header("timestamp", timestamp.to_string());
            client = client.header(
                "Authorization",
                format!("Apollo {}:{}", &self.client_config.app_id, signature),
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
        let body: String = match response.text().await {
            Ok(b) => b,
            Err(e) => return Err(Error::Reqwest(e)),
        };

        trace!("Response body {} for namespace {}", body, self.namespace);

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
    /// * `config` - The configuration to cache
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If caching succeeds
    /// * `Err(Error::Io)` - If file operations fail
    /// * `Err(Error::Serde)` - If serialization fails
    #[cfg(not(target_arch = "wasm32"))]
    fn write_to_file_cache(&self, config: &Value) -> Result<(), Error> {
        debug!("writing cache file {}", self.file_path.display());

        // Create parent directories if they don't exist
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let cache_item = CacheItem {
            timestamp: Utc::now().timestamp(),
            config: config.clone(),
        };

        let cache_content = serde_json::to_string(&cache_item)?;

        std::fs::write(&self.file_path, cache_content)?;
        trace!(
            "Wrote cache file {} for namespace {}",
            self.file_path.display(),
            self.namespace
        );

        Ok(())
    }

    /// Adds an event listener to the cache.
    ///
    /// Listeners are closures that will be called when the cache is successfully refreshed.
    /// The listener will receive a `Result<Value, Error>`, which will be `Ok(new_config)`
    /// containing the newly fetched configuration.
    ///
    ///
    /// The listener is a callback function that conforms to the [`EventListener`] type alias:
    /// `Arc<dyn Fn(Result<Value, Error>) + Send + Sync>`.
    ///
    /// - `Value` is `serde_json::Value` representing the full configuration for the namespace.
    /// - `Error` is `crate::cache::Error` indicating a failure during the refresh process.
    ///
    /// Listeners are called when the cache is successfully refreshed with new configuration,
    /// or when an error occurs during a refresh attempt.
    ///
    /// # Arguments
    ///
    /// * `listener` - The event listener to register. It must be an `Arc`-wrapped, thread-safe
    ///   closure (`Send + Sync`).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use serde_json::Value;
    /// use apollo_rust_client::cache::{Cache, Error, EventListener};
    /// // Assuming `cache` is an existing Arc<Cache> instance
    ///
    /// let listener: EventListener = Arc::new(|result: Result<Value, Error>| {
    ///     match result {
    ///         Ok(config) => println!("Cache refreshed, new config: {:?}", config),
    ///         Err(e) => println!("Cache refresh failed: {:?}", e),
    ///     }
    /// });
    /// cache.add_listener(listener).await;
    /// ```
    pub async fn add_listener(&self, listener: EventListener) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
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
/// * `Err(Error::UrlParse)` - If URL parsing fails during signature generation
///
/// # Errors
///
/// This function will return an error if:
/// - The URL cannot be parsed (including relative URLs without a base)
/// - The URL structure is malformed
///
/// # Examples
///
/// ```rust,ignore
/// use apollo_rust_client::sign;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let signature = sign(1576478257344, "/configs/100004458/default/application?ip=10.0.0.1", "secret_key")?;
///     println!("Generated signature: {}", signature);
///     Ok(())
/// }
/// ```
pub(crate) fn sign(timestamp: i64, url: &str, secret: &str) -> Result<String, Error> {
    let u = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => match e {
            ParseError::RelativeUrlWithoutBase => {
                let base_url = Url::parse("http://localhost:8080").unwrap();
                base_url.join(url).unwrap()
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

    let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    let result: [u8; 20] = mac.finalize().into_bytes().into();

    // Convert the result to a string using base64
    let code = Base64Display::new(&result, &base64::engine::general_purpose::STANDARD);
    Ok(code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{client_config::ClientConfig, setup, TempDir};
    use std::sync::Arc;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_concurrent_get_value() {
        setup();
        let temp_dir = TempDir::new("apollo_concurrent_get_test");

        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            secret: None,
            cache_dir: Some(temp_dir.path().to_str().unwrap().to_string()),
            label: None,
            ip: None,
            allow_insecure_https: None,
            #[cfg(not(target_arch = "wasm32"))]
            cache_ttl: None,
        };

        let cache = Arc::new(Cache::new(config, "application"));

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
