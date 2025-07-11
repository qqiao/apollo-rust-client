[中文简体](../zh-CN/Configuration.md) | [中文繁體](../zh-TW/Configuration.md)
[Back to Home](Home.md)

# Configuration

The Apollo Rust client supports flexible configuration through the `ClientConfig` struct, which can be initialized directly or from environment variables.

## Configuration Options

### Required Fields

- **`app_id`**: Your application ID in Apollo (String)
- **`config_server`**: The address of the Apollo configuration server (String)
- **`cluster`**: The cluster name, typically "default" (String)

### Optional Fields

- **`secret`**: The optional secret for authentication (Option<String>)
- **`cache_dir`**: Directory to store local cache files (Option<String>)
  - For native Rust: Defaults to `/opt/data/{app_id}/config-cache` if not specified
  - For WASM: Not applicable, always `None`
- **`label`**: The label for grayscale releases (Option<String>)
- **`ip`**: The IP address for grayscale releases (Option<String>)

## Direct Configuration

```rust
use apollo_rust_client::client_config::ClientConfig;

let client_config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: Some("my-secret-key".to_string()),
    cache_dir: Some("/custom/cache/path".to_string()),
    label: Some("production".to_string()),
    ip: Some("192.168.1.100".to_string()),
};
```

## Environment Variable Configuration

The client can be configured using environment variables:

```rust
use apollo_rust_client::client_config::ClientConfig;

// This will read from environment variables
let client_config = ClientConfig::from_env()?;
```

### Environment Variables

- **`APP_ID`**: Your application ID (required)
- **`APOLLO_CONFIG_SERVICE`**: The Apollo config server URL (required)
- **`IDC`**: The cluster name (optional, defaults to "default")
- **`APOLLO_ACCESS_KEY_SECRET`**: The secret key for authentication (optional)
- **`APOLLO_CACHE_DIR`**: The cache directory path (optional)
- **`APOLLO_LABEL`**: The label for grayscale releases (optional)

Note: The `ip` field is not configurable via environment variables and must be set manually if needed.

## WASM Configuration

For WebAssembly targets, use the simplified constructor:

```javascript
import { ClientConfig } from "@qqiao/apollo-rust-client";

const clientConfig = new ClientConfig(
  "my-app", // app_id
  "http://apollo-server:8080", // config_server
  "default" // cluster
);

// Set optional properties
clientConfig.secret = "my-secret-key";
clientConfig.label = "production";
clientConfig.ip = "192.168.1.100";
```

## Grayscale Release Configuration

For grayscale releases, you can configure the client to identify itself with specific labels or IP addresses:

### Using Labels

```rust
let client_config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    label: Some("beta-group".to_string()),
    // ... other fields
};
```

### Using IP Address

```rust
let client_config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    ip: Some("10.0.1.100".to_string()),
    // ... other fields
};
```

## Security Considerations

- **Secret Key**: Store secret keys securely and avoid hardcoding them in source code
- **Environment Variables**: Use environment variables for sensitive configuration in production
- **HTTPS**: Always use HTTPS for the `config_server` URL in production environments
- **Access Control**: Ensure proper network access controls are in place for Apollo server communication

## Best Practices

1. **Use environment variables** for production deployments
2. **Set appropriate cache directories** for native applications
3. **Configure grayscale parameters** for staged rollouts
4. **Use meaningful cluster names** for different environments
5. **Implement proper error handling** for configuration failures
