[中文简体](../zh-CN/Home.md) | [中文繁體](../zh-TW/Home.md)

# Apollo Rust Client

A robust Rust client for the Apollo Configuration Centre, with support for WebAssembly for browser and Node.js environments.

[![Crates.io](https://img.shields.io/crates/v/apollo-rust-client.svg)](https://crates.io/crates/apollo-rust-client)
[![Docs.rs](https://docs.rs/apollo-rust-client/badge.svg)](https://docs.rs/apollo-rust-client)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/qqiao/apollo-rust-client/blob/main/LICENSE)
[![Rust CI](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml/badge.svg)](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml)

## Quick Start

- **[Installation](Installation.md)** - Get started with Rust or WebAssembly
- **[Rust Usage](Rust-Usage.md)** - Native Rust applications and examples
- **[JavaScript Usage](JavaScript-Usage.md)** - Browser and Node.js environments
- **[Configuration](Configuration.md)** - Client configuration options and setup

## Key Features

### Configuration Management

- **Multiple Configuration Formats**: Properties, JSON, Text with automatic detection
- **Type-Safe API**: Compile-time guarantees and runtime type conversion
- **Automatic Format Detection**: Based on namespace file extensions
- **Custom Type Deserialization**: Support for deserializing JSON into custom structs

### Cross-Platform Support

- **Native Rust**: Full feature set with file caching and background tasks
- **WebAssembly**: Optimized for browser environments with memory-only caching
- **JavaScript Interop**: Seamless integration with JavaScript and TypeScript

### Real-Time Updates

- **Background Polling**: Automatic configuration refresh with configurable intervals
- **Event Listeners**: Subscribe to configuration change notifications
- **Manual Refresh**: On-demand configuration updates
- **Graceful Error Handling**: Continues operation even with network issues

### Caching & Performance

- **Multi-Level Caching**: In-memory and file-based caching (native targets)
- **Concurrent Access**: Thread-safe operations with async locking
- **Cache Invalidation**: Automatic cache updates on configuration changes
- **Optimized for WASM**: Memory-only caching for browser environments

### Security & Authentication

- **Secret-Based Authentication**: HMAC-SHA1 signature support
- **Grayscale Releases**: IP and label-based configuration targeting
- **Secure Connections**: HTTPS support for Apollo server communication
- **Environment-Based Configuration**: Secure credential management

## Configuration Formats

The library automatically detects configuration formats based on namespace names:

| Format         | Namespace Examples                     | Description                                   |
| -------------- | -------------------------------------- | --------------------------------------------- |
| **Properties** | `"application"`, `"config.properties"` | Key-value pairs (default format)              |
| **JSON**       | `"config.json"`, `"settings.json"`     | Structured JSON data with custom type support |
| **Text**       | `"readme.txt"`, `"content.txt"`        | Plain text content                            |
| **YAML**       | `"config.yaml"`, `"config.yml"`        | YAML format with structured data support      |
| **XML**        | `"config.xml"`                         | XML format (planned)                          |

## Documentation Sections

### User Guide

- **[Features](Features.md)** - Comprehensive feature overview and capabilities
- **[Installation](Installation.md)** - Setup instructions for Rust and WebAssembly
- **[Configuration](Configuration.md)** - Client configuration options and environment variables
- **[Rust Usage](Rust-Usage.md)** - Native Rust examples, patterns, and best practices
- **[JavaScript Usage](JavaScript-Usage.md)** - WebAssembly usage in JavaScript environments
- **[Error Handling](Error-Handling.md)** - Comprehensive error handling guide and best practices
- **[WASM Memory Management](WASM-Memory-Management.md)** - Memory management for WebAssembly environments
- **[Upgrade Guide](Upgrade-Guide.md)** - Upgrade between different versions

### Library Design & Architecture

- **[Design Overview](Design-Overview.md)** - High-level architecture and design concepts
- **[ClientConfig Details](Design-ClientConfig.md)** - Configuration structure and options
- **[Client Details](Design-Client.md)** - Main client implementation and lifecycle
- **[Cache Details](Design-Cache.md)** - Caching system and namespace management
- **[WASM Design Considerations](Design-WASM.md)** - WebAssembly-specific design decisions

## API Overview

### Rust API

```rust
use apollo_rust_client::{Client, client_config::ClientConfig};

// Create and configure client
let config = ClientConfig { /* ... */ };
let mut client = Client::new(config);

// Start background refresh
client.start().await?;

// Get typed namespace
let namespace = client.namespace("application").await?;

// Handle different formats
match namespace {
    Namespace::Properties(props) => {
        let value = props.get_string("key").await;
    }
    Namespace::Json(json) => {
        let config: MyConfig = json.to_object()?;
    }
    Namespace::Yaml(yaml) => {
        let config: MyConfig = yaml.to_object()?;
    }
    Namespace::Text(text) => {
        println!("Content: {}", text);
    }
}
```

### JavaScript API

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

// Create and configure client
const config = new ClientConfig(
  "app-id",
  "http://apollo-server:8080",
  "default"
);
const client = new Client(config);

// Start background refresh
await client.start();

// Get cache and retrieve values
const cache = await client.namespace("application");
const value = await cache.get_string("key");

// Always free memory in WASM
cache.free();
client.free();
config.free();
```

## Platform Differences

### Native Rust Features

- File-based caching with automatic persistence
- Background refresh tasks with full threading support
- Environment variable configuration
- Complete async/await support with all standard library features

### WebAssembly Features

- Memory-only caching optimized for browser environments
- Single-threaded execution with spawn_local for tasks
- JavaScript interop with automatic type conversion
- Explicit memory management with free() methods

## Version Information

**Current Version**: 0.6.2

### Recent Changes (v0.6.2)

- **Typed Namespaces**: Support for multiple configuration formats with automatic detection
- **Event Listeners**: Real-time configuration change notifications
- **Enhanced Error Handling**: Comprehensive error types and better error reporting
- **WASM Improvements**: Better memory management and JavaScript interop
- **Background Polling**: Configurable automatic configuration refresh

## Getting Help

- **[GitHub Issues](https://github.com/qqiao/apollo-rust-client/issues)** - Bug reports and feature requests
- **[Documentation](https://docs.rs/apollo-rust-client)** - API documentation on docs.rs
- **[Examples](https://github.com/qqiao/apollo-rust-client/tree/main/examples)** - Sample code and usage patterns

## Contributing

Contributions are welcome! Please see our [contributing guidelines](https://github.com/qqiao/apollo-rust-client/blob/main/CONTRIBUTING.md) for details on how to:

- Report bugs and request features
- Submit pull requests
- Improve documentation
- Add new configuration format support

## License

This project is licensed under the Apache License Version 2.0. See the [LICENSE](https://github.com/qqiao/apollo-rust-client/blob/main/LICENSE) file for details.
