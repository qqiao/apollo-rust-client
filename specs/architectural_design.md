# SDD Architectural Design Viewpoint

This document details the architectural structure, context, compile-time configurations, and runtime execution patterns of the `apollo-rust-client`.

---

## 1. Context and Architecture Diagram

The `apollo-rust-client` operates as a central middleware library connecting application consumers to remote Apollo Configuration Services. It exhibits two distinct execution models depending on the targeted compilation target: **Native (Desktop/Server)** or **WebAssembly (Browser/NodeJS)**.

```mermaid
graph TD
    %% Define Nodes
    subgraph ApolloServer [Apollo Configuration Center]
        ServerPort[REST API /configfiles/json/...]
    end

    subgraph NativeEnv [Native Platform Environment - Server/Desktop]
        ClientNative[Client Struct]
        BgWorker[Background Polling Loop - Tokio]
        FileCache[Local File Cache - Platform cache directory]
        MemCacheNative[In-Memory Namespace Caches - RwLock]
        ListenerTasks[Ordered synchronous listeners]

        ClientNative -->|Manages| MemCacheNative
        ClientNative -->|Controls| BgWorker
        BgWorker -->|Triggers refresh| MemCacheNative
        MemCacheNative -->|Reads/Writes| FileCache
        MemCacheNative -->|Network Request| ServerPort
        MemCacheNative -->|Invokes after locks release| ListenerTasks
    end

    subgraph WasmEnv [WASM Environment - Web Browser / Node.js]
        ClientWasm[Client Struct]
        MemCacheWasm[In-Memory Namespace Caches]
        LocalStorage[TTL-bound localStorage cache]
        BgWorkerWasm[Cancelable spawn_local polling]
        JsInterop[wasm-bindgen JS Glue Layer]
        JsCallbacks[JavaScript Event Callbacks]

        ClientWasm -->|Manages| MemCacheWasm
        ClientWasm -->|Controls| BgWorkerWasm
        MemCacheWasm -->|Reads/Writes| LocalStorage
        ClientWasm -->|Exposes via| JsInterop
        MemCacheWasm -->|Network Fetch| ServerPort
        MemCacheWasm -->|Invokes sync| JsCallbacks
    end

    %% Styles
    classDef native fill:#e1f5fe,stroke:#0288d1,stroke-width:2px;
    classDef wasm fill:#efebe9,stroke:#5d4037,stroke-width:2px;
    classDef server fill:#f1f8e9,stroke:#558b2f,stroke-width:2px;
    class ClientNative,BgWorker,FileCache,MemCacheNative,ListenerTasks native;
    class ClientWasm,MemCacheWasm,JsInterop,JsCallbacks wasm;
    class ServerPort server;
```

---

## 2. Platform-Specific Design Differences

To support highly disparate computing models, the client implements platform-specific adaptations:

| Design Dimension | Native Target (`not(target_arch = "wasm32")`) | WebAssembly Target (`target_arch = "wasm32"`) |
| :--- | :--- | :--- |
| **Concurrency Model** | Multi-threaded asynchronous execution (futures/tokio). | Single-threaded synchronous/asynchronous execution loop (Web API/Promise). |
| **Task Spawning** | Spawns background OS tasks via `tokio::spawn`. | Uses single-threaded `wasm_bindgen_futures::spawn_local`. |
| **Caching Tier** | Multi-level cache: Memory + Persistent Local Storage (disk). | Multi-level cache: Memory + `localStorage` (when available in JS runtime); falls back to memory-only when unavailable. |
| **Listener Dispatch** | Invokes synchronously in registration order after all internal locks are released; callback panics are isolated. | Invokes synchronously via `js_sys::Function::call2`. |
| **Configuration Retrieval** | Client returns a strongly-typed `Namespace` enum to Rust code. | Client returns a Properties class, plain JSON/YAML object, or text string as `JsValue` to JS. Listener Properties payloads are plain objects. |
| **Resource Cleanup** | Managed automatically by Rust's RAII drop implementation. | Requires JavaScript context to call `.free()` manually on WASM class instances (ClientConfig, Client, Properties). |

---

## 3. Structural Design & Conditional Compilation (`cfg_if!`)

Structural design adaptations are resolved at compile time using the standard `cfg_if::cfg_if!` macro. This eliminates platform overhead and ensures compiled packages are minimal.

### 3.1 Namespace Retrieval Compilation Signature
Depending on target compilation flags, the public client API undergoes interface morphing:

```rust
// Native Rust Compilation: Returns strongly typed Enum Namespace
pub async fn namespace(&self, namespace: &str) -> Result<namespace::Namespace, Error>;

// WebAssembly Compilation: Returns parsed namespace data as JsValue
// (Properties class, plain JSON/YAML object, or text string)
#[wasm_bindgen(js_name = "namespace")]
pub async fn namespace_wasm(&self, namespace: &str) -> Result<wasm_bindgen::JsValue, Error>;
```

### 3.2 Event Listener Function Aliasing
The type definition of observers changes to reflect thread safety guarantees needed by native multi-threading:

```rust
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // WASM: Does not require Send + Sync bounds as WASM environment is single-threaded
        pub type EventListener = Arc<dyn Fn(Result<Namespace, Error>)>;
    } else {
        // Native: Requires Send + Sync bounds to enable cross-thread messaging
        pub type EventListener = Arc<dyn Fn(Result<Namespace, Error>) + Send + Sync>;
    }
}
```

---

## 4. Execution Viewpoint

### 4.1 Native Background Refresh Loop
When `Client::start()` is called on a native environment:
1. Spawns an async background task via `tokio::spawn`.
2. Loops continuously while the `running` shared boolean flag remains `true`.
3. In each iteration:
   - Locks the namespace cache directory (`self.namespaces.read()`).
   - Copies reference counts of all caches.
   - Drops the locks to avoid blocking concurrent configuration readers.
   - Refreshes up to four namespaces concurrently without holding the namespace or value locks across I/O.
   - Sleeps for the configured interval (defaults to **30 seconds**) with bounded exponential backoff and jitter after failures.
4. `stop()` and `Drop` abort the task promptly, including while it is sleeping or performing network I/O.

### 4.2 WebAssembly Integration Lifecycle
For WASM targets:
1. JavaScript invokes `let client = new Client(config);`.
2. The JS runtime allocates heap spaces inside the WASM linear memory layout.
3. `client.start()` launches the same polling policy with `spawn_local`; `client.stop()` or `free()` cancels it.
4. Fresh localStorage entries can satisfy cold loads, while expired entries trigger a remote fetch and remain available as stale fallback if that fetch fails.
3. Network requests use standard `fetch` APIs bridged through `reqwest` WASM capabilities.
4. **Persistent Cache**: In browsers, fetched configurations are saved to `localStorage` under `apollo_cache_v2_{sha1(identity)}`, with TTL validation and stale fallback.
5. **Memory Lifespans**: Because Javascript garbage collection does not clean up Rust-allocated structures directly, calling `.free()` manually on the returned `Client`, `ClientConfig`, or `Properties` objects from JS triggers memory deallocation on the WASM heap to prevent browser memory leaks. Raw JSON/YAML/Text configurations do not need manual freeing as they are returned as standard JS objects/values.
