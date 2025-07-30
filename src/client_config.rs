//! Configuration management for the Apollo client.
//!
//! This module provides the `ClientConfig` struct and related functionality for configuring
//! the Apollo client. It supports both direct configuration and environment variable-based
//! configuration, with platform-specific optimizations for native Rust and WebAssembly targets.
//!
//! # Configuration Sources
//!
//! - **Direct Configuration**: Manually specify all configuration fields
//! - **Environment Variables**: Automatically load configuration from environment variables
//! - **Mixed Approach**: Load from environment variables and override specific fields
//!
//! # Platform Support
//!
//! - **Native Rust**: Full feature set including file caching and environment variable support
//! - **WebAssembly**: Optimized for browser environments with memory-only caching
//!
//! # Examples
//!
//! ## Direct Configuration
//!
//! ```rust
//! use apollo_rust_client::client_config::ClientConfig;
//!
//! let config = ClientConfig {
//!     app_id: "my-app".to_string(),
//!     config_server: "http://apollo-server:8080".to_string(),
//!     cluster: "default".to_string(),
//!     secret: Some("secret-key".to_string()),
//!     cache_dir: None, // Uses default
//!     label: Some("production".to_string()),
//!     ip: Some("192.168.1.100".to_string()),
//!     cache_ttl: None,
//! };
//! ```
//!
//! ## Environment Variable Configuration
//!
//! ```rust,no_run
//! use apollo_rust_client::client_config::ClientConfig;
//!
//! // Requires APP_ID and APOLLO_CONFIG_SERVICE environment variables
//! let config = ClientConfig::from_env()?;
//! # Ok::<(), apollo_rust_client::client_config::Error>(())
//! ```

use cfg_if::cfg_if;
use wasm_bindgen::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Environment variable is not set: {1}")]
    EnvVar(std::env::VarError, String),
}

/// Configuration settings for the Apollo client.
///
/// This struct contains all the necessary information to connect to and interact with
/// an Apollo Configuration Center. It supports various configuration options including
/// authentication, caching, and grayscale release targeting.
///
/// # Required Fields
///
/// - `app_id`: Your application identifier in Apollo
/// - `config_server`: The Apollo configuration server URL
/// - `cluster`: The cluster name (typically "default")
///
/// # Optional Fields
///
/// - `secret`: Authentication secret key for secure access
/// - `cache_dir`: Local cache directory (native targets only)
/// - `label`: Labels for grayscale release targeting
/// - `ip`: IP address for grayscale release targeting
///
/// # Examples
///
/// ## Minimal Configuration
///
/// ```rust
/// use apollo_rust_client::client_config::ClientConfig;
///
/// let config = ClientConfig {
///     app_id: "my-app".to_string(),
///     config_server: "http://apollo-server:8080".to_string(),
///     cluster: "default".to_string(),
///     secret: None,
///     cache_dir: None,
///     label: None,
///     ip: None,
///     cache_ttl: None,
/// };
/// ```
///
/// ## Full Configuration
///
/// ```rust
/// use apollo_rust_client::client_config::ClientConfig;
///
/// let config = ClientConfig {
///     app_id: "my-app".to_string(),
///     config_server: "http://apollo-server:8080".to_string(),
///     cluster: "production".to_string(),
///     secret: Some("secret-key".to_string()),
///     cache_dir: Some("/custom/cache/path".to_string()),
///     label: Some("canary,beta".to_string()),
///     ip: Some("192.168.1.100".to_string()),
///     cache_ttl: None,
/// };
/// ```
#[derive(Clone, Debug)]
#[wasm_bindgen(getter_with_clone)]
pub struct ClientConfig {
    /// The unique identifier for your application in Apollo.
    ///
    /// This is used to identify which application's configuration to retrieve
    /// from the Apollo Configuration Center.
    pub app_id: String,

    /// The cluster name to connect to.
    ///
    /// Clusters allow you to organize different environments or deployment
    /// groups. Common values include "default", "production", "staging", etc.
    pub cluster: String,

    /// The directory to store local cache files (native targets only).
    ///
    /// On native Rust targets, this specifies where configuration files should
    /// be cached locally. If `None`, defaults to `/opt/data/{app_id}/config-cache`.
    /// On WebAssembly targets, this is always `None` as file system access is not available.
    pub cache_dir: Option<String>,

    /// The Apollo configuration server URL.
    ///
    /// This should be the base URL of your Apollo Configuration Center server,
    /// including the protocol (http/https) and port if necessary.
    /// Example: "http://apollo-server:8080"
    pub config_server: String,

    /// Optional secret key for authentication with the Apollo server.
    ///
    /// If your Apollo namespace requires authentication, provide the secret key here.
    /// This is used to generate HMAC-SHA1 signatures for secure access to protected
    /// configuration namespaces.
    pub secret: Option<String>,

    /// Labels for grayscale release targeting.
    ///
    /// Comma-separated list of labels that identify this client instance.
    /// Apollo can use these labels to determine which configuration version
    /// to serve during grayscale releases. Example: "canary,beta"
    pub label: Option<String>,

    /// IP address for grayscale release targeting.
    ///
    /// The IP address of this client instance. Apollo can use this IP address
    /// to determine which configuration version to serve during grayscale releases
    /// based on IP-based targeting rules.
    pub ip: Option<String>,

    /// Time-to-live for the cache, in seconds.
    ///
    /// When using `from_env`, this defaults to 600 seconds (10 minutes) if
    /// the `APOLLO_CACHE_TTL` environment variable is not set.
    pub cache_ttl: Option<u64>,
}

impl From<Error> for JsValue {
    fn from(error: Error) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

#[wasm_bindgen]
impl ClientConfig {
    /// Create a new configuration from environment variables.
    ///
    /// # Returns
    ///
    /// A new configuration instance.
    pub fn from_env() -> Result<Self, Error> {
        let app_id =
            std::env::var("APP_ID").map_err(|e| (Error::EnvVar(e, "APP_ID".to_string())))?;
        let secret = std::env::var("APOLLO_ACCESS_KEY_SECRET")
            .map_err(|e| (Error::EnvVar(e, "APOLLO_ACCESS_KEY_SECRET".to_string())))
            .ok();
        let cluster = std::env::var("IDC").unwrap_or("default".to_string());
        let config_server = std::env::var("APOLLO_CONFIG_SERVICE")
            .map_err(|e| (Error::EnvVar(e, "APOLLO_CONFIG_SERVICE".to_string())))?;
        let label = std::env::var("APOLLO_LABEL")
            .map_err(|e| (Error::EnvVar(e, "APOLLO_LABEL".to_string())))
            .ok();
        let cache_dir = std::env::var("APOLLO_CACHE_DIR").ok();
        let cache_ttl = std::env::var("APOLLO_CACHE_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .or(Some(600));
        Ok(Self {
            app_id,
            secret,
            cluster,
            config_server,
            cache_dir,
            label,
            ip: None,
            cache_ttl,
        })
    }
}

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        impl ClientConfig {
            /// Returns the path to the cache directory for the Apollo client.
            ///
            /// This method constructs a `std::path::PathBuf` representing the directory
            /// where Apollo configuration cache files will be stored. The logic is as follows:
            ///
            /// 1.  It uses the `cache_dir` field from the `ClientConfig` instance if it's set.
            /// 2.  If `cache_dir` is `None`, it defaults to `/opt/data`.
            /// 3.  It then appends the `app_id` (from `ClientConfig`) as a subdirectory.
            /// 4.  Finally, it appends `config-cache` as another subdirectory.
            ///
            /// # Examples
            ///
            /// - If `cache_dir` is `Some("/my/custom/path".to_string())` and `app_id` is `"my_app"`,
            ///   the result will be `/my/custom/path/my_app/config-cache`.
            /// - If `cache_dir` is `None` and `app_id` is `"another_app"`,
            ///   the result will be `/opt/data/another_app/config-cache`.
            ///
            /// # Returns
            ///
            /// A `std::path::PathBuf` for the cache directory.
            pub(crate) fn get_cache_dir(&self) -> std::path::PathBuf {
                let base = std::path::PathBuf::from(
                    &self
                        .cache_dir
                        .clone()
                        .unwrap_or_else(|| String::from("/opt/data")),
                );
                base.join(&self.app_id).join("config-cache")
            }
        }
    } else {
        #[wasm_bindgen]
        impl ClientConfig {
            /// Creates a new `ClientConfig` instance specifically for wasm32 targets.
            ///
            /// This constructor takes essential configuration parameters (`app_id`, `config_server`, `cluster`)
            /// directly as arguments. Other configuration fields are initialized to `None` or their
            /// default values:
            /// - `cache_dir`: `None` (file system caching is not typically used in wasm32).
            /// - `secret`: `None`.
            /// - `label`: `None`.
            /// - `ip`: `None`.
            ///
            /// This is in contrast to the `from_env` method, which attempts to read all
            /// configuration values from environment variables.
            ///
            /// # Arguments
            ///
            /// * `app_id` - The unique identifier for your application.
            /// * `config_server` - The Apollo config server URL.
            /// * `cluster` - The cluster name (e.g., "default").
            ///
            /// # Returns
            ///
            /// A new `ClientConfig` instance.
            #[wasm_bindgen(constructor)]
            pub fn new(app_id: String, config_server: String, cluster: String) -> Self {
                Self {
                    app_id,
                    config_server,
                    cluster,
                    cache_dir: None,
                    secret: None,
                    label: None,
                    ip: None,
                    cache_ttl: None,
                }
            }
        }
    }
}
