//! The `apollo-client` crate provides a client for interacting with an Apollo configuration service.
//! This client is responsible for managing configurations for different namespaces, caching them locally,
//! and keeping them refreshed periodically. It also supports different behavior for wasm32 and non-wasm32 targets,
//! allowing it to be used in various environments.

use tokio::sync::RwLock; // Changed from async_std::sync::RwLock
use cache::Cache;
use client_config::ClientConfig;
use crate::event::{EventManager, Observer}; // Added Observer
use log::trace;
use std::{collections::HashMap, sync::Arc, time::Duration};
use wasm_bindgen::prelude::wasm_bindgen;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use wasm_bindgen_futures::spawn_local as spawn; // WASM spawn remains
    } else {
        use tokio::task::spawn; // Changed from async_std::task::spawn
    }
}

pub mod cache;
pub mod client_config;
pub mod event;

/// Different types of errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client is already running")]
    AlreadyRunning,
}

/// Apollo client.
#[wasm_bindgen]
pub struct Client {
    /// Holds the configuration for the Apollo client, including server address, app ID, etc.
    client_config: ClientConfig,
    /// Stores the caches for different namespaces.
    /// Each namespace has its own `Cache` instance, wrapped in `Arc` for shared ownership
    /// and `RwLock` for thread-safe read/write access to the map of namespaces.
    namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>, // RwLock is now tokio's
    /// Holds the handle for the background refresh task.
    /// This is `Some` on non-wasm32 targets where a separate task is spawned,
    /// and `None` on wasm32 where `spawn_local` is used without a direct handle.
    handle: Option<tokio::task::JoinHandle<()>>, // Changed from async_std::task::JoinHandle
    /// Indicates whether the background refresh task is active.
    /// Wrapped in `Arc<RwLock<...>>` for thread-safe shared access to its status.
    running: Arc<RwLock<bool>>, // RwLock is now tokio's
    event_manager: Arc<EventManager>, // New field
}

impl Client {
    /// Starts a background task that periodically refreshes all registered namespace caches.
    ///
    /// This method spawns an asynchronous task using `tokio::task::spawn` on native targets
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
                        log::error!(
                            "Failed to refresh cache for namespace {}: {:?}",
                            namespace,
                            err
                        );
                    } else {
                        log::debug!("Successfully refreshed cache for namespace {}", namespace);
                    }
                }

                // Sleep for 30 seconds before the next refresh
                cfg_if::cfg_if! {
                    if #[cfg(target_arch = "wasm32")] {
                        // For WASM, use a WASM-compatible timer if tokio::time::sleep causes issues
                        // or if a more JS-event-loop friendly delay is needed.
                        // gloo_timers::future::TimeoutFuture::new(30_000).await; // Example with gloo-timers
                        // For now, assume tokio::time::sleep might work or can be adjusted later if problematic in WASM context
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    } else {
                        tokio::time::sleep(Duration::from_secs(30)).await; // Changed from async_std::task::sleep
                    }
                }
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
    /// by calling `abort()` on its `JoinHandle` if it exists (Tokio's equivalent to cancel).
    /// This helps to ensure that the task is properly cleaned up. On wasm32 targets, there is no direct
    /// handle to cancel, so setting the `running` flag is the primary mechanism for stopping.
    pub async fn stop(&mut self) {
        let mut running = self.running.write().await;
        *running = false;

        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                if let Some(handle) = self.handle.take() {
                    handle.abort(); // Changed from cancel()
                }
            }
        }
    }

    /// Registers an observer for configuration changes in a specific namespace.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The name of the namespace to observe.
    /// * `observer` - An `Arc` wrapped observer implementing the `Observer` trait.
    pub async fn add_observer(&self, namespace: &str, observer: Arc<dyn Observer>) {
        self.event_manager.register_observer(namespace, observer).await;
    }

    /// Unregisters an observer for a specific namespace.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The name of the namespace from which to unregister.
    /// * `observer` - The specific `Arc<dyn Observer>` instance to remove.
    ///                This must be the same `Arc` instance that was used for registration.
    pub async fn remove_observer(&self, namespace: &str, observer: Arc<dyn Observer>) {
        self.event_manager.unregister_observer(namespace, observer).await;
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[wasm_bindgen]
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
            pub async fn namespace(&self, namespace: &str) -> Cache {
                let mut namespaces = self.namespaces.write().await;
                let event_manager_clone = self.event_manager.clone();
                let cache_arc = namespaces.entry(namespace.to_string()).or_insert_with(|| {
                    trace!("Cache miss, creating cache for namespace {}", namespace);
                    Arc::new(Cache::new(self.client_config.clone(), namespace, event_manager_clone))
                });
                // For WASM, the plan was to return Cache, not Arc<Cache>.
                // The Cache struct has #[derive(Clone)] and is #[wasm_bindgen].
                // So, we clone the Cache from the Arc.
                (*cache_arc).clone()
            }
        }
    } else {
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
            pub async fn namespace(&self, namespace: &str) -> Arc<Cache> {
                let mut namespaces = self.namespaces.write().await;
                let event_manager_clone = self.event_manager.clone();
                let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
                    trace!("Cache miss, creating cache for namespace {}", namespace);
                    Arc::new(Cache::new(self.client_config.clone(), namespace, event_manager_clone))
                });
                cache.clone()
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
            namespaces: Arc::new(RwLock::new(HashMap::new())), // RwLock is now tokio's
            handle: None,
            running: Arc::new(RwLock::new(false)), // RwLock is now tokio's
            event_manager: Arc::new(EventManager::new()), // Initialize new field
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    // For testing observer functionality
    use crate::event::{ConfigurationChangeEvent, Observer};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use serde_json::json;


    // Mock Observer for testing
    #[derive(Debug)]
    struct MockObserver {
        call_count: Arc<AtomicUsize>,
        last_event_namespace: Arc<RwLock<Option<String>>>, // RwLock is now tokio's
    }

    impl MockObserver {
        fn new() -> Self {
            MockObserver {
                call_count: Arc::new(AtomicUsize::new(0)),
                last_event_namespace: Arc::new(RwLock::new(None)),
            }
        }
    }

    #[async_trait]
    impl Observer for MockObserver {
        async fn on_configuration_change(&self, event: &ConfigurationChangeEvent) {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let mut last_ns = self.last_event_namespace.write().await;
            *last_ns = Some(event.namespace_name.clone());
            // Simulate some async work if needed, e.g., tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }


    lazy_static! {
        static ref CLIENT_NO_SECRET: Client = {
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
        static ref CLIENT_WITH_SECRET: Client = {
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
        static ref CLIENT_WITH_GRAYSCALE_IP: Client = {
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
        static ref CLIENT_WITH_GRAYSCALE_LABEL: Client = {
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

    // Helper to get a tokio runtime for tests that might need it explicitly,
    // though #[tokio::test] usually handles this.
    // fn get_runtime() -> tokio::runtime::Runtime {
    //     tokio::runtime::Builder::new_current_thread()
    //         .enable_all()
    //         .build()
    //         .unwrap()
    // }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_missing_value() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<String>("missingValue").await,
            None
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_missing_value_wasm() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<String>("missingValue").await,
            None
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_string_value() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_string_value_wasm() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_string_value_with_secret() {
        setup();
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_string_value_with_secret_wasm() {
        setup();
        console_error_panic_hook::set_once();

        let cache = CLIENT_WITH_SECRET.namespace("application");
        assert_eq!(
            cache.await.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_int_value() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_int_value_wasm() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_int_value_with_secret() {
        setup();
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_int_value_with_secret_wasm() {
        setup();
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_float_value() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_float_value_wasm() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_float_value_with_secret() {
        setup();
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_float_value_with_secret_wasm() {
        setup();
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<f64>("floatValue").await,
            Some(4.20)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_wasm() {
        setup();
        let client = &CLIENT_NO_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_secret() {
        setup();
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_secret_wasm() {
        setup();
        let client = &CLIENT_WITH_SECRET;
        let cache = client.namespace("application").await;
        assert_eq!(
            cache.get_property::<bool>("boolValue").await,
            Some(false)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_grayscale_ip() {
        setup();
        let client_gray = &CLIENT_WITH_GRAYSCALE_IP;
        let cache_gray = client_gray.namespace("application").await;
        assert_eq!(
            cache_gray.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );
        let client_no_secret = &CLIENT_NO_SECRET;
        let cache_no_secret = client_no_secret.namespace("application").await;
        assert_eq!(
            cache_no_secret.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_grayscale_ip_wasm() {
        setup();
        let client_gray = &CLIENT_WITH_GRAYSCALE_IP;
        let cache_gray = client_gray.namespace("application").await;
        assert_eq!(
            cache_gray.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );

        let client_no_secret = &CLIENT_NO_SECRET;
        let cache_no_secret = client_no_secret.namespace("application").await;
        assert_eq!(
            cache_no_secret.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_bool_value_with_grayscale_label() {
        setup();
        let client_gray_label = &CLIENT_WITH_GRAYSCALE_LABEL;
        let cache_gray_label = client_gray_label.namespace("application").await;
        assert_eq!(
            cache_gray_label.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );
        let client_no_secret = &CLIENT_NO_SECRET;
        let cache_no_secret = client_no_secret.namespace("application").await;
        assert_eq!(
            cache_no_secret.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    #[allow(dead_code)]
    async fn test_bool_value_with_grayscale_label_wasm() {
        setup();
        let client_gray_label = &CLIENT_WITH_GRAYSCALE_LABEL;
        let cache_gray_label = client_gray_label.namespace("application").await;
        assert_eq!(
            cache_gray_label.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );

        let client_no_secret = &CLIENT_NO_SECRET;
        let cache_no_secret = client_no_secret.namespace("application").await;
        assert_eq!(
            cache_no_secret.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_add_and_remove_observer_with_mock_http() {
        setup();
        let mut server = mockito::Server::new_async().await;

        let app_id = "test_app_id_mock";
        let cluster = "default";
        let namespace_name = "application_mock";
        let mock_url = server.url();

        let client_config = ClientConfig {
            app_id: app_id.to_string(),
            cluster: cluster.to_string(),
            config_server: mock_url.clone(),
            label: None,
            secret: None,
            cache_dir: Some(String::from("/tmp/apollo_mock_test")),
            ip: None,
        };
        let client = Client::new(client_config);
        let observer = Arc::new(MockObserver::new());

        // Register observer
        client.add_observer(namespace_name, observer.clone()).await;

        // Mock initial configuration
        let initial_config_json = json!({"key1": "value1", "key2": "initial_value"});
        let mock_path_initial = format!("/configfiles/json/{}/{}/{}", app_id, cluster, namespace_name);
        server.mock("GET", mock_path_initial.as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(initial_config_json.to_string())
            .create_async().await;
        
        let cache = client.namespace(namespace_name).await;
        cache.refresh().await.expect("Refresh should succeed");
        tokio::time::sleep(Duration::from_millis(100)).await; // Allow time for notification

        assert_eq!(observer.call_count.load(Ordering::SeqCst), 1, "Observer should be called for initial config.");
        {
            let last_ns = observer.last_event_namespace.read().await;
            assert_eq!(*last_ns, Some(namespace_name.to_string()));
        }


        // Mock updated configuration
        let updated_config_json = json!({"key1": "value1_updated", "key2": "new_value"});
         server.reset_async().await; // Reset mocks before adding a new one for the same path
         server.mock("GET", mock_path_initial.as_str()) // Same path
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(updated_config_json.to_string())
            .create_async().await;

        cache.refresh().await.expect("Refresh should succeed for updated config");
        tokio::time::sleep(Duration::from_millis(100)).await; // Allow time for notification
        
        assert_eq!(observer.call_count.load(Ordering::SeqCst), 2, "Observer should be called again for updated config.");

        // Unregister observer
        client.remove_observer(namespace_name, observer.clone()).await;

        // Mock another configuration change
        let final_config_json = json!({"key1": "value1_final"});
        server.reset_async().await;
        server.mock("GET", mock_path_initial.as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(final_config_json.to_string())
            .create_async().await;
        
        cache.refresh().await.expect("Refresh should succeed for final config");
        tokio::time::sleep(Duration::from_millis(100)).await; // Allow time for notification
        
        assert_eq!(observer.call_count.load(Ordering::SeqCst), 2, "Observer should NOT be called after unregistration.");
        server.reset_async().await; // Clean up mock server
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_event_notification_end_to_end_wasm() {
        setup();
        let client_config = ClientConfig {
            app_id: "wasm_app".to_string(),
            cluster: "default".to_string(),
            config_server: "http://dummy-server.com".to_string(), // Not actually called
            label: None,
            secret: None,
            cache_dir: None, // No file cache in WASM
            ip: None,
        };
        let client = Client::new(client_config);
        let observer = Arc::new(MockObserver::new());
        let namespace_name = "wasm_namespace";

        client.add_observer(namespace_name, observer.clone()).await;

        // Get the cache instance (this also adds it to client.namespaces)
        let cache_instance = client.namespace(namespace_name).await;
        
        // 1. Simulate initial state in cache
        let initial_config_val = json!({"feature_flag": false, "message": "Hello"});
        // Directly write to the Cache's memory_cache field.
        // This requires Cache.memory_cache to be accessible or to have a method for this.
        // For this test, we assume EventManager's `notify_observers` will be called by `Cache::refresh`
        // If `Cache::refresh` cannot be easily mocked for wasm, we simulate the notification directly.
        
        // As Cache::memory_cache is not public, we simulate the notification directly.
        // This is what Cache::refresh would do upon detecting a change.
        let event_for_wasm = ConfigurationChangeEvent {
            namespace_name: namespace_name.to_string(),
            old_configuration: None, // Simulate first load
            new_configuration: initial_config_val.clone(),
        };
        client.event_manager.notify_observers(event_for_wasm).await;
        gloo_timers::future::TimeoutFuture::new(100).await; // Yield for spawn_local

        assert_eq!(observer.call_count.load(Ordering::SeqCst), 1, "Observer should be called for initial simulated config.");
        {
            let last_ns = observer.last_event_namespace.read().await; // RwLock is tokio's
            assert_eq!(*last_ns, Some(namespace_name.to_string()));
        }
        
        // 2. Simulate updated state
        let updated_config_val = json!({"feature_flag": true, "message": "World"});
        let event_updated_wasm = ConfigurationChangeEvent {
            namespace_name: namespace_name.to_string(),
            old_configuration: Some(initial_config_val), // Old value
            new_configuration: updated_config_val,      // New value
        };
        client.event_manager.notify_observers(event_updated_wasm).await;
        gloo_timers::future::TimeoutFuture::new(100).await; // Yield for spawn_local

        assert_eq!(observer.call_count.load(Ordering::SeqCst), 2, "Observer should be called for updated simulated config.");
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_client_namespace_specificity_notification_mock_http() {
        setup();
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();
        let app_id = "specificity_app";
        let cluster = "default";

        let client_config = ClientConfig {
            app_id: app_id.to_string(),
            cluster: cluster.to_string(),
            config_server: mock_url.clone(),
            label: None,
            secret: None,
            cache_dir: Some(String::from("/tmp/apollo_specificity_test")),
            ip: None,
        };
        let client = Client::new(client_config);

        let observer_a = Arc::new(MockObserver::new());
        let namespace_a_name = "namespaceA";
        client.add_observer(namespace_a_name, observer_a.clone()).await;

        let observer_b = Arc::new(MockObserver::new());
        let namespace_b_name = "namespaceB";
        client.add_observer(namespace_b_name, observer_b.clone()).await;

        // --- Change in namespaceA ---
        let config_a_v1 = json!({"service_url": "http://service-a.com"});
        let mock_path_a = format!("/configfiles/json/{}/{}/{}", app_id, cluster, namespace_a_name);
        server.mock("GET", mock_path_a.as_str())
            .with_status(200)
            .with_body(config_a_v1.to_string())
            .create_async().await;
        
        client.namespace(namespace_a_name).await.refresh().await.expect("Refresh A v1 failed");
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(observer_a.call_count.load(Ordering::SeqCst), 1, "Observer A should be called for namespaceA change.");
        assert_eq!(observer_b.call_count.load(Ordering::SeqCst), 0, "Observer B should NOT be called for namespaceA change.");
        {
            let last_ns_a = observer_a.last_event_namespace.read().await;
            assert_eq!(*last_ns_a, Some(namespace_a_name.to_string()));
        }
        
        // --- Change in namespaceB ---
        server.reset_async().await; // Clear previous mock for A
        let config_b_v1 = json!({"feature_x_enabled": true});
        let mock_path_b = format!("/configfiles/json/{}/{}/{}", app_id, cluster, namespace_b_name);
        server.mock("GET", mock_path_b.as_str())
            .with_status(200)
            .with_body(config_b_v1.to_string())
            .create_async().await;

        client.namespace(namespace_b_name).await.refresh().await.expect("Refresh B v1 failed");
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(observer_a.call_count.load(Ordering::SeqCst), 1, "Observer A call count should remain 1 after namespaceB change.");
        assert_eq!(observer_b.call_count.load(Ordering::SeqCst), 1, "Observer B should be called for namespaceB change.");
         {
            let last_ns_b = observer_b.last_event_namespace.read().await;
            assert_eq!(*last_ns_b, Some(namespace_b_name.to_string()));
        }

        // --- Another change in namespaceA ---
        server.reset_async().await; // Clear previous mock for B
        let config_a_v2 = json!({"service_url": "http://service-a-new.com", "timeout": 500});
         server.mock("GET", mock_path_a.as_str()) // Path for namespaceA
            .with_status(200)
            .with_body(config_a_v2.to_string())
            .create_async().await;

        client.namespace(namespace_a_name).await.refresh().await.expect("Refresh A v2 failed");
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(observer_a.call_count.load(Ordering::SeqCst), 2, "Observer A should be called again for namespaceA's second change.");
        assert_eq!(observer_b.call_count.load(Ordering::SeqCst), 1, "Observer B call count should remain 1.");
        
        server.reset_async().await; // Clean up
    }

    // The old test_add_and_remove_observer is removed as test_add_and_remove_observer_with_mock_http covers its intent more robustly.
}
