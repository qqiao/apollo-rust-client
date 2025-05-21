//! Configuration for the Apollo client.

use cfg_if::cfg_if;
use wasm_bindgen::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Environment variable is not set: {1}")]
    EnvVar(std::env::VarError, String),
}

/// Configuration for the Apollo client.
///
/// This struct holds the necessary information to connect to an Apollo Configuration center.
#[derive(Clone, Debug)]
#[wasm_bindgen(getter_with_clone)]
pub struct ClientConfig {
    /// The unique identifier for your application.
    pub app_id: String,

    /// The cluster name to connect to (e.g., "default").
    pub cluster: String,

    /// The directory to store the cache files.
    pub cache_dir: Option<String>,

    /// The Apollo config server URL to connect to.
    pub config_server: String,

    /// Secret key for authentication with the Apollo server.
    pub secret: Option<String>,

    /// The label of the Apollo client. This is used to identify the client in
    /// the Apollo server in the case of a grayscale release.
    pub label: Option<String>,

    /// The IP address of the Apollo client. This is used to identify the client
    /// in the Apollo server in the case of a grayscale release.
    pub ip: Option<String>,
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
        Ok(Self {
            app_id,
            secret,
            cluster,
            config_server,
            cache_dir,
            label,
            ip: None,
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
                }
            }
        }
    }
}
