[中文简体](../zh-CN/Design-Cache.md) | [中文繁體](../zh-TW/Design-Cache.md)
[Back to Home](Home.md)

# Cache Details

The `Cache` struct is responsible for managing the configuration for a single Apollo namespace. It handles fetching, parsing, storing, refreshing, and providing access to configuration data.

## Fields

- **`client_config: ClientConfig`**:
  A copy of the `ClientConfig` containing settings like server URL, app ID, etc.

- **`namespace: String`**:
  The name of the Apollo namespace this cache instance is responsible for (e.g., "application", "config.json").

- **`loading: Arc<RwLock<bool>>`**:
  A flag to prevent concurrent `refresh()` operations on the same `Cache` instance. If a refresh is already in progress, other calls to `refresh()` will return `Error::AlreadyLoading`.

- **`checking_cache: Arc<RwLock<bool>>`**:
  _(Non-WASM only)_ A flag to prevent concurrent cache initialization operations. This prevents multiple concurrent attempts to read and parse the same file cache.

- **`memory_cache: Arc<RwLock<Option<serde_json::Value>>>`**:
  An in-memory store of the parsed configuration for the namespace.

  - `serde_json::Value`: The raw configuration data as JSON
  - `Option<...>`: It's `None` if the cache hasn't been populated yet or if a fetch failed
  - `Arc<RwLock<...>>`: Allows thread-safe interior mutability for reading and updating the memory cache

- **`listeners: Arc<RwLock<Vec<EventListener>>>`**:
  A collection of event listeners that will be notified when the cache is refreshed.

  - For native targets: `Arc<dyn Fn(Result<Namespace, Error>) + Send + Sync>`
  - For WASM targets: `Arc<dyn Fn(Result<serde_json::Value, Error>)>`

- **`file_path: std::path::PathBuf`**:
  _(Non-WASM only)_ The full path to the local file where this namespace's configuration is cached. It's typically constructed as `{cache_dir}/{app_id}_{cluster}_{namespace}.cache.json`.

## Core Methods & Logic

### `new(client_config: ClientConfig, namespace: &str) -> Self`

Creates a new cache instance for a specific namespace.

- Constructs the cache file path (non-WASM only)
- Initializes all internal state to default values
- Sets up thread-safe containers for caching and synchronization

### `get_value(&self) -> Result<serde_json::Value, Error>`

Retrieves the raw configuration value for the namespace. This method:

1. **Checks concurrent access**: Acquires `checking_cache` lock to prevent concurrent operations
2. **Memory cache check**: First checks if data is available in `memory_cache`
3. **File cache check** (non-WASM): Attempts to load from file if memory cache is empty
4. **Refresh operation**: Triggers `refresh()` if no cached data is available
5. **Returns value**: Returns the cached JSON value or an error

### `refresh(&self) -> Result<(), Error>`

Fetches the latest configuration from the Apollo server and updates caches:

1. **Concurrency control**: Acquires `loading` lock to prevent concurrent refreshes
2. **URL construction**: Builds the Apollo server request URL with namespace and query parameters
3. **Authentication**: Adds HMAC-SHA1 signature if `secret` is configured
4. **HTTP request**: Sends GET request to Apollo server
5. **Cache updates**:
   - Updates file cache (non-WASM only)
   - Updates memory cache
   - Notifies all registered event listeners
6. **Error handling**: Proper cleanup of locks on failure

### `add_listener(&self, listener: EventListener)`

Registers an event listener for configuration changes.

- **Native targets**: Listener receives `Result<Namespace, Error>`
- **WASM targets**: Listener receives `Result<serde_json::Value, Error>`
- Listeners are called when `refresh()` successfully updates the configuration

## Platform-Specific Methods

### WASM-Only Methods

These methods are only available on WASM targets and provide direct property access:

- **`get_string(&self, key: &str) -> Option<String>`**:
  Retrieves a string property from the configuration.

- **`get_int(&self, key: &str) -> Option<i64>`**:
  Retrieves an integer property from the configuration.

- **`get_float(&self, key: &str) -> Option<f64>`**:
  Retrieves a float property from the configuration.

- **`get_bool(&self, key: &str) -> Option<bool>`**:
  Retrieves a boolean property from the configuration.

- **`add_listener_wasm(&self, js_listener: JsFunction)`**:
  Registers a JavaScript function as an event listener. The function receives `(error, data)` parameters.

### Native-Only Methods

- **File-based caching**: Automatic persistence of configuration to disk
- **Background refresh**: Integration with client-level background refresh tasks
- **Thread safety**: Full `Send + Sync` compliance for multi-threaded environments

## Error Handling

The `Cache` can return the following errors:

- **`AlreadyLoading`**: Concurrent refresh attempts
- **`AlreadyCheckingCache`**: Concurrent cache initialization attempts
- **`NamespaceNotFound`**: Namespace doesn't exist on Apollo server
- **`Reqwest`**: Network communication errors
- **`UrlParse`**: Invalid URL construction
- **`Serde`**: JSON parsing errors
- **`Io`**: File system errors (native only)

## Event Listener System

### Native Rust

```rust
let listener = Arc::new(|result: Result<Namespace, Error>| {
    match result {
        Ok(namespace) => {
            match namespace {
                Namespace::Properties(props) => {
                    // Handle properties update
                }
                Namespace::Json(json) => {
                    // Handle JSON update
                }
                Namespace::Text(text) => {
                    // Handle text update
                }
            }
        }
        Err(e) => {
            eprintln!("Configuration update failed: {}", e);
        }
    }
});
cache.add_listener(listener).await;
```

### WASM (JavaScript)

```javascript
await cache.add_listener((data, error) => {
  if (error) {
    console.error("Configuration update error:", error);
  } else {
    console.log("Configuration updated:", data);
    // data is the raw JSON configuration object
  }
});
```

## Memory Management

### Native Rust

- Automatic memory management through Rust's ownership system
- `Arc` and `RwLock` provide safe concurrent access
- Event listeners are automatically cleaned up when cache is dropped

### WASM

- **Critical**: Must call `free()` on cache instances when done
- Event listeners are automatically cleaned up when cache is freed
- Memory-only caching (no file system access)

## Performance Considerations

- **Concurrent access**: Uses `RwLock` for efficient read-heavy workloads
- **Memory efficiency**: Stores raw JSON and parses on-demand
- **Network efficiency**: Includes authentication and proper error handling
- **File caching**: Reduces network requests on restart (native only)
