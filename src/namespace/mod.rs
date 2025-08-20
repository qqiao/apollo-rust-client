//! Namespace module for handling different configuration data formats.
//!
//! This module provides abstractions for working with various configuration data formats
//! such as JSON, Properties, YAML, XML, and plain text. It includes functionality to
//! automatically detect the format based on namespace naming conventions and convert
//! raw JSON values into appropriate typed representations.
//!
//! # Supported Formats
//!
//! - **Properties**: Key-value pairs, typically used for application configuration
//! - **JSON**: Structured JSON data with full object support
//! - **YAML**: Structured YAML data with full object support
//! - **XML**: XML format (planned, currently commented out)
//! - **Text**: Plain text content
//!
//! # Usage
//!
//! The module automatically detects the format based on file extensions in the namespace name:
//! - `.json` → JSON format
//! - `.yaml` or `.yml` → YAML format
//! - `.xml` → XML format
//! - `.txt` → Text format
//! - No extension → Properties format (default)

use json::Json;
use properties::Properties;
use yaml::Yaml;

pub mod json;
pub mod properties;
pub mod yaml;

/// Comprehensive error types that can occur when working with namespaces.
///
/// This enum covers all possible error conditions that may arise during namespace
/// operations, from format detection to parsing and type conversion failures.
///
/// # Error Categories
///
/// - **JSON Errors**: Issues with JSON namespace processing and deserialization
/// - **YAML Errors**: Issues with YAML namespace processing and deserialization
/// - **Text Errors**: Issues with text content extraction and processing
/// - **XML Errors**: Issues with XML format (currently unsupported)
///
/// # Examples
///
/// ```rust
/// use apollo_rust_client::namespace::{get_namespace, Error};
///
/// match get_namespace("config.json", json_value) {
///     Ok(namespace) => {
///         // Handle successful namespace creation
///     }
///     Err(Error::Json(json_error)) => {
///         // Handle JSON-specific errors
///         eprintln!("JSON error: {}", json_error);
///     }
///     Err(Error::Yaml(yaml_error)) => {
///         // Handle YAML-specific errors
///         eprintln!("YAML error: {}", yaml_error);
///     }
///     Err(Error::Text(text_error)) => {
///         // Handle text-specific errors
///         eprintln!("Text error: {}", text_error);
///     }
///     Err(Error::Xml(xml_error)) => {
///         // Handle XML-specific errors (currently unsupported)
///         eprintln!("XML error: {}", xml_error);
///     }
///     Err(e) => {
///         // Handle other errors
///         eprintln!("Error: {}", e);
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to process JSON namespace.
    ///
    /// This error occurs when there are issues with JSON format detection,
    /// parsing, or deserialization operations specific to JSON namespaces.
    #[error("Failed to get JSON namespace: {0}")]
    Json(#[from] json::Error),

    /// Failed to process YAML namespace.
    ///
    /// This error occurs when there are issues with YAML format detection,
    /// parsing, or deserialization operations specific to YAML namespaces.
    #[error("Failed to get YAML namespace: {0}")]
    Yaml(#[from] yaml::Error),

    /// Failed to process text namespace.
    ///
    /// This error occurs when there are issues with text content extraction
    /// or processing from the configuration data.
    #[error("Failed to get text namespace: {0}")]
    Text(String),

    /// Failed to process XML namespace.
    ///
    /// This error occurs when XML format is detected but XML processing
    /// is not yet supported by the library.
    #[error("Failed to get XML namespace: {0}")]
    Xml(String),
    // #[error("Failed to get Properties namespace: {0}")]
    // Properties(properties::Error),
}

/// Represents different types of configuration data formats.
///
/// This enum provides a unified interface for working with various configuration
/// formats, allowing the client to handle different data types transparently.
///
/// # Examples
///
/// ```ignore
/// use apollo_client::namespace::Namespace;
///
/// // Working with JSON data
/// let json_namespace = Namespace::Json(json_data);
///
/// // Working with Properties data
/// let props_namespace = Namespace::Properties(properties_data);
///
/// // Working with plain text
/// let text_namespace = Namespace::Text("configuration content".to_string());
/// ```
#[derive(Clone, Debug)]
pub enum Namespace {
    /// Properties format - key-value pairs for application configuration
    Properties(Properties),
    /// JSON format - structured data with full object support
    Json(Json),
    /// YAML format - structured YAML data with full object support
    Yaml(Yaml),
    /// XML format - planned support for XML configuration files
    // Xml(T),
    /// Plain text format - raw string content
    Text(String),
}

impl From<Namespace> for wasm_bindgen::JsValue {
    fn from(val: Namespace) -> Self {
        match val {
            Namespace::Properties(properties) => properties.into(),
            Namespace::Json(json) => json.into(),
            Namespace::Yaml(yaml) => yaml.into(),
            Namespace::Text(text) => text.into(),
        }
    }
}

/// Internal enum for identifying namespace data formats.
///
/// This enum is used internally by the format detection logic to determine
/// the appropriate format based on namespace naming conventions.
#[derive(Clone, Debug, PartialEq)]
enum NamespaceType {
    /// Properties format (default when no extension is specified)
    Properties,
    /// JSON format (detected by `.json` extension)
    Json,
    /// YAML format (detected by `.yaml` or `.yml` extensions)
    Yaml,
    /// XML format (detected by `.xml` extension)
    Xml,
    /// Plain text format (detected by `.txt` extension)
    Text,
}

/// Determines the namespace type based on the namespace string.
///
/// This function analyzes the namespace string to detect the intended data format
/// based on file extension conventions. If no extension is present, it defaults
/// to the Properties format.
///
/// # Arguments
///
/// * `namespace` - The namespace identifier string, potentially containing a file extension
///
/// # Returns
///
/// A `NamespaceType` enum variant indicating the detected format
///
/// # Examples
///
/// ```rust
/// // These examples show the internal logic (function is private)
/// // get_namespace_type("app.config") -> NamespaceType::Properties
/// // get_namespace_type("settings.json") -> NamespaceType::Json
/// // get_namespace_type("config.yaml") -> NamespaceType::Yaml
/// // get_namespace_type("data.xml") -> NamespaceType::Xml
/// // get_namespace_type("readme.txt") -> NamespaceType::Text
/// ```
fn get_namespace_type(namespace: &str) -> NamespaceType {
    let parts = namespace.split('.').collect::<Vec<&str>>();
    if parts.len() == 1 {
        NamespaceType::Properties
    } else {
        match parts.last().unwrap().to_lowercase().as_str() {
            "json" => NamespaceType::Json,
            "yaml" | "yml" => NamespaceType::Yaml,
            "xml" => NamespaceType::Xml,
            _ => NamespaceType::Text,
        }
    }
}

/// Creates a `Namespace` instance from a namespace identifier and JSON value.
///
/// This function serves as the main entry point for converting raw JSON data
/// into the appropriate typed namespace representation. It automatically detects
/// the format based on the namespace string and creates the corresponding variant.
///
/// # Arguments
///
/// * `namespace` - The namespace identifier string used for format detection
/// * `value` - The raw JSON value to be converted into the appropriate format
///
/// # Returns
///
/// * `Ok(Namespace)` - A namespace variant containing the typed representation of the data
/// * `Err(Error::Json)` - If JSON format processing fails
/// * `Err(Error::Yaml)` - If YAML format processing fails
/// * `Err(Error::Text)` - If text format content extraction fails
/// * `Err(Error::Xml)` - If XML format is detected (not yet supported)
///
/// # Errors
///
/// This function will return an error if:
/// - XML format is detected (not yet supported)
/// - Text format content cannot be extracted from the JSON value
/// - JSON or YAML parsing fails during format conversion
/// - The namespace format detection logic encounters unexpected data
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use apollo_client::namespace::get_namespace;
///
/// let json_data = json!({"content": "{\"key\": \"value\"}"});
/// let namespace = get_namespace("config.json", json_data)?;
/// // Returns Namespace::Json variant
///
/// let props_data = json!({"app.name": "MyApp", "app.version": "1.0"});
/// let namespace = get_namespace("application", props_data)?;
/// // Returns Namespace::Properties variant
/// ```
pub(crate) fn get_namespace(namespace: &str, value: serde_json::Value) -> Result<Namespace, Error> {
    match get_namespace_type(namespace) {
        NamespaceType::Properties => Ok(Namespace::Properties(properties::Properties::from(value))),
        NamespaceType::Json => Ok(Namespace::Json(json::Json::try_from(value)?)),
        NamespaceType::Yaml => Ok(Namespace::Yaml(yaml::Yaml::try_from(value)?)),
        NamespaceType::Text => {
            // Extract text content from the JSON value
            let text_content = match value.get("content") {
                Some(serde_json::Value::String(s)) => s.clone(),
                _ => {
                    return Err(Error::Text(
                        "Failed to get text content from JSON value".to_string(),
                    ));
                }
            };
            Ok(Namespace::Text(text_content))
        }
        NamespaceType::Xml => {
            // XML format is not yet implemented
            Err(Error::Xml("XML format is not yet supported".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_namespace_type_properties() {
        // Test cases that should return Properties type
        assert_eq!(get_namespace_type("application"), NamespaceType::Properties);
        assert_eq!(get_namespace_type("config"), NamespaceType::Properties);
        assert_eq!(get_namespace_type("database"), NamespaceType::Properties);
        assert_eq!(
            get_namespace_type("app-settings"),
            NamespaceType::Properties
        );
    }

    #[test]
    fn test_get_namespace_type_json() {
        // Test cases that should return Json type
        assert_eq!(get_namespace_type("config.json"), NamespaceType::Json);
        assert_eq!(get_namespace_type("settings.json"), NamespaceType::Json);
        assert_eq!(get_namespace_type("app.config.json"), NamespaceType::Json);
        assert_eq!(get_namespace_type("data.JSON"), NamespaceType::Json); // Test case insensitive
    }

    #[test]
    fn test_get_namespace_type_yaml() {
        // Test cases that should return Yaml type
        assert_eq!(get_namespace_type("config.yaml"), NamespaceType::Yaml);
        assert_eq!(get_namespace_type("settings.yml"), NamespaceType::Yaml);
        assert_eq!(get_namespace_type("app.config.yaml"), NamespaceType::Yaml);
        assert_eq!(get_namespace_type("data.YAML"), NamespaceType::Yaml); // Test case insensitive
        assert_eq!(get_namespace_type("config.YML"), NamespaceType::Yaml); // Test case insensitive
    }

    #[test]
    fn test_get_namespace_type_xml() {
        // Test cases that should return Xml type
        assert_eq!(get_namespace_type("config.xml"), NamespaceType::Xml);
        assert_eq!(get_namespace_type("settings.xml"), NamespaceType::Xml);
        assert_eq!(get_namespace_type("app.config.xml"), NamespaceType::Xml);
        assert_eq!(get_namespace_type("data.XML"), NamespaceType::Xml); // Test case insensitive
    }

    #[test]
    fn test_get_namespace_type_text() {
        // Test cases that should return Text type
        assert_eq!(get_namespace_type("readme.txt"), NamespaceType::Text);
        assert_eq!(get_namespace_type("notes.txt"), NamespaceType::Text);
        assert_eq!(get_namespace_type("config.TXT"), NamespaceType::Text); // Test case insensitive
    }

    #[test]
    fn test_get_namespace_type_unsupported_extensions() {
        // Test cases with unsupported extensions that should default to Text
        assert_eq!(get_namespace_type("config.ini"), NamespaceType::Text);
        assert_eq!(get_namespace_type("settings.cfg"), NamespaceType::Text);
        assert_eq!(get_namespace_type("app.properties"), NamespaceType::Text);
        assert_eq!(get_namespace_type("data.csv"), NamespaceType::Text);
        assert_eq!(get_namespace_type("config.toml"), NamespaceType::Text);
        assert_eq!(get_namespace_type("settings.conf"), NamespaceType::Text);
        assert_eq!(get_namespace_type("app.unknown"), NamespaceType::Text);
        assert_eq!(get_namespace_type("file.xyz"), NamespaceType::Text);
    }

    #[test]
    fn test_get_namespace_type_edge_cases() {
        // Test edge cases
        assert_eq!(get_namespace_type(""), NamespaceType::Properties); // Empty string
        assert_eq!(get_namespace_type(".json"), NamespaceType::Json); // Leading dot
        assert_eq!(get_namespace_type("file."), NamespaceType::Text); // Trailing dot with no extension
        assert_eq!(get_namespace_type("file..json"), NamespaceType::Json); // Double dots
        assert_eq!(
            get_namespace_type("config.json.backup"),
            NamespaceType::Text
        ); // Multiple extensions
    }
}
