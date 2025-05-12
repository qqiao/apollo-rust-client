#!/bin/bash

rm -fr /tmp/apollo && \
RUST_LOG=apollo_rust_client=trace cargo test --lib -- --nocapture && \
RUST_LOG=apollo_rust_client=trace cargo test --examples -- --nocapture
