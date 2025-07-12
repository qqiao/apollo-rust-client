//! The `apollo-client` crate provides a client for interacting with an Apollo configuration service.
//! This client is responsible for managing configurations for different namespaces, caching them locally,
//! and keeping them refreshed periodically. It also supports different behavior for wasm32 and non-wasm32 targets,
//! allowing it to be used in various environments.

use crate::namespace::Namespace;
use async_std::sync::RwLock;
use cache::Cache;
use client_config::ClientConfig;
use log::{debug, error, trace};
use std::{collections::HashMap, sync::Arc, time::Duration};
use wasm_bindgen::prelude::wasm_bindgen;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use wasm_bindgen_futures::spawn_local as spawn;
    } else {
        use async_std::task::spawn as spawn;
    }
}

mod cache;

pub mod client_config;
pub mod namespace;

/// Different types of errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client is already running")]
    AlreadyRunning,

    #[error("Namespace error: {0}")]
    Namespace(#[from] namespace::Error),

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

/// Apollo client.
#[wasm_bindgen]
pub struct Client {
    /// Holds the configuration for the Apollo client, including server address, app ID, etc.
    client_config: ClientConfig,
    /// Stores the caches for different namespaces.
    /// Each namespace has its own `Cache` instance, wrapped in `Arc` for shared ownership
    /// and `RwLock` for thread-safe read/write access to the map of namespaces.
    namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>,
    /// Holds the handle for the background refresh task.
    /// This is `Some` on non-wasm32 targets where a separate task is spawned,
    /// and `None` on wasm32 where `spawn_local` is used without a direct handle.
    handle: Option<async_std::task::JoinHandle<()>>,
    /// Indicates whether the background refresh task is active.
    /// Wrapped in `Arc<RwLock<...>>` for thread-safe shared access to its status.
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
    async fn cache(&self, namespace: &str) -> Arc<Cache> {
        let mut namespaces = self.namespaces.write().await;
        let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
            trace!("Cache miss, creating cache for namespace {namespace}");
            Arc::new(Cache::new(self.client_config.clone(), namespace))
        });
        cache.clone()
    }

    pub async fn add_listener(&self, namespace: &str, listener: EventListener) {
        let mut namespaces = self.namespaces.write().await;
        let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
            trace!("Cache miss, creating cache for namespace {namespace}");
            Arc::new(Cache::new(self.client_config.clone(), namespace))
        });
        cache.add_listener(listener).await;
    }

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
    pub async fn start(&mut self) -> Result<(), Error> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::AlreadyRunning);
        }

        *running = true;

        let running = self.running.clone();
        let namespaces = self.namespaces.clone();

        // Spawn a background thread to refresh caches
        let _handle = spawn(async move {
            loop {
                let running = running.read().await;
                if !*running {
                    break;
                }

                let namespaces = namespaces.read().await;
                // Refresh each namespace's cache
                for (namespace, cache) in namespaces.iter() {
                    if let Err(err) = cache.refresh().await {
                        error!("Failed to refresh cache for namespace {namespace}: {err:?}");
                    } else {
                        debug!("Successfully refreshed cache for namespace {namespace}");
                    }
                }

                // Sleep for 30 seconds before the next refresh
                async_std::task::sleep(Duration::from_secs(30)).await;
            }
        });

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                self.handle = None;
            } else {
                self.handle = Some(_handle);
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
    pub fn new(client_config: ClientConfig) -> Self {
        Self {
            client_config,
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
    /// `function(error, data)`
    /// - `error`: A string describing the error if one occurred during the configuration
    ///            fetch or processing. If the operation was successful, this will be `null`.
    /// - `data`: The JSON configuration object (if the refresh was successful and data
    ///           could be serialized) or `null` if an error occurred or serialization failed.
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

            // Call the JavaScript listener: listener(error, value)
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
    #[wasm_bindgen(js_name = "add_listener")]
    pub async fn namespace_wasm(&self, namespace: &str) -> Result<wasm_bindgen::JsValue, Error> {
        let cache = self.cache(namespace).await;
        let value = cache.get_value().await?;
        Ok(namespace::get_namespace(namespace, value)?.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use lazy_static::lazy_static;

    #[cfg(not(target_arch = "wasm32"))]
    lazy_static! {
        pub(crate) static ref CLIENT_NO_SECRET: Client = {
            let config = ClientConfig {
                app_id: String::from("101010101"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: None,
                secret: None,
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: None,
            };
            Client::new(config)
        };
        pub(crate) static ref CLIENT_WITH_SECRET: Client = {
            let config = ClientConfig {
                app_id: String::from("101010102"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: None,
                secret: Some(String::from("53bf47631db540ac9700f0020d2192c8")),
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: None,
            };
            Client::new(config)
        };
        pub(crate) static ref CLIENT_WITH_GRAYSCALE_IP: Client = {
            let config = ClientConfig {
                app_id: String::from("101010101"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: None,
                secret: None,
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: Some(String::from("1.2.3.4")),
            };
            Client::new(config)
        };
        pub(crate) static ref CLIENT_WITH_GRAYSCALE_LABEL: Client = {
            let config = ClientConfig {
                app_id: String::from("101010101"),
                cluster: String::from("default"),
                config_server: String::from("http://81.68.181.139:8080"),
                label: Some(String::from("GrayScale")),
                secret: None,
                cache_dir: Some(String::from("/tmp/apollo")),
                ip: None,
            };
            Client::new(config)
        };
    }

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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_missing_value() {
        setup();
        let properties = match CLIENT_NO_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };

        assert_eq!(
            properties.get_property::<String>("missingValue").await,
            None
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_missing_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let cache = client.namespace("application");
        assert_eq!(
            cache.await.get_property::<String>("missingValue").await,
            None
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_string_value() {
        setup();
        let properties = match CLIENT_NO_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_string_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let cache = client.namespace("application");
        assert_eq!(
            cache.await.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_string_value_with_secret() {
        setup();
        let properties = match CLIENT_WITH_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_string_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let cache = client.namespace("application");
        assert_eq!(
            cache.await.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_int_value() {
        setup();
        let properties = match CLIENT_NO_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(properties.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_int_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let cache = client.namespace("application");
        assert_eq!(cache.await.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_int_value_with_secret() {
        setup();
        let properties = match CLIENT_WITH_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(properties.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_int_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let cache = client.namespace("application");
        assert_eq!(cache.await.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_float_value() {
        setup();
        let properties = match CLIENT_NO_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_float_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let cache = client.namespace("application");
        assert_eq!(
            cache.await.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_float_value_with_secret() {
        setup();
        let properties = match CLIENT_WITH_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_float_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let cache = client.namespace("application");
        assert_eq!(
            cache.await.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value() {
        setup();
        let properties = match CLIENT_NO_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_wasm() {
        setup();
        let client = create_client_no_secret();
        let cache = client.namespace("application");
        assert_eq!(
            cache.await.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_secret() {
        setup();
        let properties = match CLIENT_WITH_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_secret_wasm() {
        setup();
        let client = create_client_with_secret();
        let cache = client.namespace("application");
        assert_eq!(
            cache.await.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_grayscale_ip() {
        setup();
        let properties = match CLIENT_WITH_GRAYSCALE_IP
            .namespace("application")
            .await
            .unwrap()
        {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );
        let properties = match CLIENT_NO_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_grayscale_ip_wasm() {
        setup();
        let client1 = create_client_with_grayscale_ip();
        let cache = client1.namespace("application");
        assert_eq!(
            cache.await.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );

        let client2 = create_client_no_secret();
        let cache = client2.namespace("application");
        assert_eq!(
            cache.await.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_grayscale_label() {
        setup();
        let properties = match CLIENT_WITH_GRAYSCALE_LABEL
            .namespace("application")
            .await
            .unwrap()
        {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );
        let properties = match CLIENT_NO_SECRET.namespace("application").await.unwrap() {
            namespace::Namespace::Properties(properties) => properties,
            _ => panic!("Expected Properties namespace"),
        };
        assert_eq!(
            properties.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_grayscale_label_wasm() {
        setup();
        let client1 = create_client_with_grayscale_label();
        let cache = client1.namespace("application").await;
        assert_eq!(
            cache.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );

        let client2 = create_client_no_secret();
        let cache = client2.namespace("application").await;
        assert_eq!(
            cache.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
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
        };
        Client::new(config)
    }
}
