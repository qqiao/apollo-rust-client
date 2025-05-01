use cache::Cache;
use config::Config;

pub mod cache;
pub mod config;

/// Different types of errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

/// Apollo client.
pub struct Client {
    config: Config,
}

impl Client {
    /// Create a new Apollo client.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the Apollo client.
    ///
    /// # Returns
    ///
    /// A new Apollo client.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Get a cache for a given namespace.
    ///
    /// # Arguments
    ///
    /// * `name_space` - The namespace to get the cache for.
    ///
    /// # Returns
    ///
    /// A cache for the given namespace.
    pub fn namespace(&self, name_space: &str) -> &Cache {
        todo!()
    }
}

#[cfg(test)]
mod tests {}
