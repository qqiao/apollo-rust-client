#!/bin/sh

# build wasm
rm -rf pkg
wasm-pack build --target nodejs

# build main
cargo build --all --verbose
