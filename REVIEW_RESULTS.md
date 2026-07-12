# Code Review Results — apollo-rust-client v0.7.0

**Review date:** 2026-07-12
**Scope:** Architecture, correctness, documentation, testing (per `REVIEW.md`)
**Verification performed:** `cargo clippy --all-targets` (2 warnings), `cargo test --doc` (**8 doctest compile failures**), targeted unit tests (pass). The wiremock integration suite (`scripts/test.sh`) could not be executed because the Docker daemon was unavailable in the review environment; mock mappings were reviewed statically.

---

## Executive Summary

The overall architecture is sound for a polling-based configuration client: a clean three-layer design (`Client` → per-namespace `Cache` → typed `Namespace`), a shared `reqwest::Client` for connection pooling, a correct HMAC-SHA1 signature implementation (verified against the official Apollo test vector), and a sensible double-checked-locking pattern to prevent thundering-herd fetches.

However, there are several findings that matter for a library running in production:

1. **HTTP status codes are never checked** — error responses can be parsed and *cached as configuration*.
2. **The native file cache key omits the cluster** — clients in different clusters can serve each other's cached config.
3. **The WASM story diverges sharply from the documentation** — `start()` does nothing on WASM, `start`/`stop` aren't even exported to JavaScript, and the localStorage cache never expires, so WASM clients can be permanently frozen on first-fetched config.
4. **8 doctests fail to compile**, and the CI test script never runs doctests, so this regression went unnoticed.

---

## Critical

### C1. HTTP response status is never validated — error bodies can be cached as config

**Files:** `src/cache.rs` L589–597 (`execute_request`), L610–625 (`parse_response`), L476–515 (`fetch_remote_config`)

Neither `execute_request` nor `parse_response` calls `response.error_for_status()` or inspects `response.status()`. Consequences:

- A 404 (nonexistent namespace) or 401 (bad/expired secret) produces a confusing `Error::Serde` ("expected value at line 1…") instead of a meaningful error, or worse:
- Any error response with a JSON body — e.g. Spring's `{"timestamp":..., "status":404, "error":"Not Found"}` or a JSON-speaking gateway/load-balancer error — parses successfully, is **written to the file cache / localStorage** (`fetch_remote_config` L500–512), stored in memory, and served to the application as if it were legitimate configuration. Grayscale keys silently disappear; typed lookups return `None`.

**Suggestion:** Check the status before parsing; introduce dedicated error variants (e.g. `Error::HttpStatus(StatusCode)`, `Error::Unauthorized`, `Error::NamespaceNotFound`). Only cache bodies from 2xx responses.

### C2. Native file cache is not isolated by cluster

**File:** `src/cache.rs` L242–258 (`Cache::new`)

The cache file name is built from `namespace` + optional `ip` + optional `label`, and the directory from `cache_dir`/`app_id` (`client_config.rs` L387–395). The **cluster is not part of the path**. Two processes (or two `Client`s) with the same `app_id` and `cache_dir` but different `cluster`s will read/write the same `*.cache.json` file and can serve each other's configuration — a silent cross-environment config bleed. Note the WASM key *does* include the cluster (`apollo_cache_{app_id}_{cluster}_{namespace}`, L260–266), so this is an inconsistency as well as a correctness bug.

**Suggestion:** Include the cluster (and ideally `config_server`) in the file path, mirroring the WASM key. Provide a cache-version bump / migration note since existing deployments will re-fetch once.

### C3. WASM: `start()` does nothing, and `start`/`stop` are not exported to JavaScript at all

**Files:** `src/lib.rs` L402–453 (`start`), L410–413 (wasm branch), L563–687 (`#[wasm_bindgen] impl` block); `README.md` L177–178; `docs/wiki/en/JavaScript-Usage.md` L30, L104, L174

Two distinct problems:

1. On `wasm32`, `start()` merely sets `self.handle = None` after flipping the `running` flag — **no task is spawned, no polling ever happens**. The doc comment (L382–384), `docs/wiki/en/Design-Client.md`, `Design-WASM.md`, `Features.md`, `Home.md`, and `specs/architectural_design.md` all claim `wasm_bindgen_futures::spawn_local` is used. `wasm-bindgen-futures` is even declared as a dependency (Cargo.toml L42) but `spawn_local` appears nowhere in `src/` (verified by grep).
2. `start`, `stop`, and `preload` live in the *non-annotated* `impl Client` block, so they are not exported to the JS bindings at all. The README (L178: `await client.start();`) and `JavaScript-Usage.md` instruct users to call a method that **throws `TypeError: client.start is not a function`** at runtime.

**Suggestion:** Either implement WASM polling via `spawn_local` + a cancellation flag (and export `start`/`stop` with `#[wasm_bindgen]`), or explicitly document that WASM is fetch-on-demand only and remove `start()` from all JS examples.

### C4. WASM localStorage cache never expires → configuration frozen forever

**File:** `src/cache.rs` L399–410 (`load_and_cache`, wasm branch)

The wasm branch deserializes the `CacheItem` and uses `cache_item.config` unconditionally — the `timestamp` field is written (L504–507) but **never checked**. There is no `cache_ttl` on the WASM `ClientConfig` (it's `cfg`'d out, `client_config.rs` L229–230). Combined with C3 (no background refresh on WASM) and the fact that `Cache::refresh` is not reachable from JS, a browser client that has ever cached a namespace in localStorage will **never contact the server again** for that namespace. For a production configuration client this defeats the purpose of centralized config.

**Suggestion:** Honor the stored `timestamp` with a TTL (even a hard-coded conservative default), and/or treat localStorage as a fallback only (fetch remote first, fall back to localStorage on network failure).

---

## High

### H1. Documentation examples do not compile — 8 doctest failures

**Files:** `src/lib.rs` doc examples at L23, L97, L209, L343, L494; `src/client_config.rs` doc examples at L34, L128, L147; `README.md` L64–135

Verified with `cargo test --doc`:

```
error[E0063]: missing fields `http_client` and `refresh_interval` in initializer of `ClientConfig`
test result: FAILED. 6 passed; 8 failed; 24 ignored
```

When `refresh_interval` and `http_client` were added to `ClientConfig`, none of the struct-literal examples were updated. The README's Quick Start has the same problem, *plus* its `match namespace { ... }` (L93–123) omits the `Namespace::Yaml` arm of a non-exhaustive match — it cannot compile either. This directly violates the project's own rule (".agent/rules/rust.md" #3: keep documentation valid on every change).

Root cause of non-detection: `scripts/test.sh` runs `cargo test --lib` only — **doctests are never executed in CI** (`.github/workflows/rust.yml` delegates to that script).

**Suggestion:** Fix all examples (better: use `ClientConfig::from_env()` or a builder in examples to reduce churn), add `cargo test --doc` (or plain `cargo test`) to `scripts/test.sh`, and consider `#[non_exhaustive]` + a builder/`Default` impl for `ClientConfig` so field additions stop breaking every downstream initializer — this struct is a public API and every new field is currently a semver-breaking change.

### H2. `stop()` can block for a full refresh interval

**File:** `src/lib.rs` L422–447 (background loop), L464–475 (`stop`)

In the background task, `let running = running.read().await;` acquires a read guard that is held **for the entire loop iteration** — including all namespace refreshes and the `tokio::time::sleep(refresh_interval)` — because it is not dropped until the iteration ends. `stop()` first awaits `self.running.write().await`, which must wait for that read guard. With the default 30s interval (or longer), `stop()` blocks up to that long before it even reaches `handle.abort()`.

**Suggestion:** Drop the guard immediately (`if !*running.read().await { break; }`), or replace the flag with `tokio_util::sync::CancellationToken` / `tokio::select!` over sleep + shutdown signal. Also consider awaiting the aborted handle so in-flight file writes finish (see M5).

### H3. `Cache::refresh()` holds the memory write lock across the network fetch

**File:** `src/cache.rs` L440–450

`refresh()` acquires `self.memory.write().await` *before* calling `fetch_remote_config()`. Every reader (`get_value` fast path, L322) blocks for the duration of the HTTP round trip — up to the 10-second timeout — on **every background refresh cycle**, for every namespace. In a hot path where config is read per-request, this causes periodic latency spikes.

**Suggestion:** Fetch first, then lock briefly to swap the value:

```rust
let config = self.fetch_remote_config().await?;
let listeners = { let mut w = self.memory.write().await; w.replace(config.clone()); self.listeners.read().await.clone() };
```

### H4. Namespace format detection misclassifies legitimate Apollo namespaces

**File:** `src/namespace/mod.rs` L198–210 (`get_namespace_type`), L258–269 (text branch)

Any namespace containing a dot with an unrecognized "extension" is treated as `Text`. This breaks two common, legitimate Apollo cases:

- **`.properties` suffix** (e.g. `application.properties`) — a properties namespace, classified here as Text.
- **Public / association namespaces**, which are conventionally named `{department}.{namespace}` (e.g. `TEST1.apollo`) and are properties-format. The server returns a JSON map with no `"content"` key, so `get_namespace` fails hard with `Error::Text("Failed to get text content...")` — public namespaces are effectively **unusable**.

The unit test `test_get_namespace_type_unsupported_extensions` (L330–340) actually enshrines the wrong behavior (`app.properties → Text`).

**Suggestion:** Map a trailing `.properties` to `Properties`; for unknown "extensions", default to Properties (matching Apollo's own convention) or inspect the payload shape (`content` key present → text-like; otherwise properties). Only `.txt` should map to Text.

### H5. Configuration never refreshes unless `start()` is called; TTL semantics inconsistent

**Files:** `src/cache.rs` L320–360 (`get_value`), L374–398 (file-cache TTL check); `src/client_config.rs` L298–301

Once `memory` is populated, `get_value` returns it forever; `cache_ttl` is only consulted when loading the *file* cache on a cold start. A native client that never calls `start()` will never see a config change for the process lifetime. Additionally, defaults are inconsistent: `from_env()` defaults `cache_ttl` to `Some(600)`, but direct struct construction leaves it `None` = *never stale* — a surprising divergence that is only partially documented (`client_config.rs` L224–228).

**Suggestion:** Document loudly that `start()` (or manual `refresh`) is required for updates; consider honoring `cache_ttl` for the in-memory tier as well, and unify the default TTL between construction paths.

### H6. Listeners are never notified of errors, contrary to documentation

**Files:** `src/cache.rs` L440–450 (`refresh`), L452–474 (`notify_listeners`), doc at L667–682; `src/lib.rs` L612–640 (JS listener docs); `README.md` L126–131

`refresh()` propagates fetch errors with `?` before `notify_listeners` is ever called, and `load_and_cache` does the same. So a listener can only ever receive `Err(...)` from a namespace *parse* failure — never from network/auth/server failures. Yet `add_listener`'s docs ("Listeners are called ... or when an error occurs during a refresh attempt"), the JS callback signature `(data, error)`, and the README all promise error delivery. Production users relying on the error callback for alerting will never get one for the most common failure class.

**Suggestion:** Notify listeners with `Err(...)` in the failure path of `refresh()` (and possibly `load_and_cache`), or correct all documentation.

---

## Medium

### M1. "Real-Time Updates" overstates the architecture

**Files:** `README.md` L17; `src/lib.rs` L15; `src/cache.rs` L526–548

The client polls `/configfiles/json/...`, which per Apollo's design is a server-side *cached* endpoint (may lag ~up to a minute) — the official clients achieve near-real-time updates via `/notifications/v2` long polling plus the uncached `/configs` endpoint with `releaseKey`. With a 30s default poll on a cached endpoint, worst-case propagation is minutes, not real-time. There is also no retry/backoff, no meta-server discovery, and no multi-server failover.

**Suggestion:** Either implement long polling (`/notifications/v2`) as the update trigger, or soften the claim to "periodic background polling". Consider retry with jittered backoff in the refresh loop.

### M2. Secret authentication is not actually verified by the mocks

**Files:** `tests/wiremock/mappings/application_secret.json`; `src/cache.rs` L563–577

The mapping for app `101010102` matches only method + URL path. The `Authorization`/`timestamp` headers produced by `build_http_request` are never asserted, so `test_string_value_with_secret` et al. would pass even if header wiring were completely broken. The `sign()` function itself is well-tested (matches the official Apollo test vector — good), but the end-to-end request signing is untested.

**Suggestion:** Add wiremock header matchers, e.g. `"Authorization": {"matches": "Apollo 101010102:.+"}` and `"timestamp": {"matches": "\\d+"}`, and a negative mapping returning 401 when the header is absent (which would also exercise C1's status handling once fixed).

### M3. Mock/test coverage gaps

**Files:** `tests/wiremock/mappings/`, `src/lib.rs` tests

The mappings are up to date with the endpoints the client calls (`/configfiles/json/...`, ip/label query params — good), but coverage is missing for:

- Text namespaces (`.txt`) — the `Namespace::Text` path has no integration test at all.
- Error responses: 404 (unknown namespace), 401 (bad secret), 500, malformed JSON body.
- File-cache behavior: TTL expiry, corrupt cache file fallback, cache reuse across client restarts.
- `preload()` — no test on either platform.
- `test_custom_refresh_interval` (lib.rs L1533–1564) starts/stops the loop but asserts nothing about refreshes actually occurring (no wiremock request-count verification).
- `stop()` responsiveness (would have caught H2).

Also note the shared static clients + shared temp cache dir (`test_cache_dir()`, lib.rs L738–740) make some tests order/state-dependent (`rm -fr /tmp/apollo` in `scripts/test.sh` is compensating for this).

### M4. `cfg!(test)` behavior baked into production code

**Files:** `src/lib.rs` L416–420; `src/client_config.rs` L305–308

Refresh-interval clamping uses `if cfg!(test) { 1 } else { 30 }`. Test-specific behavior in shipped logic is a smell, and the clamping logic is duplicated in `from_env()` and `start()`. (Note `cfg!(test)` is false when the crate is used as a dependency, so downstream users always get 30 — but the duplication invites drift.)

**Suggestion:** Clamp in exactly one place (`start()`), and make the minimum a named constant. For tests, use a crate-private setter or accept small intervals unconditionally.

### M5. Blocking, non-atomic file I/O inside async context

**File:** `src/cache.rs` L377–380 (read), L642–665 (`write_to_file_cache`)

`std::fs` calls run on the async executor thread (blocking it), and `std::fs::write` is not atomic — a process crash or the `handle.abort()` in `stop()` can leave a truncated/corrupt cache file. The load path tolerates parse failure (falls through to remote), which limits the blast radius, but atomicity is cheap to get.

**Suggestion:** Use `tokio::fs` or `spawn_blocking`, and write to a temp file + `rename` for atomic replacement.

### M6. Dead/unreachable branch in `get_value`

**File:** `src/cache.rs` L327–359

`should_load` is only ever `true` when the loop breaks, so the `else` branch returning `Err(Error::NamespaceNotFound)` (L356–359) is unreachable, and `NamespaceNotFound` is a misleading error for a concurrency fallback. Simplify: make the loop simply proceed to load, removing the boolean entirely.

### M7. Undocumented public API and duplicated logic in `Client::add_listener`

**File:** `src/lib.rs` L297–308

`pub async fn add_listener` has **no doc comment** — a direct violation of project rule #1 (".agent/rules/rust.md": all public APIs must be documented). Its body also duplicates `Client::cache()` verbatim; it should just call `self.cache(namespace).await`.

### M8. `preload()` misreports task-join failures as I/O errors

**File:** `src/lib.rs` L550–554

A `JoinError` (panic/cancellation) is wrapped into `cache::Error::Io` — semantically wrong and hard to distinguish from real file errors downstream. Add a dedicated variant (e.g. `Error::Task(String)`).

### M9. Test dependencies compiled into published WASM artifacts

**File:** `Cargo.toml` L36–42

`wasm-bindgen-test`, `wasm-logger`, etc. are declared as regular `[target.'cfg(target_arch = "wasm32")'.dependencies]` rather than dev-dependencies, so the test framework is linked into the published npm package (binary bloat). Move test-only crates to `[target.'cfg(target_arch = "wasm32")'.dev-dependencies]`.

### M10. Stale/incorrect API documentation (beyond the compile failures)

- `src/cache.rs` L667–702: `add_listener` docs describe `Result<Value, Error>` and `Arc<dyn Fn(Result<Value, Error>)...>`; the actual type is `Result<Namespace, crate::Error>`.
- `src/lib.rs` L186–188: mentions `Namespace<T>` — no such generic exists.
- `README.md` L192: `cache.add_listener((data, error) => ...)` — the JS API is `client.add_listener(namespace, fn)` (listener lives on `Client`, takes two args).
- `README.md` L274–284 "ClientConfig Fields" omits `allow_insecure_https`, `cache_ttl`, `refresh_interval`, `http_client`.
- `docs/wiki/zh-CN/Design-Client.md` L46–48 still describes `async_std::task::spawn` — the project migrated to tokio; the zh-CN wiki is at least one architecture generation out of date.
- `src/namespace/mod.rs` L139–142: the doc comment "XML format - planned support..." floats above a commented-out variant and effectively merges into `Text`'s docs.
- Several doc examples reference a crate named `apollo_client` instead of `apollo_rust_client` (e.g. `namespace/mod.rs` L120, `properties.rs` L45, `json.rs` L17).

---

## Low

### L1. Clippy warnings (2)

- `src/cache.rs` L572: redundant `&` in `format!` argument.
- `src/lib.rs` L271: `http_client` field name ends with struct-name pattern (pedantic).

Project rules mandate clippy; keep the build warning-free.

### L2. `Client::cache()` takes the write lock even on cache hit

**File:** `src/lib.rs` L284–295. Every `namespace()` call serializes on the `namespaces` write lock. Use a read-lock fast path and upgrade only on miss.

### L3. `config_server` trailing-slash fragility

**File:** `src/cache.rs` L527–533. `format!("{}/configfiles/...", config_server)` produces `//configfiles/...` if the user supplies a trailing slash; some servers/proxies 404 on this. Trim or use `Url::join`.

### L4. `sign()` fallback `unwrap`s

**File:** `src/cache.rs` L756–758. `base_url.join(url).unwrap()` can panic on pathological relative inputs. Propagate the error instead.

### L5. Env-var mutating test can race under parallel test execution

**File:** `src/lib.rs` L1566–1593 (`test_refresh_interval_clamping`). `std::env::set_var` affects the whole process; if another `from_env` test is ever added it will flake. Consider a serial-test guard or dependency-injected env lookup.

### L6. Minor dependency housekeeping

- `chrono` pinned twice with different minimums (`0.4.45` main, `0.4.44` wasm) — unify.
- `web-sys` is a dev-dependency for all targets but only used in wasm tests.
- `hmac 0.13` / `sha1 0.11` / `digest 0.11`: verify these are stable (non-RC) lines before the next release; the 0.12/0.10 series are the widely-deployed stable ones.

### L7. `Error::EnvVar` message inaccuracy

**File:** `src/client_config.rs` L100–101. Message says "Environment variable is not set" but `VarError` also covers `NotUnicode`.

---

## Positive Observations

- **Signature implementation is correct**: `sign()` matches the official Apollo test vector (`EoKyziXvKqzHgwx+ijDJwgVTDgE=`) and handles both full URLs and path-only inputs; tests cover both (`cache.rs` L865–880).
- **Thundering-herd protection** in `get_value` (loading flag + `Notify`) is a genuinely good pattern, and `test_concurrent_get_value` exercises it.
- **Deadlock regression test** (`test_concurrent_namespace_hang_repro`, lib.rs L1476–1530) shows evidence of a real bug being fixed and locked in — good practice.
- **Feature-flag hygiene**: the `compile_error!` guards for `native-tls`/`rustls` mutual exclusivity and WASM incompatibility (lib.rs L64–71) are exactly right.
- **Grayscale cache isolation** by ip/label in cache keys on both platforms (modulo C2's missing cluster).
- The wiremock mappings correctly model the `/configfiles/json` payload shapes for properties (`flat map`) and json/yaml (`{"content": ...}`) namespaces, including priority-based grayscale matching.

---

## Prioritized Recommendations

| # | Action | Addresses |
|---|--------|-----------|
| 1 | Validate HTTP status before parsing/caching responses | C1 |
| 2 | Add cluster to the native file-cache path | C2 |
| 3 | Implement or honestly document the WASM update story (spawn_local polling or fetch-on-demand + TTL); export `start`/`stop` to JS or delete them from JS docs | C3, C4 |
| 4 | Fix all doc examples; run `cargo test` (not `--lib`) in `scripts/test.sh`; add a builder/`Default` for `ClientConfig` | H1 |
| 5 | Fix `stop()` latency and `refresh()` lock-across-network | H2, H3 |
| 6 | Correct namespace type detection for `.properties` and dotted public namespaces | H4 |
| 7 | Notify listeners on refresh errors (or fix the docs) | H6 |
| 8 | Add wiremock header matching for auth, error-response mappings, `.txt` mapping, and request-count assertions | M2, M3 |
| 9 | Consider `/notifications/v2` long polling for genuinely real-time updates | M1 |
