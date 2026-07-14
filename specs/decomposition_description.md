# SDD Decomposition Description Viewpoint

This viewpoint decomposes the `apollo-rust-client` into its primary component blocks: **`ClientConfig`**, **`Client`**, **`Cache`**, and **`Namespace`**. Each module/struct is detailed below, specifying its purpose, state, and functionality.

---

## 1. Component Tree Overview

The relationship between the components is hierarchical, structured as follows:

```
Client (Main Entry)
  ├── ClientConfig (Connection and Settings)
  └── Namespaces (HashMap inside RwLock)
        └── Cache (Namespace-specific cache instances)
              ├── memory (RwLock<Option<CacheItem>>)
              ├── load_lock (Mutex single-flight gate)
              ├── listeners (RwLock<Vec<EventListener>>)
              └── Namespace (Typed format representations)
                    ├── Properties (Key-Value Strings)
                    ├── Json (serde_json::Value wrapper)
                    ├── Yaml (noyalib-backed document)
                    └── Text (Raw plain String)
```

---

## 2. Structural Elements Detail

### 2.1 `ClientConfig`
Encapsulates all server connectivity settings, credentials, local file path parameters, and target configurations.

- **Role**: Configuration data object. Passed down to client caches upon creation.
- **Attributes**:
  - `app_id: String` (Required): Unique application identifier registered in Apollo.
  - `config_server: String` (Required): Fully qualified URL of the configuration service endpoint.
  - `cluster: String` (Required): Cluster designation. Defaults to `"default"`.
  - `secret: Option<String>` (Optional): Encryption secret key used to compute authentication signatures.
  - `cache_dir: Option<String>` (Optional, Native-only): Override the cache base directory. The default follows platform cache-directory conventions; versioned hashed filenames isolate request identities.
  - `cache_ttl: Option<u64>` (Optional): Number of seconds memory, disk, and localStorage values remain fresh. Defaults to 600; zero means always revalidate cached values in the background.
  - `refresh_interval: Option<u64>` (Optional): Native/WASM polling interval in seconds. Defaults to 30 and must be greater than zero.
  - `request_timeout: Option<u64>` (Optional): Complete request and body timeout. Defaults to 10 and must be greater than zero.
  - `label: Option<String>` (Optional): Metadata tag for canary releases.
  - `ip: Option<String>` (Optional): Client node IP address for grayscale routing.
  - `allow_insecure_https: Option<bool>` (Optional): Cert bypass flag for development testing. On WebAssembly (`wasm32`) targets, this is ignored and triggers a warning log because SSL/TLS validation is strictly controlled by the browser sandbox environment.
  - `http_client: Option<reqwest::Client>` (Optional, Native-only): Injected custom HTTP client instance containing pre-configured corporate proxies, custom request headers, or tracing interceptors. Skip in WASM.
- **Key Methods**:
  - `from_env() -> Result<Self, Error>`: Reads matching native environment variables or Node.js `globalThis.process.env`; browsers return a clear environment error.
  - `get_cache_dir() -> PathBuf`: Computes the actual folder path for cached files.

---

### 2.2 `Client`
The primary operational coordinator and entry point. It manages lifecycle worker states and coordinates namespace caches.

- **Role**: State Manager and Cache Factory.
- **State Properties**:
  - `config: ClientConfig`: Active connection settings.
  - `namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>`: Live registry mapping namespace keys (e.g. `application`, `app.json`) to `Cache` instances.
  - `handle: Option<tokio::task::JoinHandle<()>>` (Native-only): Reference hook to clean up/cancel background polling tasks.
  - `running: Arc<AtomicBool>`: Lock-free lifecycle state shared with the background task.
  - `http_client: reqwest::Client`: Reused HTTP worker client sharing connection pools.
- **Key Methods**:
  - `new(config: ClientConfig) -> Result<Self, Error>`: Validates configuration and constructs the HTTP client without silent fallback.
  - `start(&mut self) -> Result<(), Error>`: Spawns the background worker task that refreshes all caches every 30 seconds.
  - `stop(&mut self)`: Shuts down background loops and cancels task execution handles.
  - `preload(&self, namespaces: &[impl AsRef<str>]) -> Result<(), Error>`: Concurrently fetches configurations for selected namespaces at initialization.
  - `namespace(&self, namespace: &str) -> Result<Namespace, Error>`: Looks up or generates the namespace cache, retrieves the JSON config, and parses it into a typed representation.

---

### 2.3 `Cache`
Handles retrieving, updating, and holding configuration data for a single namespace.

- **Role**: Active caching unit.
- **State Properties**:
  - `client_config: ClientConfig`: Copy of credentials and connection URLs.
  - `namespace: String`: The namespace name.
  - `memory: Arc<RwLock<Option<CacheItem>>>`: Timestamped in-memory JSON document cache.
  - `listeners: Arc<RwLock<Vec<EventListener>>>`: Ordered observers notified on changes and refresh errors.
  - `wasm_cache_key: String` (WASM-only): The unique key used for browser localStorage.
  - `load_lock: Arc<Mutex<()>>`: Cancellation-safe single-flight gate for cold loads.
  - `refresh_lock: Arc<Mutex<()>>`: Coalesces manual, polling, and stale-while-revalidate refreshes.
  - `file_path: PathBuf` (Native-only): Dedicated storage path on disk.
  - `http_client: reqwest::Client`: Reused HTTP client instance.
- **Key Methods**:
  - `get_value(&self) -> Result<Value, Error>`: Executes the TTL-aware read flow, returning stale values immediately while one background revalidation runs.
  - `refresh(&self) -> Result<(), Error>`: Requests remote synchronization, coalescing with an in-flight refresh.
  - `notify_listeners(&self, config: &Value, listeners: &[EventListener])`: Invokes listeners synchronously in registration order after internal locks are released.

---

### 2.4 `Namespace` (Enum)
A high-level typed wrapper enabling consumers to interact with configurations using native formats.

- **Role**: Representation Model.
- **Enum Variants**:
  - `Properties(Properties)`: Simple key-value string mapping.
  - `Json(Json)`: Deep structured JSON.
  - `Yaml(Yaml)`: Structured YAML parsing.
  - `Text(String)`: Unstructured raw content.

#### `Properties`
Contains key-value maps. Provides methods for type conversions:
- `get_string(&self, key: &str) -> Option<String>`
- `get_int(&self, key: &str) -> Option<i64>`
- `get_float(&self, key: &str) -> Option<f64>`
- `get_bool(&self, key: &str) -> Option<bool>`
- `get_property<T: FromStr>(&self, key: &str) -> Option<T>` (generic parser)

#### `Json`
Maintains a `serde_json::Value`.
- `to_object<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error>`: Converts raw JSON config directly to typed Rust structs.

#### `Yaml`
Holds a YAML formatted string.
- `to_object<T: DeserializeOwned + 'static>(&self) -> Result<T, noyalib::Error>`: Converts YAML 1.1-compatible documents directly to typed Rust structs.
