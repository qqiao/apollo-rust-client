//! Properties namespace implementation for handling key-value configuration data.
//!
//! This module provides the `Properties` struct, which wraps a `serde_json::Value`,
//! and provides methods for working with properties-style configuration data.
//! Properties format is commonly used for application configuration where data
//! is stored as simple key-value pairs.
//!
//! The `Properties` struct is typically created automatically by the namespace
//! detection system when a namespace name has no file extension (defaulting to
//! properties format), but can also be created directly from any `serde_json::Value`.
//!
//! The properties implementation supports automatic parsing of the following types:
//! - `String` - Text values
//! - `i64` - Integer values
//! - `f64` - Floating-point values
//! - `bool` - Boolean values
//!
//! # Example: `.properties` file format
//!
//! ```properties
//! app.name = MyApplication
//! app.version = 1.0.0
//! app.port = 8080
//! app.debug = true
//! ```

use log::debug;
use wasm_bindgen::prelude::wasm_bindgen;

/// A wrapper around `serde_json::Value` for properties-style configuration data.
///
/// This struct provides a type-safe interface for working with properties configuration
/// data retrieved from Apollo. Properties are typically stored as key-value pairs where
/// all values are strings that can be parsed into various types.
///
/// # Thread Safety
///
/// This struct is `Clone` and `Debug`, making it easy to work with in concurrent
/// environments. The underlying `serde_json::Value` is also thread-safe.
///
/// # Examples
///
/// ```rust,ignore
/// use serde_json::json;
/// use apollo_client::namespace::properties::Properties;
///
/// let props_data = json!({
///     "database.host": "localhost",
///     "database.port": "5432",
///     "database.ssl": "true"
/// });
///
/// let properties = Properties::from(props_data);
///
/// let host = properties.get_string("database.host");
/// assert_eq!(host, Some("localhost".to_string()));
///
/// let port = properties.get_int("database.port");
/// assert_eq!(port, Some(5432));
///
/// let ssl = properties.get_bool("database.ssl");
/// assert_eq!(ssl, Some(true));
/// ```
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Properties {
    /// The underlying JSON value containing the properties data
    value: serde_json::Value,
}

impl Properties {
    /// Gets a property value and attempts to parse it into the specified type.
    ///
    /// This is a generic method that can parse string values into any type that
    /// implements `FromStr`. It first retrieves the value as a string, then
    /// attempts to parse it into the target type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The target type to parse into. Must implement `std::str::FromStr`.
    ///
    /// # Arguments
    ///
    /// * `key` - The property key to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(T)` - The parsed value if the key exists and parsing succeeds
    /// * `None` - If the key doesn't exist, the value is not a string, or parsing fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    /// use apollo_client::namespace::properties::Properties;
    ///
    /// let props_data = json!({"timeout": "30", "retries": "3"});
    /// let properties = Properties::from(props_data);
    ///
    /// let timeout: Option<u32> = properties.get_property("timeout");
    /// assert_eq!(timeout, Some(30));
    ///
    /// let retries: Option<i32> = properties.get_property("retries");
    /// assert_eq!(retries, Some(3));
    /// ```
    #[must_use]
    pub fn get_property<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        debug!("Getting property for key {key}");

        let value = self.value.get(key)?;
        value.as_str().and_then(|s| s.parse::<T>().ok())
    }
}

#[wasm_bindgen]
impl Properties {
    /// Get a property from the cache as a string.
    ///
    /// This method retrieves a property value and returns it as a `String`.
    /// It's a convenience method that wraps `get_property::<String>()`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The property value as a string if it exists
    /// * `None` - If the key doesn't exist or the value cannot be converted to a string
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    /// use apollo_client::namespace::properties::Properties;
    ///
    /// let props_data = json!({"app.name": "MyApp"});
    /// let properties = Properties::from(props_data);
    ///
    /// let app_name = properties.get_string("app.name");
    /// assert_eq!(app_name, Some("MyApp".to_string()));
    /// ```
    #[must_use]
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get_property::<String>(key)
    }

    /// Get a property from the cache as an integer.
    ///
    /// This method retrieves a property value and attempts to parse it as an `i64`.
    /// It's a convenience method that wraps `get_property::<i64>()`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// * `Some(i64)` - The property value as an integer if it exists and can be parsed
    /// * `None` - If the key doesn't exist or the value cannot be parsed as an integer
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    /// use apollo_client::namespace::properties::Properties;
    ///
    /// let props_data = json!({"server.port": "8080"});
    /// let properties = Properties::from(props_data);
    ///
    /// let port = properties.get_int("server.port");
    /// assert_eq!(port, Some(8080));
    /// ```
    #[must_use]
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get_property::<i64>(key)
    }

    /// Get a property from the cache as a float.
    ///
    /// This method retrieves a property value and attempts to parse it as an `f64`.
    /// It's a convenience method that wraps `get_property::<f64>()`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// * `Some(f64)` - The property value as a float if it exists and can be parsed
    /// * `None` - If the key doesn't exist or the value cannot be parsed as a float
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    /// use apollo_client::namespace::properties::Properties;
    ///
    /// let props_data = json!({"timeout.seconds": "30.5"});
    /// let properties = Properties::from(props_data);
    ///
    /// let timeout = properties.get_float("timeout.seconds");
    /// assert_eq!(timeout, Some(30.5));
    /// ```
    #[must_use]
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get_property::<f64>(key)
    }

    /// Get a property from the cache as a boolean.
    ///
    /// This method retrieves a property value and attempts to parse it as a `bool`.
    /// It's a convenience method that wraps `get_property::<bool>()`. The parsing
    /// follows Rust's standard boolean parsing rules (accepts "true"/"false").
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// * `Some(bool)` - The property value as a boolean if it exists and can be parsed
    /// * `None` - If the key doesn't exist or the value cannot be parsed as a boolean
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    /// use apollo_client::namespace::properties::Properties;
    ///
    /// let props_data = json!({"debug.enabled": "true"});
    /// let properties = Properties::from(props_data);
    ///
    /// let debug_enabled = properties.get_bool("debug.enabled");
    /// assert_eq!(debug_enabled, Some(true));
    /// ```
    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_property::<bool>(key)
    }
}

/// Converts a `serde_json::Value` into a `Properties` instance.
///
/// This implementation allows for easy creation of `Properties` instances from
/// raw JSON data, typically used by the namespace detection system. The JSON
/// value is expected to be an object where keys are property names and values
/// are the property values (typically strings).
///
/// # Examples
///
/// ```rust,ignore
/// use serde_json::json;
/// use apollo_client::namespace::properties::Properties;
///
/// let props_data = json!({
///     "app.name": "MyApp",
///     "app.version": "1.0.0"
/// });
///
/// let properties = Properties::from(props_data);
///
/// let app_name = properties.get_string("app.name");
/// assert_eq!(app_name, Some("MyApp".to_string()));
///
/// let version = properties.get_string("app.version");
/// assert_eq!(version, Some("1.0.0".to_string()));
/// ```
impl From<serde_json::Value> for Properties {
    fn from(value: serde_json::Value) -> Self {
        Self { value }
    }
}
