use std::{collections::HashMap, sync::Arc};

use cache::Cache;
use client_config::ClientConfig;

pub mod cache;
pub mod client_config;

/// Different types of errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

/// Apollo client.
pub struct Client {
    client_config: ClientConfig,
    namespaces: HashMap<String, Arc<Cache>>,
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
            namespaces: HashMap::new(),
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
    pub fn namespace(&mut self, namespace: &str) -> Arc<Cache> {
        let cache = self
            .namespaces
            .entry(namespace.to_string())
            .or_insert_with(|| Arc::new(Cache::new(self.client_config.clone(), namespace)));
        cache.clone()
    }
}

#[cfg(test)]
mod tests {}
