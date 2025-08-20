# Upgrade Guide

This guide provides step-by-step instructions for upgrading between different versions of the Apollo Rust Client.

## Table of Contents

- [Upgrading from v0.5.x to v0.6.0](#upgrading-from-v05x-to-v060)
- [Upgrading from v0.4.x to v0.5.0](#upgrading-from-v04x-to-v050)
- [Upgrading from v0.3.x to v0.4.0](#upgrading-from-v03x-to-v040)

## Upgrading from v0.5.x to v0.6.0

### ⚠️ Breaking Changes

This is a **minor version bump** (0.6.0) due to significant breaking changes in the API.

#### 1. Namespace Getter Methods (Most Important)

All `get_*` methods in namespace types are now **synchronous** instead of async:

```rust
// Before (0.5.x) - All methods were async
let app_name = properties.get_string("app.name").await;
let port = properties.get_int("server.port").await;
let debug = properties.get_bool("debug.enabled").await;
let custom = properties.get_property::<MyType>("config").await;

// After (0.6.0) - All methods are now synchronous
let app_name = properties.get_string("app.name");
let port = properties.get_int("server.port");
let debug = properties.get_bool("debug.enabled");
let custom = properties.get_property::<MyType>("config");
```

#### 2. JSON and YAML Deserialization

The `to_object<T>()` methods are now synchronous:

```rust
// Before (0.5.x) - Methods were async
let config: DatabaseConfig = json_namespace.to_object().await?;
let settings: ServerConfig = yaml_namespace.to_object().await?;

// After (0.6.0) - Methods are now synchronous
let config: DatabaseConfig = json_namespace.to_object()?;
let settings: ServerConfig = yaml_namespace.to_object()?;
```

#### 3. API Changes

- `Client::new()` constructor parameter renamed from `client_config` to `config`
- `Client` struct field renamed from `client_config` to `config`

```rust
// Before
let client = Client::new(client_config);

// After
let client = Client::new(config);
```

#### 4. Function Visibility Changes

Several internal functions are now `pub(crate)` instead of public:

- `get_namespace()` function is now `pub(crate)`
- `Cache` struct is now `pub(crate)`

### Migration Steps

1. **Update Cargo.toml**:

   ```toml
   [dependencies]
   apollo-rust-client = "0.6.0"
   ```

2. **Remove `.await` from all namespace getter methods**:

   ```rust
   // Search and replace all occurrences of:
   .get_string("key").await → .get_string("key")
   .get_int("key").await → .get_int("key")
   .get_bool("key").await → .get_bool("key")
   .get_property::<Type>("key").await → .get_property::<Type>("key")
   ```

3. **Remove `.await` from deserialization methods**:

   ```rust
   // Search and replace all occurrences of:
   .to_object().await → .to_object()
   ```

4. **Update constructor calls** (if using old parameter name):

   ```rust
   // Search and replace:
   Client::new(client_config) → Client::new(config)
   ```

5. **Remove direct imports of internal types**:

   ```rust
   // Remove these imports if they exist:
   use apollo_rust_client::cache::Cache;
   use apollo_rust_client::namespace::get_namespace;

   // Use the public API instead:
   use apollo_rust_client::{Client, ClientConfig};
   ```

### Example Migration

```rust
// Before (0.5.x)
use apollo_rust_client::{Client, client_config::ClientConfig};

let client_config = ClientConfig::from_env()?;
let mut client = Client::new(client_config);

let namespace = client.namespace("application").await?;
match namespace {
    apollo_rust_client::namespace::Namespace::Properties(properties) => {
        let app_name = properties.get_string("app.name").await;
        let port = properties.get_int("server.port").await;
        // ... more code
    }
    apollo_rust_client::namespace::Namespace::Json(json) => {
        let config: DatabaseConfig = json.to_object().await?;
        // ... more code
    }
    _ => {}
}

// After (0.6.0)
use apollo_rust_client::{Client, client_config::ClientConfig};

let config = ClientConfig::from_env()?;
let mut client = Client::new(config);

let namespace = client.namespace("application").await?;
match namespace {
    apollo_rust_client::namespace::Namespace::Properties(properties) => {
        let app_name = properties.get_string("app.name");
        let port = properties.get_int("server.port");
        // ... more code
    }
    apollo_rust_client::namespace::Namespace::Json(json) => {
        let config: DatabaseConfig = json.to_object()?;
        // ... more code
    }
    _ => {}
}
```

## Upgrading from v0.4.x to v0.5.0

### New Features Added

- **Multiple Configuration Formats**: Support for JSON, YAML, and Text formats
- **Cache TTL**: Time-to-live mechanism for cache expiration
- **Enhanced Namespace System**: Unified namespace handling with format detection
- **Improved Error Handling**: Better error types and handling

### New Configuration Fields

- `cache_ttl: Option<u64>` - Cache expiration time in seconds (default: 600)
- `APOLLO_CACHE_TTL` environment variable support

### Migration Steps

1. **Update Cargo.toml**:

   ```toml
   [dependencies]
   apollo-rust-client = "0.5.0"
   ```

2. **Add cache TTL configuration** (optional):

   ```rust
   let config = ClientConfig {
       app_id: "my-app".to_string(),
       config_server: "http://apollo-server:8080".to_string(),
       cluster: "default".to_string(),
       secret: None,
       cache_dir: None,
       label: None,
       ip: None,
       cache_ttl: Some(300), // 5 minutes, or use APOLLO_CACHE_TTL env var
   };
   ```

3. **No code changes required** - all new fields are optional and backward compatible

4. **Environment variable support** (optional):
   ```bash
   export APOLLO_CACHE_TTL=300  # 5 minutes
   ```

### Example Migration

```rust
// Before (0.4.x)
let config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: None,
    ip: None,
};

// After (0.5.0) - Add cache_ttl field
let config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: None,
    ip: None,
    cache_ttl: Some(600), // 10 minutes default
};
```

## Upgrading from v0.3.x to v0.4.0

### Breaking Changes

- `cache_dir` field type changed from `PathBuf` to `Option<String>`

### Migration Steps

1. **Update Cargo.toml**:

   ```toml
   [dependencies]
   apollo-rust-client = "0.4.0"
   ```

2. **Update cache_dir field type**:

   ```rust
   // Old (0.3.x)
   use std::path::PathBuf;

   let config = ClientConfig {
       app_id: "my-app".to_string(),
       config_server: "http://apollo-server:8080".to_string(),
       cluster: "default".to_string(),
       secret: None,
       cache_dir: Some(PathBuf::from("/opt/cache")),
       label: None,
       ip: None,
   };

   // New (0.4.0+)
   let config = ClientConfig {
       app_id: "my-app".to_string(),
       config_server: "http://apollo-server:8080".to_string(),
       cluster: "default".to_string(),
       secret: None,
       cache_dir: Some("/opt/cache".to_string()),
       label: None,
       ip: None,
   };
   ```

## General Upgrade Tips

### 1. Incremental Upgrades

It's recommended to upgrade incrementally:

- v0.3.x → v0.4.x → v0.5.x → v0.6.0

This allows you to test each version and catch any issues early.

### 2. Testing After Each Upgrade

After each upgrade:

1. Run your test suite
2. Test in a staging environment
3. Verify all functionality works as expected

### 3. Dependency Updates

Some upgrades may require updating other dependencies:

```bash
cargo update
cargo build
cargo test
```

### 4. Breaking Changes Summary

| Version         | Breaking Changes                   | Migration Effort |
| --------------- | ---------------------------------- | ---------------- |
| v0.3.x → v0.4.0 | `cache_dir` type change            | Low              |
| v0.4.x → v0.5.0 | None (new features only)           | None             |
| v0.5.x → v0.6.0 | Async to sync methods, API changes | High             |

### 5. Getting Help

If you encounter issues during upgrade:

1. Check the [CHANGELOG.md](../../../CHANGELOG.md) for detailed change information
2. Review the [Error Handling Guide](Error-Handling.md) for troubleshooting
3. Open an issue on GitHub with details about your specific problem

## Version Compatibility Matrix

| Feature                    | v0.3.x | v0.4.x | v0.5.x | v0.6.0 |
| -------------------------- | ------ | ------ | ------ | ------ |
| Properties format          | ✅     | ✅     | ✅     | ✅     |
| JSON format                | ❌     | ❌     | ✅     | ✅     |
| YAML format                | ❌     | ❌     | ✅     | ✅     |
| Text format                | ❌     | ❌     | ✅     | ✅     |
| Cache TTL                  | ❌     | ❌     | ✅     | ✅     |
| Async getter methods       | ✅     | ✅     | ✅     | ❌     |
| Sync getter methods        | ❌     | ❌     | ❌     | ✅     |
| Multiple namespace formats | ❌     | ❌     | ✅     | ✅     |
