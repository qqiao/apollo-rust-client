# Changelog

All notable changes to the apollo-rust-client project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2025-07-29

### Added

- **Multiple Configuration Format Support**: Added comprehensive support for different configuration formats:
  - **JSON Format**: Full support for structured JSON data with automatic detection via `.json` file extensions
  - **YAML Format**: Complete YAML configuration support with automatic detection via `.yaml` or `.yml` extensions
  - **Text Format**: Plain text configuration support with automatic detection via `.txt` extensions
  - **Properties Format**: Enhanced existing properties format support (default when no extension is specified)
- **Automatic Format Detection**: Intelligent namespace format detection based on file extensions in namespace names
- **Cache TTL Support**: Added Time-To-Live (TTL) mechanism for cache expiration (native targets only):
  - New `cache_ttl` field in `ClientConfig` struct (native targets only)
  - Default TTL of 600 seconds (10 minutes) when using `from_env()`
  - Configurable via `APOLLO_CACHE_TTL` environment variable (native targets only)
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
  - Added `cache_ttl` field documentation in all code examples (native targets only)
  - Updated README files to include `APOLLO_CACHE_TTL` environment variable (native targets only)
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
