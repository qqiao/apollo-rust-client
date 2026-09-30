# Tasks and verification

- [x] Implement isolated bootstrap and activation (AC-001/002): fresh setup and repeated setup succeeded on Debian 13 x86_64
- [x] Cover validation/reuse/failure paths (AC-002/003): eight Node setup regression tests pass, including checksum rejection and inherited install-root isolation
- [x] Document host packages, reset recovery and Docker boundary (AC-004): source review against Cargo.toml, CI and integration scripts
- [x] Add fresh CI bootstrap smoke (SC-001/002): executes setup twice, regression tests and native Clippy
- [x] Run available repository checks and record verification limits (SC-003)

## Verification evidence (2026-09-30)

Fresh and repeated bootstrap: Rust 1.98.1 stable, Node 24.19.0, wasm-pack 0.15.0,
OpenSSL 3.5.7 on Debian 13 x86_64. Bash syntax, eight isolated setup regression
checks and the documentation link checker pass (490 local links). The complete
`scripts/test.sh fast` passes: all three Clippy configurations, native/Rustls/doc/WASM
tests, 96 Node tooling tests, and four canonical examples under both TLS backends.
`cargo build --locked --all` also passes. `scripts/build.sh` reaches the WASM
optimization step but cannot download the Binaryen 117 release archive from GitHub
in this workspace; wasm-opt was not disabled to conceal that limit. Docker is
unavailable locally, so real Apollo integration was not run here. The [draft PR's
CI checks](https://github.com/qqiao/apollo-rust-client/pull/140/checks) provide the
current remote build/integration and fresh-bootstrap results. No macOS/ARM/Windows
automatic setup support or validation is claimed.
