[中文简体](../zh-CN/Features.md) | [中文繁體](../zh-TW/Features.md)
[Back to Home](Home.md)

# Features

## Core Features

- **Support for multiple namespaces**: Manage configuration for different application components
- **Type-safe configuration management**: Automatic type conversion and validation
- **Async/await support**: Full asynchronous API for non-blocking operations
- **Cross-platform compatibility**: Works on native Rust and WebAssembly targets
- **Comprehensive error handling**: Detailed error diagnostics and recovery

## Configuration Format Support

- **Properties format**: Traditional key-value pairs (default)
- **JSON format**: Structured JSON configuration with custom type deserialization
- **Text format**: Plain text content for simple configurations
- **Automatic format detection**: Based on namespace file extensions
- **Planned formats**: YAML and XML support coming soon

## Real-time Updates

- **Background polling**: Automatic configuration refresh with configurable intervals
- **Event listeners**: Subscribe to configuration change notifications
- **Manual refresh**: On-demand configuration updates
- **Graceful error handling**: Continues operation even with network issues

## Caching & Performance

- **Multi-level caching**: In-memory and file-based caching (native targets)
- **Optimized for WASM**: Memory-only caching for browser environments
- **Concurrent access**: Thread-safe operations with async locking
- **Cache invalidation**: Automatic cache updates on configuration changes

## Security & Authentication

- **Secret-based authentication**: HMAC-SHA1 signature support
- **Grayscale releases**: IP and label-based configuration targeting
- **Secure connections**: HTTPS support for Apollo server communication
- **Environment-based configuration**: Secure credential management

## Developer Experience

- **Flexible client configuration**: Direct instantiation or environment variables
- **Comprehensive documentation**: Detailed guides and examples
- **Type-safe APIs**: Compile-time guarantees for configuration access
- **Memory management**: Automatic cleanup with explicit control for WASM
