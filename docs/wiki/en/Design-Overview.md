[中文简体](../zh-CN/Design-Overview.md) | [中文繁體](../zh-TW/Design-Overview.md)
[Back to Home](Home.md)

# Design Overview

This document provides a high-level overview of the `apollo-rust-client` library's architecture. The library is designed to be a robust client for the Apollo Configuration Centre, with support for both native Rust applications and WebAssembly environments.

## Core Modules

The library is built around a few core modules/structs:

- **`ClientConfig`**: This struct holds all the necessary settings for the Apollo client. It includes details such as the Apollo server address, application ID, cluster name, optional secret key, and cache directory (for non-WASM targets). It serves as the single source of truth for client configuration.

- **`Client`**: This is the main entry point for interacting with the Apollo configuration service. It is responsible for:

  - Managing instances of `Cache` for different configuration namespaces.
  - Initiating and managing a background task (for non-WASM targets) that periodically refreshes configuration for all requested namespaces.
  - Providing methods to access namespace-specific caches.
  - Managing event listeners for configuration change notifications.

- **`Cache`**: This struct is responsible for storing and managing the configuration for a single namespace. Its key responsibilities include:

  - Fetching configuration data from the Apollo server.
  - Storing the configuration in an in-memory cache.
  - For non-WASM targets, it also handles a file-based cache to persist configuration and reduce network requests.
  - Providing methods to retrieve individual configuration properties (WASM targets).
  - Managing event listeners for configuration change notifications.

- **`Namespace`**: This enum represents different types of configuration data formats:
  - **`Properties`**: Key-value pairs, typically used for application configuration
  - **`Json`**: Structured JSON data with full object support and custom type deserialization
  - **`Text`**: Plain text content for simple configurations
  - **`Yaml`**: YAML format support with structured data and custom type deserialization
  - **`Xml`** (planned): XML format support

## Key Interactions

1.  **`Client` and `ClientConfig`**:

    - An instance of `ClientConfig` is required to create a `Client`. The `Client` stores this configuration and passes it down to `Cache` instances it creates.

2.  **`Client` and `Cache` Management**:

    - When a specific namespace is requested via `client.namespace("namespace_name").await`, the `Client` first checks if a `Cache` for this namespace already exists.
    - If it exists, a reference to the existing `Cache` is returned (often wrapped in an `Arc` for shared ownership).
    - If it doesn't exist, the `Client` creates a new `Cache` instance, stores it for future requests, and then returns it.

3.  **Namespace Format Detection**:

    - The library automatically detects the configuration format based on the namespace name:
      - No extension → Properties format (default)
      - `.json` → JSON format
      - `.txt` → Text format
      - `.yaml` or `.yml` → YAML format
      - `.xml` → XML format (planned)

4.  **General Flow of a Configuration Request**:

    **For Native Rust targets:**
    Consider the call: `client.namespace("application").await?`

    - `client.namespace("application").await?`:
      - The `Client` looks up or creates a `Cache` for the "application" namespace.
      - Returns a typed `Namespace` enum based on the detected format.
      - The `Cache` will check its in-memory store first.
      - If not found in memory, it might check its file cache.
      - If still not found or if the cache is considered stale, the `Cache` will trigger a `refresh()` operation to fetch the latest configuration from the Apollo server.
      - The background refresh task, if running, will also periodically call `refresh()` on all active `Cache` instances.

    **For WASM targets:**
    Consider the call: `client.namespace("application").await`

    - `client.namespace("application").await`:
      - The `Client` looks up or creates a `Cache` for the "application" namespace.
      - Returns the `Cache` instance directly for property access.
      - The `Cache` will check its in-memory store first.
      - If not found, it will trigger a `refresh()` operation to fetch from the Apollo server.
      - File-based caching is not available in WASM environments.

5.  **Event Listener System**:
    - Both `Client` (native targets) and `Cache` (all targets) support event listeners.
    - Listeners are notified when configuration changes are detected.
    - For WASM targets, JavaScript functions can be registered as listeners.
    - Native targets use Rust closures wrapped in `Arc` for thread safety.

## API Differences Between Targets

### Native Rust Targets

- Returns typed `Namespace` enum from `client.namespace()`
- File-based caching available
- Event listeners at client level
- Full async/await support with background tasks

### WASM Targets

- Returns `Cache` instance directly from `client.namespace()`
- Memory-only caching
- Event listeners at cache level with JavaScript interop
- Single-threaded execution model

## Configuration Formats

The library supports multiple configuration formats with automatic detection:

### Properties Format

```rust
// Detected for: "application", "config.properties"
match namespace {
    Namespace::Properties(props) => {
        let value = props.get_string("app.name").await?;
        let port = props.get_int("server.port").await?;
    }
    _ => {}
}
```

### JSON Format

```rust
// Detected for: "config.json", "settings.json"
match namespace {
    Namespace::Json(json) => {
        let config: MyConfig = json.to_object()?;
    }
    _ => {}
}
```

### Text Format

```rust
// Detected for: "readme.txt", "content.txt"
match namespace {
    Namespace::Text(content) => {
        println!("Content: {}", content);
    }
    _ => {}
}
```

## Error Handling

The library uses a comprehensive error handling system:

- **Client errors**: `AlreadyRunning`, `Namespace`, `Cache`
- **Cache errors**: `AlreadyLoading`, `AlreadyCheckingCache`, `NamespaceNotFound`, `Reqwest`, `UrlParse`, `Serde`, `Io`
- **Namespace errors**: `Json`, `ContentNotFound`, `DeserializeError`

All errors implement the standard `Error` trait and provide detailed error messages for debugging and monitoring.
