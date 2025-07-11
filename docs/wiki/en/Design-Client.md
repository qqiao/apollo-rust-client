[中文简体](../zh-CN/Design-Client.md) | [中文繁體](../zh-TW/Design-Client.md)
[Back to Home](Home.md)

# Client Details

The `Client` struct is the primary user-facing interface for interacting with the Apollo configuration service. It manages configuration settings, namespace-specific caches, background refresh tasks, and event listeners.

## Fields

- **`client_config: ClientConfig`**:
  An instance of `ClientConfig` that holds all the configuration settings for this client. This is cloned and passed to each `Cache` instance created by the client.

- **`namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>`**:
  A thread-safe, shared collection of `Cache` instances.

  - `HashMap`: Maps a namespace name (String) to its corresponding `Cache`.
  - `Arc<Cache>`: Each `Cache` is wrapped in an `Arc` to allow shared ownership between the `Client` and potentially other parts of the application that hold onto a cache.
  - `RwLock`: Provides interior mutability with read-write locking, allowing multiple threads to read the `HashMap` concurrently or one thread to write to it (e.g., when adding a new namespace).
  - `Arc<RwLock<...>>`: The entire `HashMap` is wrapped in an `Arc` so that the background refresh task can also safely access and iterate over the namespaces.

- **`handle: Option<async_std::task::JoinHandle<()>>`**:
  _(Non-WASM only)_ An optional handle to the background task that periodically refreshes configurations. This is `Some` when the background task is running and `None` otherwise. It's used to `cancel` the task when `stop()` is called. For WASM, this is always `None` as task management differs.

- **`running: Arc<RwLock<bool>>`**:
  A flag to control the execution of the background refresh task.
  - `Arc<RwLock<...>>`: Shared between the `Client` and the background task. The `Client` sets it to `false` to signal the task to stop, and the task reads it to know when to exit.

## Core Methods

### `start(&mut self) -> Result<(), Error>`

Starts a background task that periodically refreshes all registered namespace caches.

This method spawns an asynchronous task using `async_std::task::spawn` on native targets or `wasm_bindgen_futures::spawn_local` on wasm32 targets. The task loops indefinitely (until `stop` is called) and performs the following actions in each iteration:

1. Iterates through all namespaces currently managed by the client.
2. Calls the `refresh` method on each namespace's `Cache` instance.
3. Logs any errors encountered during the refresh process.
4. Sleeps for a predefined interval (currently 30 seconds) before the next refresh cycle.

**Returns:**

- `Ok(())` if the background task was successfully started.
- `Err(Error::AlreadyRunning)` if the background task is already active.

### `stop(&mut self)`

Stops the background cache refresh task.

This method sets the `running` flag to `false`, signaling the background task to terminate its refresh loop. On non-wasm32 targets, it also attempts to explicitly cancel the spawned task by calling `cancel()` on its `JoinHandle` if it exists.

### `namespace(&self, namespace: &str) -> Result<Namespace, Error>` (Native Rust)

**For Native Rust targets only.**

Gets a typed namespace for the given namespace name. This method:

1. Looks up or creates a `Cache` for the specified namespace
2. Retrieves the current configuration value from the cache
3. Converts the raw JSON value into a typed `Namespace` enum based on format detection
4. Returns the typed namespace for direct use

**Arguments:**

- `namespace`: The namespace name to retrieve

**Returns:**

- `Ok(Namespace)`: A typed namespace enum (Properties, Json, or Text)
- `Err(Error)`: If the namespace cannot be retrieved or processed

**Example:**

```rust
let namespace = client.namespace("application").await?;
match namespace {
    Namespace::Properties(props) => {
        let value = props.get_string("key").await;
    }
    Namespace::Json(json) => {
        let config: MyConfig = json.to_object()?;
    }
    Namespace::Text(text) => {
        println!("Content: {}", text);
    }
}
```

### `namespace(&self, namespace: &str) -> Cache` (WASM)

**For WASM targets only.**

Gets a cache instance for the given namespace. This method:

1. Looks up or creates a `Cache` for the specified namespace
2. Returns the cache instance directly for property access
3. The cache provides methods like `get_string()`, `get_int()`, etc.

**Arguments:**

- `namespace`: The namespace name to retrieve

**Returns:**

- `Cache`: A cache instance for accessing configuration properties

**Example (JavaScript):**

```javascript
const cache = await client.namespace("application");
const value = await cache.get_string("key");
```

### `add_listener(&self, namespace: &str, listener: EventListener)` (Native Rust)

**For Native Rust targets only.**

Registers an event listener for configuration changes in a specific namespace.

**Arguments:**

- `namespace`: The namespace to listen for changes
- `listener`: An `Arc<dyn Fn(Result<Namespace, Error>) + Send + Sync>` closure

**Example:**

```rust
let listener = Arc::new(|result| {
    match result {
        Ok(namespace) => println!("Config updated: {:?}", namespace),
        Err(e) => println!("Config error: {:?}", e),
    }
});
client.add_listener("application", listener).await;
```

## Platform-Specific Behavior

### Native Rust Targets

- **Returns**: Typed `Namespace` enum from `namespace()` method
- **Caching**: Both in-memory and file-based caching
- **Background Tasks**: Full async task management with `JoinHandle`
- **Event Listeners**: Client-level event listeners with `Arc<dyn Fn>` closures
- **Thread Safety**: Full thread safety with `Send + Sync` requirements

### WASM Targets

- **Returns**: `Cache` instance directly from `namespace()` method
- **Caching**: Memory-only caching (no file system access)
- **Background Tasks**: Uses `spawn_local` without direct handles
- **Event Listeners**: Cache-level event listeners with JavaScript interop
- **Single-Threaded**: Designed for single-threaded JavaScript environments

## Error Handling

The `Client` can return the following errors:

- **`AlreadyRunning`**: When trying to start a client that's already running
- **`Namespace`**: Errors from namespace operations and format detection
- **`Cache`**: Errors from underlying cache operations

## Memory Management

### Native Rust

- Automatic memory management through Rust's ownership system
- Background tasks are properly cancelled when `stop()` is called
- `Arc` and `RwLock` provide safe concurrent access

### WASM

- **Critical**: Must call `free()` on the `Client` instance when done
- Cache instances returned by `namespace()` must also be freed
- Event listeners are automatically cleaned up when objects are freed
