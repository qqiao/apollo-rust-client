#!/bin/sh

set -eu

# Native tests launch their own random-port HTTPS mock; no external service or
# fixed port is required.
cargo clippy --all-targets -- -D warnings && \
cargo clippy --no-default-features --features rustls --all-targets -- -D warnings && \
cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings && \
RUST_LOG=apollo_rust_client=trace cargo test --all-targets -- --nocapture && \
cargo test --doc && \
RUST_BACKTRACE=1 wasm-pack test --node --lib -- --nocapture
