use log::debug;

#[derive(Clone, Debug)]
pub struct Properties {
    value: serde_json::Value,
}

impl Properties {
    pub async fn get_property<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        debug!("Getting property for key {key}");

        let value = self.value.get(key)?;
        value.as_str().and_then(|s| s.parse::<T>().ok())
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
        self.get_property::<String>(key).await
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
        self.get_property::<i64>(key).await
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
        self.get_property::<f64>(key).await
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
        self.get_property::<bool>(key).await
    }
}

impl From<serde_json::Value> for Properties {
    fn from(value: serde_json::Value) -> Self {
        Self { value }
    }
}
