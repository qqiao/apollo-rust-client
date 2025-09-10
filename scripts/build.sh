#!/bin/sh

# build wasm
wasm-pack build --target nodejs

# build main
cargo build --all
