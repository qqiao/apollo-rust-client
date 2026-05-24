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
  A flag to coordinate concurrent network fetches and prevent duplicate requests.

- **`loading_complete: Arc<Notify>`**:
  A notification trigger used to unblock waiting threads once loading succeeds.

- **`memory: Arc<RwLock<Option<serde_json::Value>>>`**:
  An in-memory store of the parsed configuration for the namespace.

  - `serde_json::Value`: The raw configuration data as JSON
  - `Option<...>`: It's `None` if the cache hasn't been populated yet or if a fetch failed
  - `Arc<RwLock<...>>`: Allows thread-safe interior mutability for reading and updating the memory cache

- **`listeners: Arc<RwLock<Vec<EventListener>>>`**:
  A collection of event listeners that will be notified when the cache is refreshed.

  - For native targets: `Arc<dyn Fn(Result<Namespace, Error>) + Send + Sync>`
  - For WASM targets: `Arc<dyn Fn(Result<Namespace, Error>)>`

- **`wasm_cache_key: String`**:
  _(WASM only)_ The isolated storage key used in local storage.

- **`file_path: std::path::PathBuf`**:
  _(Non-WASM only)_ The full path to the local file where this namespace's configuration is cached. It's typically constructed as `{cache_dir}/{app_id}_{cluster}_{namespace}.cache.json`.

- **`http_client: reqwest::Client`**:
  Reused HTTP worker client instance.

## Core Methods & Logic

### `new(client_config: ClientConfig, namespace: &str) -> Self`

Creates a new cache instance for a specific namespace.

- Constructs the cache file path (non-WASM only)
- Initializes all internal state to default values
- Sets up thread-safe containers for caching and synchronization

### `get_value(&self) -> Result<serde_json::Value, Error>`

Retrieves the raw configuration value for the namespace. This method:

1. **Fast-path check**: Reads the `memory` cache directly using a read lock.
2. **Double-Checked Locking (DCL)**: If empty, tries to acquire the write lock on `loading`. If another thread is already fetching (`*loading == true`), it drops the lock and waits on `loading_complete.notified().await`.
3. **Loader Fetch**: The loader thread proceeds to fetch:
   - Loads from local file (Native) or `localStorage` (WASM) if available and not stale.
   - Otherwise, fetches from remote Apollo server.
4. **Cache populate**: Populates `memory`, saves to persistent cache, and wakes up all waiters via `loading_complete.notify_waiters()`.

### `refresh(&self) -> Result<(), Error>`

Fetches the latest configuration from the Apollo server and updates caches:

1. **Memory Lock**: Acquires the write lock on `memory` to perform the update.
2. **Network request**: Sends a GET request to the Apollo server.
3. **Cache updates**: Replaces the memory cache and writes back to the active persistent cache (disk/localStorage).
4. **Release & Notify**: Drops the memory lock and triggers all registered event listeners.

### `add_listener(&self, listener: EventListener)`

Registers an event listener for configuration changes.

- Listeners are called when `refresh()` successfully updates the configuration.

### WASM properties format delegation

The `Cache` struct itself is not exposed directly to JavaScript. Instead, the JS binding returns a mapped `Namespace` variant. For the Properties format, this is returned as the JS `Properties` class, which exposes synchronous property getters:

- **`get_string(key: &str) -> Option<String>`**:
  Retrieves a string property from the configuration.

- **`get_int(key: &str) -> Option<i64>`**:
  Retrieves an integer property from the configuration.

- **`get_float(key: &str) -> Option<f64>`**:
  Retrieves a float property from the configuration.

- **`get_bool(key: &str) -> Option<bool>`**:
  Retrieves a boolean property from the configuration.

### Event Listeners registration

Event listeners are registered at the `Client` level using `client.add_listener(namespace, callback)` on WASM targets.

### Native-Only Methods

- **File-based caching**: Automatic persistence of configuration to disk.
- **Background refresh**: Integration with client-level background refresh tasks.
- **Thread safety**: Full `Send + Sync` compliance for multi-threaded environments.

The `Cache` can return the following errors:

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
                    let value = props.get_string("key");
                }
                Namespace::Json(json) => {
                    let config: MyConfig = json.to_object().unwrap();
                }
                Namespace::Text(text) => {
                    println!("Content: {}", text);
                }
                _ => {}
            }
        }
        Err(e) => {
            eprintln!("Configuration update failed: {}", e);
        }
    }
});
client.add_listener("application", listener).await;
```

### WASM (JavaScript)

```javascript
await client.add_listener("application", (data, error) => {
  if (error) {
    console.error("Configuration update error:", error);
  } else {
    console.log("Configuration updated:", data);
    // data is a Properties instance for Properties format, or raw JS object/string for JSON/YAML/Text formats
  }
});
```

## Memory Management

### Native Rust

- Automatic memory management through Rust's ownership system
- `Arc` and `RwLock` provide safe concurrent access
- Event listeners are automatically cleaned up when cache is dropped

### WASM

- **Critical**: Must call `.free()` on `Client`, `ClientConfig`, and returned `Properties` class instances when done.
- Raw JSON, YAML, and Text configurations do not need manual freeing as they are standard JS values.

## Performance Considerations

- **Concurrent access**: Uses `RwLock` for efficient read-heavy workloads
- **Memory efficiency**: Stores raw JSON and parses on-demand
- **Network efficiency**: Includes authentication and proper error handling
- **File caching**: Reduces network requests on restart (native only)
