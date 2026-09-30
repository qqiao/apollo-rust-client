# Tasks and verification

- [x] Implement isolated bootstrap and activation (AC-001/002): fresh setup and repeated setup succeeded on Debian 13 x86_64
- [x] Cover validation/reuse/failure paths (AC-002/003): eight Node setup regression tests pass, including checksum rejection and inherited install-root isolation
- [x] Document host packages, reset recovery and Docker boundary (AC-004): source review against Cargo.toml, CI and integration scripts
- [x] Add fresh CI bootstrap smoke (SC-001/002): executes setup twice, regression tests and native Clippy
- [ ] Complete repository build/fast checks and remote CI (SC-003)

## Verification evidence (2026-09-30)

Fresh and repeated bootstrap: Rust 1.98.1 stable, Node 24.19.0, wasm-pack 0.15.0,
OpenSSL 3.5.7 on Debian 13 x86_64. Bash syntax, eight isolated setup regression
checks and the documentation link checker pass (490 local links). Real build/fast
checks and PR CI are in progress; no full-suite success is claimed here. Docker
is unavailable in the local workspace, so real Apollo integration must be checked
in CI. No macOS/ARM/Windows automatic setup support or validation is claimed.
