#![cfg_attr(target_arch = "wasm32", allow(clippy::arc_with_non_send_sync))]

//! # Apollo Rust Client
//!
//! A robust Rust client for the Apollo Configuration Centre, with support for WebAssembly
//! for browser and Node.js environments.
//!
//! This crate provides a comprehensive client for interacting with Apollo configuration services.
//! The client manages configurations for different namespaces, supports multiple configuration
//! formats (Properties, JSON, YAML, Text), provides caching mechanisms, and offers periodic updates
//! through background polling and event listeners.
//!
//! ## Key Features
//!
//! - **Multiple Configuration Formats**: Support for Properties, JSON, Text formats with automatic detection
//! - **Cross-Platform**: Native Rust and WebAssembly targets with platform-specific optimizations
//! - **Periodic Updates**: Background polling with configurable intervals and event listeners
//! - **Comprehensive Caching**: Multi-level caching with file persistence (native) and persistent localStorage caching with high-performance Node.js in-memory fallback (WASM)
//! - **Type-Safe API**: Compile-time guarantees and runtime type conversion
//! - **Error Handling**: Detailed error diagnostics with comprehensive error types
//! - **Grayscale Release Support**: IP and label-based configuration targeting
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use apollo_rust_client::{Client, client_config::ClientConfig};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ClientConfig::builder("my-app", "http://apollo-server:8080").build()?;
//! let mut client = Client::new(config)?;
//! client.start().await?;
//!
//! let namespace = client.namespace("application").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Platform Support
//!
//! The library supports different behavior for wasm32 and non-wasm32 targets:
//!
//! - **Native Rust**: Full feature set with file caching, background tasks, and threading
//! - **WebAssembly**: Persistent localStorage caching with high-performance Node.js in-memory fallback, single-threaded execution, JavaScript interop

use crate::namespace::Namespace;
use cache::Cache;
use client_config::ClientConfig;
use futures::{StreamExt, stream};
use log::{error, trace};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::RwLock;
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(all(
    feature = "native-tls",
    feature = "rustls",
    not(target_arch = "wasm32")
))]
compile_error!(
    "Features 'native-tls' and 'rustls' are mutually exclusive on non-WASM targets. \
    Please disable default features and enable only one."
);

#[cfg(all(feature = "rustls", target_arch = "wasm32"))]
compile_error!(
    "Feature 'rustls' is not supported on WASM targets. Only native-tls (browser) is supported."
);

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use tokio::spawn as spawn;
    }
}

mod cache;

pub mod client_config;
pub mod namespace;

/// Comprehensive error types that can occur when using the Apollo client.
///
/// This enum covers all possible error conditions that may arise during client operations,
/// from initialization and configuration to runtime cache operations and namespace handling.
///
/// # Error Categories
///
/// - **Client State Errors**: Issues related to client lifecycle management
/// - **Namespace Errors**: Problems with namespace format detection and processing
/// - **Cache Errors**: Network, I/O, and caching-related failures
///
/// # Examples
///
/// ```rust,no_run
/// use apollo_rust_client::{Client, Error};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// # let config = apollo_rust_client::client_config::ClientConfig::builder(
/// #     "test", "http://localhost:8080"
/// # ).build().unwrap();
/// # let client = Client::new(config).unwrap();
/// match client.namespace("application").await {
///     Ok(namespace) => {
///         // Handle successful namespace retrieval
///     }
///     Err(Error::Cache(cache_error)) => {
///         // Handle cache-related errors (network, parsing, etc.)
///         eprintln!("Cache error: {}", cache_error);
///     }
///     Err(Error::Namespace(namespace_error)) => {
///         // Handle namespace-related errors (format detection, etc.)
///         eprintln!("Namespace error: {}", namespace_error);
///     }
///     Err(e) => {
///         // Handle other errors
///         eprintln!("Error: {}", e);
///     }
/// }
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The client background task is already running.
    ///
    /// This error occurs when attempting to start a client that is already
    /// running its background refresh task. Each client instance can only
    /// have one active background task at a time.
    #[error("Client is already running")]
    AlreadyRunning,

    /// An error occurred during namespace processing.
    ///
    /// This includes errors from format detection, parsing, or type conversion
    /// operations specific to namespace handling.
    #[error("Namespace error: {0}")]
    Namespace(#[from] namespace::Error),

    /// An error occurred during cache operations.
    ///
    /// This encompasses network errors, I/O failures, serialization issues,
    /// and other cache-related problems during configuration retrieval or storage.
    #[error("Cache error: {0}")]
    Cache(#[from] cache::Error),

    /// The supplied client configuration is invalid.
    #[error("Client configuration error: {0}")]
    Config(#[from] client_config::Error),

    /// Construction of the native HTTP client failed.
    #[error("Failed to construct HTTP client: {0}")]
    HttpClient(#[source] reqwest::Error),

    /// A background refresh failed.
    ///
    /// Listener errors are snapshots so each registered listener receives its
    /// own owned error value without erasing the primary operation's error.
    #[error("Background refresh failed: {0}")]
    Refresh(String),
}

impl From<Error> for wasm_bindgen::JsValue {
    fn from(error: Error) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                js_sys::Error::new(&error.to_string()).into()
            } else {
                error.to_string().into()
            }
        }
    }
}

// Type alias for event listeners that can be registered with the cache.
// For WASM targets, listeners don't need to be Send + Sync since WASM is single-threaded.
// Listeners receive the converted `Namespace` or a crate-level refresh error.
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Type alias for event listeners that can be registered with the cache.
        /// For WASM targets, listeners don't need to be Send + Sync since WASM is single-threaded.
        /// Listeners receive `Result<Namespace, Error>`.
        pub type EventListener = Arc<dyn Fn(Result<Namespace, Error>)>;
    } else {
        /// Type alias for event listeners that can be registered with the client.
        /// For native targets, listeners need to be `Send` and `Sync` to be safely
        /// shared across threads.
        ///
        /// Listeners receive `Result<Namespace, Error>`, containing either a fresh
        /// typed namespace or an owned refresh error.
        pub type EventListener = Arc<dyn Fn(Result<Namespace, Error>) + Send + Sync>;
    }
}

/// The main Apollo configuration client.
///
/// This struct provides the primary interface for interacting with Apollo configuration services.
/// It manages multiple namespace caches, handles background refresh tasks, and provides
/// event listener functionality for periodic configuration updates.
///
/// # Features
///
/// - **Namespace Management**: Automatically creates and manages caches for different namespaces
/// - **Background Refresh**: Optional background task that periodically refreshes all namespaces
/// - **Event Listeners**: Support for registering callbacks on configuration changes
/// - **Cross-Platform**: Works on both native Rust and WebAssembly targets
/// - **Thread Safety**: All operations are thread-safe and async-friendly
///
/// # Examples
///
/// ```rust,no_run
/// use apollo_rust_client::{Client, client_config::ClientConfig};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     // Create a client instance
/// #     let config = ClientConfig::builder("test_app", "http://localhost:8080").build()?;
/// #     let client = Client::new(config)?;
/// #
/// #     // Get properties namespace (default format)
/// #     let props_namespace = client.namespace("application").await?;
/// #
/// #     // Get JSON namespace
/// #     let json_namespace = client.namespace("config.json").await?;
/// #
/// #     // Get YAML namespace
/// #     let yaml_namespace = client.namespace("settings.yaml").await?;
/// #
/// #     Ok(())
/// # }
/// ```
#[cfg_attr(target_arch = "wasm32", allow(clippy::arc_with_non_send_sync))]
#[wasm_bindgen]
pub struct Client {
    /// The configuration settings for this Apollo client instance.
    ///
    /// Contains all necessary information to connect to Apollo servers,
    /// including server URL, application ID, cluster, authentication, and caching settings.
    config: ClientConfig,

    /// Thread-safe storage for namespace-specific caches.
    ///
    /// Each namespace gets its own `Cache` instance, wrapped in `Arc` for shared ownership.
    /// The `RwLock` provides thread-safe read/write access to the namespace map.
    /// The outer `Arc` allows the background refresh task to safely access the namespaces.
    namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>,

    /// Handle to the background refresh task (native targets only).
    ///
    /// On native targets, this holds the `JoinHandle` for periodic polling. WASM
    /// stores a separate `AbortHandle` for its `spawn_local` future.
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<tokio::task::JoinHandle<()>>,

    /// Cancellation handle for the WASM refresh future.
    #[cfg(target_arch = "wasm32")]
    abort_handle: Option<futures::future::AbortHandle>,

    /// Flag indicating whether the background refresh task is active.
    ///
    /// Wrapped in `Arc<AtomicBool>` for lock-free coordination between the client
    /// and its background task.
    running: Arc<AtomicBool>,

    /// HTTP client for making network requests.
    ///
    /// Shared across all caches to allow connection pooling and reduce overhead.
    #[allow(clippy::struct_field_names)]
    http_client: reqwest::Client,
}

impl Client {
    /// Get a cache for a given namespace.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to get the cache for.
    ///
    /// # Returns
    ///
    /// A cache for the given namespace.
    pub(crate) async fn cache(&self, namespace: &str) -> Arc<Cache> {
        if let Some(cache) = self.namespaces.read().await.get(namespace).cloned() {
            return cache;
        }

        let mut namespaces = self.namespaces.write().await;
        let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
            trace!("Cache miss, creating cache for namespace {namespace}");
            Arc::new(Cache::new(
                self.config.clone(),
                namespace,
                self.http_client.clone(),
            ))
        });
        cache.clone()
    }

    /// Registers a listener for changes and refresh errors in one namespace.
    ///
    /// Listeners run synchronously, in registration order, after internal locks
    /// are released. A panic in one listener is caught and logged so it cannot
    /// stop cache refresh or prevent later listeners from running.
    pub async fn add_listener(&self, namespace: &str, listener: EventListener) {
        let cache = self.cache(namespace).await;
        cache.add_listener(listener).await;
    }

    /// Retrieves a namespace configuration from the Apollo server.
    ///
    /// This method fetches the configuration for the specified namespace and
    /// automatically detects the format based on the namespace name. The format
    /// detection follows these rules:
    ///
    /// - **Properties format** (default): No file extension
    /// - **JSON format**: `.json` extension
    /// - **YAML format**: `.yaml` or `.yml` extension
    /// - **Text format**: `.txt` extension
    /// - **XML format**: `.xml` extension (not yet supported)
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace identifier string (e.g., "application", "config.json")
    ///
    /// # Returns
    ///
    /// * `Ok(Namespace)` - The configuration data in the appropriate format
    /// * `Err(Error::Cache)` - If cache operations fail (network, I/O, etc.)
    /// * `Err(Error::Namespace)` - If namespace format detection or processing fails
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Network requests to the Apollo server fail
    /// - Cache file operations fail (native targets only)
    /// - JSON parsing fails during configuration retrieval
    /// - Namespace format detection fails
    /// - The requested namespace format is not supported (e.g., XML)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use apollo_rust_client::{Client, client_config::ClientConfig};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     // Create a client instance
    /// #     let config = ClientConfig::builder("test_app", "http://localhost:8080").build()?;
    /// #     let client = Client::new(config)?;
    /// #
    /// #     // Get properties namespace (default format)
    /// #     let props_namespace = client.namespace("application").await?;
    /// #
    /// #     // Get JSON namespace
    /// #     let json_namespace = client.namespace("config.json").await?;
    /// #
    /// #     // Get YAML namespace
    /// #     let yaml_namespace = client.namespace("settings.yaml").await?;
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub async fn namespace(&self, namespace: &str) -> Result<namespace::Namespace, Error> {
        let cache = self.cache(namespace).await;
        let value = cache.get_value().await?;
        Ok(namespace::get_namespace(namespace, value)?)
    }

    /// Starts a background task that periodically refreshes all registered namespace caches.
    ///
    /// This method spawns an asynchronous task using `tokio::spawn` on native targets
    /// or `wasm_bindgen_futures::spawn_local` on wasm32 targets. The task loops indefinitely
    /// (until `stop` is called or the client is dropped) and performs the following actions
    /// in each iteration:
    ///
    /// 1. Copies the managed caches without retaining the namespace-map lock.
    /// 2. Refreshes up to four namespaces concurrently.
    /// 3. Logs errors and applies bounded exponential backoff with jitter.
    /// 4. Sleeps for the configured interval before the next refresh cycle.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the background task was successfully started.
    /// * `Err(Error::AlreadyRunning)` if the background task is already active.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The background task is already running (`Error::AlreadyRunning`)
    /// - Task spawning fails (though this is rare and typically indicates system resource issues)
    fn start_background(&mut self) -> Result<(), Error> {
        if self
            .running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(Error::AlreadyRunning);
        }

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let (abort_handle, abort_registration) = futures::future::AbortHandle::new_pair();
                let namespaces = self.namespaces.clone();
                let running = self.running.clone();
                let refresh_interval = self.config.effective_refresh_interval();
                wasm_bindgen_futures::spawn_local(async move {
                    let task = refresh_loop(namespaces, running, refresh_interval);
                    let _ = futures::future::Abortable::new(task, abort_registration).await;
                });
                self.abort_handle = Some(abort_handle);
            } else {
                let running = self.running.clone();
                let namespaces = self.namespaces.clone();
                let refresh_interval = self.config.effective_refresh_interval();
                let handle = spawn(refresh_loop(namespaces, running, refresh_interval));
                self.handle = Some(handle);
            }
        }

        Ok(())
    }

    /// Starts periodic background polling on native targets.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyRunning`] if polling is already active.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::unused_async)]
    pub async fn start(&mut self) -> Result<(), Error> {
        self.start_background()
    }

    /// Stops the background cache refresh task.
    ///
    /// This method sets the `running` flag to `false`, signaling the background task
    /// to terminate its refresh loop.
    ///
    /// Native and WASM targets both abort their task handle, so shutdown does not
    /// wait for a network request or refresh interval to complete.
    fn stop_background(&mut self) {
        self.running.store(false, Ordering::Release);
        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                if let Some(handle) = self.handle.take() {
                    handle.abort();
                }
            } else {
                if let Some(handle) = self.abort_handle.take() {
                    handle.abort();
                }
            }
        }
    }

    /// Stops native background polling promptly.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::unused_async)]
    pub async fn stop(&mut self) {
        self.stop_background();
    }

    /// Preloads critical namespaces during client initialization to reduce startup delays.
    ///
    /// This method fetches configuration for the specified namespaces in parallel,
    /// which can significantly reduce the perceived startup time when these namespaces
    /// are accessed later.
    ///
    /// # Arguments
    ///
    /// * `namespaces` - A slice of namespace names to preload
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all namespaces were successfully preloaded
    /// * `Err(Error)` if any namespace failed to load
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use apollo_rust_client::{Client, client_config::ClientConfig};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ClientConfig::builder("my-app", "http://apollo-server:8080").build()?;
    /// let client = Client::new(config)?;
    ///
    /// // Preload critical namespaces
    /// client.preload(&["application", "database", "redis"]).await?;
    ///
    /// // Now accessing these namespaces will be much faster
    /// let app_config = client.namespace("application").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Any of the specified namespaces fail to load
    /// - Cache operations fail during preloading
    pub async fn preload(&self, namespaces: &[impl AsRef<str>]) -> Result<(), Error> {
        let futures = namespaces.iter().map(|namespace| async move {
            let cache = self.cache(namespace.as_ref()).await;
            cache.get_value().await.map(|_| ()).map_err(Error::Cache)
        });
        futures::future::try_join_all(futures).await?;
        Ok(())
    }

    /// Forces one namespace to refresh from Apollo.
    ///
    /// Existing values remain readable while the request is in flight and are
    /// retained if the refresh fails.
    ///
    /// # Errors
    ///
    /// Returns transport, HTTP, parsing, or configuration errors from the refresh.
    pub async fn refresh(&self, namespace: &str) -> Result<(), Error> {
        self.cache(namespace).await.refresh().await?;
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.stop_background();
    }
}

async fn refresh_loop(
    namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>,
    running: Arc<AtomicBool>,
    refresh_interval: u64,
) {
    let base_interval = refresh_interval.max(1);
    let mut consecutive_failures = 0_u32;

    while running.load(Ordering::Acquire) {
        let cache_refs: Vec<_> = {
            let namespaces = namespaces.read().await;
            namespaces
                .iter()
                .map(|(name, cache)| (name.clone(), cache.clone()))
                .collect()
        };

        let results = stream::iter(cache_refs)
            .map(|(namespace, cache)| async move {
                let result = cache.refresh().await;
                if let Err(error) = &result {
                    error!("Failed to refresh cache for namespace {namespace}: {error}");
                } else {
                    log::debug!("Successfully refreshed cache for namespace {namespace}");
                }
                result
            })
            .buffer_unordered(4)
            .collect::<Vec<_>>()
            .await;

        if results.iter().any(Result::is_err) {
            consecutive_failures = consecutive_failures.saturating_add(1);
        } else {
            consecutive_failures = 0;
        }

        let factor = 1_u64 << consecutive_failures.min(4);
        let delay = base_interval
            .saturating_mul(factor)
            .min(base_interval.max(300));
        let jitter_window = delay / 10;
        let jitter = if jitter_window == 0 {
            0
        } else {
            chrono::Utc::now().timestamp_millis().unsigned_abs() % (jitter_window + 1)
        };
        platform_sleep(std::time::Duration::from_secs(delay.saturating_add(jitter))).await;
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn platform_sleep(duration: std::time::Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(target_arch = "wasm32")]
async fn platform_sleep(duration: std::time::Duration) {
    #[allow(clippy::cast_possible_truncation)]
    let milliseconds = duration.as_millis().min(u128::from(u32::MAX)) as u32;
    gloo_timers::future::TimeoutFuture::new(milliseconds).await;
}

#[wasm_bindgen]
impl Client {
    /// Create a new Apollo client.
    ///
    /// # Arguments
    ///
    /// * `client_config` - The configuration for the Apollo client.
    ///
    /// # Returns
    ///
    /// A new Apollo client.
    ///
    /// # Errors
    ///
    /// Returns an error when configuration validation or HTTP client construction fails.
    #[wasm_bindgen(constructor)]
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        config.validate()?;
        let http_client = {
            cfg_if::cfg_if! {
                if #[cfg(not(target_arch = "wasm32"))] {
                    if let Some(custom_client) = config.http_client.clone() {
                        custom_client
                    } else if config.allow_insecure_https.unwrap_or(false) {
                        reqwest::Client::builder()
                            .danger_accept_invalid_certs(true)
                            .danger_accept_invalid_hostnames(true)
                            .build()
                            .map_err(Error::HttpClient)?
                    } else {
                        reqwest::Client::new()
                    }
                } else {
                    if config.allow_insecure_https.unwrap_or(false) {
                        log::warn!(
                            "allow_insecure_https is silently ignored on wasm32 targets \
                            because SSL/TLS cert validation is strictly controlled by the browser sandbox environment."
                        );
                    }
                    reqwest::Client::new()
                }
            }
        };

        Ok(Self {
            config,
            namespaces: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(not(target_arch = "wasm32"))]
            handle: None,
            #[cfg(target_arch = "wasm32")]
            abort_handle: None,
            running: Arc::new(AtomicBool::new(false)),
            http_client,
        })
    }

    /// Starts periodic background polling in JavaScript/WASM.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyRunning`] if polling is already active.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = "start")]
    pub fn start_wasm(&mut self) -> Result<(), Error> {
        self.start_background()
    }

    /// Stops JavaScript/WASM background polling promptly.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = "stop")]
    pub fn stop_wasm(&mut self) {
        self.stop_background();
    }

    /// Forces one JavaScript/WASM namespace refresh.
    ///
    /// # Errors
    ///
    /// Returns transport, HTTP, parsing, or configuration errors.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = "refresh")]
    pub async fn refresh_wasm(&self, namespace: &str) -> Result<(), Error> {
        self.refresh(namespace).await
    }

    /// Preloads JavaScript/WASM namespaces concurrently.
    ///
    /// # Errors
    ///
    /// Returns an error if an array element is not a string or any namespace
    /// cannot be loaded.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = "preload")]
    pub async fn preload_wasm(&self, namespaces: js_sys::Array) -> Result<(), Error> {
        let namespaces = namespaces
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value.as_string().ok_or_else(|| {
                    Error::Config(client_config::Error::InvalidValue {
                        name: format!("namespaces[{index}]"),
                        value: format!("{value:?}"),
                        reason: "expected a string".to_string(),
                    })
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.preload(&namespaces).await
    }

    /// Registers a JavaScript function as an event listener for this cache (WASM only).
    ///
    /// This method is exposed to JavaScript as `addListener`.
    /// The provided JavaScript function will be called when the cache is refreshed.
    ///
    /// The JavaScript listener function is expected to have a signature like:
    /// `function(data, error)`
    /// - `data`: The JSON configuration object (if the refresh was successful and data
    ///           could be serialized) or `null` if an error occurred or serialization failed.
    /// - `error`: A string describing the error if one occurred during the configuration
    ///            fetch or processing. If the operation was successful, this will be `null`.
    ///
    /// # Arguments
    ///
    /// * `js_listener` - A JavaScript `Function` to be called on cache events.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// // Assuming `cacheInstance` is an instance of the Rust `Cache` object in JS
    /// cacheInstance.addListener((data, error) => {
    ///   if (error) {
    ///     console.error('Cache update error:', error);
    ///   } else {
    ///     console.log('Cache updated:', data);
    ///   }
    /// });
    /// // ... later, when the cache refreshes, the callback will be invoked.
    /// ```
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = "add_listener")]
    pub async fn add_listener_wasm(&self, namespace: &str, js_listener: js_sys::Function) {
        let js_listener_clone = js_listener.clone();

        let event_listener: EventListener = Arc::new(move |result: Result<Namespace, Error>| {
            let err_js_val: wasm_bindgen::JsValue;
            let data_js_val: wasm_bindgen::JsValue;

            match result {
                Ok(value) => {
                    data_js_val = value.into();
                    err_js_val = wasm_bindgen::JsValue::UNDEFINED;
                }
                Err(cache_error) => {
                    err_js_val = cache_error.into();
                    data_js_val = wasm_bindgen::JsValue::UNDEFINED;
                }
            }

            // Call the JavaScript listener: listener(data, error)
            match js_listener_clone.call2(
                &wasm_bindgen::JsValue::UNDEFINED,
                &data_js_val,
                &err_js_val,
            ) {
                Ok(_) => {
                    // JS function called successfully
                }
                Err(e) => {
                    // JS function threw an error or call failed
                    log::error!("JavaScript listener threw an error: {e:?}");
                }
            }
        });

        self.add_listener(namespace, event_listener).await; // Call the renamed Rust method
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = "namespace")]
    /// Retrieves and converts a namespace for JavaScript.
    ///
    /// # Errors
    ///
    /// Returns transport, cache, or namespace conversion errors.
    pub async fn namespace_wasm(&self, namespace: &str) -> Result<wasm_bindgen::JsValue, Error> {
        let cache = self.cache(namespace).await;
        let value = cache.get_value().await?;
        Ok(namespace::get_namespace(namespace, value)?.into())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) struct TempDir {
    path: std::path::PathBuf,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
impl TempDir {
    pub(crate) fn new(name: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "apollo-rust-client-{name}-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("temporary test directory should be writable");
        Self { path }
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
impl Drop for TempDir {
    fn drop(&mut self) {
        // Ignore errors, e.g. if the directory was already removed
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
pub(crate) fn setup() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            wasm_logger::init(wasm_logger::Config::default());
            console_error_panic_hook::set_once();
        } else {
            let _ = env_logger::builder().is_test(true).try_init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    fn test_server_url() -> String {
        std::env::var("APOLLO_TEST_SERVER")
            .unwrap_or_else(|_| String::from("http://localhost:8080"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn test_cache_dir() -> String {
        std::env::temp_dir()
            .join("apollo")
            .to_string_lossy()
            .to_string()
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn wiremock_request_count(url_path: &str) -> u64 {
        reqwest::Client::new()
            .post(format!("{}/__admin/requests/count", test_server_url()))
            .json(&serde_json::json!({"method": "GET", "urlPath": url_path}))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["count"]
            .as_u64()
            .unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn test_client_for_app(app_id: &str, temp_dir: &TempDir) -> Client {
        let config = ClientConfig::builder(app_id, test_server_url())
            .cache_dir(temp_dir.path().to_string_lossy())
            .build()
            .unwrap();
        Client::new(config).unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_test_client(
        app_id: &str,
        secret: Option<&str>,
        ip: Option<&str>,
        label: Option<&str>,
    ) -> Client {
        let mut builder =
            ClientConfig::builder(app_id, test_server_url()).cache_dir(test_cache_dir());
        if let Some(secret) = secret {
            builder = builder.secret(secret);
        }
        if let Some(ip) = ip {
            builder = builder.ip(ip);
        }
        if let Some(label) = label {
            builder = builder.label(label);
        }
        Client::new(builder.build().unwrap()).unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn client_no_secret() -> Client {
        native_test_client("101010101", None, None, None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn client_with_secret() -> Client {
        native_test_client(
            "101010102",
            Some("53bf47631db540ac9700f0020d2192c8"),
            None,
            None,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn client_with_grayscale_ip() -> Client {
        native_test_client("101010101", None, Some("1.2.3.4"), None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn client_with_grayscale_label() -> Client {
        native_test_client("101010101", None, None, Some("GrayScale"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_missing_value() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_no_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };

        assert_eq!(properties.get_property::<String>("missingValue"), None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn properties_public_and_text_namespaces_use_the_correct_types() {
        let properties = client_no_secret()
            .namespace("config.properties")
            .await
            .unwrap();
        assert!(matches!(properties, Namespace::Properties(_)));
        let public = client_no_secret().namespace("FX.apollo").await.unwrap();
        assert!(matches!(public, Namespace::Properties(_)));
        let text = client_no_secret().namespace("readme.txt").await.unwrap();
        assert!(matches!(text, Namespace::Text(_)));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn wiremock_status_and_malformed_responses_are_typed_errors() {
        for (app_id, expected_status) in [("http-401", 401), ("http-429", 429), ("http-500", 500)] {
            let temp_dir = TempDir::new(app_id);
            let error = test_client_for_app(app_id, &temp_dir)
                .namespace("application")
                .await
                .unwrap_err();
            assert!(matches!(
                error,
                Error::Cache(cache::Error::HttpStatus { status, .. }) if status == expected_status
            ));
        }
        let temp_dir = TempDir::new("malformed");
        let malformed = test_client_for_app("malformed", &temp_dir)
            .namespace("application")
            .await
            .unwrap_err();
        assert!(matches!(malformed, Error::Cache(cache::Error::Serde(_))));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn request_timeout_has_a_typed_error() {
        let temp_dir = TempDir::new("timeout");
        let error = test_client_for_app("timeout", &temp_dir)
            .namespace("application")
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            Error::Cache(cache::Error::Timeout { seconds: 10 })
        ));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_missing_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_string("missingValue"), None);
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_string_value() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_no_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };

        assert_eq!(
            properties.get_property::<String>("stringValue"),
            Some("string value".to_string())
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_string_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(
                        properties.get_string("stringValue"),
                        Some("string value".to_string())
                    );
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_string_value_with_secret() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_with_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(
            properties.get_property::<String>("stringValue"),
            Some("string value".to_string())
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_string_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(
                        properties.get_string("stringValue"),
                        Some("string value".to_string())
                    );
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_int_value() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_no_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(properties.get_property::<i32>("intValue"), Some(42));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_int_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_int("intValue"), Some(42));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_int_value_with_secret() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_with_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(properties.get_property::<i32>("intValue"), Some(42));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_int_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_int("intValue"), Some(42));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_float_value() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_no_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(properties.get_property::<f64>("floatValue"), Some(4.20));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_float_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_float("floatValue"), Some(4.20));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_float_value_with_secret() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_with_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(properties.get_property::<f64>("floatValue"), Some(4.20));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_float_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_float("floatValue"), Some(4.20));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_no_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(properties.get_property::<bool>("boolValue"), Some(false));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_bool("boolValue"), Some(false));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_secret() {
        setup();
        let namespace::Namespace::Properties(properties) =
            client_with_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(properties.get_property::<bool>("boolValue"), Some(false));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let namespace = client.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_bool("boolValue"), Some(false));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_grayscale_ip() {
        setup();
        let namespace::Namespace::Properties(properties) = client_with_grayscale_ip()
            .namespace("application")
            .await
            .unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue"),
            Some(true)
        );
        let namespace::Namespace::Properties(properties) =
            client_no_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue"),
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_grayscale_ip_wasm() {
        setup();
        let client1 = create_client_with_grayscale_ip();
        let namespace = client1.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_bool("grayScaleValue"), Some(true));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }

        let client2 = create_client_no_secret();
        let namespace = client2.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_bool("grayScaleValue"), Some(false));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_grayscale_label() {
        setup();
        let namespace::Namespace::Properties(properties) = client_with_grayscale_label()
            .namespace("application")
            .await
            .unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue"),
            Some(true)
        );
        let namespace::Namespace::Properties(properties) =
            client_no_secret().namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue"),
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_grayscale_label_wasm() {
        setup();
        let client1 = create_client_with_grayscale_label();
        let namespace = client1.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_bool("grayScaleValue"), Some(true));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }

        let client2 = create_client_no_secret();
        let namespace = client2.namespace("application").await;
        match namespace {
            Ok(namespace) => match namespace {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(properties.get_bool("grayScaleValue"), Some(false));
                }
                _ => panic!("Expected Properties namespace"),
            },
            Err(e) => panic!("Expected Properties namespace, got error: {e:?}"),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_no_secret() -> Client {
        let config = ClientConfig::builder("101010101", test_server_url())
            .build()
            .expect("test client configuration should be valid");
        Client::new(config).expect("test client configuration should be valid")
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_with_secret() -> Client {
        let config = ClientConfig::builder("101010102", test_server_url())
            .secret("53bf47631db540ac9700f0020d2192c8")
            .build()
            .expect("test client configuration should be valid");
        Client::new(config).expect("test client configuration should be valid")
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_with_grayscale_ip() -> Client {
        let config = ClientConfig::builder("101010101", test_server_url())
            .ip("1.2.3.4")
            .build()
            .expect("test client configuration should be valid");
        Client::new(config).expect("test client configuration should be valid")
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_with_grayscale_label() -> Client {
        let config = ClientConfig::builder("101010101", test_server_url())
            .label("GrayScale")
            .build()
            .expect("test client configuration should be valid");
        Client::new(config).expect("test client configuration should be valid")
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test] // Re-enable for WASM
    async fn test_add_listener_and_notify_on_refresh() {
        setup();

        // Shared state to check if listener was called and what it received
        let listener_called_flag = Arc::new(Mutex::new(false));
        let received_config_data = Arc::new(Mutex::new(None::<Namespace>));

        let temp_dir = TempDir::new("apollo_listener_test");

        // ClientConfig similar to CLIENT_NO_SECRET from lib.rs tests
        // Using the same external test server and app_id as tests in lib.rs
        let config = ClientConfig {
            config_server: test_server_url(), // Use external test server
            app_id: "101010101".to_string(),  // Use existing app_id from lib.rs tests
            cluster: "default".to_string(),
            cache_dir: Some(temp_dir.path().to_str().unwrap().to_string()), // Use a writable directory
            secret: None,
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

        let client = Client::new(config).expect("test client configuration should be valid");

        let flag_clone = listener_called_flag.clone();
        let data_clone = received_config_data.clone();

        let listener: EventListener = Arc::new(move |result| {
            let mut called_guard = flag_clone.lock().unwrap();
            *called_guard = true;
            if let Ok(config_value) = result {
                match config_value {
                    Namespace::Properties(_) => {
                        let mut data_guard = data_clone.lock().unwrap();
                        *data_guard = Some(config_value.clone());
                    }
                    _ => {
                        panic!("Expected Properties namespace, got {config_value:?}");
                    }
                }
            }
            // In a real scenario, avoid panicking in a listener.
            // For a test, this is acceptable to signal issues.
        });

        client.add_listener("application", listener).await;

        let cache = client.cache("application").await;

        // Perform a refresh. This should trigger the listener.
        // The test Apollo server (localhost:8071) should have some known config for "SampleApp" "application" namespace.
        match cache.refresh().await {
            Ok(()) => log::debug!("Refresh successful for test_add_listener_and_notify_on_refresh"),
            Err(e) => panic!("Cache refresh failed during test: {e:?}"),
        }

        // Give the async listener task time to complete
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                // For WASM, listeners are synchronous so no wait needed
            } else {
                // For native targets, use tokio sleep
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        // Check if the listener was called
        let called = *listener_called_flag.lock().unwrap();
        assert!(called, "Listener was not called.");

        // Check if config data was received
        let config_data_guard = received_config_data.lock().unwrap();
        assert!(
            config_data_guard.is_some(),
            "Listener did not receive config data."
        );

        // Optionally, assert specific content if known.
        // Assert based on known data for app_id "101010101", namespace "application"
        // from the external test server. Example: "stringValue"
        if let Some(value) = config_data_guard.as_ref() {
            match value {
                Namespace::Properties(properties) => {
                    assert_eq!(
                        properties.get_string("stringValue"),
                        Some(String::from("string value")),
                        "Received config data does not match expected content for stringValue."
                    );
                }
                _ => {
                    panic!("Expected Properties namespace, got {value:?}");
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_add_listener_wasm_and_notify() {
        setup();
        let global = js_sys::global();
        let data_key = wasm_bindgen::JsValue::from_str("__apolloListenerData");
        let error_key = wasm_bindgen::JsValue::from_str("__apolloListenerError");
        let _ = js_sys::Reflect::delete_property(&global, &data_key);
        let _ = js_sys::Reflect::delete_property(&global, &error_key);
        let js_listener = js_sys::Function::new_with_args(
            "data, error",
            "globalThis.__apolloListenerData = data; globalThis.__apolloListenerError = error;",
        );

        let client = create_client_no_secret();
        client.add_listener_wasm("application", js_listener).await;

        let cache = client.cache("application").await;
        match cache.refresh().await {
            Ok(()) => web_sys::console::log_1(&"WASM Test: Refresh successful".into()),
            Err(e) => panic!("WASM Test: Cache refresh failed: {e:?}"),
        }

        let data = js_sys::Reflect::get(&global, &data_key).unwrap();
        let error = js_sys::Reflect::get(&global, &error_key).unwrap();
        assert!(!data.is_undefined(), "JavaScript listener received no data");
        assert!(
            error.is_undefined(),
            "JavaScript listener received an error"
        );
        let _ = js_sys::Reflect::delete_property(&global, &data_key);
        let _ = js_sys::Reflect::delete_property(&global, &error_key);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_concurrent_namespace_hang_repro() {
        setup();

        let temp_dir = TempDir::new("apollo_hang_test");

        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: test_server_url(),
            secret: None,
            cache_dir: Some(temp_dir.path().to_str().unwrap().to_string()),
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: None,
            refresh_interval: None,
            http_client: None,
        };

        let client =
            Arc::new(Client::new(config).expect("test client configuration should be valid"));
        let client_in_listener = client.clone();

        let listener_triggered = Arc::new(Mutex::new(false));
        let listener_triggered_in_listener = listener_triggered.clone();

        let listener: EventListener = Arc::new(move |_| {
            let client_in_listener = client_in_listener.clone();
            let listener_triggered_in_listener = listener_triggered_in_listener.clone();
            tokio::spawn(async move {
                {
                    let mut triggered = listener_triggered_in_listener.lock().unwrap();
                    if *triggered {
                        // Avoid infinite loops if the listener is called more than once.
                        return;
                    }
                    *triggered = true;
                }

                // This is the recursive call that should no longer trigger a deadlock.
                let _ = client_in_listener.namespace("application").await;
            });
        });

        client.add_listener("application", listener).await;

        let test_body = async {
            let _ = client.namespace("application").await;
        };

        // The test should not hang. If it does, timeout will fail it.
        let res = tokio::time::timeout(std::time::Duration::from_secs(10), test_body).await;
        assert!(res.is_ok(), "Test timed out, which indicates a deadlock.");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_custom_refresh_interval() {
        setup();

        let temp_dir = TempDir::new("apollo_custom_refresh_interval");

        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: test_server_url(),
            secret: None,
            cache_dir: Some(temp_dir.path().to_str().unwrap().to_string()),
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: None,
            refresh_interval: Some(1), // 1 second interval for fast testing
            http_client: None,
        };

        let path = "/configfiles/json/101010101/default/application";
        let before = wiremock_request_count(path).await;
        let mut client = Client::new(config).expect("test client configuration should be valid");
        // Preload namespace so it's registered
        let _ = client.namespace("application").await;

        let res = client.start().await;
        assert!(res.is_ok(), "Failed to start client background task");

        // Let it run for 2.5 seconds (which should trigger a couple of refreshes with 1s interval)
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

        client.stop().await;
        let after = wiremock_request_count(path).await;
        assert!(
            after >= before + 3,
            "expected initial load plus at least two periodic refreshes, before={before}, after={after}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn lifecycle_is_idempotent_prompt_and_cancelled_on_drop() {
        let config = ClientConfig::builder("lifecycle-test", "http://127.0.0.1:9")
            .refresh_interval(60)
            .build()
            .unwrap();
        let mut client = Client::new(config).unwrap();
        client.start().await.unwrap();
        assert!(matches!(client.start().await, Err(Error::AlreadyRunning)));
        tokio::time::timeout(std::time::Duration::from_millis(50), client.stop())
            .await
            .expect("stop waited for the refresh interval");
        assert!(!client.running.load(Ordering::Acquire));

        client.start().await.unwrap();
        let running = client.running.clone();
        let abort_handle = client.handle.as_ref().unwrap().abort_handle();
        drop(client);
        tokio::task::yield_now().await;
        assert!(!running.load(Ordering::Acquire));
        assert!(abort_handle.is_finished());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn preload_supports_parallel_duplicates_and_propagates_http_errors() {
        let temp_dir = TempDir::new("preload_behavior");
        let config = ClientConfig::builder("101010101", test_server_url())
            .cache_dir(temp_dir.path().to_string_lossy())
            .build()
            .unwrap();
        let client = Client::new(config).unwrap();
        client
            .preload(&[
                "application",
                "application.json",
                "application",
                "application.yml",
            ])
            .await
            .unwrap();
        assert!(matches!(
            client.namespace("application.json").await.unwrap(),
            Namespace::Json(_)
        ));
        assert!(matches!(
            client.preload(&["does-not-exist"]).await.unwrap_err(),
            Error::Cache(cache::Error::HttpStatus { status: 404, .. })
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_refresh_interval_validation() {
        unsafe {
            std::env::set_var("APP_ID", "101010101");
            std::env::set_var("APOLLO_CONFIG_SERVICE", "http://localhost:8080");
            std::env::set_var("APOLLO_REFRESH_INTERVAL", "0");
        }
        assert!(matches!(
            ClientConfig::from_env(),
            Err(client_config::Error::InvalidValue { .. })
        ));

        unsafe {
            std::env::set_var("APOLLO_REFRESH_INTERVAL", "15");
        }
        let config2 = ClientConfig::from_env().unwrap();
        assert_eq!(config2.refresh_interval, Some(15));

        unsafe {
            std::env::remove_var("APP_ID");
            std::env::remove_var("APOLLO_CONFIG_SERVICE");
            std::env::remove_var("APOLLO_REFRESH_INTERVAL");
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_wasm_local_storage_caching() {
        use wasm_bindgen::prelude::Closure;
        setup();

        // 1. Setup mock localStorage backed by a Rust HashMap
        let store = Arc::new(Mutex::new(HashMap::<String, String>::new()));

        let store_clone1 = store.clone();
        let get_item = Closure::wrap(Box::new(move |key: String| -> wasm_bindgen::JsValue {
            let map = store_clone1.lock().unwrap();
            if let Some(val) = map.get(&key) {
                wasm_bindgen::JsValue::from_str(val)
            } else {
                wasm_bindgen::JsValue::NULL
            }
        }) as Box<dyn Fn(String) -> wasm_bindgen::JsValue>);

        let store_clone2 = store.clone();
        let set_item = Closure::wrap(Box::new(move |key: String, value: String| {
            let mut map = store_clone2.lock().unwrap();
            map.insert(key, value);
        }) as Box<dyn Fn(String, String)>);

        let mock_storage = js_sys::Object::new();
        js_sys::Reflect::set(
            &mock_storage,
            &wasm_bindgen::JsValue::from_str("getItem"),
            get_item.as_ref(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &mock_storage,
            &wasm_bindgen::JsValue::from_str("setItem"),
            set_item.as_ref(),
        )
        .unwrap();

        // Inject mock_storage into globalThis
        let global = js_sys::global();
        js_sys::Reflect::set(
            &global,
            &wasm_bindgen::JsValue::from_str("localStorage"),
            &mock_storage,
        )
        .unwrap();

        // 2. Setup ClientConfig and cache key
        let config = ClientConfig {
            app_id: "101010101".to_string(),
            cluster: "default".to_string(),
            config_server: "http://localhost:8080".to_string(),
            secret: None,
            cache_dir: None,
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: Some(600),
            refresh_interval: Some(30),
        };

        // Construct mock config data in cache format directly using JSON value
        let cache_item = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp(),
            "config": {
                "stringValue": "localstorage value"
            }
        });
        let cache_content = serde_json::to_string(&cache_item).unwrap();

        let cache = cache::Cache::new(config, "application", reqwest::Client::new());

        // Save directly to our mock localStorage using the versioned identity.
        let cache_key = cache.wasm_cache_key();
        {
            let mut map = store.lock().unwrap();
            map.insert(cache_key.to_string(), cache_content);
        }

        // Retrieve cache. Should hit Tier 2 (mock localStorage) and return the data!
        let value = cache.get_value().await.unwrap();
        assert_eq!(
            value.get("stringValue").and_then(|v| v.as_str()),
            Some("localstorage value"),
            "Cache failed to load configuration from mocked local storage"
        );

        // Keep Closures alive until the end of the test
        get_item.into_js_value();
        set_item.into_js_value();

        // Cleanup: remove localStorage from globalThis to keep tests clean
        let _ = js_sys::Reflect::delete_property(
            &global,
            &wasm_bindgen::JsValue::from_str("localStorage"),
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn test_wasm_cache_key_isolation() {
        setup();

        // 1. Base configuration
        let config1 = ClientConfig {
            app_id: "app1".to_string(),
            cluster: "default".to_string(),
            config_server: "http://localhost:8080".to_string(),
            secret: None,
            cache_dir: None,
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: Some(600),
            refresh_interval: Some(30),
        };
        let cache1 = cache::Cache::new(config1, "application", reqwest::Client::new());
        assert!(cache1.wasm_cache_key().starts_with("apollo_cache_v2_"));

        // 2. Different cluster
        let config2 = ClientConfig {
            app_id: "app1".to_string(),
            cluster: "prod".to_string(),
            config_server: "http://localhost:8080".to_string(),
            secret: None,
            cache_dir: None,
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: Some(600),
            refresh_interval: Some(30),
        };
        let cache2 = cache::Cache::new(config2, "application", reqwest::Client::new());
        assert_ne!(cache1.wasm_cache_key(), cache2.wasm_cache_key());

        // 3. Different namespace
        let config3 = ClientConfig {
            app_id: "app1".to_string(),
            cluster: "default".to_string(),
            config_server: "http://localhost:8080".to_string(),
            secret: None,
            cache_dir: None,
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: Some(600),
            refresh_interval: Some(30),
        };
        let cache3 = cache::Cache::new(config3, "other_namespace", reqwest::Client::new());
        assert_ne!(cache1.wasm_cache_key(), cache3.wasm_cache_key());

        // 4. Grayscale targeting: IP and label present
        let config4 = ClientConfig {
            app_id: "app1".to_string(),
            cluster: "default".to_string(),
            config_server: "http://localhost:8080".to_string(),
            secret: None,
            cache_dir: None,
            label: Some("gray".to_string()),
            ip: Some("192.168.1.1".to_string()),
            allow_insecure_https: None,
            cache_ttl: Some(600),
            refresh_interval: Some(30),
        };
        let cache4 = cache::Cache::new(config4, "application", reqwest::Client::new());
        assert_ne!(cache1.wasm_cache_key(), cache4.wasm_cache_key());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn test_wasm_lifecycle_starts_stops_and_rejects_duplicate_start() {
        setup();
        let mut client = create_client_no_secret();
        client.start_wasm().unwrap();
        assert!(client.running.load(Ordering::Acquire));
        assert!(client.abort_handle.is_some());
        assert!(matches!(client.start_wasm(), Err(Error::AlreadyRunning)));
        client.stop_wasm();
        assert!(!client.running.load(Ordering::Acquire));
        assert!(client.abort_handle.is_none());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn test_wasm_allow_insecure_https_warning() {
        setup();

        let config = ClientConfig {
            app_id: "101010101".to_string(),
            cluster: "default".to_string(),
            config_server: "http://localhost:8080".to_string(),
            secret: None,
            cache_dir: None,
            label: None,
            ip: None,
            allow_insecure_https: Some(true),
            cache_ttl: Some(600),
            refresh_interval: Some(30),
        };

        // Construct client. This will trigger the log::warn! call.
        let _client = Client::new(config);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_custom_http_client_injection() {
        setup();
        // Create a custom reqwest::Client with an extremely short timeout of 1ms
        let custom_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();

        let temp_dir = TempDir::new("apollo_custom_http_test");

        let config = ClientConfig {
            config_server: test_server_url(),
            app_id: "101010101".to_string(),
            cluster: "default".to_string(),
            cache_dir: Some(temp_dir.path().to_str().unwrap().to_string()),
            secret: None,
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: None,
            refresh_interval: None,
            http_client: Some(custom_client),
        };

        let client = Client::new(config).expect("test client configuration should be valid");

        // Fetching namespace should fail because of the 1ms timeout!
        let result = client.namespace("application").await;
        assert!(
            result.is_err(),
            "Expected request to fail due to custom injected HTTP client timeout"
        );

        // Verify the error represents a timeout or request failure
        let err_str = result.err().unwrap().to_string();
        assert!(
            err_str.contains("timeout") || err_str.contains("error") || err_str.contains("reqwest"),
            "Expected error to mention timeout or request failure, got: {err_str}"
        );
    }
}
