//! Configuration for the Apollo client.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("App ID is not set")]
    AppId(std::env::VarError),
}

/// Configuration for the Apollo client.
///
/// This struct holds the necessary information to connect to an Apollo Configuration center.
#[derive(Debug)]
pub struct Config {
    /// The unique identifier for your application.
    pub app_id: String,

    /// The cluster name to connect to (e.g., "default").
    pub cluster: String,

    /// The directory to store the cache files.
    pub cache_dir: PathBuf,

    /// List of Apollo meta server URLs to connect to.
    pub meta_server: Option<String>,

    /// Secret key for authentication with the Apollo server.
    pub secret: Option<String>,

    /// The label of the Apollo client.
    pub label: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_id: "".to_string(),
            cluster: "default".to_string(),
            cache_dir: PathBuf::from("/opt/data").join("config-cache"),
            meta_server: None,
            secret: None,
            label: None,
        }
    }
}

impl Config {
    /// Create a new configuration from environment variables.
    ///
    /// # Returns
    ///
    /// A new configuration instance.
    pub fn from_env() -> Result<Self, Error> {
        let app_id = std::env::var("APP_ID").map_err(Error::AppId)?;
        let secret = std::env::var("APOLLO_ACCESS_KEY_SECRET").ok();
        let cluster = std::env::var("IDC").unwrap_or("default".to_string());
        let meta_server = determine_meta_server();
        let label = std::env::var("APOLLO_LABEL").ok();
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
            meta_server,
            cache_dir,
            label,
        })
    }
}

fn determine_meta_server() -> Option<String> {
    let meta_server = std::env::var("APOLLO_META").ok();
    if meta_server.is_some() {
        return meta_server;
    }

    // when there's no APOLLO_META, we need to determine the meta server based on the environment
    let env = match std::env::var("ENV").ok() {
        Some(env) => env.to_uppercase(),
        None => {
            return None;
        }
    };

    std::env::var(format!("{}_META", env)).ok()
}
