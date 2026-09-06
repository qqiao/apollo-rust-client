# Current technical design and engineering guide

This document describes the current implementation. It is separate from the user-facing feature requirements and is not a reconstructed historical design decision record or a proposed implementation plan.

## Structure and execution

| Location | Current responsibility |
|---|---|
| [src/lib.rs](../../src/lib.rs) | Client registry/API, polling lifecycle, public error wrappers, WASM bindings and integration tests |
| [src/client_config.rs](../../src/client_config.rs) | Configuration/builder, validation, environment and native cache-directory selection |
| [src/cache.rs](../../src/cache.rs) | HTTP/signing, persistence, freshness, shared refresh, events and failure state |
| [src/namespace/](../../src/namespace/mod.rs) | Format routing, Properties and JSON/YAML typed adapters |
| [src/test_support.rs](../../src/test_support.rs) | Random-port self-signed native HTTPS fixture |
| [tests/readme.rs](../../tests/readme.rs) | Compile-only README API example |
| [tests/apollo/](../../tests/apollo/README.md) | Real Apollo integration environment, Compose stack, and fixture manifest |
| [scripts/](../../scripts/test.sh) | Test/build entry points and generated JS export smoke |
| [docs/wiki/](../../docs/wiki/en/Home.md) | Multilingual usage/design reference |

The manifest specifies edition 2024 with `rlib`/`cdylib` outputs. It uses Tokio, reqwest, serde/serde_json, noyalib with YAML compatibility, URL, HMAC/SHA1/Base64, and wasm-bindgen. The manifest/lockfile own versions; the feature specs do not pin implementation dependencies. Native `native-tls` and `rustls` features are mutually exclusive; `rustls` on WASM is a compile error. Default native TLS is not a statement that the dependency graph is entirely pure Rust.

## Retrieval and retention

A Client owns a namespace-string registry and one reusable HTTP client. A cache holds the raw JSON response and retrieval timestamp, listener list, load/refresh mutexes, refresh generation, last error, and retry state. Registry locks are released before cache operations. Memory/listener-list locks are not held across remote I/O or callbacks; the separate refresh and sometimes load mutexes can remain held during callbacks.

Read flow: memory → persistent storage when memory is absent → awaited shared remote retrieval on a miss. A stale value is returned while a separately spawned revalidation attempts the refresh mutex. Successful retrieval attempts persistence before memory replacement. Generation changes let overlapping refresh waiters reuse a completed result. Cold-load cancellation releases its held locks.

Persistence stores `{ "timestamp": <Unix seconds>, "config": <raw JSON> }`. The v2 identity hashes length-framed server (trailing slashes removed), app, cluster, namespace, and optional IP/label with presence markers. It omits secrets and timings. Length framing uses architecture-sized integers; the digest is not a cross-architecture interchange format or encryption.

Native storage uses platform-standard project cache directories plus `config-cache`, or `<explicit base>/apollo-rust-client/config-cache`; temporary-directory fallback is available. Files are `v2-<digest>.cache.json`. Writes use unique exclusive temporary files followed by flush/rename. Startup best-effort cleanup removes names starting `v2-` with a `tmp` extension, without age/active-writer checking. WASM uses `apollo_cache_v2_<digest>` keys in reflectively accessed localStorage, degrading to memory on missing/failed access. No old-cache migration is implemented.

## Updates and cancellation

The poller immediately snapshots eligible registered namespaces, refreshes up to four concurrently, waits for the batch, then sleeps the base interval. Native execution uses a Tokio task; WASM uses a local future with an abort handle. Stop/drop cancels that owned poller, not independently spawned stale revalidation tasks.

For base interval b and failure count n, the nominal retry delay is `min(b * 2^min(n,4), max(b,300))` with saturating multiplication and integer ±10% jitter. Only polling tests the stored retry timestamp. Manual refresh and stale-read revalidation bypass it. Success clears failure state. The healthy polling sleep itself has no jitter in the current code despite broader wording in some documentation.

## Style and constraints

The source uses documented public interfaces, snake_case functions/fields, PascalCase types, typed errors, and explicit platform cfg branches. Example from the current public implementation:

```rust
pub async fn refresh(&self, namespace: &str) -> Result<(), Error> {
    self.cache(namespace).await.refresh().await?;
    Ok(())
}
```

Follow [repository rules](../../.agent/rules/rust.md): document public APIs, keep affected documentation current, respect ignored files, use latest-stable Rust guidance, use Clippy instead of cargo check, prefer pure-Rust dependencies, and obtain permission if adding a non-pure-Rust dependency when no pure-Rust alternative exists. Pedantic Clippy is configured; CI treats warnings as errors. Never invent approval, historical rationale, test results, coverage targets, or product SLOs.

## Commands and verification method

Run from the repository root. Prerequisites for fast testing are Rust with the WASM target, wasm-pack, Node 24+, and dependency availability. Real Apollo integration testing additionally requires a Docker daemon with Docker Compose v2.20+.

| Purpose | Command | Docker Required? |
|---|---|---|
| Required test entry point (full suite) | `scripts/test.sh` | Yes (for integration phase) |
| Fast checks (unit, fault, doc, WASM unit, Clippy x3) | `scripts/test.sh fast` | **No** |
| Real Apollo integration suite | `scripts/test.sh integration` | Yes |
| Focused integration suite (native, rustls, wasm) | `scripts/test.sh integration --suite <name>` | Yes |
| Filtered integration tests | `scripts/test.sh integration --filter <pattern>` | Yes |
| Recovery cleanup tool | `scripts/apollo-test.sh cleanup` | Yes |
| Native lint | `cargo clippy --all-targets -- -D warnings` | No |
| Rustls lint | `cargo clippy --no-default-features --features rustls --all-targets -- -D warnings` | No |
| WASM lint | `cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings` | No |
| Native build | `cargo build --all --verbose` | No |
| Full native/WASM build and JS smoke | `scripts/build.sh` | No |
| API documentation | `cargo doc --no-deps` | No |

Use `scripts/test.sh` for testing instead of a substitute standalone cargo-test invocation. In its default mode, it executes fast checks (Clippy across all three targets, native unit/fault tests, native Rustls unit/fault tests, doctests, and Node WASM unit tests) followed by the real Apollo integration phase. The integration phase provisions a disposable Apollo 2.5.2 stack via Docker Compose, dynamically discovers loopback ports, idempotently seeds declarative fixtures, and executes all three runtime suites (`native`, `rustls`, and `wasm`). If Docker is not available or for fast inner-loop development, `scripts/test.sh fast` runs all lints and unit tests without Docker.

Deterministic network fault tests (hung fetches, TCP connection resets, HTTP 429/500 error classifications, malformed payloads) remain covered by in-process unit test fixtures (`MockHttpsServer` and scoped WASM test stubs), while real Apollo compatibility (multi-format serialization, authentication signature verification, grayscale IP/label routing, and release publication/polling) is validated against genuine Apollo services.

Acceptance verification should control request completion and clock progression where possible: hold a fixture response open to test stale reads, count overlapping requests, compare known fixture values, and distinguish a scheduled timeout from a production wall-clock SLO. Existing tests sometimes use short timing budgets; these are test conditions, not newly adopted operational commitments.
