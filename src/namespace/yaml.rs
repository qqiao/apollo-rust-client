//! YAML namespace implementation for handling structured YAML configuration data.
//!
//! This module provides the `Yaml` struct which wraps a `serde_yaml::Value` and
//! provides methods for working with YAML-formatted configuration data. It supports
//! deserialization into custom types and maintains the original YAML structure.
//!
//! # Usage
//!
//! The `Yaml` struct is typically created automatically by the namespace detection
//! system when a namespace name contains a `.yml` or `.yaml` extension, but can also be
//! created directly from any `serde_yaml::Value`.
//!
//! # Examples
//!
//! ```ignore
//! use serde_yaml;
//! use apollo_client::namespace::yaml::Yaml;
//!
//! let yaml_data = serde_yaml::from_str("name: MyApp\nversion: 1.0.0").unwrap();
//! let yaml_namespace = Yaml::from(yaml_data);
//! ```

use log::trace;
use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to get content from YAML value")]
    ContentNotFound,

    #[error("Failed to deserialize YAML value: {0}")]
    DeserializeError(#[from] serde_yaml::Error),
}

/// A wrapper around `serde_yaml::Value` for YAML-formatted configuration data.
///
/// This struct provides a type-safe interface for working with YAML configuration
/// data retrieved from Apollo. It maintains the original YAML structure while
/// providing convenient methods for deserialization into custom types.
///
/// # Thread Safety
///
/// This struct is `Clone` and `Debug`, making it easy to work with in concurrent
/// environments. The underlying `serde_yaml::Value` is also thread-safe.
///
/// # Examples
///
/// ```ignore
/// use serde_yaml;
/// use apollo_client::namespace::yaml::Yaml;
///
/// let yaml_data = serde_yaml::from_str("database:\n  host: localhost\n  port: 5432").unwrap();
///
/// let yaml_namespace = Yaml::from(yaml_data);
/// ```
#[derive(Clone, Debug)]
pub struct Yaml {
    /// The underlying YAML value containing the configuration data
    string: String,
}

impl From<Yaml> for wasm_bindgen::JsValue {
    fn from(val: Yaml) -> Self {
        serde_wasm_bindgen::to_value(&val.string).unwrap()
    }
}

impl Yaml {
    /// Deserializes the YAML data into a custom type.
    ///
    /// This method attempts to deserialize the stored YAML value into any type
    /// that implements `DeserializeOwned`. This is useful for converting the
    /// raw YAML configuration into strongly-typed structs.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The target type to deserialize into. Must implement `DeserializeOwned`.
    ///
    /// # Returns
    ///
    /// An instance of type `T` containing the deserialized data.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use serde::{Deserialize, Serialize};
    /// use serde_yaml;
    /// use apollo_client::namespace::yaml::Yaml;
    ///
    /// #[derive(Deserialize, Serialize)]
    /// struct DatabaseConfig {
    ///     host: String,
    ///     port: u16,
    /// }
    ///
    /// let yaml_data = serde_yaml::from_str("host: localhost\nport: 5432").unwrap();
    ///
    /// let yaml_namespace = Yaml::from(yaml_data);
    /// let config: DatabaseConfig = yaml_namespace.to_object().unwrap();
    /// ```
    pub fn to_object<T: DeserializeOwned>(&self) -> Result<T, serde_yaml::Error> {
        trace!("string: {:?}", self.string);
        serde_yaml::from_str(&self.string)
    }
}

/// Converts a `serde_json::Value` into a `Yaml` instance.
///
/// This implementation allows for easy creation of `Yaml` instances from
/// raw JSON data, typically used by the namespace detection system.
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use apollo_client::namespace::yaml::Yaml;
///
/// let json_data = json!({"content": "name: MyApp\nversion: 1.0.0"});
/// let yaml_namespace = Yaml::try_from(json_data).unwrap();
/// ```
impl TryFrom<serde_json::Value> for Yaml {
    type Error = crate::namespace::yaml::Error;

    fn try_from(json_value: serde_json::Value) -> Result<Self, Self::Error> {
        let content_string = match json_value.get("content") {
            Some(serde_json::Value::String(s)) => s,
            None => return Err(Error::ContentNotFound),
            _ => return Err(Error::ContentNotFound),
        };
        trace!("content_string: {content_string:?}");

        Ok(Self {
            string: content_string.clone(),
        })
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
    async fn test_yaml_to_object() {
        crate::tests::setup();
        let yaml_namespace = crate::namespace::yaml::Yaml::try_from(serde_json::json!({
            "content": "host: \"localhost\"\nport: 8080\nrun: true"
        }))
        .unwrap();
        let result: TestStruct = yaml_namespace.to_object().unwrap();
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
            .namespace("application.yml")
            .await
            .unwrap();

        let result = match namespace {
            crate::namespace::Namespace::Yaml(yaml_namespace) => yaml_namespace.to_object(),
            _ => panic!("Namespace is not a YAML namespace"),
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
