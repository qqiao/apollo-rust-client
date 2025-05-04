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

pub mod cache;
pub mod client_config;

/// Different types of errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client is already running")]
    AlreadyRunning,
}

/// Apollo client.
pub struct Client {
    client_config: ClientConfig,
    namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>,
    handle: Option<JoinHandle<()>>,
    running: Arc<RwLock<bool>>,
}

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
    pub fn new(client_config: ClientConfig) -> Self {
        Self {
            client_config,
            namespaces: Arc::new(RwLock::new(HashMap::new())),
            handle: None,
            running: Arc::new(RwLock::new(false)),
        }
    }

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

impl Drop for Client {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_config_no_secret() -> ClientConfig {
        ClientConfig {
            app_id: String::from("101010101"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            label: None,
            secret: None,
            cache_dir: Some(PathBuf::from("/tmp/apollo")),
        }
    }

    fn get_config_with_secret() -> ClientConfig {
        ClientConfig {
            app_id: String::from("101010102"),
            cluster: String::from("default"),
            config_server: String::from("http://81.68.181.139:8080"),
            label: None,
            secret: Some(String::from("53bf47631db540ac9700f0020d2192c8")),
            cache_dir: Some(PathBuf::from("/tmp/apollo")),
        }
    }

    pub(crate) fn setup() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_missing_value() {
        setup();
        let config = get_config_no_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(cache.get_property::<String>("missingValue").await, None);
    }

    #[tokio::test]
    async fn test_string_value() {
        setup();
        let config = get_config_no_secret();

        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[tokio::test]
    async fn test_string_value_with_secret() {
        setup();
        let config = get_config_with_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(
            cache.get_property::<String>("stringValue").await,
            Some("string value".to_string())
        );
    }

    #[tokio::test]
    async fn test_int_value() {
        setup();
        let config = get_config_no_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[tokio::test]
    async fn test_int_value_with_secret() {
        setup();
        let config = get_config_with_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(cache.get_property::<i32>("intValue").await, Some(42));
    }

    #[tokio::test]
    async fn test_float_value() {
        setup();
        let config = get_config_no_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(cache.get_property::<f64>("floatValue").await, Some(4.20));
    }

    #[tokio::test]
    async fn test_float_value_with_secret() {
        setup();
        let config = get_config_with_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(cache.get_property::<f64>("floatValue").await, Some(4.20));
    }

    #[tokio::test]
    async fn test_bool_value() {
        setup();
        let config = get_config_no_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(cache.get_property::<bool>("boolValue").await, Some(false));
    }

    #[tokio::test]
    async fn test_bool_value_with_secret() {
        setup();
        let config = get_config_with_secret();
        let client = Client::new(config);
        let cache = client.namespace("application");
        assert_eq!(cache.get_property::<bool>("boolValue").await, Some(false));
    }
}
