# apollo-rust-client

A robust Rust client for the Apollo Configuration Centre, with support for WebAssembly for browser and Node.js environments.

[![Crates.io](https://img.shields.io/crates/v/apollo-rust-client.svg)](https://crates.io/crates/apollo-rust-client)
[![Docs.rs](https://docs.rs/apollo-rust-client/badge.svg)](https://docs.rs/apollo-rust-client)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust CI](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml/badge.svg)](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml)

## Features

- **Multiple Configuration Formats**: Support for Properties, JSON, YAML, Text formats with planned XML support
- **Automatic Format Detection**: Based on namespace file extensions
- **Type-Safe Configuration Management**: Compile-time guarantees and runtime type conversion
- **Cross-Platform Support**: Native Rust and WebAssembly targets
- **Real-Time Updates**: Background polling with configurable intervals and event listeners
- **Comprehensive Caching**: Multi-level caching with file persistence (native) and memory-only (WASM)
- **Async/Await Support**: Full asynchronous API for non-blocking operations
- **Error Handling**: Detailed error diagnostics with comprehensive error types
- **Grayscale Release Support**: IP and label-based configuration targeting
- **Flexible Configuration**: Direct instantiation or environment variable configuration
- **Memory Management**: Automatic cleanup with explicit control for WASM environments

## Installation

### Rust

Add the following to your `Cargo.toml`:

```toml
[dependencies]
apollo-rust-client = "0.6.0"
```

Alternatively, you can use `cargo add`:

```bash
cargo add apollo-rust-client
```

### WebAssembly (npm)

```bash
npm install @qqiao/apollo-rust-client
```

## Quick Start

### Rust Usage

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client configuration
    let client_config = ClientConfig {
        app_id: "your_app_id".to_string(),
        cluster: "default".to_string(),
        config_server: "http://your-apollo-server:8080".to_string(),
        secret: Some("your_apollo_secret".to_string()),
        cache_dir: None, // Uses default: /opt/data/{app_id}/config-cache
        label: None,     // For grayscale releases
        ip: None,        // For grayscale releases
        allow_insecure_https: None, // Allow self-signed certificates
        #[cfg(not(target_arch = "wasm32"))]
        cache_ttl: None, // Uses default: 600 seconds (10 minutes)
    };

    let mut client = Client::new(client_config);

    // Start background polling for configuration updates
    client.start().await?;

    // Get configuration for different namespace formats
    let namespace = client.namespace("application").await?;

    // Handle different configuration formats
    match namespace {
        apollo_rust_client::namespace::Namespace::Properties(properties) => {
            // Properties format (default) - key-value pairs
            if let Some(app_name) = properties.get_string("app.name") {
                println!("Application name: {}", app_name);
            }

            if let Some(port) = properties.get_int("server.port") {
                println!("Server port: {}", port);
            }

            if let Some(debug) = properties.get_bool("debug.enabled") {
                println!("Debug enabled: {}", debug);
            }
        }
        apollo_rust_client::namespace::Namespace::Json(json) => {
            // JSON format - structured data
            #[derive(serde::Deserialize)]
            struct Config {
                name: String,
                version: String,
            }

            let config: Config = json.to_object()?;
            println!("Config: {} v{}", config.name, config.version);
        }
        apollo_rust_client::namespace::Namespace::Text(text) => {
            // Text format - plain text content
            println!("Text content: {}", text);
        }
    }

    // Add event listener for configuration changes
    client.add_listener("application", std::sync::Arc::new(|result| {
        match result {
            Ok(namespace) => println!("Configuration updated: {:?}", namespace),
            Err(e) => eprintln!("Configuration update error: {}", e),
        }
    })).await;

    Ok(())
}
```

#### Configuration via Environment Variables

```rust
use apollo_rust_client::client_config::ClientConfig;

// Initialize from environment variables
let client_config = ClientConfig::from_env()?;
```

**Environment Variables:**

- `APP_ID`: Your application ID (required)
- `APOLLO_CONFIG_SERVICE`: The Apollo config server URL (required)
- `IDC`: The cluster name (optional, defaults to "default")
- `APOLLO_ACCESS_KEY_SECRET`: The secret key for authentication (optional)
- `APOLLO_LABEL`: Comma-separated list of labels for grayscale rules (optional)
- `APOLLO_CACHE_DIR`: Directory to store local cache (optional)
- `APOLLO_CACHE_TTL`: Time-to-live for cache in seconds (optional, defaults to 600, native targets only)
- `APOLLO_ALLOW_INSECURE_HTTPS`: Whether to allow insecure HTTPS connections (optional, defaults to false)

### JavaScript/WebAssembly Usage

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  // Create client configuration
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // Set optional properties
  clientConfig.secret = "your_apollo_secret";
  clientConfig.label = "production";
  clientConfig.ip = "192.168.1.100";

  const client = new Client(clientConfig);

  // Start background polling
  await client.start();

  // Get configuration cache
  const cache = await client.namespace("application");

  // Retrieve different data types
  const appName = await cache.get_string("app.name");
  const serverPort = await cache.get_int("server.port");
  const debugEnabled = await cache.get_bool("debug.enabled");
  const timeout = await cache.get_float("timeout.seconds");

  console.log(`App: ${appName}, Port: ${serverPort}, Debug: ${debugEnabled}`);

  // Add event listener for configuration changes
  await cache.add_listener((data, error) => {
    if (error) {
      console.error("Configuration update error:", error);
    } else {
      console.log("Configuration updated:", data);
    }
  });

  // IMPORTANT: Release memory when done
  cache.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```

## Configuration Formats

The library automatically detects configuration formats based on namespace names:

### Properties Format (Default)

- **Namespace**: `"application"`, `"config.properties"`
- **Format**: Key-value pairs
- **Example**: `{"app.name": "MyApp", "server.port": "8080"}`

### JSON Format

- **Namespace**: `"config.json"`, `"settings.json"`
- **Format**: Structured JSON data
- **Example**: `{"database": {"host": "localhost", "port": 5432}}`

### Text Format

- **Namespace**: `"readme.txt"`, `"content.txt"`
- **Format**: Plain text content
- **Example**: Raw text content

### YAML Format

- **Namespace**: `"config.yaml"`, `"config.yml"`
- **Format**: Structured YAML data
- **Example**: Complex configuration structures with nested objects

```rust
#[derive(serde::Deserialize)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
}

#[derive(serde::Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(serde::Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    ssl: bool,
}

if let apollo_rust_client::namespace::Namespace::Yaml(yaml) = namespace {
    let config: AppConfig = yaml.to_object()?;
    println!("Server: {}:{}", config.server.host, config.server.port);
    println!("Database: {}:{}", config.database.host, config.database.port);
}
```

### Planned Formats

- **XML**: `"config.xml"`

## Configuration Options

### ClientConfig Fields

- **`app_id`**: Your application ID in Apollo (required)
- **`config_server`**: The Apollo config server URL (required)
- **`cluster`**: The cluster name (required, typically "default")
- **`secret`**: Optional secret key for authentication
- **`cache_dir`**: Directory for local cache files (native only)
  - Default: `/opt/data/{app_id}/config-cache`
  - WASM: Always `None` (memory-only caching)
- **`label`**: Label for grayscale releases (optional)
- **`ip`**: IP address for grayscale releases (optional)

## Error Handling

The library provides comprehensive error handling:

```rust
use apollo_rust_client::Error;

match client.namespace("application").await {
    Ok(namespace) => {
        // Handle successful configuration retrieval
    }
    Err(Error::Cache(cache_error)) => {
        // Handle cache-related errors (network, parsing, etc.)
        eprintln!("Cache error: {}", cache_error);
    }
    Err(Error::Namespace(namespace_error)) => {
        // Handle namespace-related errors (format detection, etc.)
        eprintln!("Namespace error: {}", namespace_error);
    }
    Err(e) => {
        // Handle other errors
        eprintln!("Error: {}", e);
    }
}
```

## Memory Management (WASM)

For WebAssembly environments, explicit memory management is required:

```javascript
// Always call free() on WASM objects when done
cache.free();
client.free();
clientConfig.free();
```

This prevents memory leaks by releasing Rust-allocated memory on the WebAssembly heap.

## Advanced Usage

### Event Listeners

```rust
// Rust - Add event listener
client.add_listener("application", std::sync::Arc::new(|result| {
    match result {
        Ok(namespace) => println!("Config updated: {:?}", namespace),
        Err(e) => eprintln!("Update error: {}", e),
    }
})).await;
```

```javascript
// JavaScript - Add event listener
await cache.add_listener((data, error) => {
  if (error) {
    console.error("Update error:", error);
  } else {
    console.log("Config updated:", data);
  }
});
```

### Grayscale Releases

```rust
let client_config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: Some("canary,beta".to_string()),  // Multiple labels
    ip: Some("192.168.1.100".to_string()),  // Client IP
    #[cfg(not(target_arch = "wasm32"))]
    cache_ttl: None,
};
```

### Custom JSON Deserialization

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    ssl: bool,
}

if let apollo_rust_client::namespace::Namespace::Json(json) = namespace {
    let db_config: DatabaseConfig = json.to_object()?;
    println!("Database: {}:{}", db_config.host, db_config.port);
}
```

## API Changes in v0.6.0

- **Typed Namespaces**: Support for multiple configuration formats with automatic detection
- **Event Listeners**: Real-time configuration change notifications
- **Enhanced Error Handling**: Comprehensive error types and better error reporting
- **WASM Improvements**: Better memory management and JavaScript interop
- **Background Polling**: Configurable automatic configuration refresh

## Documentation

For comprehensive documentation, visit our [wiki](docs/wiki/en/Home.md):

- **[Installation Guide](docs/wiki/en/Installation.md)** - Setup instructions
- **[Rust Usage](docs/wiki/en/Rust-Usage.md)** - Native Rust examples
- **[JavaScript Usage](docs/wiki/en/JavaScript-Usage.md)** - WASM usage in JavaScript
- **[Configuration](docs/wiki/en/Configuration.md)** - Configuration options
- **[Features](docs/wiki/en/Features.md)** - Feature overview
- **[Error Handling](docs/wiki/en/Error-Handling.md)** - Error handling guide
- **[Design Overview](docs/wiki/en/Design-Overview.md)** - Architecture documentation

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the Apache License Version 2.0 - see the [LICENSE](LICENSE) file for details.
