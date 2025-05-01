//! Cache for the Apollo client.

use super::Error;
use serde::de::DeserializeOwned;

/// A cache for a given namespace.
pub struct Cache {}

impl Cache {
    /// Get a configuration from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the configuration for.
    ///
    /// # Returns
    ///
    /// The configuration for the given key.
    pub fn get_config<D: DeserializeOwned>(&self, key: &str) -> Result<D, Error> {
        todo!()
    }
}
