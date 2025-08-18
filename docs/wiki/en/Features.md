[中文简体](../zh-CN/Features.md) | [中文繁體](../zh-TW/Features.md)
[Back to Home](Home.md)

# Features

The Apollo Rust Client provides a comprehensive set of features for managing configuration data across different environments and platforms. This document details all available features and their capabilities.

## Configuration Management

### Multiple Configuration Formats

The library supports various configuration formats with automatic detection:

#### Properties Format (Default)

- **Detection**: No extension, `.properties` extension
- **Use Case**: Traditional key-value configuration
- **Type Support**: String, integer, float, boolean with automatic parsing
- **Example**: `{"app.name": "MyApp", "server.port": "8080"}`

```rust
match namespace {
    Namespace::Properties(props) => {
        let app_name = props.get_string("app.name").await;
        let port = props.get_int("server.port").await;
        let debug = props.get_bool("debug.enabled").await;
    }
    _ => {}
}
```

#### JSON Format

- **Detection**: `.json` extension
- **Use Case**: Structured configuration data
- **Type Support**: Full JSON object support with custom type deserialization
- **Example**: `{"database": {"host": "localhost", "port": 5432}}`

```rust
#[derive(serde::Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
}

match namespace {
    Namespace::Json(json) => {
        let config: DatabaseConfig = json.to_object()?;
    }
    _ => {}
}
```

#### Text Format

- **Detection**: `.txt` extension
- **Use Case**: Plain text content, documentation, templates
- **Type Support**: Raw string content
- **Example**: Plain text files, README content, configuration templates

```rust
match namespace {
    Namespace::Text(content) => {
        println!("Content: {}", content);
    }
    _ => {}
}
```

#### YAML Format

- **Detection**: `.yaml`, `.yml` extensions
- **Use Case**: Structured configuration data with human-readable format
- **Type Support**: Full YAML object support with custom type deserialization
- **Example**: Complex configuration structures, nested objects

```rust
#[derive(serde::Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    credentials: CredentialConfig,
}

#[derive(serde::Deserialize)]
struct CredentialConfig {
    username: String,
    password: String,
}

match namespace {
    Namespace::Yaml(yaml) => {
        let config: DatabaseConfig = yaml.to_object()?;
        println!("Database: {}:{}", config.host, config.port);
    }
    _ => {}
}
```

#### Planned Formats

- **XML**: `.xml` extension (coming soon)

### Type-Safe Configuration Access

- **Compile-Time Safety**: Rust's type system ensures configuration access is safe
- **Runtime Type Conversion**: Automatic parsing from strings to target types
- **Error Handling**: Graceful handling of type conversion failures
- **Optional Values**: Support for optional configuration keys

### Automatic Format Detection

The library automatically detects configuration formats based on namespace naming conventions:

```rust
// Properties format (default)
let props = client.namespace("application").await?;

// JSON format
let json = client.namespace("config.json").await?;

// YAML format
let yaml = client.namespace("config.yaml").await?;

// Text format
let text = client.namespace("readme.txt").await?;
```

## Cross-Platform Support

### Native Rust Features

#### Full Feature Set

- Complete async/await support with all standard library features
- Multi-threaded background refresh tasks
- File-based caching with automatic persistence
- Environment variable configuration support
- Full error handling with detailed diagnostics

#### File System Integration

- Automatic cache directory creation
- Persistent configuration storage
- Offline access to cached configurations
- Configurable cache locations

#### Threading and Concurrency

- Background refresh tasks using `async_std::task::spawn`
- Thread-safe operations with `Arc<RwLock<T>>`
- Concurrent namespace access
- Non-blocking operations

### WebAssembly Features

#### Browser Optimization

- Memory-only caching optimized for browser environments
- Single-threaded execution with `spawn_local` for tasks
- JavaScript interop with automatic type conversion
- Explicit memory management with `free()` methods

#### JavaScript Integration

- Seamless TypeScript/JavaScript bindings
- Automatic type conversion between Rust and JavaScript
- Event listeners with JavaScript callback support
- Promise-based async operations

#### Memory Management

- Explicit memory control to prevent leaks
- Automatic cleanup of Rust-allocated memory
- Clear API for resource management

## Real-Time Updates

### Background Polling

#### Automatic Refresh

- Configurable polling intervals (default: 30 seconds)
- Automatic refresh of all active namespaces
- Graceful error handling during network issues
- Continues operation even with temporary server unavailability

#### Manual Control

- Start/stop background refresh on demand
- Manual refresh of specific namespaces
- Configurable refresh strategies

```rust
// Start background refresh
client.start().await?;

// Stop background refresh
client.stop().await;

// Manual refresh of specific cache
cache.refresh().await?;
```

### Event Listeners

#### Real-Time Notifications

- Subscribe to configuration change events
- Immediate notification when configurations update
- Support for multiple listeners per namespace
- Error notifications for failed updates

#### Platform-Specific Implementation

- **Native Rust**: Thread-safe closures with `Send + Sync`
- **WebAssembly**: JavaScript function callbacks

```rust
// Rust event listener
client.add_listener("application", Arc::new(|result| {
    match result {
        Ok(namespace) => println!("Config updated: {:?}", namespace),
        Err(e) => eprintln!("Update error: {}", e),
    }
})).await;
```

```javascript
// JavaScript event listener
await cache.add_listener((data, error) => {
  if (error) {
    console.error("Update error:", error);
  } else {
    console.log("Config updated:", data);
  }
});
```

## Caching & Performance

### Multi-Level Caching

#### Cache Hierarchy

1. **Memory Cache**: Fast in-memory storage for immediate access
2. **File Cache** (native only): Persistent storage to reduce network requests
3. **Remote Fetch**: Retrieval from Apollo server when cache misses occur

#### Cache Management

- Automatic cache invalidation on configuration changes
- Configurable cache directories and file naming
- Cache isolation for different namespaces and grayscale targets

### Concurrent Access Control

#### Thread Safety

- All cache operations are thread-safe and async-friendly
- Prevents race conditions during cache initialization
- Concurrent read access with exclusive write operations
- Deadlock prevention through careful lock ordering

#### Performance Optimization

- Lazy loading of namespace caches
- Efficient memory usage with `Arc` sharing
- Minimal network requests through intelligent caching
- Optimized for high-concurrency scenarios

## Security & Authentication

### Secret-Based Authentication

#### HMAC-SHA1 Signatures

- Secure authentication using HMAC-SHA1 signatures
- Timestamp-based request signing
- Protection against replay attacks
- Configurable secret keys per namespace

```rust
let config = ClientConfig {
    app_id: "my-app".to_string(),
    secret: Some("secret-key".to_string()),
    allow_insecure_https: None,
    // ... other fields
};
```

### Grayscale Release Support

#### IP-Based Targeting

- Configuration targeting based on client IP addresses
- Support for IP ranges and specific IP matching
- Automatic IP detection and inclusion in requests

#### Label-Based Targeting

- Flexible labeling system for client identification
- Support for multiple labels per client
- Comma-separated label specification

```rust
let config = ClientConfig {
    app_id: "my-app".to_string(),
    label: Some("canary,beta".to_string()),
    ip: Some("192.168.1.100".to_string()),
    allow_insecure_https: None,
    // ... other fields
};
```

### Secure Communication

#### HTTPS Support

- Full HTTPS support for secure communication with Apollo servers
- TLS certificate validation
- Secure credential transmission

#### Environment-Based Configuration

- Secure credential management through environment variables
- Separation of configuration from code
- Support for different environments (dev, staging, production)

## Developer Experience

### Comprehensive Error Handling

#### Detailed Error Types

- Specific error types for different failure scenarios
- Comprehensive error messages with context
- Structured error handling with `Result<T, E>` types
- Error propagation and transformation

#### Error Categories

- **Client Errors**: Lifecycle and state management issues
- **Cache Errors**: Network, I/O, and caching failures
- **Namespace Errors**: Format detection and parsing issues

### Flexible Configuration

#### Multiple Configuration Methods

- Direct struct instantiation
- Environment variable loading
- Mixed configuration approaches
- Runtime configuration updates

#### Configuration Validation

- Automatic validation of required fields
- Clear error messages for missing configuration
- Type safety for configuration parameters

### Documentation & Examples

#### Comprehensive Documentation

- Detailed API documentation with examples
- Architecture documentation for advanced users
- Platform-specific guides and best practices
- Troubleshooting guides and FAQ

#### Code Examples

- Complete working examples for common use cases
- Platform-specific examples (Rust vs WebAssembly)
- Integration examples with popular frameworks
- Performance optimization examples

## Version Compatibility

### Semantic Versioning

- Follows semantic versioning principles
- Clear upgrade paths between versions
- Backward compatibility guarantees
- Deprecation warnings for breaking changes

### Feature Flags

- Conditional compilation for different platforms
- Optional features to reduce binary size
- Platform-specific optimizations
- Future-proof architecture for new features

## Monitoring & Observability

### Logging Support

- Comprehensive logging with configurable levels
- Structured logging for better observability
- Performance metrics and timing information
- Debug information for troubleshooting

### Health Checks

- Built-in health check capabilities
- Connection status monitoring
- Cache health and performance metrics
- Automatic recovery from transient failures
