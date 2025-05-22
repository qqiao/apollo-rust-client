//! Cache for the Apollo client.

use crate::client_config::ClientConfig;
use crate::event::{ConfigurationChangeEvent, EventManager}; // Added imports
use tokio::sync::RwLock; // Changed from async_std::sync::RwLock
use base64::display::Base64Display;
use cfg_if::cfg_if;
use chrono::Utc;
use hmac::{Hmac, Mac};
use log::{debug, trace};
use serde_json::Value;
use sha1::Sha1;
use std::sync::Arc; // Ensure Arc is imported
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
    checking_cache: Arc<RwLock<bool>>, // Ensure this is tokio::sync::RwLock
    memory_cache: Arc<RwLock<Option<Value>>>, // Ensure this is tokio::sync::RwLock
    event_manager: Arc<EventManager>, // New field

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
    /// * `event_manager` - The event manager for observer notifications.
    ///
    /// # Returns
    ///
    /// A new cache for the given namespace.
    pub(crate) fn new(
        client_config: ClientConfig,
        namespace: &str,
        event_manager: Arc<EventManager>, // New parameter
    ) -> Self {
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
            checking_cache: Arc::new(RwLock::new(false)), // Tokio RwLock
            memory_cache: Arc::new(RwLock::new(None)),    // Tokio RwLock
            event_manager, // Store the passed event_manager
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
        // Use a block to ensure the write lock is released immediately after setting checking_cache.
        {
            let mut checking_cache_guard = self.checking_cache.write().await; // Tokio RwLock
            if *checking_cache_guard {
                return Err(Error::AlreadyCheckingCache);
            }
            *checking_cache_guard = true;
        } // checking_cache_guard is dropped here

        // First we check memory cache
        let memory_cache_guard = self.memory_cache.read().await; // Tokio RwLock
        if let Some(value) = memory_cache_guard.as_ref() {
            debug!("Memory cache found, using memory cache for key {}", key);
            if let Some(v) = value.get(key) {
                self.checking_cache.write().await.replace(false);
                return Ok(v.clone());
            }
        }
        drop(memory_cache_guard); // Release read lock
        debug!("No memory cache found");

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let file_path = self.file_path.clone();
                if file_path.exists() {
                    trace!("Cache file {} exists, using file cache", file_path.display());
                    let file = match std::fs::File::open(file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            self.checking_cache.write().await.replace(false);
                            return Err(Error::Io(e));
                        }
                    };
                    let config_from_file: Value = match serde_json::from_reader(file) {
                        Ok(c) => c,
                        Err(e) => {
                            self.checking_cache.write().await.replace(false);
                            return Err(Error::Serde(e));
                        }
                    };
                    self.memory_cache.write().await.replace(config_from_file.clone()); // Update memory with file content
                     // Check key in newly loaded config
                    if let Some(v) = config_from_file.get(key) {
                        self.checking_cache.write().await.replace(false);
                        return Ok(v.clone());
                    } else {
                         self.checking_cache.write().await.replace(false);
                        return Err(Error::KeyNotFound(key.to_string()));
                    }
                } else {
                    trace!("Cache file {} doesn't exist, refreshing cache", file_path.display());
                    match self.refresh().await { // refresh will update memory_cache
                        Ok(_) => (), // memory_cache is updated within refresh
                        Err(e) => {
                            self.checking_cache.write().await.replace(false);
                            return Err(e);
                        }
                    }
                }
            } else { // WASM target, no file cache, just refresh if not in memory
                match self.refresh().await {
                    Ok(_) => (), // memory_cache is updated within refresh
                    Err(e) => {
                        self.checking_cache.write().await.replace(false);
                        return Err(e);
                    }
                }
            }
        }

        // Re-check memory cache after potential refresh or file load
        let memory_cache_guard_after_load = self.memory_cache.read().await; // Tokio RwLock
        if let Some(value) = memory_cache_guard_after_load.as_ref() {
            if let Some(v) = value.get(key) {
                self.checking_cache.write().await.replace(false);
                Ok(v.clone())
            } else {
                self.checking_cache.write().await.replace(false);
                Err(Error::KeyNotFound(key.to_string()))
            }
        } else {
            // This case should ideally not be reached if refresh populates the cache.
            // If refresh fails and there's no old cache, this is the path.
            self.checking_cache.write().await.replace(false);
            Err(Error::KeyNotFound(key.to_string())) 
        }
    }

    pub(crate) async fn refresh(&self) -> Result<(), Error> {
        let mut loading_guard = self.loading.write().await;
        if *loading_guard {
            return Err(Error::AlreadyLoading);
        }
        *loading_guard = true;
        // Drop the guard early after setting the flag, as the HTTP request can take time.
        // However, we need to ensure it's set back to false at the end.
        // A common pattern is to use a RAII guard that resets the flag on drop,
        // or manage it carefully as done here.

        trace!("Refreshing cache for namespace {}", self.namespace);
        
        let url_string = format!(
            "{}/configfiles/json/{}/{}/{}",
            self.client_config.config_server,
            self.client_config.app_id,
            self.client_config.cluster,
            self.namespace
        );

        let mut url_parsed = match Url::parse(&url_string) {
            Ok(u) => u,
            Err(e) => {
                *loading_guard = false; // Reset flag before error return
                return Err(Error::UrlParse(e));
            }
        };
        if let Some(ip) = &self.client_config.ip {
            url_parsed.query_pairs_mut().append_pair("ip", ip);
        }
        if let Some(label) = &self.client_config.label {
            url_parsed.query_pairs_mut().append_pair("label", label);
        }

        trace!("Url {} for namespace {}", url_parsed, self.namespace);

        let mut client_builder = reqwest::Client::new().get(url_parsed.as_str());
        if self.client_config.secret.is_some() {
            let timestamp = Utc::now().timestamp_millis();
            let signature = match sign(
                timestamp,
                url_parsed.as_str(),
                self.client_config.secret.as_ref().unwrap(),
            ) {
                Ok(s) => s,
                Err(e) => {
                    *loading_guard = false; // Reset flag
                    return Err(e);
                }
            };
            client_builder = client_builder.header("timestamp", timestamp.to_string());
            client_builder = client_builder.header(
                "Authorization",
                format!("Apollo {}:{}", &self.client_config.app_id, signature),
            );
        }
        
        // Release the loading guard before making the await call for HTTP request
        drop(loading_guard);

        let response = match client_builder.send().await {
            Ok(r) => r,
            Err(e) => {
                self.loading.write().await.replace(false); // Reset flag
                return Err(Error::Reqwest(e));
            }
        };

        let body = match response.text().await {
            Ok(b) => b,
            Err(e) => {
                self.loading.write().await.replace(false); // Reset flag
                return Err(Error::Reqwest(e));
            }
        };
        
        trace!("Response body {} for namespace {}", body, self.namespace);

        let new_config: Value = match serde_json::from_str(&body) {
            Ok(c) => c,
            Err(e) => {
                self.loading.write().await.replace(false); // Reset flag
                debug!("error parsing config: {}", e);
                return Err(Error::Serde(e));
            }
        };
        trace!("parsed new_config: {:?}", new_config);

        // --- Start of new logic for change detection and event notification ---
        let old_config_option: Option<Value>;
        { 
            let memory_cache_guard = self.memory_cache.read().await;
            old_config_option = memory_cache_guard.clone();
        } 

        let changed = match &old_config_option {
            Some(old_c) => *old_c != new_config,
            None => true, 
        };

        if changed {
            debug!("Configuration changed for namespace {}. Notifying observers.", self.namespace);
            let event = ConfigurationChangeEvent {
                namespace_name: self.namespace.clone(),
                old_configuration: old_config_option.clone(), 
                new_configuration: new_config.clone(),      
            };
            
            let em = self.event_manager.clone();
            // Notifying observers is async, but we don't want to block refresh completion.
            // EventManager itself handles spawning tasks for observers.
            em.notify_observers(event).await; // `event` is cloned inside notify_observers if needed per observer.

        } else {
            trace!("No configuration change detected for namespace {}.", self.namespace);
        }
        // --- End of new logic ---

        cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                debug!("writing cache file {}", self.file_path.display());
                if let Some(parent) = self.file_path.parent() {
                    match std::fs::create_dir_all(parent) {
                        Ok(_) => (),
                        Err(e) => {
                            self.loading.write().await.replace(false); // Reset flag
                            return Err(Error::Io(e));
                        }
                    }
                }
                match std::fs::write(&self.file_path, &body) { // Write original body
                    Ok(_) => {
                        trace!("Wrote cache file {} for namespace {}", self.file_path.display(), self.namespace);
                    },
                    Err(e) => {
                        self.loading.write().await.replace(false); // Reset flag
                        return Err(Error::Io(e));
                    }
                }
            }
        }
        
        self.memory_cache.write().await.replace(new_config);

        trace!("Refreshed cache for namespace {}", self.namespace);
        self.loading.write().await.replace(false); // Reset flag at the end of successful operation

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
