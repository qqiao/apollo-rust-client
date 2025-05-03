use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use cache::Cache;
use client_config::ClientConfig;

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
    handle: Option<thread::JoinHandle<()>>,
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
        let cache = namespaces
            .entry(namespace.to_string())
            .or_insert_with(|| Arc::new(Cache::new(self.client_config.clone(), namespace)));
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
                let mut running = running.read().unwrap();
                if !*running {
                    break;
                }

                let namespaces = namespaces.read().unwrap();
                // Refresh each namespace's cache
                for (namespace, cache) in namespaces.iter() {
                    if let Err(err) = cache.refresh() {
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
mod tests {}
