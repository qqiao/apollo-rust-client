//! Cache for the Apollo client.

use crate::client_config::ClientConfig;
use async_std::sync::RwLock;
use base64::display::Base64Display;
use cfg_if::cfg_if;
use chrono::Utc;
use hmac::{Hmac, Mac};
use log::{debug, trace};
use serde_json::Value;
use sha1::Sha1;
use std::sync::Arc;
use url::{ParseError, Url};
use wasm_bindgen::prelude::*;

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
    #[error("Already loading")]
    AlreadyLoading,
    #[error("Already checking cache")]
    AlreadyCheckingCache,
}

/// A cache for a given namespace.
#[derive(Clone)]
#[wasm_bindgen]
pub struct Cache {
    client_config: ClientConfig,
    namespace: String,
    loading: Arc<RwLock<bool>>,
    checking_cache: Arc<RwLock<bool>>,
    memory_cache: Arc<RwLock<Option<Value>>>,

    #[cfg(not(target_arch = "wasm32"))]
    file_path: std::path::PathBuf,
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
        let mut file_name = namespace.to_string();
        if let Some(ip) = &client_config.ip {
            file_name.push_str(&format!("_{}", ip));
        }
        if let Some(label) = &client_config.label {
            file_name.push_str(&format!("_{}", label));
        }

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let file_path = client_config
                    .get_cache_dir()
                    .join(format!("{}.cache.json", file_name));
            }
        }

        Self {
            client_config,
            namespace: namespace.to_string(),

            loading: Arc::new(RwLock::new(false)),
            checking_cache: Arc::new(RwLock::new(false)),
            memory_cache: Arc::new(RwLock::new(None)),

            #[cfg(not(target_arch = "wasm32"))]
            file_path,
        }
    }

    /// Get a property from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a string.
    pub async fn get_property<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        debug!("Getting property for key {}", key);
        let value = self.get_value(key).await.ok()?;

        value.as_str().and_then(|s| s.parse::<T>().ok())
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
    ///
    /// This method attempts to retrieve a configuration value associated with the given `key`.
    /// The process involves several steps:
    ///
    /// 1.  **In-Memory Cache Check**: It first checks an in-memory cache (`self.memory_cache`).
    ///     If the value is found, it's returned immediately. `self.memory_cache` is protected by an `RwLock`
    ///     to allow concurrent reads and exclusive writes.
    ///
    /// 2.  **File-Based Cache Check (non-wasm32 targets only)**: If the value is not in the memory cache
    ///     and the target architecture is not wasm32, it attempts to load the cache from a local file.
    ///     The file path is determined by the `client_config` and namespace. If the file exists, its
    ///     contents are parsed, and the in-memory cache is updated.
    ///
    /// 3.  **Refresh Operation**: If the value is not found in either the in-memory cache or the
    ///     file-based cache (or if on wasm32 where file cache is not used), a `self.refresh()`
    ///     operation is triggered to fetch the latest configuration from the Apollo server.
    ///     The in-memory cache (and file cache on non-wasm32) will be updated by the `refresh` method.
    ///
    /// To prevent multiple concurrent attempts to initialize or check the cache from the file system
    /// or via refresh, this method uses an `RwLock` named `self.checking_cache`.
    /// If another task is already performing this check/initialization, the current task will return
    /// `Err(Error::AlreadyCheckingCache)`. This indicates that a cache lookup or population is
    /// already in progress, and the caller should typically retry shortly.
    async fn get_value(&self, key: &str) -> Result<Value, Error> {
        debug!("Getting value for key {}", key);

        // Check if the cache file exists
        let mut checking_cache = self.checking_cache.write().await;
        if *checking_cache {
            return Err(Error::AlreadyCheckingCache);
        }
        *checking_cache = true;

        // First we check memory cache
        let memory_cache = self.memory_cache.read().await;
        if let Some(value) = memory_cache.as_ref() {
            debug!("Memory cache found, using memory cache for key {}", key);
            if let Some(v) = value.get(key) {
                *checking_cache = false;
                return Ok(v.clone());
            }
        }
        drop(memory_cache);
        debug!("No memory cache found");

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let file_path = self.file_path.clone();
                if file_path.exists() {
                    trace!("Cache file {} exists, using file cache", file_path.display());
                    let file = match std::fs::File::open(file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            *checking_cache = false;
                            return Err(Error::Io(e));
                        }
                    };
                    let config: Value = serde_json::from_reader(file)?;
                    self.memory_cache.write().await.replace(config.clone());
                } else {
                    trace!("Cache file {} doesn't exist, refreshing cache", file_path.display());
                    match self.refresh().await {
                        Ok(_) => (),
                        Err(e) => {
                            *checking_cache = false;
                            return Err(e);
                        }
                    }
                }
            } else {
                match self.refresh().await {
                    Ok(_) => (),
                    Err(e) => {
                        *checking_cache = false;
                        return Err(e);
                    }
                }
            }
        }

        let memory_cache = self.memory_cache.read().await;
        if let Some(value) = memory_cache.as_ref() {
            if let Some(v) = value.get(key) {
                *checking_cache = false;
                Ok(v.clone())
            } else {
                *checking_cache = false;
                Err(Error::KeyNotFound(key.to_string()))
            }
        } else {
            *checking_cache = false;
            Err(Error::KeyNotFound(key.to_string()))
        }
    }

    /// Refreshes the cache by fetching the latest configuration from the Apollo server.
    ///
    /// This method performs the following steps to update the cache for the current namespace:
    ///
    /// 1.  **Construct URL**: It constructs the request URL for the Apollo configuration service
    ///     based on `client_config` (server address, app ID, cluster) and the current namespace.
    /// 2.  **Add Query Parameters**: If `client_config.ip` or `client_config.label` are set,
    ///     they are added as query parameters (`ip` and `label` respectively) to the request URL.
    ///     This is often used for grayscale release rules.
    /// 3.  **Authentication Headers (if secret is present)**:
    ///     *   If `client_config.secret` is provided, it generates a signature using the `sign` function.
    ///     *   The current `timestamp` (in milliseconds) and an `Authorization` header
    ///         (e.g., `Apollo <app_id>:<signature>`) are added to the HTTP request.
    /// 4.  **Send HTTP GET Request**: It sends an HTTP GET request to the constructed URL.
    /// 5.  **Update Caches**:
    ///     *   On a successful response, the response body (JSON configuration) is parsed.
    ///     *   For non-wasm32 targets, the fetched configuration is written to a local file cache
    ///         (path determined by `client_config.cache_dir` and namespace details).
    ///     *   The in-memory cache (`self.memory_cache`) is updated with the new configuration.
    ///
    /// To prevent multiple concurrent refresh operations for the same cache instance, this method
    /// uses an `RwLock` named `self.loading`. If another task is already refreshing this cache,
    /// the current task will return `Err(Error::AlreadyLoading)`. This indicates that a refresh
    /// is already in progress, and the caller should typically wait for the ongoing refresh to
    /// complete rather than initiating a new one.
    pub(crate) async fn refresh(&self) -> Result<(), Error> {
        let mut loading = self.loading.write().await;
        if *loading {
            return Err(Error::AlreadyLoading);
        }
        *loading = true;

        trace!("Refreshing cache for namespace {}", self.namespace);
        let url = format!(
            "{}/configfiles/json/{}/{}/{}",
            self.client_config.config_server,
            self.client_config.app_id,
            self.client_config.cluster,
            self.namespace
        );

        let mut url = match Url::parse(&url) {
            Ok(u) => u,
            Err(e) => {
                *loading = false;
                return Err(Error::UrlParse(e));
            }
        };
        if let Some(ip) = &self.client_config.ip {
            url.query_pairs_mut().append_pair("ip", ip);
        }
        if let Some(label) = &self.client_config.label {
            url.query_pairs_mut().append_pair("label", label);
        }

        trace!("Url {} for namespace {}", url, self.namespace);

        let mut client = reqwest::Client::new().get(url.as_str());
        if self.client_config.secret.is_some() {
            let timestamp = Utc::now().timestamp_millis();
            let signature = sign(
                timestamp,
                url.as_str(),
                self.client_config.secret.as_ref().unwrap(),
            )?;
            client = client.header("timestamp", timestamp.to_string());
            client = client.header(
                "Authorization",
                format!("Apollo {}:{}", &self.client_config.app_id, signature),
            );
        }

        let response = match client.send().await {
            Ok(r) => r,
            Err(e) => {
                *loading = false;
                return Err(Error::Reqwest(e));
            }
        };

        let body = match response.text().await {
            Ok(b) => b,
            Err(e) => {
                *loading = false;
                return Err(Error::Reqwest(e));
            }
        };

        trace!("Response body {} for namespace {}", body, self.namespace);
        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                debug!("writing cache file {}", self.file_path.display());
                 // Create parent directories if they don't exist
                if let Some(parent) = self.file_path.parent() {
                    match std::fs::create_dir_all(parent) {
                        Ok(_) => (),
                        Err(e) => {
                            *loading = false;
                            return Err(Error::Io(e));
                        }
                    }
                }
                match std::fs::write(&self.file_path, body.clone()) {
                    Ok(_) => {
                        trace!("Wrote cache file {} for namespace {}", self.file_path.display(), self.namespace);
                    },
                    Err(e) => {
                        *loading = false;
                        return Err(Error::Io(e));
                    }
                }
            }
        }

        let config: Value = match serde_json::from_str(&body) {
            Ok(c) => c,
            Err(e) => {
                *loading = false;
                debug!("error parsing config: {}", e);
                return Err(Error::Serde(e));
            }
        };
        trace!("parsed config: {:?}", config);
        self.memory_cache.write().await.replace(config);

        trace!("Refreshed cache for namespace {}", self.namespace);
        *loading = false;

        Ok(())
    }
}

#[wasm_bindgen]
impl Cache {
    /// Get a property from the cache as a string.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a string.
    pub async fn get_string(&self, key: &str) -> Option<String> {
        self.get_property::<String>(key).await
    }

    /// Get a property from the cache as an integer.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as an integer.
    pub async fn get_int(&self, key: &str) -> Option<i64> {
        self.get_property::<i64>(key).await
    }

    /// Get a property from the cache as a float.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a float.
    pub async fn get_float(&self, key: &str) -> Option<f64> {
        debug!("Getting property for key {}", key);
        let value = self.get_value(key).await.ok()?;

        value.as_str().and_then(|s| s.parse::<f64>().ok())
    }

    /// Get a property from the cache as a boolean.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the property for.
    ///
    /// # Returns
    ///
    /// The property for the given key as a boolean.
    pub async fn get_bool(&self, key: &str) -> Option<bool> {
        debug!("Getting property for key {}", key);
        let value = self.get_value(key).await.ok()?;

        value.as_str().and_then(|s| s.parse::<bool>().ok())
    }
}

/// Generates a signature for Apollo API authentication using HMAC-SHA1.
///
/// This function takes a timestamp, the request URL (or its path and query), and an Apollo secret key
/// to create a Base64 encoded signature. This signature is typically used in the `Authorization`
/// header when making requests to the Apollo configuration service.
///
/// # Arguments
///
/// * `timestamp` - The current timestamp in milliseconds since the Unix epoch. This is used as part of the message to be signed.
/// * `url` - The URL being requested. This can be a full URL or just the path and query string.
/// * `secret` - The Apollo secret key used for generating the HMAC-SHA1 signature.
///
/// # Returns
///
/// A `Result` which is:
/// * `Ok(String)`: A Base64 encoded string representing the signature.
/// * `Err(Error)`: An error if URL parsing fails.
pub(crate) fn sign(timestamp: i64, url: &str, secret: &str) -> Result<String, Error> {
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
    trace!("input for signing: {}", input);

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
