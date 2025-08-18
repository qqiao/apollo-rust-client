[中文简体](../zh-CN/Rust-Usage.md) | [中文繁體](../zh-TW/Rust-Usage.md)
[Back to Home](Home.md)

# Rust Usage

The following example demonstrates how to initialize `ClientConfig`, start the client, fetch a namespace, and retrieve configuration properties in a Rust application.

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
        allow_insecure_https: None,
        #[cfg(not(target_arch = "wasm32"))]
        cache_ttl: None, // Uses default: 600 seconds (10 minutes)
    };
    let mut client = Client::new(client_config);

    // Start background polling for configuration updates.
    client.start().await?;

    // Get configuration for the "application" namespace.
    let namespace = client.namespace("application").await?;

    // The namespace is now typed based on the format detected from the namespace name
    match namespace {
        apollo_rust_client::namespace::Namespace::Properties(properties) => {
            // Example: Retrieve a string property.
            match properties.get_string("some_key").await {
                Some(value) => println!("Property 'some_key': {}", value),
                None => println!("Property 'some_key' not found"),
            }

            // Example: Retrieve an integer property.
            match properties.get_int("meaningOfLife").await {
                Some(value) => println!("Property 'meaningOfLife': {}", value),
                None => println!("Property 'meaningOfLife' not found"),
            }

            // Example: Retrieve a float property.
            match properties.get_float("pi").await {
                Some(value) => println!("Property 'pi': {}", value),
                None => println!("Property 'pi' not found"),
            }

            // Example: Retrieve a boolean property.
            match properties.get_bool("debug_enabled").await {
                Some(value) => println!("Property 'debug_enabled': {}", value),
                None => println!("Property 'debug_enabled' not found"),
            }

            // Example: Retrieve a generic property with custom type.
            match properties.get_property::<u32>("port").await {
                Some(value) => println!("Property 'port': {}", value),
                None => println!("Property 'port' not found"),
            }
        }
        apollo_rust_client::namespace::Namespace::Json(json) => {
            // For JSON namespaces, you can deserialize to custom types
            use serde::{Deserialize, Serialize};

            #[derive(Deserialize, Serialize)]
            struct DatabaseConfig {
                host: String,
                port: u16,
                ssl: bool,
            }

            match json.to_object::<DatabaseConfig>() {
                Ok(config) => {
                    println!("Database host: {}", config.host);
                    println!("Database port: {}", config.port);
                    println!("Database SSL: {}", config.ssl);
                }
                Err(e) => println!("Failed to deserialize JSON config: {}", e),
            }
        }
        apollo_rust_client::namespace::Namespace::Yaml(yaml) => {
            // For YAML namespaces, you can deserialize to custom types
            use serde::{Deserialize, Serialize};

            #[derive(Deserialize, Serialize)]
            struct ServerConfig {
                host: String,
                port: u16,
                database: DatabaseConfig,
            }

            #[derive(Deserialize, Serialize)]
            struct DatabaseConfig {
                host: String,
                port: u16,
                credentials: CredentialConfig,
            }

            #[derive(Deserialize, Serialize)]
            struct CredentialConfig {
                username: String,
                password: String,
            }

            match yaml.to_object::<ServerConfig>() {
                Ok(config) => {
                    println!("Server: {}:{}", config.host, config.port);
                    println!("Database: {}:{}", config.database.host, config.database.port);
                    println!("DB User: {}", config.database.credentials.username);
                }
                Err(e) => println!("Failed to deserialize YAML config: {}", e),
            }
        }
        apollo_rust_client::namespace::Namespace::Text(text) => {
            println!("Text namespace content: {}", text);
        }
    }

    Ok(())
}
```

## Configuration via Environment Variables

The `ClientConfig` can also be initialized from environment variables using `ClientConfig::from_env()`. This is useful for server-side applications where configuration is often passed via the environment. The following variables are recognized:

- `APP_ID`: Your application ID.
- `APOLLO_ACCESS_KEY_SECRET`: The secret key for your namespace (if applicable).
- `IDC`: The cluster name (defaults to "default" if not set).
- `APOLLO_CONFIG_SERVICE`: The Apollo config server URL.
- `APOLLO_LABEL`: The label for grayscale releases.
- `APOLLO_CACHE_DIR`: The cache directory path.

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize from environment variables
    let client_config = ClientConfig::from_env()?;
    let mut client = Client::new(client_config);

    client.start().await?;

    let namespace = client.namespace("application").await?;

    // Handle the namespace based on its type
    match namespace {
        apollo_rust_client::namespace::Namespace::Properties(properties) => {
            // Work with properties
            if let Some(value) = properties.get_string("app.name").await {
                println!("Application name: {}", value);
            }
        }
        _ => {
            println!("Received non-properties namespace");
        }
    }

    Ok(())
}
```

## Event Listeners (Native Rust Only)

For native Rust applications, you can register event listeners to be notified when configuration changes:

```rust
use apollo_rust_client::{Client, EventListener};
use apollo_rust_client::client_config::ClientConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig {
        app_id: "your_app_id".to_string(),
        cluster: "default".to_string(),
        secret: None,
        config_server: "http://your-apollo-server:8080".to_string(),
        cache_dir: None,
        label: None,
        ip: None,
        allow_insecure_https: None,
    };
    let mut client = Client::new(client_config);

    // Register an event listener before starting
    let listener: EventListener = Arc::new(|result| {
        match result {
            Ok(namespace) => {
                println!("Configuration updated: {:?}", namespace);
            }
            Err(e) => {
                println!("Configuration update failed: {:?}", e);
            }
        }
    });

    client.add_listener("application", listener).await;
    client.start().await?;

    // Your application logic here

    Ok(())
}
```

## Namespace Format Detection

The library automatically detects the configuration format based on the namespace name:

- **Properties format** (default): `application`, `database.properties` → Key-value pairs
- **JSON format**: `config.json`, `settings.json` → Structured JSON data
- **Text format**: `readme.txt`, `content.txt` → Plain text content
- **YAML format**: `config.yaml`, `app.yml` → YAML configuration with structured data support
- **XML format** (planned): `config.xml` → XML configuration

## Error Handling

The library uses Rust's `Result` type for error handling. Common error scenarios include:

- **Network errors**: Connection issues with the Apollo server
- **Authentication errors**: Invalid secret key or unauthorized access
- **Parsing errors**: Invalid JSON or configuration format
- **Cache errors**: File system issues or cache corruption

```rust
match client.namespace("application").await {
    Ok(namespace) => {
        // Handle successful namespace retrieval
        match namespace {
            apollo_rust_client::namespace::Namespace::Properties(properties) => {
                // Work with properties
            }
            _ => {
                // Handle other namespace types
            }
        }
    }
    Err(e) => {
        eprintln!("Failed to get namespace: {}", e);
        // Handle error appropriately
    }
}
```
