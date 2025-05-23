[中文简体](../zh-CN/Rust-Usage.md) | [中文繁體](../zh-TW/Rust-Usage.md)
[Back to Home](Home.md)

## Rust Usage

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
    };
    let mut client = Client::new(client_config);
    // Start background polling for configuration updates.
    client.start().await?;

    // Get configuration for the "application" namespace.
    let cache = client.namespace("application").await;

    // Example: Retrieve a string property.
    match cache.get_property::<String>("some_key").await {
        Some(value) => println!("Property 'some_key': {}", value),
        None => println!("Property 'some_key' not found"),
    }

    // Example: Retrieve an integer property.
    match cache.get_property::<i64>("meaningOfLife").await {
        Some(value) => println!("Property 'meaningOfLife': {}", value),
        None => println!("Property 'meaningOfLife' not found"),
    }
    Ok(())
}
```

### Configuration via Environment Variables (Rust)

The `ClientConfig` can also be initialized from environment variables using `ClientConfig::from_env()`. This is useful for server-side applications where configuration is often passed via the environment. The following variables are recognized:

- `APP_ID`: Your application ID.
- `APOLLO_ACCESS_KEY_SECRET`: The secret key for your namespace (if applicable).
- `IDC`: The cluster name (defaults to "default" if not set).
- `APOLLO_CONFIG_SERVICE`: The URL of the Apollo Config Service.
- `APOLLO_LABEL`: Comma-separated list of labels for grayscale rules.
- `APOLLO_CACHE_DIR`: Directory to store local cache.

Note: The `ip` field is not set via `from_env()`.
