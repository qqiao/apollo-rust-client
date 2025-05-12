use cache::Cache;
use client_config::ClientConfig;
use futures::executor::block_on;
use log::trace;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::Duration,
};
use wasm_bindgen::prelude::wasm_bindgen;

pub mod cache;
pub mod client_config;

/// Different types of errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client is already running")]
    AlreadyRunning,
}

/// Apollo client.
#[wasm_bindgen]
pub struct Client {
    client_config: ClientConfig,
    namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>,
    handle: Option<JoinHandle<()>>,
    running: Arc<RwLock<bool>>,
}

impl Client {
    pub fn start(&mut self) -> Result<(), Error> {
        let mut running = self.running.write().unwrap();
        if *running {
            return Err(Error::AlreadyRunning);
        }

        *running = true;

        let running = self.running.clone();
        let namespaces = self.namespaces.clone();
        // Spawn a background thread to refresh caches
        let handle = thread::spawn(move || {
            loop {
                let running = running.read().unwrap();
                if !*running {
                    break;
                }

                let namespaces = namespaces.read().unwrap();
                // Refresh each namespace's cache
                for (namespace, cache) in namespaces.iter() {
                    if let Err(err) = block_on(cache.refresh()) {
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
                thread::sleep(Duration::from_secs(30));
            }
        });

        self.handle = Some(handle);

        Ok(())
    }

    pub fn stop(&mut self) {
        let mut running = self.running.write().unwrap();
        *running = false;
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
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
            pub fn namespace(&self, namespace: &str) -> Cache {
                let mut namespaces = self.namespaces.write().unwrap();
                let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
                    trace!("Cache miss, creating cache for namespace {}", namespace);
                    Arc::new(Cache::new(self.client_config.clone(), namespace))
                });
                let cache = (*cache).clone();
                (*cache).to_owned()
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
            pub fn namespace(&self, namespace: &str) -> Arc<Cache> {
                let mut namespaces = self.namespaces.write().unwrap();
                let cache = namespaces.entry(namespace.to_string()).or_insert_with(|| {
                    trace!("Cache miss, creating cache for namespace {}", namespace);
                    Arc::new(Cache::new(self.client_config.clone(), namespace))
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
            namespaces: Arc::new(RwLock::new(HashMap::new())),
            handle: None,
            running: Arc::new(RwLock::new(false)),
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use wasm_bindgen_test::wasm_bindgen_test;

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

    #[tokio::test]
    async fn test_missing_value() {
        setup();
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(cache.get_property::<String>("missingValue").await, None);
    }

    #[tokio::test]
    async fn test_string_value() {
        setup();
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[wasm_bindgen_test]
    async fn test_string_value_wasm() {
        setup();
        console_error_panic_hook::set_once();
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(
            cache.get_string("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[tokio::test]
    async fn test_string_value_with_secret() {
        setup();
        console_error_panic_hook::set_once();
        let cache = CLIENT_WITH_SECRET.namespace("application");
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[tokio::test]
    async fn test_int_value() {
        setup();
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[tokio::test]
    async fn test_int_value_with_secret() {
        setup();
        let cache = CLIENT_WITH_SECRET.namespace("application");
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[tokio::test]
    async fn test_float_value() {
        setup();
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(cache.get_property::<f64>("floatValue").await, Some(4.20));
    }

    #[tokio::test]
    async fn test_float_value_with_secret() {
        setup();
        let cache = CLIENT_WITH_SECRET.namespace("application");
        assert_eq!(cache.get_property::<f64>("floatValue").await, Some(4.20));
    }

    #[tokio::test]
    async fn test_bool_value() {
        setup();
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(cache.get_property::<bool>("boolValue").await, Some(false));
    }

    #[tokio::test]
    async fn test_bool_value_with_secret() {
        setup();
        let cache = CLIENT_WITH_SECRET.namespace("application");
        assert_eq!(cache.get_property::<bool>("boolValue").await, Some(false));
    }

    #[tokio::test]
    async fn test_bool_value_with_grayscale_ip() {
        setup();
        let cache = CLIENT_WITH_GRAYSCALE_IP.namespace("application");
        assert_eq!(
            cache.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(
            cache.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }

    #[tokio::test]
    async fn test_bool_value_with_grayscale_label() {
        setup();
        let cache = CLIENT_WITH_GRAYSCALE_LABEL.namespace("application");
        assert_eq!(
            cache.get_property::<bool>("grayScaleValue").await,
            Some(true)
        );
        let cache = CLIENT_NO_SECRET.namespace("application");
        assert_eq!(
            cache.get_property::<bool>("grayScaleValue").await,
            Some(false)
        );
    }
}
