use std::sync::Arc;

use log::debug;

use crate::cache::Cache;

#[derive(Clone)]
pub enum Namespace<T> {
    Properties(Properties),
    Json(T),
    Yaml(T),
    Xml(T),
    Text(String),
}

#[derive(Clone)]
pub struct Properties {
    pub(crate) cache: Arc<Cache>,
}

impl Properties {
    pub async fn get_property<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.cache.get_property(key).await
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
    pub async fn get_string(&self, key: &str) -> Option<String> {
        self.cache.get_property::<String>(key).await
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
    pub async fn get_int(&self, key: &str) -> Option<i64> {
        self.cache.get_property::<i64>(key).await
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
    pub async fn get_float(&self, key: &str) -> Option<f64> {
        self.cache.get_property::<f64>(key).await
    }

    /// Get a property from the cache as a boolean.
    //
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a boolean.
    pub async fn get_bool(&self, key: &str) -> Option<bool> {
        self.cache.get_property::<bool>(key).await
    }
}
