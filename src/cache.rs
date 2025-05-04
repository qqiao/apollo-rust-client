//! Cache for the Apollo client.

use crate::client_config::ClientConfig;
use base64::display::Base64Display;
use hmac::{Hmac, Mac};
use log::{debug, trace};
use serde_json::Value;
use sha1::Sha1;
use std::{
    fs::File,
    path::PathBuf,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use url::{ParseError, Url};
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
    #[error("Url parse error: {0}")]
    UrlParse(#[from] url::ParseError),
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

        let mut client = reqwest::Client::new().get(&url);
        if self.client_config.secret.is_some() {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let signature = sign(timestamp, &url, self.client_config.secret.as_ref().unwrap())?;
            client = client.header("timestamp", timestamp.to_string());
            client = client.header(
                "Authorization",
                format!("Apollo {}:{}", &self.client_config.app_id, signature),
            );
        }

        let response = client.send().await?;

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

pub(crate) fn sign(timestamp: u128, url: &str, secret: &str) -> Result<String, Error> {
    let u = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => match e {
            ParseError::RelativeUrlWithoutBase => {
                let base_url = Url::parse("http://localhost:8080").unwrap();
                base_url.join(url).unwrap()
            }
            _ => {
                return Err(Error::UrlParse(e));
            }
        },
    };
    let mut path_and_query = String::from(u.path());
    if let Some(query) = u.query() {
        path_and_query.push_str(&format!("?{}", query));
    }
    let input = format!("{}\n{}", timestamp, path_and_query);
    trace!("Input {}", input);

    type HmacSha1 = Hmac<Sha1>;
    let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    let result: [u8; 20] = mac.finalize().into_bytes().into();

    // Convert the result to a string using base64
    let code = Base64Display::new(&result, &base64::engine::general_purpose::STANDARD);
    Ok(code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::setup;

    #[test]
    fn test_sign_with_path() {
        let url = "/configs/100004458/default/application?ip=10.0.0.1";
        let secret = "df23df3f59884980844ff3dada30fa97";
        let signature = sign(1576478257344, url, secret).unwrap();
        assert_eq!(signature, "EoKyziXvKqzHgwx+ijDJwgVTDgE=");
    }

    #[test]
    fn test_sign_url() {
        setup();
        let url = "http://localhost:8080/configs/100004458/default/application?ip=10.0.0.1";
        let secret = "df23df3f59884980844ff3dada30fa97";
        let signature = sign(1576478257344, url, secret).unwrap();
        assert_eq!(signature, "EoKyziXvKqzHgwx+ijDJwgVTDgE=");
    }
}
