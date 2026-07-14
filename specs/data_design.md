# SDD Data and Concurrency Design Viewpoint

This viewpoint details the data structures, disk serialization formats, multi-level caching strategies, and concurrency control mechanisms used within the `apollo-rust-client`.

---

## 1. Data Structures & Serialization Schemas

### 1.1 In-Memory Representation
Configuration contents are represented internally using the `serde_json::Value` enum. This represents arbitrary JSON structures (`Object`, `Array`, `String`, `Number`, `Bool`, `Null`).

- When properties are accessed via Rust generic parser `Properties::get_property<T: FromStr>`, the target parses from string values dynamically.
- When object mapping is called via `Json::to_object` or `Yaml::to_object`, serialization maps JSON or YAML documents into strongly-typed Rust structures using `serde` reflection.

### 1.2 On-Disk Serialized Schema (`CacheItem`)
On native targets, configuration data is persisted locally in JSON files inside `{cache_dir}/apollo-rust-client/config-cache/` to ensure offline resilience and speed up application start times.

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
- Implemented via `Arc<RwLock<Option<CacheItem>>>` in `Cache::memory`, so TTL applies to memory as well as persistent storage.
- Non-blocking reads: Multiple threads can concurrently read configurations via read locks without blocking each other.
- Thread-safe updates: Exclusive write locks are acquired only when modifying cached values.

### Tier 2: Persistent Local Cache (Native / WebAssembly)
- **Native Targets**: Uses `v2-{sha1(identity)}.cache.json`, where the length-delimited identity includes cache version, configuration server, app, cluster, namespace, IP, and label. Raw caller identifiers never become path components.
- **WebAssembly Targets (Browser)**: Uses the same versioned identity under `apollo_cache_v2_{sha1(identity)}`. Entries use the configured TTL and stale data is retained as an availability fallback.
- **WebAssembly Targets (Node.js)**: Safely and silently falls back to Tier 1 in-memory caching if no browser `localStorage` is found at runtime, ensuring complete cross-platform execution compatibility without crashes.

### Tier 3: Remote Server Fetch
- Reaches out to Apollo Server via HTTPS.
- Triggered if Tiers 1 and 2 are empty or if local persistent caches are stale/not found.
- Non-success HTTP statuses are returned as typed errors and are never cached.
- On success, memory is updated and persistence is attempted on a best-effort basis. A persistence failure cannot discard a valid remote response.

---

## 3. Concurrency and Synchronization

Multi-threaded safety is critical in high-throughput native services. `apollo-rust-client` implements precise lock synchronization patterns.

### 3.1 Cancellation-Safe Single Flight
To prevent multiple tasks from concurrently requesting the same namespace configuration (the **Thundering Herd** problem), `Cache::get_value` uses a Tokio mutex as a cancellation-safe single-flight gate:

1. **Fast-Path Check**: Acquire the read lock on `memory` to check if a value is present. If it is, return it immediately without writing.
2. **Load Gate**: A stale/missing reader acquires `load_lock`, then repeats the freshness check. Cancellation automatically drops the mutex guard, so later callers cannot be stranded by a boolean or lost notification.

### 3.2 Deadlock Prevention in Observers
Event listener callbacks (observers) are notified upon successful configuration updates. Because these callbacks might execute user-defined logic that queries the same `Client` (causing re-entrant read requests), locks must be managed carefully:

- In `load_and_cache()` and `refresh()`, the client releases the write lock on `memory` **before** calling `notify_listeners()`.
- Listeners are invoked synchronously in registration order after locks are released. Success is emitted only when the configuration value changed; refresh errors are emitted as owned `Error::Refresh` values. Panics are caught and logged so one listener cannot suppress later listeners.
