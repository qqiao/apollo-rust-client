[中文简体](../zh-CN/Design-Cache.md) | [中文繁體](../zh-TW/Design-Cache.md)
[Back to Home](Home.md)

# Cache Details

The `Cache` struct is responsible for managing the configuration for a single Apollo namespace. It handles fetching, parsing, storing, and refreshing this configuration.

## Fields

-   **`client_config: ClientConfig`**:
    A copy of the `ClientConfig` containing settings like server URL, app ID, etc.

-   **`namespace: String`**:
    The name of the Apollo namespace this cache instance is responsible for (e.g., "application", "FX.Properties").

-   **`loading: Arc<RwLock<bool>>`**:
    A flag to prevent concurrent `refresh()` operations on the same `Cache` instance. If a refresh is already in progress, other calls to `refresh()` (or methods that trigger it) will wait or return early.

-   **`checking_cache: Arc<RwLock<bool>>`**:
    *(Non-WASM only)* A flag similar to `loading`, but specifically for the initial check/load from the file cache when a property is first requested. This prevents multiple concurrent attempts to read and parse the same file cache.

-   **`memory_cache: Arc<RwLock<Option<serde_json::Value>>>`**:
    An in-memory store of the parsed configuration for the namespace.
    -   `serde_json::Value`: The configuration is typically a JSON object (map of string keys to string values), so `Value` can represent this structure.
    -   `Option<...>`: It's `None` if the cache hasn't been populated yet or if a fetch failed.
    -   `Arc<RwLock<...>>`: Allows thread-safe interior mutability for reading and updating the memory cache.

-   **`file_path: std::path::PathBuf`**:
    *(Non-WASM only)* The full path to the local file where this namespace's configuration is cached. It's typically constructed as `{cache_dir}/{app_id}_{cluster}_{namespace}.json`, where `cache_dir` is determined by `client_config.get_cache_dir()`. For WASM, this field still exists but is not actively used for file I/O.

## Core Methods & Logic

-   **`new(client_config: ClientConfig, namespace: String) -> Self`**:
    The constructor. It initializes `loading` and `checking_cache` to `false`, `memory_cache` to `None`, and constructs the `file_path` based on the provided configuration.

-   **`get_property<T: DeserializeOwned + Send + Sync + 'static>(&self, key: &str) -> impl Future<Output = Option<T>> + Send`**:
    (And `get_value(key)` which is similar but returns `Option<serde_json::Value>`)
    This is the primary method for retrieving a configuration value. It follows a tiered lookup logic:
    1.  **Check Memory Cache**: It first tries to get the value from `self.memory_cache`.
    2.  **Check File Cache (Non-WASM)**:
        -   If not in memory and on a non-WASM target, it attempts to load the configuration from the `self.file_path`.
        -   This involves acquiring a lock on `self.checking_cache`.
        -   If the file exists, its content is read, parsed as JSON, and then `self.memory_cache` is updated. The value is then retrieved from the (now populated) memory cache.
    3.  **Refresh if Necessary**:
        -   If the value is still not found in the memory cache (either because it wasn't there initially or the file cache load failed or didn't contain the key), it calls `self.refresh().await`.
        -   After `refresh()` completes, it tries to get the value from `self.memory_cache` again.
    -   The retrieved `serde_json::Value` is then cloned and an attempt is made to deserialize it into the requested type `T`.

-   **`refresh(&self) -> impl Future<Output = Result<(), Error>> + Send`**:
    Fetches the latest configuration from the Apollo server and updates both the memory and file caches.
    1.  **Lock `loading`**: Acquires a write lock on `self.loading`. If already locked, it waits. This prevents concurrent refresh operations.
    2.  **Construct URL**: Builds the request URL for the Apollo server:
        `{config_server}/configs/{app_id}/{cluster}/{namespace_encoded}?ip={ip}&label={label}`
        -   `namespace_encoded`: The namespace name is URL-encoded.
        -   `ip` and `label` are taken from `client_config` and added as query parameters if present.
    3.  **Authentication (Signing)**:
        -   If `client_config.secret` is `Some(secret)`, the request needs to be signed.
        -   A timestamp is generated.
        -   The `sign()` helper function is called with the timestamp, the relative URL path (e.g., `/configs/...`), and the secret.
        -   The `sign()` function computes an HMAC-SHA1 signature.
        -   The signature and timestamp are added as `Authorization` and `Timestamp` headers to the HTTP request.
    4.  **HTTP GET Request**: An asynchronous HTTP GET request is made to the constructed URL using a library like `reqwest`.
    5.  **Process Response**:
        -   If the request is successful (e.g., HTTP 200 OK):
            -   The response body is parsed as JSON into `serde_json::Value`.
            -   `*self.memory_cache.write().unwrap() = Some(parsed_value)`.
            -   *(Non-WASM only)*: The parsed JSON is serialized back to a string and written to `self.file_path`. This involves creating the cache directory if it doesn't exist.
        -   If the request returns HTTP 304 Not Modified: It means the configuration hasn't changed since the last fetch (Apollo uses ETag-like mechanisms, though this client doesn't explicitly manage ETags, the server might respond with 304 if client polls frequently). The cache remains unchanged.
        -   Other HTTP errors or network errors result in an `Error` being returned.
    6.  **Unlock `loading`**: The lock is released when the method exits.

## Helper `sign(timestamp: &str, url_path: &str, secret: &str) -> String` function

-   This internal static function is responsible for generating the signature required by Apollo for authenticated requests.
-   **Input**: Timestamp string, the URL path (e.g., `/configs/appId/cluster/namespace`), and the secret key.
-   **Process**:
    1.  Creates a string to sign: `"{timestamp}\n{url_path}"`.
    2.  Uses an HMAC-SHA1 implementation (from the `hmac` and `sha1` crates).
    3.  Computes the HMAC-SHA1 hash of the string to sign using the secret key.
    4.  Encodes the resulting hash in Base64.
-   **Output**: The Base64 encoded signature string.

This structured approach ensures that configuration is efficiently retrieved, cached, and kept up-to-date, while also handling potential concurrency issues and authentication requirements.
