#!/bin/bash

rm -fr /tmp/apollo # Keep this, it's fine

echo "Running native Rust tests..."
RUST_LOG=apollo_rust_client=trace cargo test --lib -- --nocapture
NATIVE_TEST_EXIT_CODE=$? # Capture exit code

echo "Running WASM tests..."
wasm-pack test --node --lib -- --nocapture
WASM_TEST_EXIT_CODE=$? # Capture exit code

# Exit with a non-zero code if any test suite failed
if [ $NATIVE_TEST_EXIT_CODE -ne 0 ] || [ $WASM_TEST_EXIT_CODE -ne 0 ]; then
  echo "One or more test suites failed."
  exit 1
else
  echo "All test suites passed."
  exit 0
fi
