//! # Cache for the Apollo Client
//!
//! This module provides the `Cache` struct, responsible for fetching, storing,
//! and managing configurations for a specific Apollo namespace. It supports
//! in-memory caching, optional file-based caching (for non-WASM targets),
//! and an event system to notify listeners of configuration updates.
//!
//! ## Event Handling
//!
//! The `Cache` can notify interested parties when configuration changes are detected
//! after a successful `refresh()` operation. This is achieved by registering listeners,
//! which can be either Rust components implementing the `EventListener` trait or
//! JavaScript functions.
//!
//! Upon a successful configuration refresh, a `ConfigUpdateEvent` is broadcast,
//! containing the namespace and a map of all configuration key-value pairs
//! (currently, it broadcasts the full configuration, not just deltas).

use crate::client_config::ClientConfig;
use crate::event_system::{ConfigUpdateEvent, ConfigValue, Event, EventListener};
use async_std::sync::RwLock;
use base64::display::Base64Display;
use js_sys::Function;
use std::collections::HashMap;
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
    listeners: Vec<Arc<dyn EventListener>>, // TODO: Consider thread-safety implications if Cache needs to be Send + Sync with JS listeners.

    #[cfg(not(target_arch = "wasm32"))]
    file_path: std::path::PathBuf,
}

// Wrapper for JavaScript function listeners
struct JsEventListenerWrapper {
    js_function: Function,
}

impl EventListener for JsEventListenerWrapper {
    fn on_event(&self, event: &Event) {
        match event {
            Event::ConfigUpdate(details) => {
                let event_js_obj = js_sys::Object::new();
                // It's good practice to handle potential errors from Reflect::set, though unwrap_or_default is pragmatic here.
                js_sys::Reflect::set(
                    &event_js_obj,
                    &JsValue::from_str("namespace"),
                    &JsValue::from_str(&details.namespace),
                )
                .unwrap_or_default();

                match details.get_changes_as_js_value() {
                    Ok(changes_val) => {
                        js_sys::Reflect::set(
                            &event_js_obj,
                            &JsValue::from_str("changes"),
                            &changes_val,
                        )
                        .unwrap_or_default();
                    }
                    Err(_) => {
                        // If conversion fails, set 'changes' to null or undefined in JS
                        js_sys::Reflect::set(
                            &event_js_obj,
                            &JsValue::from_str("changes"),
                            &JsValue::NULL,
                        )
                        .unwrap_or_default();
                    }
                }

                let this = JsValue::NULL; // 'this' context for the JS function call
                match self.js_function.call1(&this, &event_js_obj.into()) {
                    Ok(_) => {}
                    Err(e) => {
                        // Ensure web_sys is in Cargo.toml features = ["console"]
                        web_sys::console::error_2(
                            &JsValue::from_str("Error in JS event listener callback:"),
                            &e,
                        );
                    }
                }
            }
        }
    }
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
    ///
    /// Note: Event listeners are not automatically added; use `add_listener` or `add_js_listener`
    /// after creating the `Cache` instance if event notifications are needed.
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
            listeners: Vec::new(),

            #[cfg(not(target_arch = "wasm32"))]
            file_path,
        }
    }

    /// Adds a Rust-side listener that implements the `EventListener` trait.
    ///
    /// Listeners registered through this method will have their `on_event` method
    /// called when a configuration update occurs for this cache's namespace.
    ///
    /// # Arguments
    ///
    /// * `listener` - An `Arc`-wrapped object that implements the `EventListener` trait.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Ensure this example is adapted to your crate's structure and module paths.
    /// // use std::sync::{Arc, Mutex};
    /// // use std::collections::HashMap; // If you use it in your listener
    /// // use apollo_client_rs::{Cache, ClientConfig, Event, EventListener, ConfigValue, event_system}; // Adjust crate name
    ///
    /// // #[derive(Debug)]
    /// // struct MyRustListener {
    /// //     received_events: Mutex<Vec<event_system::Event>>, // Use the concrete Event type
    /// // }
    /// //
    /// // impl EventListener for MyRustListener {
    /// //     fn on_event(&self, event: &event_system::Event) {
    /// //         match event {
    /// //             event_system::Event::ConfigUpdate(details) => {
    /// //                 println!("MyRustListener received ConfigUpdate for namespace: {}", details.namespace);
    /// //                 // Access details.changes (HashMap<String, ConfigValue>) if needed
    /// //             }
    /// //         }
    /// //         self.received_events.lock().unwrap().push(event.clone());
    /// //     }
    /// // }
    /// //
    /// // async fn run_example() { // async due to cache operations
    /// //     let client_config = ClientConfig::new(
    /// //         "http://some-apollo-server:8080".to_string(),
    /// //         "your_app_id".to_string(),
    /// //         "default".to_string(),
    /// //         None, None, None, None
    /// //     );
    /// //     let mut cache = Cache::new(client_config, "application.properties"); // Example namespace
    /// //     let listener = Arc::new(MyRustListener { received_events: Mutex::new(Vec::new()) });
    /// //     cache.add_listener(listener.clone());
    /// //
    /// //     // Simulate a config update by calling refresh.
    /// //     // In a real application, refresh() would fetch from the server.
    /// //     // If the server provides new data, listeners will be notified.
    /// //     // match cache.refresh().await {
    /// //     //     Ok(()) => {
    /// //     //         if !listener.received_events.lock().unwrap().is_empty() {
    /// //     //             println!("Listener received an event after refresh.");
    /// //     //         } else {
    /// //     //             println!("Refresh successful, but no new event for listener (config might be unchanged or no listener for this specific type).");
    /// //     //         }
    /// //     //     },
    /// //     //     Err(e) => eprintln!("Cache refresh failed: {:?}", e),
    /// //     // }
    /// // }
    /// ```
    pub fn add_listener(&mut self, listener: Arc<dyn EventListener>) {
        self.listeners.push(listener);
    }

    /// Helper function to broadcast an event to all registered listeners.
    /// This is called internally after a successful configuration refresh.
    fn broadcast_event(&self, event: &Event) {
        for listener in &self.listeners {
            listener.on_event(event);
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
    ///
    /// Upon successful fetch and parsing of new configuration data, this method
    /// also triggers the broadcasting of a `ConfigUpdateEvent` to all registered
    /// listeners, notifying them of the potential changes.
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
        trace!("parsed config: {:?}", config_value_json);

        // Convert serde_json::Value to HashMap<String, ConfigValue>
        let mut config_changes: HashMap<String, ConfigValue> = HashMap::new();
        if let Value::Object(map) = &config_value_json {
            for (k, v) in map {
                let cfg_val = match v {
                    Value::String(s) => ConfigValue::String(s.clone()),
                    Value::Number(n) => ConfigValue::Number(n.as_f64().unwrap_or(0.0)), // Handle potential conversion failure
                    Value::Bool(b) => ConfigValue::Boolean(*b),
                    _ => continue, // Skip non-primitive types or handle them as needed
                };
                config_changes.insert(k.clone(), cfg_val);
            }
        }

        self.memory_cache.write().await.replace(config_value_json);

        // Broadcast the config update event
        let config_update_event_details = ConfigUpdateEvent::new(self.namespace.clone(), config_changes);
        self.broadcast_event(&Event::ConfigUpdate(config_update_event_details));

        trace!("Refreshed cache for namespace {}", self.namespace);
        *loading = false;

        Ok(())
    }
}

#[wasm_bindgen]
impl Cache {
    /// Adds a JavaScript function as an event listener.
    ///
    /// The provided JavaScript function will be called when a configuration update
    /// event occurs for this cache's namespace. The function will receive a
    /// JavaScript object as its first argument, containing:
    /// - `namespace` (String): The namespace of the updated configuration.
    /// - `changes` (Object): An object where keys are configuration item names
    ///   and values are their new values (converted from `ConfigValue` to JS primitives).
    ///
    /// Note: Due to the nature of `js_sys::Function`, the `EventListener` trait
    /// (and thus `Cache` when holding JS listeners) is not `Send` or `Sync`.
    /// This is generally acceptable in single-threaded WASM environments.
    ///
    /// --- JavaScript Example ---
    /// ```javascript
    /// // Assuming 'cache' is an instance of your WASM 'Cache' object,
    /// // likely obtained from initializing your WASM module.
    ///
    /// function myConfigListener(event) {
    ///   console.log("JS listener received event for namespace:", event.namespace);
    ///   console.log("Changes:", event.changes); // event.changes will be a JS object
    ///
    ///   // Example: Access a specific changed value
    ///   // if (event.changes && typeof event.changes.timeout !== 'undefined') {
    ///   //   console.log("New timeout value:", event.changes.timeout);
    ///   //   handleTimeoutChange(event.changes.timeout);
    ///   // }
    /// }
    ///
    /// // cache.addEventListener(myConfigListener);
    ///
    /// // To stop listening, you can currently remove all listeners:
    /// // cache.removeAllListeners();
    /// // A future enhancement might allow removing specific JS listeners.
    /// ```
    #[wasm_bindgen(js_name = addEventListener)]
    pub fn add_js_listener(&mut self, js_function: Function) {
        let wrapper = Arc::new(JsEventListenerWrapper { js_function });
        self.listeners.push(wrapper);
    }

    /// Removes all registered listeners (both Rust and JavaScript) from this cache instance.
    ///
    /// After calling this, no listeners will be notified of subsequent configuration updates
    /// unless new listeners are added.
    #[wasm_bindgen(js_name = removeAllListeners)]
    pub fn remove_all_listeners(&mut self) {
        self.listeners.clear();
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
    use crate::event_system::{Event, EventListener, ConfigValue, ConfigUpdateEvent}; // Ensure these are in scope
    use crate::client_config::ClientConfig; // For creating Cache instances
    use std::cell::RefCell;
    use std::sync::Arc;
    use std::collections::HashMap;
    use js_sys::Function; // For the JS listener registration test
    use wasm_bindgen_test::wasm_bindgen_test; // If testing wasm specific parts, like JsEventListenerWrapper interaction
                                           // Might not be needed if we only test Rust logic for add_js_listener.

    // Mock Event Listener for testing Rust-side listener logic
    #[derive(Debug)]
    struct MockListener {
        received_events: RefCell<Vec<Event>>,
    }

    impl MockListener {
        fn new() -> Self {
            Self {
                received_events: RefCell::new(Vec::new()),
            }
        }
    }

    impl EventListener for MockListener {
        fn on_event(&self, event: &Event) {
            self.received_events.borrow_mut().push(event.clone());
        }
    }

    // Helper to create a Cache instance for tests
    fn create_test_cache(namespace: &str) -> Cache {
        let client_config = ClientConfig::new(
            "http://localhost:8080".to_string(), // Dummy server URL
            "test_app_id".to_string(),
            "default".to_string(), // Dummy cluster
            None, None, None, None,
        );
        Cache::new(client_config, namespace)
    }

    // Helper to create sample changes for events
    fn sample_changes_map() -> HashMap<String, ConfigValue> {
        let mut changes = HashMap::new();
        changes.insert("key_string".to_string(), ConfigValue::String("value1".to_string()));
        changes.insert("key_number".to_string(), ConfigValue::Number(123.45));
        changes.insert("key_bool".to_string(), ConfigValue::Boolean(true));
        changes
    }

    #[test]
    fn test_rust_listener_receives_config_update() {
        let mut cache = create_test_cache("test_namespace_rust_listener");
        let listener = Arc::new(MockListener::new());
        cache.add_listener(listener.clone());

        let changes = sample_changes_map();
        cache.trigger_test_config_update(changes.clone());

        let received = listener.received_events.borrow();
        assert_eq!(received.len(), 1, "Listener should have received one event.");

        match &received[0] {
            Event::ConfigUpdate(details) => {
                assert_eq!(details.namespace, "test_namespace_rust_listener");
                assert_eq!(details.changes.get("key_string"), Some(&ConfigValue::String("value1".to_string())));
                assert_eq!(details.changes.get("key_number"), Some(&ConfigValue::Number(123.45)));
                assert_eq!(details.changes.get("key_bool"), Some(&ConfigValue::Boolean(true)));
            } // _ => panic!("Incorrect event type received"), // Not needed if only ConfigUpdate exists
        }
    }

    #[test]
    fn test_remove_all_listeners() {
        let mut cache = create_test_cache("test_namespace_remove_listener");
        let listener = Arc::new(MockListener::new());
        cache.add_listener(listener.clone());

        cache.trigger_test_config_update(sample_changes_map());
        assert_eq!(listener.received_events.borrow().len(), 1, "Listener should have received one event before removal.");

        cache.remove_all_listeners();
        assert!(cache.listeners.is_empty(), "Listeners list should be empty after remove_all_listeners.");

        cache.trigger_test_config_update(sample_changes_map()); // Simulate another event
        assert_eq!(listener.received_events.borrow().len(), 1, "Listener should not receive new events after removal.");
    }

    #[test]
    fn test_multiple_rust_listeners_receive_event() {
        let mut cache = create_test_cache("test_namespace_multi_listeners");
        let listener1 = Arc::new(MockListener::new());
        let listener2 = Arc::new(MockListener::new());

        cache.add_listener(listener1.clone());
        cache.add_listener(listener2.clone());

        let changes = sample_changes_map();
        cache.trigger_test_config_update(changes.clone());

        assert_eq!(listener1.received_events.borrow().len(), 1, "Listener 1 should have received one event.");
        assert_eq!(listener2.received_events.borrow().len(), 1, "Listener 2 should have received one event.");

        match &listener1.received_events.borrow()[0] {
             Event::ConfigUpdate(details) => {
                assert_eq!(details.changes, changes, "Listener 1 received incorrect event data.");
            }
        }
        match &listener2.received_events.borrow()[0] {
             Event::ConfigUpdate(details) => {
                assert_eq!(details.changes, changes, "Listener 2 received incorrect event data.");
            }
        }
    }

    #[test]
    fn test_broadcast_with_no_listeners() {
        let cache = create_test_cache("test_namespace_no_listeners");
        // No listeners added
        cache.trigger_test_config_update(sample_changes_map());
        // Test passes if no panic/error occurs.
        // Can also check internal state if relevant, but here just ensuring it handles empty list.
        assert!(cache.listeners.is_empty(), "Cache should have no listeners registered for this test.");
    }

    #[test]
    #[cfg(target_arch = "wasm32")] // This test is more meaningful in a WASM context for js_sys::Function
    fn test_js_listener_registration_structurally_works() {
        use wasm_bindgen_test::wasm_bindgen_test; // Required for wasm tests
        wasm_bindgen_test_configure!(run_in_browser); // or run_in_worker

        let mut cache = create_test_cache("test_namespace_js_listener_reg");
        assert_eq!(cache.listeners.len(), 0, "Cache should start with no listeners.");

        // Create a dummy JS function. In a real WASM test env, this would be a JS function.
        // For a pure Rust unit test simulating WASM, this is tricky.
        // `Function::new_no_args("return 1;")` is a way to create a simple JS function.
        let dummy_js_function = Function::new_no_args("console.log('dummy js func created');");

        cache.add_js_listener(dummy_js_function);
        assert_eq!(cache.listeners.len(), 1, "Cache should have one listener after adding JS listener.");
        // Further checks could inspect the type of listener if we had a way to distinguish JsEventListenerWrapper,
        // but for now, checking length is a good structural verification.
    }

    // Original tests from the file
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
