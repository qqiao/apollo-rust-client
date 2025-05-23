[中文简体](../zh-CN/Design-Client.md) | [中文繁體](../zh-TW/Design-Client.md)
[Back to Home](Home.md)

# Client Details

The `Client` struct is the primary user-facing interface for interacting with the Apollo configuration service. It manages configuration settings, namespace-specific caches, and background refresh tasks.

## Fields

-   **`client_config: ClientConfig`**:
    An instance of `ClientConfig` that holds all the configuration settings for this client. This is cloned and passed to each `Cache` instance created by the client.

-   **`namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>`**:
    A thread-safe, shared collection of `Cache` instances.
    -   `HashMap`: Maps a namespace name (String) to its corresponding `Cache`.
    -   `Arc<Cache>`: Each `Cache` is wrapped in an `Arc` to allow shared ownership between the `Client` and potentially other parts of the application that hold onto a cache.
    -   `RwLock`: Provides interior mutability with read-write locking, allowing multiple threads to read the `HashMap` concurrently or one thread to write to it (e.g., when adding a new namespace).
    -   `Arc<RwLock<...>>`: The entire `HashMap` is wrapped in an `Arc` so that the background refresh task can also safely access and iterate over the namespaces.

-   **`handle: Option<async_std::task::JoinHandle<()>>`**:
    *(Non-WASM only)* An optional handle to the background task that periodically refreshes configurations. This is `Some` when the background task is running and `None` otherwise. It's used to `cancel` the task when `stop()` is called. For WASM, this is always `None` as task management differs.

-   **`running: Arc<RwLock<bool>>`**:
    A flag to control the execution of the background refresh task.
    -   `Arc<RwLock<...>>`: Shared between the `Client` and the background task. The `Client` sets it to `false` to signal the task to stop, and the task reads it to know when to exit.

## Core Methods

-   **`new(client_config: ClientConfig) -> Self`**:
    The constructor for the `Client`. It initializes the `namespaces` map as empty and sets `handle` to `None` and `running` to `false` (initially).

-   **`namespace(&mut self, name: &str) -> impl Future<Output = Result<CACHE_RETURN_TYPE, Error>> + Send`**:
    Retrieves a `Cache` for the given namespace name.
    -   It first checks if a `Cache` for the namespace already exists in the `namespaces` map.
    -   If yes, it returns the existing `Cache`.
    -   If no, it creates a new `Cache` instance with the client's `client_config` and the given namespace name, stores it in the `namespaces` map, and then returns it.
    -   **Return Type Difference**:
        -   **Non-WASM**: Returns `Arc<Cache>`. This allows the caller to share ownership of the cache.
        -   **WASM**: Returns `Cache`. In the WASM context, direct shared ownership with Rust is less common, and JavaScript manages the `Cache` object's lifecycle. The `Cache` itself uses `Arc` internally for some fields.

-   **`start(&mut self) -> impl Future<Output = Result<(), Error>> + Send`**:
    Starts the background polling mechanism for configuration updates.
    -   If already running, it does nothing.
    -   Sets `self.running` to `true`.
    -   **Spawning the Task**:
        -   **Non-WASM (native)**: Uses `async_std::task::spawn` to create a new OS thread for the background task. The `handle` field stores the `JoinHandle` for this task.
        -   **WASM**: Uses `async_std::task::spawn_local` because `spawn` (which can send futures to other threads) is not always available or suitable in single-threaded WASM environments typical of browsers.
    -   **Background Task Logic**:
        1.  Enters a loop that continues as long as `*self.running.read().unwrap()` is `true`.
        2.  Sleeps for a fixed interval (currently 30 seconds).
        3.  Iterates over all `Cache` instances stored in `self.namespaces`.
        4.  For each `Cache`, it calls `cache.refresh().await`. Any errors during refresh are logged but do not stop the loop.
    -   The method itself returns quickly after spawning the task.

-   **`stop(&mut self)`**:
    Signals the background task to terminate.
    -   Sets `*self.running.write().unwrap()` to `false`.
    -   *(Non-WASM only)*: If `self.handle` is `Some`, it calls `handle.cancel().await` to stop the background task. It then sets `self.handle` to `None`.

## Thread Safety and Concurrency

-   The use of `Arc` and `RwLock` is crucial for managing shared access to `namespaces` and the `running` flag, especially between the main application thread (calling `Client` methods) and the background refresh task (in non-WASM environments).
-   `Cache` instances are also designed to be internally thread-safe for their operations.
