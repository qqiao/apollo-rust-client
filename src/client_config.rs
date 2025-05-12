//! Configuration for the Apollo client.

use cfg_if::cfg_if;
use wasm_bindgen::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Environment variable is not set: {0}")]
    EnvVar(#[from] std::env::VarError),
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

impl ClientConfig {
    /// Create a new configuration from environment variables.
    ///
    /// # Returns
    ///
    /// A new configuration instance.
    pub fn from_env() -> Result<Self, Error> {
        let app_id = std::env::var("APP_ID").map_err(Error::EnvVar)?;
        let secret = std::env::var("APOLLO_ACCESS_KEY_SECRET")
            .map_err(Error::EnvVar)
            .ok();
        let cluster = std::env::var("IDC").unwrap_or("default".to_string());
        let config_server = std::env::var("APOLLO_CONFIG_SERVICE").map_err(Error::EnvVar)?;
        let label = std::env::var("APOLLO_LABEL").map_err(Error::EnvVar).ok();
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
            /// Create a new configuration from environment variables.
            ///
            /// # Returns
            ///
            /// A new configuration instance.
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
