# apollo-rust-client

A robust Rust client for the Apollo Configuration Centre, with support for WebAssembly for browser and Node.js environments.

[![Crates.io](https://img.shields.io/crates/v/apollo-rust-client.svg)](https://crates.io/crates/apollo-rust-client)
[![Docs.rs](https://docs.rs/apollo-rust-client/badge.svg)](https://docs.rs/apollo-rust-client)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust CI](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml/badge.svg)](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml)

## Features

- Support for multiple namespaces
- Type-safe configuration management
- Async/await support
- Error handling with detailed diagnostics
- Real-time configuration updates through background polling (manual start required via `client.start()`).
- Flexible client configuration, including direct instantiation or from environment variables (Rust).

## Installation

### Rust

Add the following to your `Cargo.toml`:

```toml
[dependencies]
apollo-rust-client = "0.5.0" # Please verify and use the latest version
```

Alternatively, you can use `cargo add`:

```bash
cargo add apollo-rust-client
```

### WebAssembly (npm)

```bash
npm install @qqiao/apollo-rust-client
```

## Usage

### Rust Usage

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig {
        app_id: "your_app_id".to_string(),
        cluster: "default".to_string(),
        secret: Some("your_apollo_secret".to_string()),
        config_server: "http://your-apollo-server:8080".to_string(),
        cache_dir: None,
        label: None,
        ip: None,
    };
    let mut client = Client::new(client_config);

    // Start background polling for configuration updates.
    client.start().await?;

    // Get configuration for the "application" namespace
    let namespace = client.namespace("application").await?;

    // The namespace is now typed based on the format detected from the namespace name
    match namespace {
        apollo_rust_client::namespace::Namespace::Properties(properties) => {
            // Example: Retrieve a string property
            match properties.get_string("some_key").await {
                Some(value) => println!("Property 'some_key': {}", value),
                None => println!("Property 'some_key' not found"),
            }

            // Example: Retrieve an integer property
            match properties.get_int("meaningOfLife").await {
                Some(value) => println!("Property 'meaningOfLife': {}", value),
                None => println!("Property 'meaningOfLife' not found"),
            }
        }
        apollo_rust_client::namespace::Namespace::Json(json) => {
            // For JSON namespaces, you can deserialize to custom types
            // let config: MyConfig = json.to_object()?;
            println!("JSON namespace received");
        }
        apollo_rust_client::namespace::Namespace::Text(text) => {
            println!("Text namespace content: {}", text);
        }
    }

    Ok(())
}
```

#### Configuration via Environment Variables (Rust)

The `ClientConfig` can also be initialized from environment variables using `ClientConfig::from_env()`. This is useful for server-side applications where configuration is often passed via the environment. The following variables are recognized:

- `APP_ID`: Your application ID.
- `APOLLO_ACCESS_KEY_SECRET`: The secret key for your namespace (if applicable).
- `IDC`: The cluster name (defaults to "default" if not set).
- `APOLLO_CONFIG_SERVICE`: The URL of the Apollo Config Service.
- `APOLLO_LABEL`: Comma-separated list of labels for grayscale rules.
- `APOLLO_CACHE_DIR`: Directory to store local cache.

Note: The `ip` field is not set via `from_env()`.

### WebAssembly (JavaScript) Usage

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  // Basic configuration: app_id, config_server URL, cluster name
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // Set optional properties directly on the instance if needed
  clientConfig.secret = "your_apollo_secret"; // Example: if your Apollo namespace requires a secret
  // clientConfig.label = "your_instance_label"; // For grayscale releases
  // clientConfig.ip = "client_ip_address"; // For grayscale releases

  const client = new Client(clientConfig);

  // Optional: Start background polling for configuration updates.
  // This is a non-blocking operation.
  await client.start();

  // Get configuration for the "application" namespace
  const cache = await client.namespace("application");

  // Example: Retrieve a string property
  const stringVal = await cache.get_string("some_key");
  if (stringVal !== undefined) {
    console.log("Property 'some_key':", stringVal);
  } else {
    console.log("Property 'some_key' not found.");
  }

  // Example: Retrieve an integer property using get_int
  const intVal = await cache.get_int("meaningOfLife");
  if (intVal !== undefined) {
    console.log("Property 'meaningOfLife':", intVal);
  } else {
    console.log("Property 'meaningOfLife' not found or not an integer.");
  }

  // Example: Add event listener for configuration changes
  await cache.add_listener((error, data) => {
    if (error) {
      console.error("Configuration update error:", error);
    } else {
      console.log("Configuration updated:", data);
    }
  });

  // IMPORTANT: Release Rust memory for WASM objects when they are no longer needed
  cache.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```

Properties like `secret`, `label`, `ip`, and `cache_dir` can be set on the `clientConfig` instance directly after construction if needed. `cache_dir` is typically not used in browser environments due to filesystem limitations.

#### Memory Management in WASM

To prevent memory leaks in WebAssembly, explicitly call the `free()` method on `ClientConfig`, `Client`, and `Cache` instances once they are no longer in use. This releases the memory allocated by Rust on the WebAssembly heap.

## Configuration

The client supports the following configuration options:

- `app_id`: Your application ID in Apollo.
- `cluster`: The cluster name (default: "`default`").
- `secret`: The optional secret for the given `app_id`.
- `config_server`: The address of the configuration server.
- `cache_dir`: Directory to store local cache. For non-WASM targets (native Rust applications), if not specified, it defaults to a path constructed as `/opt/data/{app_id}/config-cache`. For WASM targets, this field is optional and defaults to `None`; filesystem caching is generally not applicable or used in browser environments.
- `label`: The label of the current instance. Used to identify the current instance for a grayscale release.
- `ip`: The IP address of your application. Used to identify the current instance for a grayscale release.

## Error Handling

Most client operations return a `Result<T, apollo_rust_client::Error>` in Rust, or a Promise that may reject with an error in JavaScript. Common errors include configuration fetch failures (e.g., network issues, Apollo server errors) or issues with the local cache. Ensure your application code includes appropriate error handling logic (e.g., using `match` or `?` in Rust, or `.catch()` with Promises in JavaScript).

## Key API Changes in v0.5.0

- **Typed Namespaces**: The library now supports multiple configuration formats (Properties, JSON, Text) with automatic format detection based on namespace names
- **Event Listeners**: Added support for event listeners to react to configuration changes
- **Improved Error Handling**: Enhanced error types and better error reporting
- **WASM Improvements**: Better memory management and JavaScript interop for WebAssembly targets

## Namespace Format Detection

The library automatically detects the configuration format based on the namespace name:

- No extension or `.properties` → Properties format (key-value pairs)
- `.json` → JSON format (structured data)
- `.txt` → Text format (plain text content)
- `.yaml` or `.yml` → YAML format (planned)
- `.xml` → XML format (planned)

## TODO

- `Cargo.toml` update for better conditional compilation. Although everything
  works just fine currently, due to the fact that all dependencies are listed
  flat, automation and IDEs are slowed down significantly.
- Added other namespace format supports, for example YAML, JSON etc.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the Apache License Version 2.0 - see the
[LICENSE](LICENSE) file for details.
