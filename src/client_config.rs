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
//! ## Environment Variables
//!
//! The following environment variables are supported:
//! - `APP_ID`: Your application identifier in Apollo
//! - `APOLLO_CONFIG_SERVICE`: The Apollo configuration server URL
//! - `IDC`: The cluster name (defaults to "default")
//! - `APOLLO_ACCESS_KEY_SECRET`: Authentication secret key
//! - `APOLLO_LABEL`: Labels for grayscale release targeting
//! - `APOLLO_CACHE_DIR`: Local cache directory
//! - `APOLLO_CACHE_TTL`: Cache time-to-live in seconds
//! - `APOLLO_REFRESH_INTERVAL`: Periodic refresh interval in seconds
//! - `APOLLO_REQUEST_TIMEOUT`: Complete request timeout in seconds
//! - `APOLLO_ALLOW_INSECURE_HTTPS`: Whether to allow insecure HTTPS connections
//!
//! # Platform Support
//!
//! - **Native Rust**: Full feature set including file caching and environment variable support
//! - **WebAssembly**: Optimized for browser environments with persistent localStorage caching (and Node.js in-memory fallback)
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
//!     allow_insecure_https: None,
//!     #[cfg(not(target_arch = "wasm32"))]
//!     http_client: None,
//!     refresh_interval: Some(30),
//!     request_timeout: Some(10),
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

/// Default persistent-cache time-to-live in seconds.
pub const DEFAULT_CACHE_TTL_SECONDS: u64 = 600;

/// Default interval between periodic background refreshes in seconds.
pub const DEFAULT_REFRESH_INTERVAL_SECONDS: u64 = 30;

/// Default timeout for a complete Apollo HTTP request, in seconds.
pub const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 10;

/// Comprehensive error types that can occur during client configuration.
///
/// This enum covers all possible error conditions that may arise during
/// client configuration operations, from environment variable access to
/// configuration validation.
///
/// # Error Categories
///
/// - **Environment Variable Errors**: Issues with accessing or parsing environment variables
///
/// # Examples
///
/// ```rust
/// use apollo_rust_client::client_config::{ClientConfig, Error};
///
/// match ClientConfig::from_env() {
///     Ok(config) => {
///         // Handle successful configuration creation
///     }
///     Err(Error::EnvVar(var_error, var_name)) => {
///         // Handle missing or invalid environment variables
///         eprintln!("Environment variable '{}' error: {}", var_name, var_error);
///     }
///     Err(e) => {
///         // Handle other errors
///         eprintln!("Configuration error: {}", e);
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An environment variable access error occurred.
    ///
    /// This error occurs when attempting to read an environment variable
    /// that is not set or cannot be accessed. The error includes both
    /// the underlying system error and the name of the variable that failed.
    #[error("Failed to read environment variable {1}: {0}")]
    EnvVar(std::env::VarError, String),

    /// A configuration value was present but invalid.
    #[error("Invalid value for {name} ({value:?}): {reason}")]
    InvalidValue {
        /// Configuration field or environment-variable name.
        name: String,
        /// Invalid value supplied by the caller.
        value: String,
        /// Human-readable validation failure.
        reason: String,
    },
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
/// - `allow_insecure_https`: Whether to allow insecure HTTPS connections (self-signed certificates)
/// - `cache_ttl`: Cache freshness lifetime (`0` means always revalidate)
/// - `refresh_interval`: Periodic polling interval
/// - `request_timeout`: Complete request and response-body timeout
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
///     allow_insecure_https: None,
///     #[cfg(not(target_arch = "wasm32"))]
///     http_client: None,
///     refresh_interval: Some(30),
///     request_timeout: Some(10),
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
///     allow_insecure_https: Some(true), // Allow self-signed certificates
///     #[cfg(not(target_arch = "wasm32"))]
///     http_client: None,
///     refresh_interval: Some(30),
///     request_timeout: Some(10),
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

    /// The base directory used to store native local cache files.
    ///
    /// On native Rust targets, this specifies where configuration files should
    /// be cached locally. If `None`, a platform-standard application cache directory is used.
    /// On WebAssembly targets this value is ignored because filesystem access is unavailable.
    pub cache_dir: Option<String>,

    /// The Apollo configuration server URL.
    ///
    /// This should be the base URL of your Apollo Configuration Center server,
    /// including the protocol (http/https) and port if necessary.
    /// Example: "http://apollo-server:8080"
    #[allow(clippy::doc_markdown)]
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

    /// Whether to allow insecure HTTPS connections (self-signed certificates).
    ///
    /// When set to `true`, the client will accept self-signed SSL certificates
    /// and other insecure HTTPS connections. This is useful in company internal
    /// networks or development environments where self-signed certificates are used.
    ///
    /// **Warning**: Setting this to `true` reduces security by bypassing SSL
    /// certificate validation. Only use this in trusted internal networks.
    pub allow_insecure_https: Option<bool>,

    /// Time-to-live for memory and persistent cache entries, in seconds.
    ///
    /// When using `from_env`, this defaults to 600 seconds (10 minutes) if
    /// the `APOLLO_CACHE_TTL` environment variable is not set.
    /// The same TTL is enforced for native files and browser localStorage.
    /// Zero is supported as an always-revalidate mode: cached values remain
    /// available immediately while a single background refresh is attempted.
    pub cache_ttl: Option<u64>,

    /// The refresh interval in seconds for native and WASM background polling.
    ///
    /// When using `from_env`, this defaults to 30 seconds if
    /// the `APOLLO_REFRESH_INTERVAL` environment variable is not set.
    /// The value must be greater than zero.
    pub refresh_interval: Option<u64>,

    /// Timeout for a complete Apollo request, including response-body reads, in seconds.
    ///
    /// This outer timeout is applied even when [`ClientConfig::http_client`] supplies a
    /// custom native HTTP client. A value of zero is rejected during validation.
    pub request_timeout: Option<u64>,

    /// A pre-configured `reqwest::Client` (native targets only) to allow custom HTTP pools, proxies, headers, or tracers.
    ///
    /// If not specified, defaults to standard client construction.
    #[cfg(not(target_arch = "wasm32"))]
    #[wasm_bindgen(skip)]
    pub http_client: Option<reqwest::Client>,
}

/// Builder for a validated [`ClientConfig`].
///
/// The builder is the preferred Rust construction API because new optional
/// settings can be added without breaking downstream struct literals.
#[derive(Clone, Debug)]
pub struct ClientConfigBuilder {
    config: ClientConfig,
}

impl ClientConfigBuilder {
    /// Sets the Apollo cluster. The default is `default`.
    #[must_use]
    pub fn cluster(mut self, cluster: impl Into<String>) -> Self {
        self.config.cluster = cluster.into();
        self
    }

    /// Sets the optional Apollo access-key secret.
    #[must_use]
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(secret.into());
        self
    }

    /// Sets the base directory for native persistent cache files.
    #[must_use]
    pub fn cache_dir(mut self, cache_dir: impl Into<String>) -> Self {
        self.config.cache_dir = Some(cache_dir.into());
        self
    }

    /// Sets the optional label used for grayscale selection.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.config.label = Some(label.into());
        self
    }

    /// Sets the optional client IP used for grayscale selection.
    #[must_use]
    pub fn ip(mut self, ip: impl Into<String>) -> Self {
        self.config.ip = Some(ip.into());
        self
    }

    /// Enables or disables acceptance of invalid HTTPS certificates on native targets.
    #[must_use]
    pub fn allow_insecure_https(mut self, allow: bool) -> Self {
        self.config.allow_insecure_https = Some(allow);
        self
    }

    /// Sets the cache time-to-live in seconds.
    #[must_use]
    pub fn cache_ttl(mut self, seconds: u64) -> Self {
        self.config.cache_ttl = Some(seconds);
        self
    }

    /// Sets the periodic refresh interval in seconds.
    #[must_use]
    pub fn refresh_interval(mut self, seconds: u64) -> Self {
        self.config.refresh_interval = Some(seconds);
        self
    }

    /// Sets the timeout for the complete request and response-body read.
    #[must_use]
    pub fn request_timeout(mut self, seconds: u64) -> Self {
        self.config.request_timeout = Some(seconds);
        self
    }

    /// Supplies a custom native HTTP client.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.config.http_client = Some(client);
        self
    }

    /// Validates and returns the completed configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] when a required identifier or the server
    /// URL is invalid.
    pub fn build(self) -> Result<ClientConfig, Error> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl ClientConfig {
    /// Creates a builder with stable defaults.
    #[must_use]
    pub fn builder(
        app_id: impl Into<String>,
        config_server: impl Into<String>,
    ) -> ClientConfigBuilder {
        ClientConfigBuilder {
            config: Self {
                app_id: app_id.into(),
                cluster: "default".to_string(),
                cache_dir: None,
                config_server: config_server.into(),
                secret: None,
                label: None,
                ip: None,
                allow_insecure_https: None,
                cache_ttl: Some(DEFAULT_CACHE_TTL_SECONDS),
                refresh_interval: Some(DEFAULT_REFRESH_INTERVAL_SECONDS),
                request_timeout: Some(DEFAULT_REQUEST_TIMEOUT_SECONDS),
                #[cfg(not(target_arch = "wasm32"))]
                http_client: None,
            },
        }
    }

    /// Validates required identifiers and the Apollo server URL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] for empty identifiers, unsupported URL
    /// schemes, URLs that cannot be used as a base, or URLs containing a query
    /// or fragment.
    pub fn validate(&self) -> Result<(), Error> {
        validate_nonempty("app_id", &self.app_id)?;
        validate_nonempty("cluster", &self.cluster)?;

        let parsed = url::Url::parse(&self.config_server).map_err(|error| Error::InvalidValue {
            name: "config_server".to_string(),
            value: self.config_server.clone(),
            reason: error.to_string(),
        })?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.cannot_be_a_base() {
            return Err(Error::InvalidValue {
                name: "config_server".to_string(),
                value: self.config_server.clone(),
                reason: "expected an HTTP(S) base URL".to_string(),
            });
        }
        if parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(Error::InvalidValue {
                name: "config_server".to_string(),
                value: self.config_server.clone(),
                reason: "base URL must not contain a query or fragment".to_string(),
            });
        }
        if self.refresh_interval == Some(0) {
            return Err(Error::InvalidValue {
                name: "refresh_interval".to_string(),
                value: "0".to_string(),
                reason: "value must be greater than zero".to_string(),
            });
        }
        if self.request_timeout == Some(0) {
            return Err(Error::InvalidValue {
                name: "request_timeout".to_string(),
                value: "0".to_string(),
                reason: "value must be greater than zero".to_string(),
            });
        }
        Ok(())
    }

    /// Returns the effective cache TTL, including the documented default.
    #[must_use]
    pub(crate) fn effective_cache_ttl(&self) -> u64 {
        self.cache_ttl.unwrap_or(DEFAULT_CACHE_TTL_SECONDS)
    }

    /// Returns the effective periodic refresh interval.
    #[must_use]
    pub(crate) fn effective_refresh_interval(&self) -> u64 {
        self.refresh_interval
            .unwrap_or(DEFAULT_REFRESH_INTERVAL_SECONDS)
    }

    /// Returns the effective complete-request timeout.
    #[must_use]
    pub(crate) fn effective_request_timeout(&self) -> u64 {
        self.request_timeout
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECONDS)
    }
}

fn validate_nonempty(name: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::InvalidValue {
            name: name.to_string(),
            value: value.to_string(),
            reason: "value must not be empty".to_string(),
        });
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn native_env(name: &str) -> Result<Option<String>, Error> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(Error::EnvVar(error, name.to_string())),
    }
}

fn required_env_with<F>(lookup: &F, name: &str) -> Result<String, Error>
where
    F: Fn(&str) -> Result<Option<String>, Error>,
{
    lookup(name)?.ok_or_else(|| Error::EnvVar(std::env::VarError::NotPresent, name.to_string()))
}

fn parse_optional_env_with<T, F>(lookup: &F, name: &str) -> Result<Option<T>, Error>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
    F: Fn(&str) -> Result<Option<String>, Error>,
{
    lookup(name)?
        .map(|value| {
            value.parse::<T>().map_err(|error| Error::InvalidValue {
                name: name.to_string(),
                value,
                reason: error.to_string(),
            })
        })
        .transpose()
}

#[cfg(target_arch = "wasm32")]
fn node_env(name: &str) -> Result<Option<String>, Error> {
    node_env_from_global(&js_sys::global(), name)
}

#[cfg(target_arch = "wasm32")]
fn node_env_from_global(
    global: &wasm_bindgen::JsValue,
    name: &str,
) -> Result<Option<String>, Error> {
    let process = js_sys::Reflect::get(global, &JsValue::from_str("process")).map_err(|error| {
        Error::InvalidValue {
            name: "globalThis.process".to_string(),
            value: format!("{error:?}"),
            reason: "unable to inspect the Node.js process object".to_string(),
        }
    })?;
    if process.is_null() || process.is_undefined() {
        return Ok(None);
    }
    let env = js_sys::Reflect::get(&process, &JsValue::from_str("env")).map_err(|error| {
        Error::InvalidValue {
            name: "globalThis.process.env".to_string(),
            value: format!("{error:?}"),
            reason: "unable to inspect the Node.js environment object".to_string(),
        }
    })?;
    if env.is_null() || env.is_undefined() {
        return Ok(None);
    }
    let value = js_sys::Reflect::get(&env, &JsValue::from_str(name)).map_err(|error| {
        Error::InvalidValue {
            name: name.to_string(),
            value: format!("{error:?}"),
            reason: "unable to read the Node.js environment value".to_string(),
        }
    })?;
    if value.is_null() || value.is_undefined() {
        Ok(None)
    } else {
        value
            .as_string()
            .map(Some)
            .ok_or_else(|| Error::InvalidValue {
                name: name.to_string(),
                value: format!("{value:?}"),
                reason: "expected a string in globalThis.process.env".to_string(),
            })
    }
}

impl ClientConfig {
    pub(crate) fn from_lookup<F>(lookup: F) -> Result<Self, Error>
    where
        F: Fn(&str) -> Result<Option<String>, Error>,
    {
        let app_id = required_env_with(&lookup, "APP_ID")?;
        let secret = lookup("APOLLO_ACCESS_KEY_SECRET")?;
        let cluster = lookup("IDC")?.unwrap_or_else(|| "default".to_string());
        let config_server = required_env_with(&lookup, "APOLLO_CONFIG_SERVICE")?;
        let label = lookup("APOLLO_LABEL")?;
        let cache_dir = lookup("APOLLO_CACHE_DIR")?;
        let allow_insecure_https = parse_optional_env_with(&lookup, "APOLLO_ALLOW_INSECURE_HTTPS")?;
        let cache_ttl = parse_optional_env_with(&lookup, "APOLLO_CACHE_TTL")?
            .or(Some(DEFAULT_CACHE_TTL_SECONDS));
        let refresh_interval = parse_optional_env_with(&lookup, "APOLLO_REFRESH_INTERVAL")?
            .or(Some(DEFAULT_REFRESH_INTERVAL_SECONDS));
        let request_timeout = parse_optional_env_with(&lookup, "APOLLO_REQUEST_TIMEOUT")?
            .or(Some(DEFAULT_REQUEST_TIMEOUT_SECONDS));
        let config = Self {
            app_id,
            secret,
            cluster,
            config_server,
            cache_dir,
            label,
            ip: None,
            allow_insecure_https,
            cache_ttl,
            refresh_interval,
            request_timeout,
            #[cfg(not(target_arch = "wasm32"))]
            http_client: None,
        };
        config.validate()?;
        Ok(config)
    }
}

impl From<Error> for JsValue {
    fn from(error: Error) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        impl ClientConfig {
            /// Creates a new `ClientConfig` by reading environment variables.
            ///
            /// This function loads configuration values from the following environment variables:
            /// - `APP_ID` (required): The Apollo application ID.
            /// - `APOLLO_ACCESS_KEY_SECRET` (optional): Secret key for authentication.
            /// - `IDC` (optional): Cluster name. Defaults to `"default"` if not set.
            /// - `APOLLO_CONFIG_SERVICE` (required): The Apollo config server URL.
            /// - `APOLLO_LABEL` (optional): Comma-separated labels for grayscale release.
            /// - `APOLLO_CACHE_DIR` (optional): Directory for local cache storage.
            /// - `APOLLO_ALLOW_INSECURE_HTTPS` (optional): If set to `"true"`, allows insecure HTTPS.
            /// - `APOLLO_CACHE_TTL` (optional): Cache time-to-live in seconds. Defaults to 600 if not set.
            /// - `APOLLO_REFRESH_INTERVAL` (optional): Periodic refresh interval in seconds. Defaults to 30.
            /// - `APOLLO_REQUEST_TIMEOUT` (optional): Complete request timeout in seconds. Defaults to 10.
            ///
            /// # Returns
            ///
            /// * `Ok(ClientConfig)` if all required environment variables are present and valid.
            /// * `Err(Error)` if a required environment variable is missing or invalid.
            ///
            /// # Errors
            ///
            /// This function will return an error if:
            /// - The `APP_ID` environment variable is missing.
            /// - The `APOLLO_CONFIG_SERVICE` environment variable is missing.
            /// - Any environment variable that is expected to be a number (such as `APOLLO_CACHE_TTL`)
            ///   cannot be parsed as the correct type.
            /// - Any other required environment variable is missing or invalid.
            pub fn from_env() -> Result<Self, Error> {
                Self::from_lookup(native_env)
            }
        }
    } else {
        #[wasm_bindgen]
        impl ClientConfig {
            /// Create a new configuration from environment variables.
            ///
            /// # Returns
            ///
            /// A new configuration instance.
            ///
            /// # Errors
            ///
            /// Reads `globalThis.process.env` when running under Node.js. Browser
            /// environments return a missing-variable error because they do not expose
            /// a process environment.
            pub fn from_env() -> Result<Self, Error> {
                Self::from_lookup(node_env)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn builder_applies_consistent_defaults_and_validates() {
        let config = ClientConfig::builder("sample", "https://apollo.example/base/")
            .build()
            .unwrap();
        assert_eq!(config.cluster, "default");
        assert_eq!(config.effective_cache_ttl(), DEFAULT_CACHE_TTL_SECONDS);
        assert_eq!(
            config.effective_refresh_interval(),
            DEFAULT_REFRESH_INTERVAL_SECONDS
        );
        assert_eq!(
            config.effective_request_timeout(),
            DEFAULT_REQUEST_TIMEOUT_SECONDS
        );

        assert!(
            ClientConfig::builder("", "https://apollo.example")
                .build()
                .is_err()
        );
        assert!(
            ClientConfig::builder("sample", "ftp://apollo.example")
                .build()
                .is_err()
        );
        assert!(
            ClientConfig::builder("sample", "https://apollo.example?token=secret")
                .build()
                .is_err()
        );
        assert!(
            ClientConfig::builder("sample", "https://apollo.example")
                .refresh_interval(0)
                .build()
                .is_err()
        );
        assert!(
            ClientConfig::builder("sample", "https://apollo.example")
                .request_timeout(0)
                .build()
                .is_err()
        );
        assert!(
            ClientConfig::builder("sample", "https://apollo.example")
                .cache_ttl(0)
                .build()
                .is_ok()
        );
    }

    #[test]
    fn invalid_optional_environment_values_are_reported() {
        let lookup = |name: &str| {
            Ok(match name {
                "NUMBER" => Some("not-a-number".to_string()),
                "BOOLEAN" => Some("sometimes".to_string()),
                _ => None,
            })
        };
        assert!(matches!(
            parse_optional_env_with::<u64, _>(&lookup, "NUMBER"),
            Err(Error::InvalidValue { .. })
        ));
        assert!(matches!(
            parse_optional_env_with::<bool, _>(&lookup, "BOOLEAN"),
            Err(Error::InvalidValue { .. })
        ));
    }

    #[test]
    fn non_unicode_optional_environment_values_are_reported() {
        let lookup = |name: &str| -> Result<Option<String>, Error> {
            Err(Error::EnvVar(
                std::env::VarError::NotUnicode(std::ffi::OsString::from("invalid")),
                name.to_string(),
            ))
        };
        assert!(matches!(lookup("NON_UNICODE"), Err(Error::EnvVar(_, _))));
    }

    #[test]
    fn from_lookup_is_deterministic_and_complete() {
        let lookup = |name: &str| {
            Ok(match name {
                "APP_ID" => Some("sample".to_string()),
                "APOLLO_CONFIG_SERVICE" => Some("https://apollo.example".to_string()),
                "APOLLO_CACHE_TTL" => Some("0".to_string()),
                "APOLLO_REFRESH_INTERVAL" => Some("15".to_string()),
                "APOLLO_REQUEST_TIMEOUT" => Some("3".to_string()),
                _ => None,
            })
        };
        let config = ClientConfig::from_lookup(lookup).unwrap();
        assert_eq!(config.app_id, "sample");
        assert_eq!(config.cache_ttl, Some(0));
        assert_eq!(config.refresh_interval, Some(15));
        assert_eq!(config.request_timeout, Some(3));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn node_environment_lookup_reads_process_env() {
        let global = js_sys::Object::new();
        let process = js_sys::Object::new();
        let env = js_sys::Object::new();
        js_sys::Reflect::set(
            &env,
            &JsValue::from_str("APP_ID"),
            &JsValue::from_str("node-app"),
        )
        .unwrap();
        js_sys::Reflect::set(&process, &JsValue::from_str("env"), &env).unwrap();
        js_sys::Reflect::set(&global, &JsValue::from_str("process"), &process).unwrap();

        assert_eq!(
            node_env_from_global(&global, "APP_ID").unwrap(),
            Some("node-app".to_string())
        );
        assert_eq!(node_env_from_global(&global, "MISSING").unwrap(), None);

        let browser_global = js_sys::Object::new();
        assert_eq!(
            node_env_from_global(&browser_global, "APP_ID").unwrap(),
            None
        );
        assert!(matches!(
            ClientConfig::from_lookup(|name| node_env_from_global(&browser_global, name)),
            Err(Error::EnvVar(std::env::VarError::NotPresent, name)) if name == "APP_ID"
        ));
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
            /// 2.  If `cache_dir` is `None`, it uses the platform-standard
            ///     project cache directory.
            /// 3.  Explicit base directories receive the library-owned
            ///     `apollo-rust-client/config-cache` suffix. Application, server,
            ///     cluster, and namespace identity
            ///     are encoded in each cache filename rather than raw path segments.
            ///
            /// # Examples
            ///
            /// - If `cache_dir` is `Some("/my/custom/path".to_string())`, the
            ///   result is `/my/custom/path/apollo-rust-client/config-cache`.
            /// - If `cache_dir` is `None`, the result follows XDG conventions on
            ///   Linux, `~/Library/Caches` on macOS, and Local `AppData` on Windows.
            ///
            /// # Returns
            ///
            /// A `std::path::PathBuf` for the cache directory.
            pub(crate) fn get_cache_dir(&self) -> std::path::PathBuf {
                if let Some(cache_dir) = &self.cache_dir {
                    return std::path::PathBuf::from(cache_dir)
                        .join("apollo-rust-client")
                        .join("config-cache");
                }

                directories::ProjectDirs::from("com", "qqiao", "apollo-rust-client").map_or_else(
                    || {
                        std::env::temp_dir()
                            .join("apollo-rust-client")
                            .join("config-cache")
                    },
                    |dirs| dirs.cache_dir().join("config-cache"),
                )
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
            #[must_use]
            pub fn new(app_id: String, config_server: String, cluster: String) -> Self {
                Self {
                    app_id,
                    config_server,
                    cluster,
                    cache_dir: None,
                    secret: None,
                    label: None,
                    ip: None,
                    allow_insecure_https: None,
                    cache_ttl: Some(DEFAULT_CACHE_TTL_SECONDS),
                    refresh_interval: Some(DEFAULT_REFRESH_INTERVAL_SECONDS),
                    request_timeout: Some(DEFAULT_REQUEST_TIMEOUT_SECONDS),
                }
            }
        }
    }
}
