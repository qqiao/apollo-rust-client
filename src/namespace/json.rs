//! JSON namespace implementation for handling structured JSON configuration data.
//!
//! This module provides the `Json` struct which wraps a `serde_json::Value` and
//! provides methods for working with JSON-formatted configuration data. It supports
//! deserialization into custom types and maintains the original JSON structure.
//!
//! # Usage
//!
//! The `Json` struct is typically created automatically by the namespace detection
//! system when a namespace name contains a `.json` extension, but can also be
//! created directly from any `serde_json::Value`.
//!
//! # Examples
//!
//! ```ignore
//! use serde_json::json;
//! use apollo_client::namespace::json::Json;
//!
//! let json_data = json!({"name": "MyApp", "version": "1.0.0"});
//! let json_namespace = Json::from(json_data);
//! ```

use log::trace;
use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to get content from JSON value")]
    ContentNotFound,

    #[error("Failed to deserialize JSON value: {0}")]
    DeserializeError(#[from] serde_json::Error),
}

/// A wrapper around `serde_json::Value` for JSON-formatted configuration data.
///
/// This struct provides a type-safe interface for working with JSON configuration
/// data retrieved from Apollo. It maintains the original JSON structure while
/// providing convenient methods for deserialization into custom types.
///
/// # Thread Safety
///
/// This struct is `Clone` and `Debug`, making it easy to work with in concurrent
/// environments. The underlying `serde_json::Value` is also thread-safe.
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use apollo_client::namespace::json::Json;
///
/// let json_data = json!({
///     "database": {
///         "host": "localhost",
///         "port": 5432
///     }
/// });
///
/// let json_namespace = Json::from(json_data);
/// ```
#[derive(Clone, Debug)]
pub struct Json {
    /// The underlying JSON value containing the configuration data
    value: serde_json::Value,
}

impl From<Json> for wasm_bindgen::JsValue {
    fn from(val: Json) -> Self {
        serde_wasm_bindgen::to_value(&val.value).unwrap()
    }
}

impl Json {
    /// Deserializes the JSON data into a custom type.
    ///
    /// This method attempts to deserialize the stored JSON value into any type
    /// that implements `DeserializeOwned`. This is useful for converting the
    /// raw JSON configuration into strongly-typed structs.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The target type to deserialize into. Must implement `DeserializeOwned`.
    ///
    /// # Returns
    ///
    /// An instance of type `T` containing the deserialized data.
    ///
    /// # Panics
    ///
    /// This method currently panics (via `todo!()`) as the implementation is not
    /// yet complete. In the future, it will return a `Result` type to handle
    /// deserialization errors gracefully.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use serde::{Deserialize, Serialize};
    /// use serde_json::json;
    /// use apollo_client::namespace::json::Json;
    ///
    /// #[derive(Deserialize, Serialize)]
    /// struct DatabaseConfig {
    ///     host: String,
    ///     port: u16,
    /// }
    ///
    /// let json_data = json!({
    ///     "host": "localhost",
    ///     "port": 5432
    /// });
    ///
    /// let json_namespace = Json::from(json_data);
    /// // let config: DatabaseConfig = json_namespace.to_object(); // Will work when implemented
    /// ```
    pub fn to_object<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.value.clone())
    }
}

/// Converts a `serde_json::Value` into a `Json` instance.
///
/// This implementation allows for easy creation of `Json` instances from
/// raw JSON data, typically used by the namespace detection system.
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use apollo_client::namespace::json::Json;
///
/// let json_data = json!({"key": "value"});
/// let json_namespace = Json::from(json_data);
/// ```
impl TryFrom<serde_json::Value> for Json {
    type Error = crate::namespace::json::Error;

    fn try_from(json_value: serde_json::Value) -> Result<Self, Self::Error> {
        let content_string = match json_value.get("content") {
            Some(serde_json::Value::String(s)) => s,
            None => return Err(Error::ContentNotFound),
            _ => return Err(Error::ContentNotFound),
        };
        trace!("content_string: {content_string:?}");
        let value = serde_json::from_str(content_string.as_str())?;
        trace!("value: {value:?}");
        Ok(Self { value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        namespace::Namespace,
        tests::{CLIENT_NO_SECRET, setup},
    };
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        host: String,
        port: u16,
        run: bool,
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_json_to_object() {
        setup();
        let json_namespace = Json::try_from(json!({
            "content": "{\"host\": \"localhost\", \"port\": 8080, \"run\": true}"
        }))
        .unwrap();
        let result: TestStruct = json_namespace.to_object().unwrap();
        assert_eq!(
            result,
            TestStruct {
                host: "localhost".to_string(),
                port: 8080,
                run: true,
            }
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_namespace_to_object() {
        setup();
        let namespace = CLIENT_NO_SECRET
            .namespace("application.json")
            .await
            .unwrap();

        let result = match namespace {
            Namespace::Json(json_namespace) => json_namespace.to_object(),
            _ => panic!("Namespace is not a JSON namespace"),
        };
        let result: TestStruct = result.unwrap();
        assert_eq!(
            result,
            TestStruct {
                host: "localhost".to_string(),
                port: 8080,
                run: true,
            }
        );
    }
}
