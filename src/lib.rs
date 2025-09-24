//! # Apollo Rust Client
//!
//! A robust Rust client for the Apollo Configuration Centre, with support for WebAssembly
//! for browser and Node.js environments.
//!
//! This crate provides a comprehensive client for interacting with Apollo configuration services.
//! The client manages configurations for different namespaces, supports multiple configuration
//! formats (Properties, JSON, Text), provides caching mechanisms, and offers real-time updates
//! through background polling and event listeners.
//!
//! ## Key Features
//!
//! - **Multiple Configuration Formats**: Support for Properties, JSON, Text formats with automatic detection
//! - **Cross-Platform**: Native Rust and WebAssembly targets with platform-specific optimizations
//! - **Real-Time Updates**: Background polling with configurable intervals and event listeners
//! - **Comprehensive Caching**: Multi-level caching with file persistence (native) and memory-only (WASM)
//! - **Type-Safe API**: Compile-time guarantees and runtime type conversion
//! - **Error Handling**: Detailed error diagnostics with comprehensive error types
//! - **Grayscale Release Support**: IP and label-based configuration targeting
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use apollo_rust_client::{Client, client_config::ClientConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ClientConfig {
//!     app_id: "my-app".to_string(),
//!     config_server: "http://apollo-server:8080".to_string(),
//!     cluster: "default".to_string(),
//!     secret: None,
//!     cache_dir: None,
//!     label: None,
//!     ip: None,
//!     allow_insecure_https: None,
//!     #[cfg(not(target_arch = "wasm32"))]
//!     cache_ttl: None,
//! };
//!
//! let mut client = Client::new(config);
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
//! - **WebAssembly**: Memory-only caching, single-threaded execution, JavaScript interop

use crate::namespace::Namespace;
use async_std::sync::RwLock;
use cache::Cache;
use client_config::ClientConfig;
use log::{error, trace};
use std::{collections::HashMap, sync::Arc};
use wasm_bindgen::prelude::wasm_bindgen;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use async_std::task::spawn as spawn;
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
/// # #[tokio::main]
/// # async fn main() {
/// # let client = Client::new(apollo_rust_client::client_config::ClientConfig {
/// #     app_id: "test".to_string(),
/// #     config_server: "http://localhost:8080".to_string(),
/// #     cluster: "default".to_string(),
/// #     secret: None,
/// #     cache_dir: None,
/// #     label: None,
/// #     ip: None,
/// #     allow_insecure_https: None,
/// #     #[cfg(not(target_arch = "wasm32"))]
/// #     cache_ttl: None,
/// # });
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
}

impl From<Error> for wasm_bindgen::JsValue {
    fn from(error: Error) -> Self {
        error.to_string().into()
    }
}

// Type alias for event listeners that can be registered with the cache.
// For WASM targets, listeners don't need to be Send + Sync since WASM is single-threaded.
// Listeners are functions that take a `Result<Value, Error>` as an argument.
// `Value` is the `serde_json::Value` representing the configuration.
// `Error` is the cache's error enum.
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Type alias for event listeners that can be registered with the cache.
        /// For WASM targets, listeners don't need to be Send + Sync since WASM is single-threaded.
        /// Listeners are functions that take a `Result<Value, Error>` as an argument.
        pub type EventListener = Arc<dyn Fn(Result<Namespace, Error>)>;
    } else {
        /// Type alias for event listeners that can be registered with the client.
        /// For native targets, listeners need to be `Send` and `Sync` to be safely
        /// shared across threads.
        ///
        /// Listeners are functions that take a `Result<Namespace<T>, Error>` as an
        /// argument, where `Namespace<T>` is a fresh copy of the updated namespace,`
        /// and `Error` is the cache's error enum.
        pub type EventListener = Arc<dyn Fn(Result<Namespace, Error>) + Send + Sync>;
    }
}

/// The main Apollo configuration client.
///
/// This struct provides the primary interface for interacting with Apollo configuration services.
/// It manages multiple namespace caches, handles background refresh tasks, and provides
/// event listener functionality for real-time configuration updates.
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
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     // Create a client instance
/// #     let client = Client::new(ClientConfig {
/// #         app_id: "test_app".to_string(),
/// #         config_server: "http://localhost:8080".to_string(),
/// #         cluster: "default".to_string(),
/// #         secret: None,
/// #         cache_dir: None,
/// #         label: None,
/// #         ip: None,
/// #         allow_insecure_https: None,
/// #         #[cfg(not(target_arch = "wasm32"))]
/// #         cache_ttl: None,
/// #     });
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
    /// On non-wasm32 targets, this holds a `JoinHandle` to the spawned background task
    /// that periodically refreshes all namespace caches. On wasm32 targets, this is
    /// always `None` as task management differs in single-threaded environments.
    handle: Option<async_std::task::JoinHandle<()>>,

    /// Flag indicating whether the background refresh task is active.
    ///
    /// Wrapped in `Arc<RwLock<bool>>` for thread-safe shared access between the
    /// client and its background task. Used to coordinate task lifecycle management.
    running: Arc<RwLock<bool>>,
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
        let mut namespaces = self.namespaces.write().await;
        let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
            trace!("Cache miss, creating cache for namespace {namespace}");
            Arc::new(Cache::new(self.config.clone(), namespace))
        });
        cache.clone()
    }

    pub async fn add_listener(&self, namespace: &str, listener: EventListener) {
        let mut namespaces = self.namespaces.write().await;
        let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
            trace!("Cache miss, creating cache for namespace {namespace}");
            Arc::new(Cache::new(self.config.clone(), namespace))
        });
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
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     // Create a client instance
    /// #     let client = Client::new(ClientConfig {
    /// #         app_id: "test_app".to_string(),
    /// #         config_server: "http://localhost:8080".to_string(),
    /// #         cluster: "default".to_string(),
    /// #         secret: None,
    /// #         cache_dir: None,
    /// #         label: None,
    /// #         ip: None,
    /// #         allow_insecure_https: None,
    /// #         #[cfg(not(target_arch = "wasm32"))]
    /// #         cache_ttl: None,
    /// #     });
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
    /// This method spawns an asynchronous task using `async_std::task::spawn` on native targets
    /// or `wasm_bindgen_futures::spawn_local` on wasm32 targets. The task loops indefinitely
    /// (until `stop` is called or the client is dropped) and performs the following actions
    /// in each iteration:
    ///
    /// 1. Iterates through all namespaces currently managed by the client.
    /// 2. Calls the `refresh` method on each namespace's `Cache` instance.
    /// 3. Logs any errors encountered during the refresh process.
    /// 4. Sleeps for a predefined interval (currently 30 seconds) before the next refresh cycle.
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
    pub async fn start(&mut self) -> Result<(), Error> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::AlreadyRunning);
        }

        *running = true;

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                self.handle = None;
            } else {
                let running = self.running.clone();
                let namespaces = self.namespaces.clone();
                // Spawn a background thread to refresh caches
                let handle = spawn(async move {
                    loop {
                        let running = running.read().await;
                        if !*running {
                            break;
                        }

                        // Clone cache references before releasing the lock to prevent long-held locks
                        let cache_refs: Vec<_> = {
                            let namespaces = namespaces.read().await;
                            namespaces.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                        }; // Lock released here

                        // Refresh each namespace's cache without holding the lock
                        for (namespace, cache) in cache_refs {
                            if let Err(err) = cache.refresh().await {
                                error!("Failed to refresh cache for namespace {namespace}: {err:?}");
                            } else {
                                log::debug!("Successfully refreshed cache for namespace {namespace}");
                            }
                        }

                        // Sleep for 30 seconds before the next refresh
                        async_std::task::sleep(std::time::Duration::from_secs(30)).await;
                    }
                });
                self.handle = Some(handle);
            }
        }

        Ok(())
    }

    /// Stops the background cache refresh task.
    ///
    /// This method sets the `running` flag to `false`, signaling the background task
    /// to terminate its refresh loop.
    ///
    /// On non-wasm32 targets, it also attempts to explicitly cancel the spawned task
    /// by calling `cancel()` on its `JoinHandle` if it exists. This helps to ensure
    /// that the task is properly cleaned up. On wasm32 targets, there is no direct
    /// handle to cancel, so setting the `running` flag is the primary mechanism for stopping.
    pub async fn stop(&mut self) {
        let mut running = self.running.write().await;
        *running = false;

        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                if let Some(handle) = self.handle.take() {
                    handle.cancel().await;
                }
            }
        }
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
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ClientConfig {
    ///     app_id: "my-app".to_string(),
    ///     config_server: "http://apollo-server:8080".to_string(),
    ///     cluster: "default".to_string(),
    ///     secret: None,
    ///     cache_dir: None,
    ///     label: None,
    ///     ip: None,
    ///     allow_insecure_https: None,
    ///     #[cfg(not(target_arch = "wasm32"))]
    ///     cache_ttl: None,
    /// };
    ///
    /// let mut client = Client::new(config);
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
        #[cfg(not(target_arch = "wasm32"))]
        let mut tasks = Vec::new();

        #[cfg(target_arch = "wasm32")]
        {
            for namespace in namespaces {
                let cache = self.cache(namespace.as_ref()).await;
                cache.get_value().await?;
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            for namespace in namespaces {
                let cache = self.cache(namespace.as_ref()).await;
                let task = tokio::spawn(async move { cache.get_value().await });
                tasks.push(task);
            }

            // Wait for all preload tasks to complete
            for task in tasks {
                let result = task.await.map_err(|e| {
                    Error::Cache(cache::Error::Io(std::io::Error::other(format!(
                        "Preload task failed: {e}"
                    ))))
                })?;
                result?;
            }
        }

        Ok(())
    }
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
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            namespaces: Arc::new(RwLock::new(HashMap::new())),
            handle: None,
            running: Arc::new(RwLock::new(false)),
        }
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
            };

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
                    log::error!("JavaScript listener threw an error: {:?}", e);
                }
            }
        });

        self.add_listener(namespace, event_listener).await; // Call the renamed Rust method
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = "namespace")]
    pub async fn namespace_wasm(&self, namespace: &str) -> Result<wasm_bindgen::JsValue, Error> {
        let cache = self.cache(namespace).await;
        let value = cache.get_value().await?;
        Ok(namespace::get_namespace(namespace, value)?.into())
    }
}

#[cfg(test)]
pub(crate) struct TempDir {
    path: std::path::PathBuf,
}

#[cfg(test)]
impl TempDir {
    pub(crate) fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(name);
        // Ignore errors if the directory already exists
        let _ = std::fs::create_dir_all(&path);
        Self { path }
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }
}

#[cfg(test)]
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
            let _ = wasm_logger::init(wasm_logger::Config::default());
            console_error_panic_hook::set_once();
        } else {
            let _ = env_logger::builder().is_test(true).try_init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use std::sync::Mutex;
        } else {
            use async_std::sync::Mutex;
            use async_std::task::block_on;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) static CLIENT_NO_SECRET: std::sync::LazyLock<Client> =
        std::sync::LazyLock::new(|| {
            let config = ClientConfig {
                app_id: String::from("101010101"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: None,
                secret: None,
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: None,
                allow_insecure_https: None,
                #[cfg(not(target_arch = "wasm32"))]
                cache_ttl: None,
            };
            Client::new(config)
        });

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) static CLIENT_WITH_SECRET: std::sync::LazyLock<Client> =
        std::sync::LazyLock::new(|| {
            let config = ClientConfig {
                app_id: String::from("101010102"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: None,
                secret: Some(String::from("53bf47631db540ac9700f0020d2192c8")),
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: None,
                allow_insecure_https: None,
                #[cfg(not(target_arch = "wasm32"))]
                cache_ttl: None,
            };
            Client::new(config)
        });

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) static CLIENT_WITH_GRAYSCALE_IP: std::sync::LazyLock<Client> =
        std::sync::LazyLock::new(|| {
            let config = ClientConfig {
                app_id: String::from("101010101"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: None,
                secret: None,
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: Some(String::from("1.2.3.4")),
                allow_insecure_https: None,
                #[cfg(not(target_arch = "wasm32"))]
                cache_ttl: None,
            };
            Client::new(config)
        });

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) static CLIENT_WITH_GRAYSCALE_LABEL: std::sync::LazyLock<Client> =
        std::sync::LazyLock::new(|| {
            let config = ClientConfig {
                app_id: String::from("101010101"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: Some(String::from("GrayScale")),
                secret: None,
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: None,
                allow_insecure_https: None,
                #[cfg(not(target_arch = "wasm32"))]
                cache_ttl: None,
            };
            Client::new(config)
        });

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_missing_value() {
        setup();
        let namespace::Namespace::Properties(properties) =
            CLIENT_NO_SECRET.namespace("application").await.unwrap()
        else {
            panic!("Expected Properties namespace");
        };

        assert_eq!(properties.get_property::<String>("missingValue"), None);
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
            CLIENT_NO_SECRET.namespace("application").await.unwrap()
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
            CLIENT_WITH_SECRET.namespace("application").await.unwrap()
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
            CLIENT_NO_SECRET.namespace("application").await.unwrap()
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
            CLIENT_WITH_SECRET.namespace("application").await.unwrap()
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
            CLIENT_NO_SECRET.namespace("application").await.unwrap()
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
            CLIENT_WITH_SECRET.namespace("application").await.unwrap()
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
            CLIENT_NO_SECRET.namespace("application").await.unwrap()
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
            CLIENT_WITH_SECRET.namespace("application").await.unwrap()
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
        let namespace::Namespace::Properties(properties) = CLIENT_WITH_GRAYSCALE_IP
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
            CLIENT_NO_SECRET.namespace("application").await.unwrap()
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
            Err(e) => panic!("Expected Properties namespace, got error: {:?}", e),
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
            Err(e) => panic!("Expected Properties namespace, got error: {:?}", e),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_grayscale_label() {
        setup();
        let namespace::Namespace::Properties(properties) = CLIENT_WITH_GRAYSCALE_LABEL
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
            CLIENT_NO_SECRET.namespace("application").await.unwrap()
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
            Err(e) => panic!("Expected Properties namespace, got error: {:?}", e),
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
            Err(e) => panic!("Expected Properties namespace, got error: {:?}", e),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_no_secret() -> Client {
        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            label: None,
            secret: None,
            cache_dir: None,
            ip: None,
            allow_insecure_https: None,
        };
        Client::new(config)
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_with_secret() -> Client {
        let config = ClientConfig {
            app_id: String::from("101010102"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            label: None,
            secret: Some(String::from("53bf47631db540ac9700f0020d2192c8")),
            cache_dir: None,
            ip: None,
            allow_insecure_https: None,
        };
        Client::new(config)
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_with_grayscale_ip() -> Client {
        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            label: None,
            secret: None,
            cache_dir: None,
            ip: Some(String::from("1.2.3.4")),
            allow_insecure_https: None,
        };
        Client::new(config)
    }

    #[cfg(target_arch = "wasm32")]
    fn create_client_with_grayscale_label() -> Client {
        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            label: Some(String::from("GrayScale")),
            secret: None,
            cache_dir: None,
            ip: None,
            allow_insecure_https: None,
        };
        Client::new(config)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test] // Re-enable for WASM
    async fn test_add_listener_and_notify_on_refresh() {
        setup();

        // Shared state to check if listener was called and what it received
        let listener_called_flag = Arc::new(Mutex::new(false));
        let received_config_data = Arc::new(Mutex::new(None::<Namespace>));

        // ClientConfig similar to CLIENT_NO_SECRET from lib.rs tests
        // Using the same external test server and app_id as tests in lib.rs
        let config = ClientConfig {
            config_server: "http://81.68.181.139:8080".to_string(), // Use external test server
            app_id: "101010101".to_string(), // Use existing app_id from lib.rs tests
            cluster: "default".to_string(),
            cache_dir: Some(String::from("/tmp/apollo")), // Use a writable directory
            secret: None,
            label: None,
            ip: None,
            allow_insecure_https: None,
            #[cfg(not(target_arch = "wasm32"))]
            cache_ttl: None,
            // ..Default::default() // Be careful with Default if it doesn't set all needed fields for tests
        };

        let client = Client::new(config);

        let flag_clone = listener_called_flag.clone();
        let data_clone = received_config_data.clone();

        let listener: EventListener = Arc::new(move |result| {
            let mut called_guard = block_on(flag_clone.lock());
            *called_guard = true;
            if let Ok(config_value) = result {
                match config_value {
                    Namespace::Properties(_) => {
                        let mut data_guard = block_on(data_clone.lock());
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
        let called = *listener_called_flag.lock().await;
        assert!(called, "Listener was not called.");

        // Check if config data was received
        let config_data_guard = received_config_data.lock().await;
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
        setup(); // Existing test setup

        // Shared state to check if listener was called and what it received
        let listener_called_flag = Arc::new(Mutex::new(false));
        let received_config_data = Arc::new(Mutex::new(None::<Namespace>));

        let flag_clone = listener_called_flag.clone();
        let data_clone = received_config_data.clone();

        // Create JS Listener Function that updates our shared state
        let js_listener_func_body = format!(
            r#"
            (data, error) => {{
                // We can't use window in Node.js, so we'll use a different approach
                // The Rust closure will handle the verification
                console.log('JS Listener called with error:', error);
                console.log('JS Listener called with data:', data);
            }}
        "#
        );

        let js_listener = js_sys::Function::new_with_args("data, error", &js_listener_func_body);

        let client = create_client_no_secret();

        // Add a Rust listener to verify the functionality
        let rust_listener: EventListener = Arc::new(move |result| {
            let mut called_guard = flag_clone.lock().unwrap();
            *called_guard = true;
            if let Ok(config_value) = result {
                let mut data_guard = data_clone.lock().unwrap();
                *data_guard = Some(config_value);
            }
        });

        client.add_listener("application", rust_listener).await;

        // Add JS Listener
        client.add_listener_wasm("application", js_listener).await;

        let cache = client.cache("application").await;

        // Trigger Refresh
        match cache.refresh().await {
            Ok(_) => web_sys::console::log_1(&"WASM Test: Refresh successful".into()), // web_sys::console for logging
            Err(e) => panic!("WASM Test: Cache refresh failed: {:?}", e),
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

        // Verify Listener Was Called using our Rust listener
        let called = *listener_called_flag.lock().unwrap();
        assert!(called, "Listener was not called.");

        // Check if config data was received
        let config_data_guard = received_config_data.lock().unwrap();
        assert!(
            config_data_guard.is_some(),
            "Listener did not receive config data."
        );

        // Verify the content
        if let Some(value) = config_data_guard.as_ref() {
            match value {
                namespace::Namespace::Properties(properties) => {
                    assert_eq!(
                        properties.get_string("stringValue"),
                        Some("string value".to_string())
                    );
                }
                _ => panic!("Expected Properties namespace"),
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_concurrent_namespace_hang_repro() {
        use async_std::task;
        setup();

        let temp_dir = TempDir::new("apollo_hang_test");

        let config = ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            secret: None,
            cache_dir: Some(temp_dir.path().to_str().unwrap().to_string()),
            label: None,
            ip: None,
            allow_insecure_https: None,
            cache_ttl: None,
        };

        let client = Arc::new(Client::new(config));
        let client_in_listener = client.clone();

        let listener_triggered = Arc::new(Mutex::new(false));
        let listener_triggered_in_listener = listener_triggered.clone();

        let listener: EventListener = Arc::new(move |_| {
            let client_in_listener = client_in_listener.clone();
            let listener_triggered_in_listener = listener_triggered_in_listener.clone();
            task::spawn(async move {
                let mut triggered = listener_triggered_in_listener.lock().await;
                if *triggered {
                    // Avoid infinite loops if the listener is called more than once.
                    return;
                }
                *triggered = true;
                drop(triggered);

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
}
