# Plan

Implement 015/FR-001–005 using a Linux x86_64 Bash entry point and checkout-local
`.dev/` tool homes. Pin Node 24 (the existing CI major), wasm-pack, and rustup-init;
keep stable Rust per `.agent/rules/rust.md`. Check native TLS/OpenSSL and compiler
prerequisites before downloads. Verify archive checksums, use Cargo's locked install
for wasm-pack, and fetch the existing Cargo.lock without upgrades. Record exact
versions and generate a sourceable environment. Add mocked CLI regression tests,
a fresh-setup CI job, and contributor/agent handoff documentation. Keep existing
build and test entry points and integration lifecycle unchanged.
