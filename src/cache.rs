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
    EventListener,
    client_config::ClientConfig,
    namespace::{self, get_namespace},
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
use std::sync::Arc;
use url::{ParseError, Url};

#[derive(Serialize, Deserialize)]
struct CacheItem {
    timestamp: i64,
    config: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Namespace error: {0}")]
    Namespace(namespace::Error),

    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Url parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Already loading")]
    AlreadyLoading,
    #[error("Already checking cache")]
    AlreadyCheckingCache,
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

    /// Lock to prevent concurrent refresh operations.
    ///
    /// When a refresh is in progress, other refresh attempts will return
    /// `Error::AlreadyLoading` to prevent redundant network requests.
    loading: Arc<RwLock<bool>>,

    /// Lock to prevent concurrent cache initialization (native targets only).
    ///
    /// This prevents multiple threads from simultaneously attempting to
    /// read and parse the same cache file during initialization.
    checking_cache: Arc<RwLock<bool>>,

    /// In-memory storage for the parsed configuration data.
    ///
    /// Contains the JSON representation of the configuration. `None` indicates
    /// that the cache has not been populated or a fetch operation failed.
    memory_cache: Arc<RwLock<Option<Value>>>,

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
            file_name.push_str(&format!("_{ip}"));
        }
        if let Some(label) = &client_config.label {
            file_name.push_str(&format!("_{label}"));
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

            loading: Arc::new(RwLock::new(false)),
            checking_cache: Arc::new(RwLock::new(false)),
            memory_cache: Arc::new(RwLock::new(None)),
            listeners: Arc::new(RwLock::new(Vec::new())),

            #[cfg(not(target_arch = "wasm32"))]
            file_path,
        }
    }

    /// Get a configuration from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the configuration for.
    ///
    /// # Returns
    ///
    /// The configuration for the given key.
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
    pub(crate) async fn get_value(&self) -> Result<Value, Error> {
        debug!("Getting value for namesapce {}", self.namespace);

        // Check if the cache file exists
        let mut checking_cache = self.checking_cache.write().await;
        if *checking_cache {
            return Err(Error::AlreadyCheckingCache);
        }
        *checking_cache = true;

        // First we check memory cache
        let memory_cache = self.memory_cache.read().await;
        if let Some(value) = memory_cache.as_ref() {
            debug!(
                "Memory cache found, using memory cache for namespace {}",
                self.namespace
            );
            *checking_cache = false;
            return Ok(value.clone());
        }
        drop(memory_cache);
        debug!("No memory cache found");

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let mut needs_refresh = true;
                let file_path = self.file_path.clone();

                if file_path.exists() {
                    trace!("Cache file {} exists, attempting to use it.", file_path.display());
                    if let Ok(file) = std::fs::File::open(&file_path) {
                        if let Ok(cache_item) = serde_json::from_reader::<_, CacheItem>(file) {
                            let mut is_stale = false;
                            cfg_if::cfg_if! {
                                if #[cfg(not(target_arch = "wasm32"))] {
                                    if let Some(ttl) = self.client_config.cache_ttl {
                                        let age = Utc::now().timestamp() - cache_item.timestamp;
                                        if age > ttl as i64 {
                                            is_stale = true;
                                            trace!("Cache for {} is stale.", self.namespace);
                                        }
                                    }
                                }
                            }

                            if !is_stale {
                                trace!("Cache for {} is fresh, using it.", self.namespace);
                                self.memory_cache.write().await.replace(cache_item.config);
                                needs_refresh = false;
                            }
                        } else {
                            trace!("Failed to parse cache file, will refresh.");
                        }
                    } else {
                        trace!("Failed to open cache file, will refresh.");
                    }
                }

                if needs_refresh {
                    trace!("Refreshing cache for namespace {}", self.namespace);
                    match self.refresh().await {
                        Ok(_) => (),
                        Err(e) => {
                            *checking_cache = false;
                            return Err(e);
                        }
                    }
                }
            } else {
                match self.refresh().await {
                    Ok(_) => (),
                    Err(e) => {
                        *checking_cache = false;
                        return Err(e);
                    }
                }
            }
        }

        let memory_cache = self.memory_cache.read().await;
        if let Some(value) = memory_cache.as_ref() {
            *checking_cache = false;
            Ok(value.clone())
        } else {
            *checking_cache = false;
            Err(Error::NamespaceNotFound(self.namespace.clone()))
        }
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
    pub(crate) async fn refresh(&self) -> Result<(), Error> {
        let mut loading = self.loading.write().await;
        if *loading {
            return Err(Error::AlreadyLoading);
        }
        *loading = true;

        trace!("Refreshing cache for namespace {}", self.namespace);
        let url = format!(
            "{}/configfiles/json/{}/{}/{}",
            self.client_config.config_server,
            self.client_config.app_id,
            self.client_config.cluster,
            self.namespace
        );

        let mut url = match Url::parse(&url) {
            Ok(u) => u,
            Err(e) => {
                *loading = false;
                return Err(Error::UrlParse(e));
            }
        };
        if let Some(ip) = &self.client_config.ip {
            url.query_pairs_mut().append_pair("ip", ip);
        }
        if let Some(label) = &self.client_config.label {
            url.query_pairs_mut().append_pair("label", label);
        }

        trace!("Url {} for namespace {}", url, self.namespace);

        // Create HTTP client with optional insecure HTTPS support
        let http_client = if self.client_config.allow_insecure_https.unwrap_or(false) {
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
        };

        let mut client = http_client.get(url.as_str());
        if self.client_config.secret.is_some() {
            let timestamp = Utc::now().timestamp_millis();
            let signature = sign(
                timestamp,
                url.as_str(),
                self.client_config.secret.as_ref().unwrap(),
            )?;
            client = client.header("timestamp", timestamp.to_string());
            client = client.header(
                "Authorization",
                format!("Apollo {}:{}", &self.client_config.app_id, signature),
            );
        }

        let response = match client.send().await {
            Ok(r) => r,
            Err(e) => {
                *loading = false;
                return Err(Error::Reqwest(e));
            }
        };

        let body: String = match response.text().await {
            Ok(b) => b,
            Err(e) => {
                *loading = false;
                return Err(Error::Reqwest(e));
            }
        };

        trace!("Response body {} for namespace {}", body, self.namespace);

        let config: Value = match serde_json::from_str(&body) {
            Ok(c) => c,
            Err(e) => {
                *loading = false;
                debug!("error parsing config: {e}");
                return Err(Error::Serde(e));
            }
        };
        trace!("parsed config: {config:?}");

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                debug!("writing cache file {}", self.file_path.display());
                 // Create parent directories if they don't exist
                if let Some(parent) = self.file_path.parent() {
                    match std::fs::create_dir_all(parent) {
                        Ok(_) => (),
                        Err(e) => {
                            *loading = false;
                            return Err(Error::Io(e));
                        }
                    }
                }

                let cache_item = CacheItem {
                    timestamp: Utc::now().timestamp(),
                    config: config.clone(),
                };

                let cache_content = match serde_json::to_string(&cache_item) {
                    Ok(c) => c,
                    Err(e) => {
                        *loading = false;
                        return Err(Error::Serde(e));
                    }
                };

                match std::fs::write(&self.file_path, cache_content) {
                    Ok(_) => {
                        trace!("Wrote cache file {} for namespace {}", self.file_path.display(), self.namespace);
                    },
                    Err(e) => {
                        *loading = false;
                        return Err(Error::Io(e));
                    }
                }
            }
        }

        // Notify listeners with the new config
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(
                get_namespace(&self.namespace, config.clone()).map_err(crate::Error::Namespace),
            );
        }
        drop(listeners); // Release read lock before acquiring write lock

        self.memory_cache.write().await.replace(config);

        trace!("Refreshed cache for namespace {}", self.namespace);
        *loading = false;

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
/// A `Result` which is:
/// * `Ok(String)`: A Base64 encoded string representing the signature.
/// * `Err(Error)`: An error if URL parsing fails.
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
        path_and_query.push_str(&format!("?{query}"));
    }
    let input = format!("{timestamp}\n{path_and_query}");
    trace!("input for signing: {input}");

    type HmacSha1 = Hmac<Sha1>;
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
    use crate::tests::setup; // Optional: for logging from JS side if needed directly, or if cache refresh panics

    #[test]
    fn test_sign_with_path() {
        let url = "/configs/100004458/default/application?ip=10.0.0.1";
        let secret = "df23df3f59884980844ff3dada30fa97";
        let signature = sign(1576478257344, url, secret).unwrap();
        assert_eq!(signature, "EoKyziXvKqzHgwx+ijDJwgVTDgE=");
    }

    #[test]
    fn test_sign_url() {
        setup();
        let url = "http://localhost:8080/configs/100004458/default/application?ip=10.0.0.1";
        let secret = "df23df3f59884980844ff3dada30fa97";
        let signature = sign(1576478257344, url, secret).unwrap();
        assert_eq!(signature, "EoKyziXvKqzHgwx+ijDJwgVTDgE=");
    }
}
