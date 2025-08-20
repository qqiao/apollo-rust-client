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

/// Comprehensive error types that can occur when working with JSON namespaces.
///
/// This enum covers all possible error conditions that may arise during JSON
/// namespace operations, from content extraction to deserialization failures.
///
/// # Error Categories
///
/// - **Content Errors**: Issues with extracting content from JSON values
/// - **Deserialization Errors**: Problems with parsing JSON into custom types
///
/// # Examples
///
/// ```rust
/// use apollo_rust_client::namespace::json::{Json, Error};
/// use serde::{Deserialize, Serialize};
/// use serde_json::json;
///
/// #[derive(Debug, Deserialize, Serialize)]
/// struct MyType {
///     name: String,
///     value: i32,
/// }
///
/// // Create a sample JSON namespace
/// let json_data = json!({
///     "content": r#"{"name": "test", "value": 42}"#
/// });
/// let json_namespace = Json::try_from(json_data).unwrap();
///
/// match json_namespace.to_object::<MyType>() {
///     Ok(config) => {
///         // Handle successful deserialization
///         println!("Config: {:?}", config);
///     }
///     Err(serde_error) => {
///         // Handle JSON parsing errors
///         eprintln!("JSON parsing failed: {}", serde_error);
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to extract content from the JSON value.
    ///
    /// This error occurs when the JSON value doesn't contain the expected
    /// "content" field or when the content field is not a string.
    #[error("Failed to get content from JSON value")]
    ContentNotFound,

    /// Failed to deserialize JSON value into the target type.
    ///
    /// This error occurs when the JSON content cannot be parsed into the
    /// requested type due to format mismatches, missing fields, or type
    /// conversion failures.
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
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
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
    /// * `Ok(T)` - The deserialized configuration object
    /// * `Err(serde_json::Error)` - If deserialization fails due to format mismatches,
    ///   missing fields, or type conversion failures
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The JSON structure doesn't match the expected type
    /// - Required fields are missing from the JSON
    /// - Type conversion fails (e.g., string to number)
    /// - The JSON is malformed or invalid
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
    /// let config: DatabaseConfig = json_namespace.to_object()?;
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
/// # Arguments
///
/// * `json_value` - The raw JSON value containing configuration data
///
/// # Returns
///
/// * `Ok(Json)` - A new Json instance containing the parsed configuration
/// * `Err(Error::ContentNotFound)` - If the JSON value doesn't contain a "content" field
/// * `Err(Error::DeserializeError)` - If the content field cannot be parsed as valid JSON
///
/// # Errors
///
/// This function will return an error if:
/// - The JSON value doesn't contain a "content" field
/// - The "content" field is not a string
/// - The content string cannot be parsed as valid JSON
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use apollo_client::namespace::json::Json;
///
/// let json_data = json!({"content": "{\"key\": \"value\"}"});
/// let json_namespace = Json::try_from(json_data)?;
/// ```
impl TryFrom<serde_json::Value> for Json {
    type Error = crate::namespace::json::Error;

    fn try_from(json_value: serde_json::Value) -> Result<Self, Self::Error> {
        let Some(serde_json::Value::String(content_string)) = json_value.get("content") else {
            return Err(Error::ContentNotFound);
        };
        trace!("content_string: {content_string:?}");
        let value = serde_json::from_str(content_string.as_str())?;
        trace!("value: {value:?}");
        Ok(Self { value })
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        host: String,
        port: u16,
        run: bool,
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_json_to_object() {
        crate::tests::setup();
        let json_namespace = crate::namespace::json::Json::try_from(serde_json::json!({
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
        crate::tests::setup();
        let namespace = crate::tests::CLIENT_NO_SECRET
            .namespace("application.json")
            .await
            .unwrap();

        let result = match namespace {
            crate::namespace::Namespace::Json(json_namespace) => json_namespace.to_object(),
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
