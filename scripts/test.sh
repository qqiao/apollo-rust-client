#!/bin/sh

rm -fr /tmp/apollo && \
RUST_LOG=apollo_rust_client::namespace=trace cargo test --lib -- --nocapture && \
RUST_BACKTRACE=1 wasm-pack test --node --lib -- --nocapture
