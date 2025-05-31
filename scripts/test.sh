#!/bin/sh

rm -fr /tmp/apollo && \
RUST_LOG=apollo_rust_client=trace cargo test --lib -- --nocapture && \
wasm-pack test --node --lib -- --nocapture
