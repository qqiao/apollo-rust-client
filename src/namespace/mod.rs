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
//! - **YAML**: YAML format (planned, currently commented out)
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

pub mod json;
pub mod properties;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to get JSON namespace: {0}")]
    Json(#[from] json::Error),
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
    /// YAML format - planned support for YAML configuration files
    // Yaml(T),
    /// XML format - planned support for XML configuration files
    // Xml(T),
    /// Plain text format - raw string content
    Text(String),
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
    let parts = namespace.split(".").collect::<Vec<&str>>();
    if parts.len() == 1 {
        NamespaceType::Properties
    } else {
        match parts.last().unwrap().to_lowercase().as_str() {
            "json" => NamespaceType::Json,
            "yaml" | "yml" => NamespaceType::Yaml,
            "xml" => NamespaceType::Xml,
            "txt" => NamespaceType::Text,
            _ => todo!(),
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
/// A `Namespace` enum variant containing the typed representation of the data
///
/// # Panics
///
/// This function will panic (via `todo!()`) if:
/// - An unsupported format is detected (YAML, XML, or Text)
/// - An unrecognized file extension is encountered
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use apollo_client::namespace::get_namespace;
///
/// let json_data = json!({"key": "value"});
/// let namespace = get_namespace("config.json", json_data);
/// // Returns Namespace::Json variant
///
/// let props_data = json!({"app.name": "MyApp", "app.version": "1.0"});
/// let namespace = get_namespace("application", props_data);
/// // Returns Namespace::Properties variant
/// ```
pub(crate) fn get_namespace(namespace: &str, value: serde_json::Value) -> Result<Namespace, Error> {
    match get_namespace_type(namespace) {
        NamespaceType::Properties => Ok(Namespace::Properties(properties::Properties::from(value))),
        NamespaceType::Json => Ok(Namespace::Json(json::Json::try_from(value)?)),
        _ => todo!(),
    }
}
