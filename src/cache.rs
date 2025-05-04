//! Cache for the Apollo client.

use crate::client_config::ClientConfig;
use log::{debug, trace};
use serde_json::Value;
use std::{fs::File, path::PathBuf, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

/// A cache for a given namespace.
pub struct Cache {
    client_config: ClientConfig,
    namespace: String,
    file_path: PathBuf,
}

impl Cache {
    /// Create a new cache.
    ///
    /// # Arguments
    ///
    /// * `client_config` - The configuration for the Apollo client.
    /// * `namespace` - The namespace to get the cache for.
    ///
    /// # Returns
    ///
    /// A new cache for the given namespace.
    pub(crate) fn new(client_config: ClientConfig, namespace: &str) -> Self {
        let file_path = client_config
            .get_cache_dir()
            .join(format!("{}.cache.json", namespace));
        Self {
            client_config,
            namespace: namespace.to_string(),
            file_path,
        }
    }

    pub(crate) async fn refresh(&self) -> Result<(), Error> {
        trace!("Refreshing cache for namespace {}", self.namespace);
        let url = format!(
            "{}/configfiles/json/{}/{}/{}",
            self.client_config.config_server,
            self.client_config.app_id,
            self.client_config.cluster,
            self.namespace
        );
        trace!("Url {} for namespace {}", url, self.namespace);
        let response = reqwest::get(url).await?;
        trace!(
            "Response status code {} for namespace {}",
            response.status(),
            self.namespace
        );

        let body = response.text().await?;

        trace!("Response body {} for namespace {}", body, self.namespace);
        // Create parent directories if they don't exist
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write the response body to the cache file
        std::fs::write(&self.file_path, body)?;
        Ok(())
    }

    /// Get a configuration from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the configuration for.
    ///
    /// # Returns
    ///
    /// The configuration for the given key.
    async fn get_value(&self, key: &str) -> Result<Value, Error> {
        debug!("Getting value for key {}", key);
        let file_path = self.file_path.clone();
        // Check if the cache file exists
        if !file_path.exists() {
            trace!(
                "Cache file {} doesn't exist, fetching from server",
                file_path.display()
            );
            // File doesn't exist, try to refresh the cache
            match self.refresh().await {
                Ok(_) => {
                    // Refresh successful, continue with reading the file
                    log::debug!(
                        "Cache file created after refresh for namespace {}",
                        self.namespace
                    );
                }
                Err(err) => {
                    // Refresh failed, propagate the error
                    log::error!(
                        "Failed to refresh cache for namespace {}: {:?}",
                        self.namespace,
                        err
                    );
                    return Err(err);
                }
            }
        }
        let file = File::open(file_path.clone())?;
        trace!("Cache file {} opened", file_path.clone().display());

        let config: Value = serde_json::from_reader(file)?;
        trace!("Config {:?}", config);

        let value = config.get(key).ok_or(Error::KeyNotFound(key.to_string()))?;
        Ok(value.clone())
    }

    /// Get a property from the cache as a string.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a string.
    pub async fn get_property<T: FromStr>(&self, key: &str) -> Option<T> {
        debug!("Getting property for key {}", key);
        let value = self.get_value(key).await.ok()?;

        value.as_str().and_then(|s| s.parse::<T>().ok())
    }
}
