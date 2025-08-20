[中文简体](../zh-CN/Configuration.md) | [中文繁體](../zh-TW/Configuration.md)
[Back to Home](Home.md)

# Configuration

The Apollo Rust client supports flexible configuration through the `ClientConfig` struct, which can be initialized directly or from environment variables. This document provides comprehensive information about all configuration options, best practices, and platform-specific considerations.

## Configuration Overview

The `ClientConfig` struct serves as the central configuration object for the Apollo client. It contains all necessary information to connect to Apollo Configuration Centers and manage client behavior.

### Configuration Methods

1. **Direct Configuration**: Manually specify all configuration fields
2. **Environment Variables**: Automatically load configuration from environment variables
3. **Mixed Approach**: Load from environment variables and override specific fields
4. **Runtime Configuration**: Modify configuration at runtime (limited)

## Configuration Fields

### Required Fields

These fields are mandatory for the client to function properly:

#### `app_id` (String)

- **Description**: Your application identifier in Apollo
- **Purpose**: Identifies which application's configuration to retrieve
- **Example**: `"my-application"`, `"user-service"`, `"web-frontend"`
- **Validation**: Must be non-empty string
- **Environment Variable**: `APP_ID`

```rust
let config = ClientConfig {
    app_id: "my-application".to_string(),
    // ... other fields
};
```

#### `config_server` (String)

- **Description**: The Apollo configuration server URL
- **Purpose**: Base URL for Apollo Configuration Center API
- **Format**: Full URL including protocol and port
- **Example**: `"http://apollo-server:8080"`, `"https://config.example.com"`
- **Validation**: Must be valid URL
- **Environment Variable**: `APOLLO_CONFIG_SERVICE`

```rust
let config = ClientConfig {
    config_server: "http://apollo-server:8080".to_string(),
    // ... other fields
};
```

#### `cluster` (String)

- **Description**: The cluster name to connect to
- **Purpose**: Organizes different environments or deployment groups
- **Common Values**: `"default"`, `"production"`, `"staging"`, `"development"`
- **Default**: `"default"` (when using `from_env()`)
- **Environment Variable**: `IDC`

```rust
let config = ClientConfig {
    cluster: "production".to_string(),
    // ... other fields
};
```

### Optional Fields

These fields provide additional functionality but are not required:

#### `secret` (Option<String>)

- **Description**: Secret key for authentication with Apollo server
- **Purpose**: Generates HMAC-SHA1 signatures for secure access
- **When to Use**: When Apollo namespace requires authentication
- **Security**: Store securely, never hardcode in production
- **Environment Variable**: `APOLLO_ACCESS_KEY_SECRET`

```rust
let config = ClientConfig {
    secret: Some("your-secret-key".to_string()),
    // ... other fields
};
```

#### `cache_dir` (Option<String>)

- **Description**: Directory for local cache files (native targets only)
- **Purpose**: Persistent storage for configuration data
- **Default**: `/opt/data/{app_id}/config-cache` (native), `None` (WASM)
- **Platform**: Native Rust only, ignored on WebAssembly
- **Environment Variable**: `APOLLO_CACHE_DIR`

```rust
let config = ClientConfig {
    cache_dir: Some("/custom/cache/path".to_string()),
    // ... other fields
};
```

#### `label` (Option<String>)

- **Description**: Labels for grayscale release targeting
- **Purpose**: Identifies client instance for targeted configuration
- **Format**: Comma-separated list of labels
- **Example**: `"canary"`, `"beta,staging"`, `"region-us-west"`
- **Environment Variable**: `APOLLO_LABEL`

```rust
let config = ClientConfig {
    label: Some("canary,beta".to_string()),
    // ... other fields
};
```

#### `ip` (Option<String>)

- **Description**: IP address for grayscale release targeting
- **Purpose**: Identifies client instance by IP for targeted configuration
- **Format**: IPv4 or IPv6 address
- **Example**: `"192.168.1.100"`, `"2001:db8::1"`
- **Note**: Not set via `from_env()`, must be configured manually

```rust
let config = ClientConfig {
    ip: Some("192.168.1.100".to_string()),
    // ... other fields
};
```

#### `allow_insecure_https` (Option<bool>)

- **Description**: Whether to allow insecure HTTPS connections (self-signed certificates)
- **Purpose**: Enables connection to servers with self-signed or invalid SSL certificates
- **When to Use**: Company internal networks, development environments, testing
- **Security Warning**: Reduces security by bypassing SSL certificate validation
- **Default**: `None` (secure by default)
- **Environment Variable**: `APOLLO_ALLOW_INSECURE_HTTPS`

```rust
let config = ClientConfig {
    allow_insecure_https: Some(true), // Allow self-signed certificates
    // ... other fields
};
```

#### `cache_ttl` (Option<u64>) - Native targets only

- **Description**: Time-to-live for the file cache in seconds.
- **Purpose**: Prevents the client from using a stale cache.
- **Default**: When using `from_env`, this defaults to 600 seconds (10 minutes) if the `APOLLO_CACHE_TTL` environment variable is not set.
- **Environment Variable**: `APOLLO_CACHE_TTL`
- **Platform Support**: This field is only available on native targets (not WebAssembly) as disk caching is not supported in WASM environments.

```rust
let config = ClientConfig {
    #[cfg(not(target_arch = "wasm32"))]
    cache_ttl: Some(300), // 5 minutes
    // ... other fields
};
```

## Configuration Examples

### Minimal Configuration

```rust
use apollo_rust_client::client_config::ClientConfig;

let config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: None,
    ip: None,
    allow_insecure_https: None,
};
```

### Production Configuration

```rust
use apollo_rust_client::client_config::ClientConfig;

let config = ClientConfig {
    app_id: "production-service".to_string(),
    config_server: "https://apollo.company.com".to_string(),
    cluster: "production".to_string(),
    secret: Some("production-secret-key".to_string()),
    cache_dir: Some("/var/cache/apollo".to_string()),
    label: Some("datacenter-east,version-2.1".to_string()),
    ip: Some("10.0.1.100".to_string()),
    allow_insecure_https: None, // Secure by default in production
};
```

### Development Configuration

```rust
use apollo_rust_client::client_config::ClientConfig;

let config = ClientConfig {
    app_id: "dev-app".to_string(),
    config_server: "http://localhost:8080".to_string(),
    cluster: "development".to_string(),
    secret: None,
    cache_dir: Some("/tmp/apollo-cache".to_string()),
    label: Some("developer-workstation".to_string()),
    ip: None,
    allow_insecure_https: None,
};
```

## Environment Variable Configuration

### Using from_env()

The client can automatically load configuration from environment variables:

```rust
use apollo_rust_client::client_config::ClientConfig;

// Load configuration from environment variables
let config = ClientConfig::from_env()?;
```

### Environment Variables

#### Required Variables

- **`APP_ID`**: Your application ID (required)
- **`APOLLO_CONFIG_SERVICE`**: The Apollo config server URL (required)

#### Optional Variables

- **`IDC`**: The cluster name (optional, defaults to "default")
- **`APOLLO_ACCESS_KEY_SECRET`**: The secret key for authentication (optional)
- **`APOLLO_LABEL`**: Comma-separated list of labels for grayscale rules (optional)
- **`APOLLO_CACHE_DIR`**: Directory to store local cache (optional)
- **`APOLLO_CACHE_TTL`**: Time-to-live for the cache in seconds (optional, defaults to 600, native targets only)
- **`APOLLO_ALLOW_INSECURE_HTTPS`**: Whether to allow insecure HTTPS connections (optional, defaults to false)

#### Setting Environment Variables

##### Linux/macOS

```bash
export APP_ID="my-application"
export APOLLO_CONFIG_SERVICE="http://apollo-server:8080"
export IDC="production"
export APOLLO_ACCESS_KEY_SECRET="secret-key"
export APOLLO_LABEL="canary,beta"
export APOLLO_CACHE_DIR="/opt/apollo/cache"
export APOLLO_ALLOW_INSECURE_HTTPS="false"
```

##### Windows (PowerShell)

```powershell
$env:APP_ID = "my-application"
$env:APOLLO_CONFIG_SERVICE = "http://apollo-server:8080"
$env:IDC = "production"
$env:APOLLO_ACCESS_KEY_SECRET = "secret-key"
$env:APOLLO_LABEL = "canary,beta"
$env:APOLLO_CACHE_DIR = "C:\apollo\cache"
$env:APOLLO_ALLOW_INSECURE_HTTPS = "false"
```

##### Windows (Command Prompt)

```cmd
set APP_ID=my-application
set APOLLO_CONFIG_SERVICE=http://apollo-server:8080
set IDC=production
set APOLLO_ACCESS_KEY_SECRET=secret-key
set APOLLO_LABEL=canary,beta
set APOLLO_CACHE_DIR=C:\apollo\cache
```

### Environment File (.env)

Create a `.env` file for local development:

```env
# .env file
APP_ID=my-application
APOLLO_CONFIG_SERVICE=http://localhost:8080
IDC=development
APOLLO_ACCESS_KEY_SECRET=dev-secret-key
APOLLO_LABEL=local-dev
APOLLO_CACHE_DIR=/tmp/apollo-cache
```

Use with the `dotenv` crate:

```rust
use apollo_rust_client::client_config::ClientConfig;

// Load .env file
dotenv::dotenv().ok();

// Load configuration from environment variables
let config = ClientConfig::from_env()?;
```

## Mixed Configuration Approach

Combine environment variables with manual overrides:

```rust
use apollo_rust_client::client_config::ClientConfig;

// Load base configuration from environment
let mut config = ClientConfig::from_env()?;

// Override specific fields
config.ip = Some("192.168.1.100".to_string());
config.label = Some("custom-label".to_string());
```

## Platform-Specific Configuration

### Native Rust Configuration

Native Rust applications have access to all configuration options:

```rust
use apollo_rust_client::client_config::ClientConfig;

let config = ClientConfig {
    app_id: "native-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: Some("secret-key".to_string()),
    cache_dir: Some("/opt/apollo/cache".to_string()), // File caching available
    label: Some("native,server".to_string()),
    ip: Some("10.0.1.100".to_string()),
};
```

### WebAssembly Configuration

WebAssembly applications use a simplified configuration:

```javascript
import { ClientConfig } from "@qqiao/apollo-rust-client";

// WASM constructor takes required fields only
const config = new ClientConfig(
  "wasm-app", // app_id
  "http://apollo-server:8080", // config_server
  "default" // cluster
);

// Set optional fields after construction
config.secret = "secret-key";
config.label = "browser,wasm";
config.ip = "192.168.1.100";
// cache_dir is always None for WASM
```

## Configuration Validation

### Validation Rules

The client validates configuration on creation:

1. **Required Fields**: `app_id`, `config_server`, `cluster` must be non-empty
2. **URL Format**: `config_server` must be a valid URL
3. **Path Validation**: `cache_dir` must be a valid path (native only)
4. **IP Format**: `ip` must be a valid IPv4 or IPv6 address if provided

### Error Handling

```rust
use apollo_rust_client::client_config::{ClientConfig, Error};

match ClientConfig::from_env() {
    Ok(config) => {
        // Configuration loaded successfully
        println!("Configuration loaded: {:?}", config);
    }
    Err(Error::EnvVar(var_error, var_name)) => {
        // Environment variable error
        eprintln!("Missing environment variable {}: {}", var_name, var_error);
    }
}
```

## Security Considerations

### Secret Management

1. **Never Hardcode Secrets**: Use environment variables or secure vaults
2. **Rotate Secrets Regularly**: Implement secret rotation policies
3. **Limit Secret Scope**: Use namespace-specific secrets when possible
4. **Secure Storage**: Store secrets in encrypted configuration systems

```rust
// Good: Load from environment
let config = ClientConfig::from_env()?;

// Bad: Hardcoded secret
let config = ClientConfig {
    secret: Some("hardcoded-secret".to_string()), // Don't do this!
    // ... other fields
};
```

### Network Security

1. **Use HTTPS**: Always use HTTPS in production
2. **Validate Certificates**: Ensure proper TLS certificate validation
3. **Network Isolation**: Use VPNs or private networks when possible

```rust
let config = ClientConfig {
    config_server: "https://apollo.company.com".to_string(), // HTTPS
    // ... other fields
};
```

## Cache Configuration

### Cache Directory Structure

The cache directory structure for native applications:

```
/opt/apollo/cache/
├── my-app_default_application.cache.json
├── my-app_default_config.json.cache.json
├── my-app_production_application_192.168.1.100.cache.json
└── my-app_production_config.json_canary_beta.cache.json
```

### Cache File Naming

Cache files are named using the pattern:
`{app_id}_{cluster}_{namespace}_{ip}_{label}.cache.json`

- IP and label are included when specified
- Multiple labels are joined with underscores
- Special characters are sanitized

### Cache Permissions

Set appropriate permissions for cache directories:

```bash
# Create cache directory
sudo mkdir -p /opt/apollo/cache

# Set ownership
sudo chown -R app-user:app-group /opt/apollo/cache

# Set permissions
sudo chmod 755 /opt/apollo/cache
```

## Configuration Best Practices

### Development Environment

1. **Use Local Apollo Server**: Set up local Apollo instance for development
2. **Separate Configurations**: Use different app IDs for dev/staging/prod
3. **Enable Debug Logging**: Use debug logging for troubleshooting
4. **Fast Cache Expiry**: Use shorter cache times for rapid development

### Production Environment

1. **Use HTTPS**: Always use secure connections
2. **Enable Authentication**: Use secret keys for sensitive configurations
3. **Optimize Cache Settings**: Balance performance and freshness
4. **Monitor Configuration**: Set up monitoring for configuration changes
5. **Backup Configurations**: Implement configuration backup strategies

### Multi-Environment Setup

```rust
use apollo_rust_client::client_config::ClientConfig;

fn create_config(environment: &str) -> ClientConfig {
    match environment {
        "development" => ClientConfig {
            app_id: "myapp-dev".to_string(),
            config_server: "http://localhost:8080".to_string(),
            cluster: "default".to_string(),
            secret: None,
            cache_dir: Some("/tmp/apollo-dev".to_string()),
            label: Some("dev".to_string()),
            ip: None,
        },
        "staging" => ClientConfig {
            app_id: "myapp-staging".to_string(),
            config_server: "https://apollo-staging.company.com".to_string(),
            cluster: "staging".to_string(),
            secret: Some(std::env::var("STAGING_SECRET").unwrap()),
            cache_dir: Some("/opt/apollo/staging".to_string()),
            label: Some("staging".to_string()),
            ip: None,
        },
        "production" => ClientConfig {
            app_id: "myapp".to_string(),
            config_server: "https://apollo.company.com".to_string(),
            cluster: "production".to_string(),
            secret: Some(std::env::var("PRODUCTION_SECRET").unwrap()),
            cache_dir: Some("/opt/apollo/production".to_string()),
            label: Some("production".to_string()),
            ip: Some(std::env::var("INSTANCE_IP").unwrap()),
        },
        _ => panic!("Unknown environment: {}", environment),
    }
}
```

## Troubleshooting Configuration

### Common Issues

#### Missing Environment Variables

```bash
# Check if environment variables are set
echo $APP_ID
echo $APOLLO_CONFIG_SERVICE
```

#### Invalid URLs

```bash
# Test URL accessibility
curl -v http://your-apollo-server:8080/configs/your-app/default/application
```

#### Permission Issues

```bash
# Check cache directory permissions
ls -la /opt/apollo/cache
```

#### Network Connectivity

```bash
# Test network connectivity
ping apollo-server
telnet apollo-server 8080
```

### Debug Configuration

Enable debug logging to troubleshoot configuration issues:

```rust
use apollo_rust_client::client_config::ClientConfig;

// Enable debug logging
std::env::set_var("RUST_LOG", "apollo_rust_client=debug");
env_logger::init();

// Load configuration
let config = ClientConfig::from_env()?;
println!("Configuration loaded: {:?}", config);
```

## Configuration Migration

### Upgrading from Previous Versions

#### Version 0.5.x to 0.6.0

**Breaking Changes:**

- **Namespace Getter Methods**: All `get_*` methods are now **synchronous** instead of async
- **JSON/YAML Deserialization**: `to_object<T>()` methods are now synchronous
- **API Changes**: Constructor parameter renamed from `client_config` to `config`

**Migration Steps:**

1. Remove `.await` from all namespace getter methods:

   ```rust
   // Before (0.5.x)
   let value = properties.get_string("key").await;
   let config = json_namespace.to_object().await?;

   // After (0.6.0)
   let value = properties.get_string("key");
   let config = json_namespace.to_object()?;
   ```

2. Update constructor calls (if using old parameter name):

   ```rust
   // Before
   let client = Client::new(client_config);

   // After
   let client = Client::new(config);
   ```

#### Version 0.4.x to 0.5.0

**New Features Added:**

- **Multiple Configuration Formats**: Support for JSON, YAML, and Text formats
- **Cache TTL**: Time-to-live mechanism for cache expiration
- **Enhanced Namespace System**: Unified namespace handling with format detection

**New Configuration Fields:**

- `cache_ttl: Option<u64>` - Cache expiration time in seconds (default: 600)
- `APOLLO_CACHE_TTL` environment variable support

**Migration Steps:**

1. Update Cargo.toml to version 0.5.0:

   ```toml
   [dependencies]
   apollo-rust-client = "0.5.0"
   ```

2. Add cache TTL configuration (optional):

   ```rust
   let config = ClientConfig {
       // ... existing fields ...
       cache_ttl: Some(300), // 5 minutes, or use APOLLO_CACHE_TTL env var
   };
   ```

3. No code changes required - all new fields are optional and backward compatible

#### Version 0.3.x to 0.4.0

**Breaking Changes:**

- `cache_dir` changed from `PathBuf` to `Option<String>`

**Migration Steps:**

```rust
// Old (0.3.x)
let config = ClientConfig {
    cache_dir: Some(PathBuf::from("/opt/cache")),
    // ... other fields
};

// New (0.4.0+)
let config = ClientConfig {
    cache_dir: Some("/opt/cache".to_string()),
    // ... other fields
};
```
