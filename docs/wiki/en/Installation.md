[中文简体](../zh-CN/Installation.md) | [中文繁體](../zh-TW/Installation.md)
[Back to Home](Home.md)

# Installation

This guide covers installation and setup for the Apollo Rust Client across different platforms and environments.

## Requirements

### System Requirements

#### Native Rust

- **Rust Version**: 1.70.0 or later
- **Operating Systems**: Linux, macOS, Windows
- **Architecture**: x86_64, ARM64
- **Dependencies**: Standard Rust toolchain

#### WebAssembly

- **Node.js**: 16.0.0 or later (for Node.js environments)
- **Browsers**: Modern browsers with WebAssembly support
- **Build Tools**: `wasm-pack` for building from source

### Network Requirements

- **Apollo Server**: Access to Apollo Configuration Center
- **Protocols**: HTTP/HTTPS support
- **Ports**: Configurable (typically 8080, 8090, or 443)

## Rust Installation

### Using Cargo

Add the Apollo Rust Client to your `Cargo.toml`:

```toml
[dependencies]
apollo-rust-client = "0.6.3"
```

### Using Cargo Add

Alternatively, use the `cargo add` command:

```bash
cargo add apollo-rust-client
```

### Version Specification

#### Latest Version

```toml
[dependencies]
apollo-rust-client = "0.6.2"
```

#### Version Range

```toml
[dependencies]
apollo-rust-client = "^0.6.3"  # Compatible with 0.6.x
```

#### Specific Version

```toml
[dependencies]
apollo-rust-client = "=0.6.3"  # Exact version
```

### Feature Flags

The library supports conditional compilation for different platforms:

```toml
[dependencies]
apollo-rust-client = { version = "0.6.3", features = ["default"] }
```

Available features:

- `default`: Standard features for native Rust
- `wasm`: WebAssembly-specific optimizations (automatically enabled for wasm32 targets)

## WebAssembly Installation

### NPM Package

Install the WebAssembly package via npm:

```bash
npm install @qqiao/apollo-rust-client
```

### Yarn Package Manager

```bash
yarn add @qqiao/apollo-rust-client
```

### Package Information

- **Package Name**: `@qqiao/apollo-rust-client`
- **Version**: 0.6.3
- **Registry**: [NPM Registry](https://www.npmjs.com/package/@qqiao/apollo-rust-client)
- **Bundle Size**: Optimized for browser environments

### TypeScript Support

The package includes TypeScript definitions:

```typescript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

const config = new ClientConfig(
  "app-id",
  "http://apollo-server:8080",
  "default"
);
const client = new Client(config);
```

## Verification

### Rust Verification

Create a simple test to verify the installation:

```rust
use apollo_rust_client::{Client, client_config::ClientConfig};

fn main() {
    let config = ClientConfig {
        app_id: "test-app".to_string(),
        config_server: "http://localhost:8080".to_string(),
        cluster: "default".to_string(),
        secret: None,
        cache_dir: None,
        label: None,
        ip: None,
    };

    let client = Client::new(config);
    println!("Apollo Rust Client installed successfully!");
}
```

Run the test:

```bash
cargo run
```

### WebAssembly Verification

Create a simple JavaScript test:

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function test() {
  try {
    const config = new ClientConfig(
      "test-app",
      "http://localhost:8080",
      "default"
    );
    const client = new Client(config);

    console.log("Apollo Rust Client (WASM) installed successfully!");

    // Clean up
    client.free();
    config.free();
  } catch (error) {
    console.error("Installation verification failed:", error);
  }
}

test();
```

## Building from Source

### Prerequisites

#### Rust Toolchain

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install required targets
rustup target add wasm32-unknown-unknown
```

#### WebAssembly Tools

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Or via cargo
cargo install wasm-pack
```

### Building for Native Rust

```bash
# Clone the repository
git clone https://github.com/qqiao/apollo-rust-client.git
cd apollo-rust-client

# Build the library
cargo build --release

# Run tests
cargo test
```

### Building for WebAssembly

```bash
# Build for Node.js
wasm-pack build --target nodejs

# Build for browsers
wasm-pack build --target web

# Build for bundlers (webpack, etc.)
wasm-pack build --target bundler
```

### Custom Build Configuration

Create a custom build with specific features:

```bash
# Build with specific features
cargo build --release --features "custom-feature"

# Build for specific target
cargo build --release --target wasm32-unknown-unknown
```

## Platform-Specific Setup

### Linux

#### Ubuntu/Debian

```bash
# Install system dependencies
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### CentOS/RHEL

```bash
# Install system dependencies
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### macOS

#### Using Homebrew

```bash
# Install Xcode command line tools
xcode-select --install

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Windows

#### Using Rustup

```powershell
# Download and run rustup-init.exe from https://rustup.rs/

# Install Visual Studio Build Tools or Visual Studio with C++ support
# Download from https://visualstudio.microsoft.com/downloads/
```

#### Using Chocolatey

```powershell
# Install Chocolatey first, then:
choco install rust

# Install Visual Studio Build Tools
choco install visualstudio2019buildtools
```

## Configuration

### Environment Variables

Set up environment variables for easier configuration:

```bash
# Required variables
export APP_ID="your-app-id"
export APOLLO_CONFIG_SERVICE="http://your-apollo-server:8080"

# Optional variables
export IDC="default"
export APOLLO_ACCESS_KEY_SECRET="your-secret-key"
export APOLLO_LABEL="production,canary"
export APOLLO_CACHE_DIR="/opt/apollo/cache"
```

### Configuration Files

Create a configuration file for your application:

```toml
# apollo-config.toml
[apollo]
app_id = "your-app-id"
config_server = "http://your-apollo-server:8080"
cluster = "default"
secret = "your-secret-key"
cache_dir = "/opt/apollo/cache"
label = "production,canary"
ip = "192.168.1.100"
```

## Troubleshooting

### Common Issues

#### Network Connectivity

```bash
# Test Apollo server connectivity
curl -v http://your-apollo-server:8080/configs/your-app-id/default/application
```

#### Permission Issues

```bash
# Fix cache directory permissions
sudo mkdir -p /opt/apollo/cache
sudo chown -R $USER:$USER /opt/apollo/cache
```

#### Build Failures

##### Missing Dependencies

```bash
# Linux: Install build dependencies
sudo apt-get install build-essential pkg-config libssl-dev

# macOS: Install Xcode command line tools
xcode-select --install
```

##### WebAssembly Build Issues

```bash
# Update wasm-pack
cargo install wasm-pack --force

# Clean and rebuild
cargo clean
wasm-pack build --target nodejs
```

### Debug Mode

Enable debug logging for troubleshooting:

```rust
// Set environment variable
std::env::set_var("RUST_LOG", "apollo_rust_client=debug");

// Initialize logger
env_logger::init();
```

```javascript
// Enable debug logging in browser console
localStorage.setItem("debug", "apollo-rust-client:*");
```

## Version History

### Current Version: 0.6.3

#### New Features

- Typed namespaces with automatic format detection
- Event listeners for real-time configuration updates
- Enhanced error handling with comprehensive error types
- Improved WebAssembly support with better memory management

#### Breaking Changes

- Namespace API now returns typed enum instead of raw cache
- Event listener API has changed for better type safety
- Some configuration options have been renamed for clarity

### Previous Versions

#### 0.4.x

- Basic namespace support
- File-based caching
- WebAssembly support

#### 0.3.x

- Initial release
- Basic Apollo client functionality
- Properties format support

## Next Steps

After installation, proceed to:

1. **[Configuration](Configuration.md)** - Set up your client configuration
2. **[Rust Usage](Rust-Usage.md)** - Learn how to use the library in Rust
3. **[JavaScript Usage](JavaScript-Usage.md)** - Learn how to use the library in JavaScript
4. **[Features](Features.md)** - Explore all available features

## Support

If you encounter issues during installation:

1. Check the [GitHub Issues](https://github.com/qqiao/apollo-rust-client/issues)
2. Review the [Documentation](https://docs.rs/apollo-rust-client)
3. Join the community discussions
4. Submit a bug report if needed
