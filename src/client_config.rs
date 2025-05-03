//! Configuration for the Apollo client.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Environment variable is not set: {0}")]
    EnvVar(#[from] std::env::VarError),
}

/// Configuration for the Apollo client.
///
/// This struct holds the necessary information to connect to an Apollo Configuration center.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// The unique identifier for your application.
    pub app_id: String,

    /// The cluster name to connect to (e.g., "default").
    pub cluster: String,

    /// The directory to store the cache files.
    pub cache_dir: PathBuf,

    /// The Apollo config server URL to connect to.
    pub config_server: String,

    /// Secret key for authentication with the Apollo server.
    pub secret: Option<String>,

    /// The label of the Apollo client.
    pub label: Option<String>,
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
        let cache_dir = match std::env::var("APOLLO_CACHE_DIR") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => PathBuf::from("/opt/data")
                .join(&app_id)
                .join("config-cache"),
        };
        Ok(Self {
            app_id,
            secret,
            cluster,
            config_server,
            cache_dir,
            label,
        })
    }
}
