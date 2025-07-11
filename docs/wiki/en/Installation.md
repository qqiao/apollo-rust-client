[中文简体](../zh-CN/Installation.md) | [中文繁體](../zh-TW/Installation.md)
[Back to Home](Home.md)

# Installation

## Rust

Add the following to your `Cargo.toml`:

```toml
[dependencies]
apollo-rust-client = "0.5.0"
```

Alternatively, you can use `cargo add`:

```bash
cargo add apollo-rust-client
```

## WebAssembly (npm)

```bash
npm install @qqiao/apollo-rust-client
```

The WebAssembly version provides the same functionality as the native Rust version but is compiled to run in JavaScript environments like browsers and Node.js.
