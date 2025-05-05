# apollo-rust-client

A Rust client for [Apollo Configuration centre](https://www.apolloconfig.com/).

## Features

- Support for multiple namespaces
- Configuration updates via polling
- Type-safe configuration management
- Async/await support
- Error handling with detailed diagnostics

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
apollo-rust-client = "0.2.0"
```

## Usage

### Basic Example

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig {
        app_id: "app_id".to_string(),
        cluster: "default".to_string(),
        secret: Some("your_secret".to_string()),
        config_server: "http://your-apollo-server:8080",
        cache_dir: None,
        label: None,
        ip: None,
    };
    let client = Client::new(client_config);

    // Get configuration
    let cache = client.namespace("application");

    let configuration = cache.get_property::<i64>("meaningOfLife").await?
    println!("Configuration: {:?}", configuration);

    Ok(())
}
```

## Configuration

The client supports the following configuration options:

- `app_id`: Your application ID in Apollo
- `cluster`: The cluster name (default: "`default`")
- `secret`: The optional secret for the given `app_id`
- `config_server`: The address of the configuration server
- `cache_dir`: Directory to store local cache (default: "`/opt/data/${app_id}/config-cache`")
- `label`: The label of the current instance. Used to identify the current instance for a grayscale release.
- `ip`: The IP address of your application. Used to identify the current instance for a grayscale release.

## Error Handling

The client provides detailed error information through the `ApolloError` type, which includes:

- Network errors
- Configuration parsing errors
- Authentication errors
- Invalid response errors

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the Apache License Version 2.0 - see the
[LICENSE](LICENSE) file for details.
