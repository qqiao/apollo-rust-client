[English](../en/Installation.md) | [中文繁體](../zh-TW/Installation.md)
[返回首页](Home.md)

# 安装

本指南涵盖了在不同平台和环境下安装和设置 Apollo Rust 客户端的详细步骤。

## 要求

### 系统要求

#### 原生 Rust

- **Rust 版本**: 1.85.0 或更高版本（Rust 2024 版本需要）
- **操作系统**: Linux, macOS, Windows
- **架构**: x86_64, ARM64
- **依赖项**: 标准 Rust 工具链

#### WebAssembly

- **Node.js**: 16.0.0 或更高版本（用于 Node.js 环境）
- **浏览器**: 支持 WebAssembly 的现代浏览器
- **构建工具**:用于从源码构建的 `wasm-pack`

### 网络要求

- **Apollo 服务器**: 访问 Apollo 配置中心
- **协议**: 支持 HTTP/HTTPS
- **端口**: 可配置（通常为 8080, 8090, 或 443）

## Rust 安装

### 使用 Cargo

将 Apollo Rust 客户端添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
apollo-rust-client = "0.7.0"
```

### 使用 Cargo Add

或者，您可以使用 `cargo add` 命令：

```bash
cargo add apollo-rust-client
```

### 版本规范

#### 最新版本

```toml
[dependencies]
apollo-rust-client = "0.7.0"
```

#### 版本范围

```toml
[dependencies]
apollo-rust-client = "^0.7.0"  # 兼容 0.7.x
```

#### 特定版本

```toml
[dependencies]
apollo-rust-client = "=0.7.0"  # 确切版本
```

### 功能标志

该库支持针对不同平台的条件编译：

```toml
[dependencies]
apollo-rust-client = { version = "0.7.0", features = ["native-tls"] }
```

可用功能：

- `native-tls`: 原生 Rust 的标准功能（默认）
- `rustls`: 原生 Rust 的 Rustls 支持（替代 native-tls）

## WebAssembly 安装

### NPM 包

通过 npm 安装 WebAssembly 包：

```bash
npm install @qqiao/apollo-rust-client
```

### Yarn 包管理器

```bash
yarn add @qqiao/apollo-rust-client
```

### 包信息

- **包名称**: `@qqiao/apollo-rust-client`
- **版本**: 0.7.0
- **注册表**: [NPM Registry](https://www.npmjs.com/package/@qqiao/apollo-rust-client)
- **包大小**: 针对浏览器环境进行了优化

### TypeScript 支持

该包包含 TypeScript 定义：

```typescript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

const config = new ClientConfig(
  "app-id",
  "http://apollo-server:8080",
  "default"
);
const client = new Client(config);
```

## 验证

### Rust 验证

创建一个简单的测试来验证安装：

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

运行测试：

```bash
cargo run
```

### WebAssembly 验证

创建一个简单的 JavaScript 测试：

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

    // 清理
    client.free();
    config.free();
  } catch (error) {
    console.error("Installation verification failed:", error);
  }
}

test();
```

## 从源码构建

###先决条件

#### Rust 工具链

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装所需的目标
rustup target add wasm32-unknown-unknown
```

#### WebAssembly 工具

```bash
# 安装 wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# 或者通过 cargo
cargo install wasm-pack
```

### 构建原生 Rust

```bash
# 克隆仓库
git clone https://github.com/qqiao/apollo-rust-client.git
cd apollo-rust-client

# 构建库
cargo build --release

# 运行测试
cargo test
```

### 构建 WebAssembly

```bash
# 为 Node.js 构建
wasm-pack build --target nodejs

# 为浏览器构建
wasm-pack build --target web

# 为打包工具（webpack 等）构建
wasm-pack build --target bundler
```

### 自定义构建配置

创建具有特定功能的自定义构建：

```bash
# 使用特定功能构建
cargo build --release --features "custom-feature"

# 为特定目标构建
cargo build --release --target wasm32-unknown-unknown
```

## 平台特定设置

### Linux

#### Ubuntu/Debian

```bash
# 安装系统依赖项
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### CentOS/RHEL

```bash
# 安装系统依赖项
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### macOS

#### 使用 Homebrew

```bash
# 安装 Xcode 命令行工具
xcode-select --install

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Windows

#### 使用 Rustup

```powershell
# 从 https://rustup.rs/ 下载并运行 rustup-init.exe

# 安装 Visual Studio 构建工具或带有 C++ 支持的 Visual Studio
# 从 https://visualstudio.microsoft.com/downloads/ 下载
```

#### 使用 Chocolatey

```powershell
# 首先安装 Chocolatey，然后：
choco install rust

# 安装 Visual Studio 构建工具
choco install visualstudio2019buildtools
```

## 配置

### 环境变量

设置环境变量以便于配置：

```bash
# 必需变量
export APP_ID="your-app-id"
export APOLLO_CONFIG_SERVICE="http://your-apollo-server:8080"

# 可选变量
export IDC="default"
export APOLLO_ACCESS_KEY_SECRET="your-secret-key"
export APOLLO_LABEL="production,canary"
export APOLLO_CACHE_DIR="/opt/apollo/cache"
```

### 配置文件

为您的应用程序创建一个配置文件：

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

## 故障排除

### 常见问题

#### 网络连接

```bash
# 测试 Apollo 服务器连接
curl -v http://your-apollo-server:8080/configs/your-app-id/default/application
```

#### 权限问题

```bash
# 修复缓存目录权限
sudo mkdir -p /opt/apollo/cache
sudo chown -R $USER:$USER /opt/apollo/cache
```

#### 构建失败

##### 缺少依赖项

```bash
# Linux: 安装构建依赖项
sudo apt-get install build-essential pkg-config libssl-dev

# macOS: 安装 Xcode 命令行工具
xcode-select --install
```

##### WebAssembly 构建问题

```bash
# 更新 wasm-pack
cargo install wasm-pack --force

# 清理并重建
cargo clean
wasm-pack build --target nodejs
```

### 调试模式

启用调试日志以进行故障排除：

```rust
// 设置环境变量
std::env::set_var("RUST_LOG", "apollo_rust_client=debug");

// 初始化记录器
env_logger::init();
```

```javascript
// 在浏览器控制台中启用调试日志
localStorage.setItem("debug", "apollo-rust-client:*");
```

## 版本历史

### 当前版本: 0.7.0

#### 新功能

- 具有自动格式检测的类型化命名空间
- 用于实时配置更新的事件监听器
- 具有全面错误类型的增强错误处理
- 改进的 WebAssembly 支持和更好的内存管理

#### 破坏性变更

- 命名空间 API 现在返回类型化枚举而不是原始缓存
- 事件监听器 API 已更改以提高类型安全性
- 为了清晰起见，重命名了一些配置选项

### 以前的版本

#### 0.4.x

- 基本命名空间支持
- 基于文件的缓存
- WebAssembly 支持

#### 0.3.x

- 初始发布
- 基本 Apollo 客户端功能
- Properties 格式支持

## 下一步

安装后，请继续：

1. **[配置](Configuration.md)** - 设置您的客户端配置
2. **[Rust 用法](Rust-Usage.md)** - 了解如何在 Rust 中使用该库
3. **[JavaScript 用法](JavaScript-Usage.md)** - 了解如何在 JavaScript 中使用该库
4. **[功能特性](Features.md)** - 探索所有可用功能

## 支持

如果在安装过程中遇到问题：

1. 检查 [GitHub Issues](https://github.com/qqiao/apollo-rust-client/issues)
2. 查看 [文档](https://docs.rs/apollo-rust-client)
3. 加入社区讨论
4. 如果需要，提交错误报告
