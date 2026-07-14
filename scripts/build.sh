#!/bin/sh

set -eu

# build wasm
rm -rf pkg
wasm-pack build --target nodejs
node scripts/wasm_api_smoke.js

# build main
cargo build --all --verbose
