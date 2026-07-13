# Code Review Results — apollo-rust-client v0.7.0

**Review date:** 2026-07-14

**Scope:** Architecture, correctness, documentation, and testing, as requested by `REVIEW.md`

**Reviewed revision:** `8c73915` (`code-review`)

## Executive summary

The core decomposition is understandable and generally appropriate: `Client` owns a shared HTTP client and a map of per-namespace `Cache` objects, while `Namespace` provides format-specific access. Connection pooling, typed property access, HMAC signing, feature-conflict guards, grayscale request parameters, and the intention to provide memory plus persistent caching are all sound choices.

The current revision is nevertheless not ready for a production release. The advertised WASM target does not compile, HTTP status codes are ignored, persistent cache identities can cross clusters/environments, and the WASM cache can permanently freeze configuration. Native shutdown and refresh locking can also cause long stalls or leaked background tasks. Several of these behaviors contradict the repository's public documentation and Apollo's documented client design.

The highest-priority conclusions are:

1. **WASM is build-broken** by enabling Tokio's native-only `fs` feature for all targets.
2. **Non-success HTTP responses can be cached and served as configuration.**
3. **Persistent cache keys are not fully isolated**, allowing silent configuration bleed.
4. **The WASM lifecycle is non-functional**: no polling task is started, lifecycle methods are absent from JavaScript, and localStorage entries never expire.
5. **Native cache persistence reduces availability instead of improving it** when the cache directory is unwritable or a stale cache is the only available configuration.

## Verification performed

| Check | Result |
|---|---|
| `rustc --version` | `1.97.0` (stable, 2026-07-07) |
| `cargo test --doc` | Pass: 14 passed, 24 ignored |
| `cargo clippy --all-targets -- -D warnings` | Fail: one `useless_borrows_in_formatting` error at `src/cache.rs:567` |
| `cargo clippy --no-default-features --features rustls --all-targets -- -D warnings` | Same clippy failure; rustls otherwise compiled to the crate |
| `cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings` | Fail: Tokio rejects the enabled `fs` feature on WASM |
| `scripts/test.sh` | Could not run: the Docker/OrbStack daemon socket was unavailable, so WireMock did not start |
| WireMock mappings | Reviewed statically against the request paths and response shapes used by this client |

The passing doctest count should not be interpreted as complete documentation verification: 24 examples are marked `ignore`, including examples that refer to a nonexistent `apollo_client` crate or private APIs.

## Critical findings

### C1. The WASM target does not compile

**Files:** `Cargo.toml:24`, `src/cache.rs:373`, `src/cache.rs:642-655`

Tokio is declared globally with `features = ["sync", "rt", "time", "fs"]`. Tokio explicitly rejects `fs` when compiled for `wasm32`, producing:

```text
error: Only features sync,macros,io-util,rt,time are supported on wasm.
```

This was reproduced with the current stable toolchain and current lockfile (`tokio 1.52.3`). It breaks the project's defining cross-compilation promise and prevents the WASM tests from compiling, independent of any runtime behavior.

**Recommendation:** Move `fs` behind a non-WASM target-specific Tokio dependency, leaving only WASM-supported features in the common dependency. Add a direct WASM compile/clippy command to CI so this regression is reported before merge.

### C2. HTTP status codes are never checked, so error responses may be cached as configuration

**Files:** `src/cache.rs:471-509`, `src/cache.rs:584-619`

`execute_request` accepts every response returned by `send()`, and `parse_response` consumes the body without calling `error_for_status()` or inspecting `status()`. A 401, 404, 429, or 500 with a JSON body therefore parses successfully, is written to disk/localStorage, placed in memory, and returned as configuration. A non-JSON error body becomes a misleading serialization error rather than an HTTP error.

This is particularly dangerous for properties namespaces: a typical JSON gateway error object is structurally acceptable as a properties map, so applications may silently receive missing or attacker-controlled keys instead of an error.

**Recommendation:** Validate status before parsing or persisting. Add explicit status-bearing error variants and map expected Apollo outcomes (401, 404, 304, 429, 5xx). Never cache a non-success response.

### C3. Persistent cache identities can cross clusters and configuration servers

**Files:** `src/cache.rs:247-273`, `src/client_config.rs:399-406`

The native cache path includes `app_id`, namespace, IP, and label, but omits `cluster` and `config_server`. Two clients using a shared cache directory for the same application but different clusters or Apollo environments can read and overwrite the same file. This can silently serve production configuration in staging or vice versa.

The WASM key includes `app_id` and `cluster` but still omits `config_server`. A page that connects to multiple Apollo environments on the same origin can therefore reuse the wrong localStorage entry.

The filename composition is also ambiguous: underscores are used as separators without escaping, so namespace/IP/label combinations can collide.

**Recommendation:** Define and version a canonical cache identity containing server origin, app ID, cluster, namespace, IP, and label. Serialize the tuple and hash or safely encode it for both native and WASM storage. Add migration/re-fetch behavior for existing cache entries.

### C4. WASM has no working update lifecycle and can remain permanently frozen on old configuration

**Files:** `src/lib.rs:403-475`, `src/lib.rs:590-714`, `src/cache.rs:394-405`; `README.md:157-198`; `docs/wiki/en/JavaScript-Usage.md:1-190`

There are three compounding defects:

- The WASM branch of `Client::start` only assigns `self.handle = None`; it never calls `spawn_local` and performs no polling.
- `start`, `stop`, and `preload` are in the non-`#[wasm_bindgen]` impl and are absent from the JavaScript API, despite JavaScript documentation calling `client.start()`.
- The localStorage read path accepts a cached `CacheItem` without checking its timestamp. WASM has no `cache_ttl` field, and `refresh` is not exposed to JavaScript.

Once C1 is fixed, a browser that has cached a namespace may still never contact Apollo for it again. Registered JS listeners have no source of future refreshes.

**Recommendation:** Either implement and export a real WASM lifecycle (polling or Apollo long polling plus cancellation) and enforce localStorage freshness, or explicitly define WASM as fetch-on-demand, fetch remote before falling back to storage, and remove the nonexistent lifecycle from all docs. The former better matches the advertised product.

## High findings

### H1. Native persistent caching turns successful fetches into failures and does not provide stale fallback

**Files:** `src/cache.rs:359-415`, `src/cache.rs:471-509`, `src/cache.rs:637-663`; `src/client_config.rs:399-406`

Every successful remote fetch must also successfully create and write the cache file before the configuration is returned. With `cache_dir: None`, the default is `/opt/data/{app_id}/config-cache`, which is commonly unwritable for an unprivileged process. In that case, a valid Apollo response is discarded and `Client::namespace` returns `Error::Io`.

When a disk cache is older than `cache_ttl`, it is discarded before the network request. If Apollo is unreachable, the stale value is not used as a fallback. This contradicts the stated purpose of persistent caching and Apollo's documented client model, in which local files restore configuration during service/network failure.

**Recommendation:** Treat persistence as best-effort after updating memory, with observable logging/metrics for failures. Retain a parsed stale item and return it when remote retrieval fails. Consider a configurable strict-persistence mode only for callers that explicitly want write failures to be fatal.

### H2. Dropping a started native client leaks its background task; `stop()` may block for the entire refresh interval

**Files:** `src/lib.rs:258-290`, `src/lib.rs:425-498`

There is no `Drop` implementation. Dropping a Tokio `JoinHandle` detaches rather than cancels its task, and the task owns cloned `Arc`s, so a dropped `Client` can continue polling and writing indefinitely.

Within the worker, `let running = running.read().await` keeps the read guard alive across every refresh and the subsequent sleep. `stop()` needs the write lock before it can call `abort()`, so it can wait for all sequential requests plus the full interval (at least 30 seconds in production). The docs explicitly claim the loop ends when the client is dropped, which is false.

**Recommendation:** Use a cancellation token/watch channel and `tokio::select!` for prompt shutdown. Drop the read guard immediately after reading the flag, await task termination where appropriate, and abort the handle in `Drop` as a final safety net.

### H3. Refresh holds the memory write lock across network and disk I/O

**File:** `src/cache.rs:435-444`

`refresh` acquires `memory.write()` before a request that can take ten seconds and before the file write in `fetch_remote_config`. All readers block during every scheduled refresh even though a valid previous value exists. This creates periodic application latency spikes, and a slow namespace worsens shutdown delay.

**Recommendation:** Fetch and persist without holding the memory lock, then take the lock only to atomically swap the new value. Preserve and serve the old value when refresh fails.

### H4. Legitimate properties and public namespaces are misclassified as text

**File:** `src/namespace/mod.rs:198-210`, `src/namespace/mod.rs:253-274`, `src/namespace/mod.rs:329-340`

Only names without a dot are treated as properties. Every unknown suffix—including `.properties`—becomes `Text`. Apollo public/associated properties namespaces conventionally contain dots (for example `FX.apollo`), so the server returns a properties map but this client looks for a string `content` field and fails.

The test suite explicitly enshrines `app.properties -> Text`, while the README and wiki advertise `config.properties` as properties. Apollo's official Open API documentation identifies dotted public namespaces such as `FX.apollo` as `properties`.

**Recommendation:** Recognize `.properties`; default unknown/no suffixes to properties unless the response has the non-properties `{ "content": ... }` shape. Restrict text handling to known text-like formats, and add public namespace integration cases.

### H5. Initial-load coordination is not cancellation-safe and has a lost-notification window

**File:** `src/cache.rs:318-352`

The task that sets `loading = true` resets it only after `load_and_cache().await`. If that future is cancelled or panics, the flag remains true forever and future callers wait indefinitely.

There is also a race between dropping the `loading` lock and constructing `loading_complete.notified()`. `notify_waiters()` does not store a permit for a waiter created after the notification, so a fast file-cache load can notify in this gap and leave the waiter asleep. The existing concurrency test exercises normal success but not forced interleavings, cancellation, panic, or loader failure.

**Recommendation:** Replace the ad-hoc flag/`Notify` pair with a cancellation-safe single-flight primitive or an explicit state machine under one mutex. If retaining `Notify`, create/pin the notification future before checking state and use a guard that resets state on drop.

### H6. Listener behavior violates the public contract

**Files:** `src/cache.rs:435-469`, `src/cache.rs:666-705`, `src/lib.rs:639-704`

- Network, authentication, HTTP, and persistence errors return before `notify_listeners`, so listeners never receive the promised refresh errors.
- Every successful poll notifies listeners even if the value is byte-for-byte unchanged, while documentation describes configuration-change notifications.
- Native listener callbacks are detached tasks; ordering is not guaranteed and panics are unobserved.
- The WASM listener test adds a JS callback but only asserts that a separate Rust callback ran.

**Recommendation:** Compare old and new values and notify success only on a change; notify failures through a clearly documented error policy. Define ordering/reentrancy guarantees, and test the actual JavaScript callback arguments.

### H7. Cache paths are built from unvalidated identifiers and can escape the cache directory

**Files:** `src/cache.rs:247-258`, `src/client_config.rs:399-406`, `src/cache.rs:521-542`

`app_id` and `namespace` are joined directly into filesystem paths. Absolute paths, separators, or `..` components can escape the configured cache root and cause writes elsewhere. The same identifiers are interpolated directly into the URL path instead of being encoded as path segments, so `?`, `#`, `/`, and traversal-like values can change request semantics.

This normally requires a caller to pass untrusted identifiers, but libraries should not turn a namespace lookup into an arbitrary-path write primitive—especially when C2 allows a JSON error body to reach the write path.

**Recommendation:** Validate Apollo identifier rules and use a hash/encoding for filenames. Build request URLs through path-segment APIs rather than string formatting.

### H8. Main README examples still do not compile and the JavaScript guide describes a different API

**Files:** `README.md:64-134`, `README.md:157-204`, `README.md:274-284`, `README.md:327-348`; corresponding sections in `README_zh.md` and `docs/wiki/**`

The Rust quick start omits `http_client` and `refresh_interval` from `ClientConfig` and omits the `Namespace::Yaml` match arm. `cache_ttl: None` is described as using a 600-second default, but direct construction with `None` means no TTL; only `from_env` supplies 600.

The JavaScript examples call nonexistent `client.start()`, then call `add_listener` on the returned namespace even though the exported method is `client.add_listener(namespace, callback)`. Several configuration field lists omit `allow_insecure_https`, `cache_ttl`, `refresh_interval`, and `http_client`.

**Recommendation:** Compile README Rust snippets in CI (for example with `doc-comment`, `skeptic`, or extracted examples) and run a JS/TypeScript smoke test against generated bindings. Treat English and translated docs as versioned API artifacts.

## Medium findings

### M1. The architecture is periodic polling, not Apollo-style real-time updates

**Files:** `src/lib.rs:403-475`, `src/cache.rs:471-543`; `README.md:8-18`; `specs/architectural_design.md`

The client repeatedly fetches `/configfiles/json` and has no `/notifications/v2` long poll, `releaseKey`/304 workflow, service discovery, server failover, retry policy, jitter, or backoff. Refreshes are sequential, so one slow namespace delays all later namespaces. Apollo's official design uses long polling to trigger immediate retrieval, plus periodic version-aware pulling as fallback.

This implementation can be a valid lightweight polling client, but calling it “real-time” is inaccurate and polling a full cached response every 30 seconds is inefficient.

**Recommendation:** Either rename the feature to periodic polling or implement the notification protocol. Refresh namespaces concurrently with bounded concurrency and use jittered retry/backoff.

### M2. Memory cache never expires and refresh depends entirely on calling `start()`

**Files:** `src/cache.rs:318-322`, `src/cache.rs:367-405`; `src/client_config.rs:239-250`

`cache_ttl` applies only to a file read during cold start. Once memory is populated, `namespace()` returns it forever. A native caller that does not call `start()` never sees a change, regardless of TTL. Direct configuration defaults (`None`) also differ from `from_env` defaults (`Some(600)`).

**Recommendation:** Define TTL semantics for every tier, expose an explicit public refresh API if background work is optional, and provide one consistent construction/default path.

### M3. Environment parsing silently ignores invalid values despite documentation promising errors

**File:** `src/client_config.rs:266-335`

Invalid `APOLLO_CACHE_TTL`, `APOLLO_REFRESH_INTERVAL`, and `APOLLO_ALLOW_INSECURE_HTTPS` values are discarded through `.parse().ok()` and replaced by defaults/`None`. The docs say numeric parse failures return an error, but the error enum cannot represent parse errors. `APOLLO_REFRESH_INTERVAL` is also absent from several environment-variable lists.

Production refresh intervals are silently clamped to at least 30 seconds in both `from_env` and `start`, while tests use a different one-second minimum via `cfg!(test)`. This is duplicated policy and makes tests exercise different behavior from release builds.

**Recommendation:** Validate and report invalid inputs, document or remove clamping, centralize defaults/validation, and avoid test-only branches in shipped logic.

### M4. Atomic file writes still race across clients/processes

**File:** `src/cache.rs:637-655`

Writing then renaming is a good improvement, but every writer uses the same deterministic `*.json.tmp` path. Two clients or processes sharing a cache identity can overwrite/rename each other's temporary file; one may fail, and—because persistence errors are fatal—the corresponding refresh fails.

**Recommendation:** Use a unique temporary file in the destination directory, flush as required, then atomically rename. Combine this with file locking or tolerate concurrent last-writer-wins updates.

### M5. `Client::cache` serializes every namespace read on a write lock

**Files:** `src/lib.rs:293-327`

Both `cache()` and `add_listener()` take the namespace map's write lock even on hits, and `add_listener` duplicates cache-creation logic. This unnecessarily serializes a common read path.

**Recommendation:** Use a read-lock fast path and acquire the write lock only on a miss; have `add_listener` call `cache()`.

### M6. Public API evolution is brittle

**Files:** `src/client_config.rs:176-258`, `src/lib.rs:590-637`

`ClientConfig` is a public struct constructed through literals on native Rust. Every new field breaks downstream source and every example, as the recent `http_client` and `refresh_interval` additions demonstrate. The native constructor also silently falls back to a default client if custom client construction for insecure HTTPS fails, hiding the actual build error.

**Recommendation:** Introduce a validated builder/constructor with stable defaults, make fields private over a deprecation cycle, and return construction errors instead of silently changing behavior.

### M7. `preload` misclassifies task failures and is not tested

**File:** `src/lib.rs:555-586`

A panic/cancellation `JoinError` is wrapped as `cache::Error::Io`, making task failures indistinguishable from filesystem failures. No native or WASM test exercises `preload`, partial failure, duplicates, or cancellation.

**Recommendation:** Add a task/join error variant or avoid spawning when unnecessary; add behavior tests.

### M8. The integration mocks match the current implementation but are not adequate protocol tests

**Files:** `tests/wiremock/mappings/*.json`, `scripts/test.sh:34-36`

The mappings correctly model the current `/configfiles/json/{app}/{cluster}/{namespace}` shapes for properties, JSON, YAML, IP, and label requests. They are not sufficient to validate a production Apollo client:

- The secret mapping does not require `Authorization` or `timestamp`, so end-to-end auth can break without a test failure.
- There are no 400/401/404/429/500, malformed-body, timeout, or retry mappings.
- There is no text namespace, `.properties`, dotted public namespace, stale cache, corrupt cache, unwritable directory, cluster/server isolation, or cache migration coverage.
- There is no `/notifications/v2`, 304, or release-key scenario because the implementation omits that protocol.
- No request-count assertion verifies single-flight loading or refresh cadence.

**Recommendation:** Add header matchers and negative auth cases, status/error mappings, namespace-format cases, persistence/fallback cases, and WireMock request verification.

### M9. Test execution and CI leave important gaps

**Files:** `scripts/test.sh`, `.github/workflows/rust.yml`, `src/lib.rs:1508-1624`, `src/cache.rs:827-866`

`scripts/test.sh` runs `cargo test --lib`, so it omits doctests and any future integration-test binaries. It does not run clippy. `test_custom_refresh_interval` waits but never asserts request counts, the concurrency tests do not cover cancellation/lost wakeups, and environment-mutating tests can race with future parallel tests. Static clients and reused temp-directory names make isolation fragile.

The build job should catch C1, but the test job alone would not. A release-quality matrix should independently compile/check native TLS, rustls, WASM, docs, and examples.

**Recommendation:** Add explicit CI commands for native clippy, rustls clippy, WASM compile/clippy, doctests, integration tests, and generated JS smoke tests. Use isolated temporary directories and serialized environment tests.

### M10. Documentation is extensive but substantially stale and internally inconsistent

**Files:** `src/lib.rs:186-201`, `src/cache.rs:666-702`, `src/namespace/*.rs`; `docs/wiki/**`; `specs/**`

Representative issues:

- Public `Client::add_listener` at `src/lib.rs:316` has no Rust doc comment, violating `.agent/rules/rust.md`.
- Listener docs describe `Result<Value, cache::Error>` and a generic `Namespace<T>`; the API uses `Result<Namespace, crate::Error>` and `Namespace` is not generic.
- Many ignored examples import `apollo_client` instead of `apollo_rust_client`, construct types through unsupported conversions, or reference private `cache` APIs.
- English WASM design docs claim `spawn_local`; Chinese design docs still describe `async_std`; the implementation uses neither on WASM and Tokio on native.
- `specs/data_design.md` documents a cluster-containing native filename that does not match the implementation.
- XML is repeatedly presented as a supported/detected format even though every XML namespace returns an error.

**Recommendation:** Make source/API behavior authoritative, reduce duplicated narrative, test every public example, and regenerate translations/specifications from a reviewed canonical version where practical.

## Low findings

### L1. Clippy is not clean

**File:** `src/cache.rs:567`

The redundant borrow in the `Authorization` formatting argument causes `cargo clippy ... -D warnings` to fail.

### L2. Base URL construction is fragile

**File:** `src/cache.rs:521-533`

A trailing slash in `config_server` creates `//configfiles/...`, which some proxies do not normalize. String interpolation also makes context-path and encoding behavior harder to reason about.

**Recommendation:** Parse the base once during configuration validation and append encoded path segments deliberately.

### L3. `sign` contains avoidable panics

**File:** `src/cache.rs:751-772`

The fallback base URL parse and `join` are unwrapped. The base literal is safe, but a malformed relative input can make `join` fail. HMAC construction is effectively infallible for HMAC keys, but its unwrap obscures that guarantee.

**Recommendation:** Propagate URL errors and use an infallible/explained construction path.

### L4. Some error variants/messages are misleading or dead

**Files:** `src/cache.rs:104-147`, `src/client_config.rs:97-103`, `src/lib.rs:575-582`

`NamespaceNotFound` is no longer constructed; request timeouts and preload join failures are labeled as I/O; `EnvVar` says “not set” even for non-Unicode values. This makes operational diagnosis harder.

## Positive observations

- The `Client -> Cache -> Namespace` layering is easy to navigate and keeps format-specific parsing separate from transport.
- A shared `reqwest::Client` enables connection pooling, and custom client injection is useful for proxies, tracing, and policy control.
- HMAC-SHA1 signing matches Apollo's published test vector for both path-only and full-URL inputs.
- IP and label query parameters use URL query encoding, and the mappings verify grayscale selection precedence.
- Native file I/O was moved to Tokio and now uses write-then-rename rather than direct truncating writes; the remaining issue is unique temp naming/concurrency.
- The feature guards preventing simultaneous native TLS and rustls selection are clear and useful.
- Doctest regressions in the Rust source were improved: all enabled doctests pass on the reviewed revision.

## Recommended remediation order

1. Fix WASM dependency partitioning and make the full target matrix build.
2. Validate HTTP statuses before parsing/caching and add status-aware errors/tests.
3. Version and fully isolate persistent cache identities; sanitize/hash filesystem keys.
4. Make persistence best-effort and implement stale-cache fallback.
5. Decide and implement the WASM update model; export only APIs that actually work and enforce cache freshness.
6. Fix native lifecycle cancellation, `Drop`, and lock-across-I/O behavior.
7. Correct `.properties` and dotted public namespace detection.
8. Make single-flight loading cancellation-safe and listeners change/error-correct.
9. Expand WireMock coverage and CI gates across native TLS, rustls, WASM, docs, and generated JavaScript.
10. Reconcile README/wiki/spec/API documentation and introduce a stable `ClientConfig` builder.

## External protocol references used

- [Apollo configuration-center/client design](https://github.com/apolloconfig/apollo/wiki/Apollo%E9%85%8D%E7%BD%AE%E4%B8%AD%E5%BF%83%E8%AE%BE%E8%AE%A1) — long polling, periodic version-aware fallback, local-file recovery, client-side failover/retry.
- [Apollo Open API documentation](https://github.com/apolloconfig/apollo/wiki/Apollo%E5%BC%80%E6%94%BE%E5%B9%B3%E5%8F%B0) — namespace formats and dotted public properties namespace examples.
- [Tokio `Notify` documentation](https://docs.rs/tokio/1.52.3/tokio/sync/struct.Notify.html) — notification/permit semantics relevant to H5.
