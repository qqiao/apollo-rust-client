#!/bin/sh

rm -fr pkg
cargo build --verbose && wasm-pack build --target nodejs
