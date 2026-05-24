# SDD Data and Concurrency Design Viewpoint

This viewpoint details the data structures, disk serialization formats, multi-level caching strategies, and concurrency control mechanisms used within the `apollo-rust-client`.

---

## 1. Data Structures & Serialization Schemas

### 1.1 In-Memory Representation
Configuration contents are represented internally using the `serde_json::Value` enum. This represents arbitrary JSON structures (`Object`, `Array`, `String`, `Number`, `Bool`, `Null`).

- When properties are accessed via Rust generic parser `Properties::get_property<T: FromStr>`, the target parses from string values dynamically.
- When object mapping is called via `Json::to_object` or `Yaml::to_object`, serialization maps JSON or YAML documents into strongly-typed Rust structures using `serde` reflection.

### 1.2 On-Disk Serialized Schema (`CacheItem`)
On native targets, configuration data is persisted locally in JSON files inside `{cache_dir}/{app_id}/config-cache/` to ensure offline resilience and speed up application start times.

Each file contains a serialized representation of the `CacheItem` struct:

```json
{
  "timestamp": 1782297856,
  "config": {
    "key1": "value1",
    "key2": "value2"
  }
}
```

#### Fields Description:
- **`timestamp`** (Integer): A 64-bit Unix timestamp representing when the configuration was fetched from the Apollo server. Used to calculate if the cache is stale compared to the configured `cache_ttl`.
- **`config`** (Value): A raw JSON object containing the cached configuration namespace.

---

## 2. Multi-Level Caching Tier Policy

The caching layer operates as an asynchronous read-through cache using three distinct tiers (for native configurations):

```
                       [ Read Configuration Request ]
                                     │
                        ( Tier 1: Memory Cache ) ───[ Hit ]──> Return Data
                                     │
                                  [ Miss ]
                                     │
                         ( Tier 2: Disk Cache ) ────[ Hit ]──> Write memory & Return
                                     │
                                  [ Miss ]
                                     │
                       ( Tier 3: Apollo Remote ) ───[ Success ]──> Write Disk, Memory & Return
```

### Tier 1: In-Memory Cache
- Implemented via `Arc<RwLock<Option<Value>>>` in `Cache::memory`.
- Non-blocking reads: Multiple threads can concurrently read configurations via read locks without blocking each other.
- Thread-safe updates: Exclusive write locks are acquired only when modifying cached values.

### Tier 2: Persistent Local Cache (Native / WebAssembly)
- **Native Targets**: Located at `{cache_dir}/{app_id}_{cluster}_{namespace}.cache.json`. Serves as backup if the memory cache is empty. If the file exists, the client parses it and checks the file age against `cache_ttl` to determine staleness.
- **WebAssembly Targets (Browser)**: Utilizes the browser's global `localStorage` wrapper to store configurations as JSON strings under an isolated key: `apollo_cache_{app_id}_{cluster}_{namespace}[_{ip}][_{label}]`. This avoids collisions across applications, clusters, and grayscale targeting while still eliminating cold-start fetch latencies upon page refreshes or route changes.
- **WebAssembly Targets (Node.js)**: Safely and silently falls back to Tier 1 in-memory caching if no browser `localStorage` is found at runtime, ensuring complete cross-platform execution compatibility without crashes.

### Tier 3: Remote Server Fetch
- Reaches out to Apollo Server via HTTPS.
- Triggered if Tiers 1 and 2 are empty or if local persistent caches are stale/not found.
- On success, the response updates the memory cache and writes back to the active persistent cache (on-disk file or browser `localStorage`).

---

## 3. Concurrency and Synchronization

Multi-threaded safety is critical in high-throughput native services. `apollo-rust-client` implements precise lock synchronization patterns.

### 3.1 Double-Checked Locking (DCL)
To prevent multiple threads from concurrently requesting the same namespace configuration (the **Thundering Herd** problem), `Cache::get_value` implements double-checked locking:

1. **Fast-Path Check**: Acquire the read lock on `memory` to check if a value is present. If it is, return it immediately without writing.
2. **Loop and Wait Coordination**:
   - If memory is empty, the thread attempts to become the loader by acquiring the write lock on `self.loading`.
   - If another thread is already loading (`*loading == true`), the thread releases its lock and blocks on `self.loading_complete.notified().await`.
   - Once the loading thread completes the network fetch, it resets `*loading = false` and calls `self.loading_complete.notify_waiters()`.
   - The blocked threads wake up, loop back, re-run the fast-path check, and retrieve the newly populated configuration from memory.

### 3.2 Deadlock Prevention in Observers
Event listener callbacks (observers) are notified upon successful configuration updates. Because these callbacks might execute user-defined logic that queries the same `Client` (causing re-entrant read requests), locks must be managed carefully:

- In `load_and_cache()` and `refresh()`, the client releases the write lock on `memory` **before** calling `notify_listeners()`.
- On native targets, `notify_listeners()` spawns each observer callback as an independent, concurrent asynchronous task via `tokio::spawn`. This decouples listener execution from the cache's execution flow, ensuring re-entrant requests do not deadlock the client.
