# Changelog

All notable changes to the apollo-rust-client project will be documented in this file.

## [Unreleased]

### Changed

- Improved documentation for the `.properties` format to be clearer and provide better examples.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-08-20

### ⚠️ BREAKING CHANGES

- **Namespace Getter Methods**: All `get_*` methods in namespace types are now **synchronous** instead of async:

  ```rust
  // Before (0.5.x) - All methods were async
  let value: Option<String> = properties.get_string("key").await;
  let number: Option<i64> = properties.get_int("port").await;
  let flag: Option<bool> = properties.get_bool("enabled").await;
  let custom: Option<MyType> = properties.get_property("config").await;

  // After (0.6.0) - All methods are now synchronous
  let value: Option<String> = properties.get_string("key");
  let number: Option<i64> = properties.get_int("port");
  let flag: Option<bool> = properties.get_bool("enabled");
  let custom: Option<MyType> = properties.get_property("config");
  ```

- **JSON and YAML Deserialization**: The `to_object<T>()` methods are now synchronous:

  ```rust
  // Before (0.5.x) - Methods were async
  let config: DatabaseConfig = json_namespace.to_object().await?;
  let settings: ServerConfig = yaml_namespace.to_object().await?;

  // After (0.6.0) - Methods are now synchronous
  let config: DatabaseConfig = json_namespace.to_object()?;
  let settings: ServerConfig = yaml_namespace.to_object()?;
  ```

- **Function Visibility Changes**: Several internal functions have been made `pub(crate)` instead of public:

  - `get_namespace()` function is now `pub(crate)` (was previously public)
  - `Cache` struct is now `pub(crate)` (was previously public)
  - This affects code that directly imported these types from the crate root

- **Minor API Changes**:
  - `Client::new()` constructor parameter renamed from `client_config` to `config`
  - `Client` struct field renamed from `client_config` to `config`

### Migration Guide

To update your code from 0.5.x to 0.6.0:

1. **Remove `.await` from all namespace getter methods** (This is the most important change):

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

2. **Remove `.await` from JSON/YAML deserialization**:

   ```rust
   // Before (0.5.x)
   let config: DatabaseConfig = json_namespace.to_object().await?;
   let settings: ServerConfig = yaml_namespace.to_object().await?;

   // After (0.6.0)
   let config: DatabaseConfig = json_namespace.to_object()?;
   let settings: ServerConfig = yaml_namespace.to_object()?;
   ```

3. **Update Client Constructor Calls** (if you were using the old parameter name):

   ```rust
   // Before
   let client = Client::new(client_config);

   // After
   let client = Client::new(config);
   ```

4. **Remove Direct Imports** (if you were importing internal types):

   ```rust
   // Before - These imports will no longer work
   use apollo_rust_client::cache::Cache;
   use apollo_rust_client::namespace::get_namespace;

   // After - Use the public API instead
   use apollo_rust_client::{Client, ClientConfig};
   ```

5. **Update Cargo.toml**:
   ```toml
   [dependencies]
   apollo-rust-client = "0.6.0"
   ```

### Changed

- **Code Quality Improvements**: Comprehensive lint fixes and code improvements across the entire codebase:
  - Enhanced error handling and improved error types throughout the codebase
  - Better async/await patterns and improved concurrency handling
  - Refactored cache implementation with improved performance and reliability
  - Enhanced namespace handling with better format detection and parsing
  - Improved client configuration with better validation and error handling
  - Updated all dependencies to latest stable versions for security and performance
  - Enhanced documentation with better examples and clearer explanations
  - Improved WebAssembly support with better memory management
  - Better cross-platform compatibility and platform-specific optimizations

### Technical Improvements

- **Cache System**: Complete refactoring of the caching mechanism:
  - Improved cache invalidation and refresh logic
  - Better handling of concurrent cache operations
  - Enhanced file cache management with improved error recovery
  - Optimized memory usage and reduced allocations
- **Namespace Handling**: Enhanced namespace format support:
  - Improved JSON, YAML, and Properties format parsing
  - Better error handling for malformed configurations
  - Enhanced format detection and validation
- **Client Configuration**: Improved configuration management:
  - Better environment variable handling
  - Enhanced validation and error reporting
  - Improved platform-specific feature handling
- **Documentation**: Comprehensive documentation updates:
  - Updated all usage examples and code samples
  - Enhanced multi-language documentation support
  - Improved API documentation with better examples
  - Better error handling guides and troubleshooting tips

## [0.5.3] - 2025-08-19

### Changed

- **Dependency Updates**: Updated transitive dependencies to latest stable versions:
  - `slab` updated from 0.4.10 to 0.4.11 (bug fix for `Slab::get_disjoint_mut` out of bounds issue)

## [0.5.2] - 2025-08-19

### Added

- **Insecure HTTPS Support**: Added `allow_insecure_https` flag to `ClientConfig` for handling self-signed certificates:
  - New `allow_insecure_https` field in `ClientConfig` struct
  - Configurable via `APOLLO_ALLOW_INSECURE_HTTPS` environment variable
  - Enables `danger_accept_invalid_certs(true)` and `danger_accept_invalid_hostnames(true)` in reqwest client
  - Useful for development environments and internal networks with self-signed certificates
  - **Warning**: Reduces security by bypassing SSL certificate validation

### Changed

- **Dependency Updates**: Updated all dependencies to latest stable versions:
  - `reqwest` updated from 0.12.22 to 0.12.23
  - `thiserror` updated from 2.0.12 to 2.0.15
  - `async-std` updated from 1.13.1 to 1.13.2
  - `tokio` updated from 1.46.1 to 1.47.1
  - `serde_json` updated from 1.0.140 to 1.0.142

### Documentation

- **Enhanced Configuration Guide**: Added comprehensive documentation for insecure HTTPS feature
- **Multi-language Support**: Updated documentation in English, Simplified Chinese, and Traditional Chinese
- **Usage Examples**: Added examples showing how to configure and use the insecure HTTPS feature
- **Security Warnings**: Added clear warnings about security implications of enabling insecure HTTPS

## Version Bump Rationale

This release is marked as **0.6.0** (minor version bump) instead of **0.5.4** (patch version) because it contains **breaking changes** that require code modifications when upgrading from 0.5.x:

- **Major API Breaking Changes**: All namespace getter methods (`get_string`, `get_int`, `get_bool`, `get_property`) are now **synchronous** instead of async
- **Deserialization Changes**: JSON and YAML `to_object<T>()` methods are now synchronous
- **API Surface Changes**: Constructor parameter names and struct field names have changed
- **Visibility Changes**: Internal types are no longer publicly accessible
- **Major Refactoring**: Significant internal improvements that affect the public API surface

The async-to-sync changes are the most significant breaking changes as they affect the core usage pattern of the library. Users upgrading from 0.5.x must remove `.await` calls from all getter methods. While the core functionality remains the same, these changes ensure better API design and internal organization. Users upgrading from 0.5.x should follow the migration guide above.

### Technical Improvements

- **HTTP Client Configuration**: Enhanced reqwest client builder with conditional insecure certificate handling
- **Environment Variable Support**: Added `APOLLO_ALLOW_INSECURE_HTTPS` environment variable support
- **Platform Compatibility**: Feature works on both native and WebAssembly targets

## [0.5.1] - 2025-07-31

### Changed

- **Platform-specific Cache TTL**: Made `cache_ttl` field native-only to align with WASM limitations:
  - Added `#[cfg(not(target_arch = "wasm32"))]` attribute to `cache_ttl` field in `ClientConfig`
  - Updated `from_env()` method to conditionally handle cache TTL only on native targets
  - Modified WASM constructor to exclude cache TTL field
  - Wrapped cache TTL logic in `cache.rs` with conditional compilation
  - Updated all documentation to reflect native-only availability
  - Reduced WASM bundle size by excluding unnecessary cache TTL code

### Documentation

- **Updated Documentation**: Clarified platform-specific nature of cache TTL:
  - Added conditional compilation attributes to all code examples
  - Updated README files to indicate native-only availability
  - Modified configuration documentation to specify platform support
  - Updated changelog entries to reflect native-only nature

## [0.5.0] - 2025-07-29

### Added

- **Multiple Configuration Format Support**: Added comprehensive support for different configuration formats:
  - **JSON Format**: Full support for structured JSON data with automatic detection via `.json` file extensions
  - **YAML Format**: Complete YAML configuration support with automatic detection via `.yaml` or `.yml` extensions
  - **Text Format**: Plain text configuration support with automatic detection via `.txt` extensions
  - **Properties Format**: Enhanced existing properties format support (default when no extension is specified)
- **Automatic Format Detection**: Intelligent namespace format detection based on file extensions in namespace names
- **Cache TTL Support**: Added Time-To-Live (TTL) mechanism for cache expiration:
  - New `cache_ttl` field in `ClientConfig` struct
  - Default TTL of 600 seconds (10 minutes) when using `from_env()`
  - Configurable via `APOLLO_CACHE_TTL` environment variable
  - Automatic cache refresh when TTL expires
- **Enhanced Namespace System**: Complete refactoring of the namespace module with:
  - Unified `Namespace` enum supporting all configuration formats
  - Type-safe conversion between different format types
  - Improved error handling with format-specific error types
- **Comprehensive Documentation**: Extensive documentation updates including:
  - Complete API documentation with examples
  - Multi-language documentation (English, Simplified Chinese, Traditional Chinese)
  - Design documentation covering architecture and implementation details
  - Usage guides for both Rust and JavaScript/WASM environments

### Changed

- **Major API Refactoring**: Significant improvements to the client API:
  - Enhanced `Client` struct with improved error handling
  - Refactored cache system with better concurrency control
  - Improved event listener system for configuration changes
- **Dependency Updates**: Updated all dependencies to latest stable versions:
  - `reqwest` updated to 0.12.22
  - `tokio` updated to 1.46.1
  - `serde_json` updated to 1.0.140
  - `serde_yaml` updated to 0.9.34
  - Added `chrono` 0.4.41 for timestamp handling
  - Added `url` 2.5.4 for improved URL parsing
- **Cache Implementation**: Complete rewrite of the caching system:
  - New `CacheItem` struct with timestamp support for TTL functionality
  - Improved file cache handling with better error recovery
  - Enhanced memory cache management
  - Better platform-specific optimizations for native vs WASM targets

### Fixed

- **Stale Cache Issue**: Fixed critical issue where cached configurations could become stale indefinitely:
  - Previously, file cache would always return cached data if file existed
  - Now implements proper TTL-based cache invalidation
  - Automatic refresh when cache expires
- **Concurrency Issues**: Improved handling of concurrent cache operations:
  - Better protection against race conditions during cache refresh
  - Enhanced loading state management
  - Improved error handling for concurrent access scenarios

### Technical Improvements

- **Code Organization**: Major restructuring of the codebase:
  - Separated namespace handling into dedicated modules (`json.rs`, `yaml.rs`, `properties.rs`)
  - Improved module organization and separation of concerns
  - Better error type definitions and handling
- **Performance Optimizations**: Various performance improvements:
  - More efficient cache lookup and storage
  - Reduced memory allocations in hot paths
  - Better async/await usage patterns
- **Platform Support**: Enhanced cross-platform compatibility:
  - Improved WASM target support with better memory management
  - Better handling of platform-specific features
  - Enhanced JavaScript interop for WASM builds

### Documentation

- **Comprehensive Wiki**: Added extensive documentation covering:
  - Configuration guide with all available options
  - Design documentation explaining architecture decisions
  - Error handling guide with troubleshooting tips
  - Installation and usage instructions for all platforms
  - Memory management guide for WASM environments
- **API Documentation**: Complete API reference with examples
- **Multi-language Support**: Documentation available in English, Simplified Chinese, and Traditional Chinese
- **Documentation Updates**: Recent improvements including:
  - Added `cache_ttl` field documentation in all code examples
  - Updated README files to include `APOLLO_CACHE_TTL` environment variable
  - Enhanced error handling documentation for namespace formats
  - Improved text format support with proper error handling
  - Better documentation for unsupported XML format

## [0.4.1] - Previous Release

### Added

- Initial release with basic Apollo client functionality
- Support for Properties format configuration
- Basic caching mechanism
- WebAssembly support
- Grayscale release support (IP and label-based targeting)

### Changed

- Dependency updates and security patches

### Fixed

- Various bug fixes and stability improvements
