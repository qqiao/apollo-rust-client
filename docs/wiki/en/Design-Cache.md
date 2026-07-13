[中文简体](../zh-CN/Design-Cache.md)
[Back to Home](Home.md)

# Cache Details

Each internal `Cache` owns one Apollo namespace. It combines a timestamped memory value, native file or browser localStorage persistence, remote retrieval, and ordered listeners.

## State and identity

- `memory: Arc<RwLock<Option<CacheItem>>>` stores both the JSON value and fetch timestamp, so `cache_ttl` applies to memory and persistence.
- `load_lock: Arc<Mutex<()>>` is a cancellation-safe single-flight gate for cold or stale loads.
- `listeners: Arc<RwLock<Vec<EventListener>>>` stores callbacks in registration order.
- Native `file_path` is `v2-{sha1(identity)}.cache.json`; WASM uses `apollo_cache_v2_{sha1(identity)}`. The length-delimited identity includes server, app, cluster, namespace, IP, and label, so caller identifiers cannot escape a directory or collide across environments.

## Read path

`get_value()` first returns a fresh memory item. On a miss it acquires `load_lock`, repeats the check, then reads native disk or browser localStorage. A fresh persistent item populates memory. Otherwise the client fetches Apollo remotely.

Only successful HTTP responses are parsed and cached. Persistence is best-effort: an unwritable directory or unavailable localStorage is logged but cannot discard a valid remote response. If transport, HTTP, or parsing fails, the newest parsed stale memory/persistent item is returned when available.

## Refresh and concurrency

`refresh()` performs network and persistence I/O without holding the memory write lock. It takes that lock only to swap the new timestamped item, so readers continue seeing the prior value during a slow refresh. Concurrent cold loads serialize through the mutex, which automatically releases if a loader is cancelled.

Native writes use a unique create-new temporary file in the destination directory, flush it, and atomically rename it. Concurrent writers therefore cannot share or truncate a deterministic temporary path.

## Listeners

Listeners run synchronously in registration order after internal locks are released. Successful callbacks are emitted only when the configuration value changed. Refresh failures are delivered as owned `Error::Refresh` values. Callback panics are caught and logged so later listeners still run.

Register native listeners with `client.add_listener(namespace, Arc::new(callback)).await`. JavaScript uses `await client.add_listener(namespace, callback)`. JavaScript callback arguments are `(data, error)`.

## Errors

Transport, typed timeouts, non-success HTTP status/body, URL construction, signing, and JSON parsing errors are preserved. Persistent read/write failures are availability warnings rather than retrieval failures.
