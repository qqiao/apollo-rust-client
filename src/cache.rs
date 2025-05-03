//! Cache for the Apollo client.

use crate::client_config::ClientConfig;
use serde_json::Value;
use std::{fs::File, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

/// A cache for a given namespace.
pub struct Cache {
    client_config: ClientConfig,
    file_path: PathBuf,
}

impl Cache {
    /// Create a new cache.
    ///
    /// # Arguments
    ///
    /// * `client_config` - The configuration for the Apollo client.
    /// * `namespace` - The namespace to get the cache for.
    ///
    /// # Returns
    ///
    /// A new cache for the given namespace.
    pub(crate) fn new(client_config: ClientConfig, namespace: &str) -> Self {
        let file_path = client_config
            .cache_dir
            .clone()
            .join(format!("{}.cache.json", namespace));
        Self {
            client_config,
            file_path,
        }
    }

    /// Get a configuration from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the configuration for.
    ///
    /// # Returns
    ///
    /// The configuration for the given key.
    fn get_value(&self, key: &str) -> Result<Value, Error> {
        let file_path = self.file_path.clone();
        let file = File::open(file_path)?;
        let config: Value = serde_json::from_reader(file)?;
        let value = config.get(key).ok_or(Error::KeyNotFound(key.to_string()))?;
        Ok(value.clone())
    }

    /// Get a property from the cache as a string.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a string.
    pub fn get_property(&self, key: &str) -> Option<String> {
        let value = self.get_value(key).ok()?;
        value.as_str().map(|s| s.to_string())
    }

    /// Get a property from the cache as a float.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a float.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        let value = self.get_value(key).ok()?;
        value.as_f64()
    }

    /// Get a property from the cache as an integer.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as an integer.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        let value = self.get_value(key).ok()?;
        value.as_i64()
    }

    /// Get a property from the cache as a boolean.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a boolean.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        let value = self.get_value(key).ok()?;
        value.as_bool()
    }

    /// Get a property from the cache as an array.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as an array of values.
    pub fn get_array(&self, key: &str) -> Option<Vec<Value>> {
        let value = self.get_value(key).ok()?;
        value.as_array().cloned()
    }
}
