[English](../en/Installation.md) | [中文简体](../zh-CN/Installation.md)
[返回首頁](Home.md)

# 安裝

本指南涵蓋了在不同平台和環境下安裝和設置 Apollo Rust 客戶端的詳細步驟。

## 要求

### 系統要求

#### 原生 Rust

- **Rust 版本**: 1.85.0 或更高版本（Rust 2024 版本需要）
- **操作系統**: Linux, macOS, Windows
- **架構**: x86_64, ARM64
- **依賴項**: 標準 Rust 工具鏈

#### WebAssembly

- **Node.js**: 16.0.0 或更高版本（用於 Node.js 環境）
- **瀏覽器**: 支持 WebAssembly 的現代瀏覽器
- **構建工具**: 用於從源碼構建的 `wasm-pack`

### 網絡要求

- **Apollo 服務器**: 訪問 Apollo 配置中心
- **協議**: 支持 HTTP/HTTPS
- **端口**: 可配置（通常為 8080, 8090, 或 443）

## Rust 安裝

### 使用 Cargo

將 Apollo Rust 客戶端添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
apollo-rust-client = "0.7.0"
```

### 使用 Cargo Add

或者，您可以使用 `cargo add` 命令：

```bash
cargo add apollo-rust-client
```

### 版本規範

#### 最新版本

```toml
[dependencies]
apollo-rust-client = "0.7.0"
```

#### 版本範圍

```toml
[dependencies]
apollo-rust-client = "^0.7.0"  # 兼容 0.7.x
```

#### 特定版本

```toml
[dependencies]
apollo-rust-client = "=0.7.0"  # 確切版本
```

### 功能標誌

該庫支持針對不同平台的條件編譯：

```toml
[dependencies]
apollo-rust-client = { version = "0.7.0", features = ["native-tls"] }
```

可用功能：

- `native-tls`: 原生 Rust 的標準功能（默認）
- `rustls`: 原生 Rust 的 Rustls 支持（替代 native-tls）

## WebAssembly 安裝

### NPM 包

通過 npm 安裝 WebAssembly 包：

```bash
npm install @qqiao/apollo-rust-client
```

### Yarn 包管理器

```bash
yarn add @qqiao/apollo-rust-client
```

### 包信息

- **包名稱**: `@qqiao/apollo-rust-client`
- **版本**: 0.7.0
- **註冊表**: [NPM Registry](https://www.npmjs.com/package/@qqiao/apollo-rust-client)
- **包大小**: 針對瀏覽器環境進行了優化

### TypeScript 支持

該包包含 TypeScript 定義：

```typescript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

const config = new ClientConfig(
  "app-id",
  "http://apollo-server:8080",
  "default"
);
const client = new Client(config);
```

## 驗證

### Rust 驗證

創建一個簡單的測試來驗證安裝：

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

運行測試：

```bash
cargo run
```

### WebAssembly 驗證

創建一個簡單的 JavaScript 測試：

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

## 從源碼構建

### 先決條件

#### Rust 工具鏈

```bash
# 安裝 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安裝所需的目標
rustup target add wasm32-unknown-unknown
```

#### WebAssembly 工具

```bash
# 安裝 wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# 或者通過 cargo
cargo install wasm-pack
```

### 構建原生 Rust

```bash
# 克隆倉庫
git clone https://github.com/qqiao/apollo-rust-client.git
cd apollo-rust-client

# 構建庫
cargo build --release

# 運行測試
cargo test
```

### 構建 WebAssembly

```bash
# 為 Node.js 構建
wasm-pack build --target nodejs

# 為瀏覽器構建
wasm-pack build --target web

# 為打包工具（webpack 等）構建
wasm-pack build --target bundler
```

### 自定義構建配置

創建具有特定功能的自定義構建：

```bash
# 使用特定功能構建
cargo build --release --features "custom-feature"

# 為特定目標構建
cargo build --release --target wasm32-unknown-unknown
```

## 平台特定設置

### Linux

#### Ubuntu/Debian

```bash
# 安裝系統依賴項
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev

# 安裝 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### CentOS/RHEL

```bash
# 安裝系統依賴項
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# 安裝 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### macOS

#### 使用 Homebrew

```bash
# 安裝 Xcode 命令行工具
xcode-select --install

# 安裝 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Windows

#### 使用 Rustup

```powershell
# 從 https://rustup.rs/ 下載並運行 rustup-init.exe

# 安裝 Visual Studio 構建工具或帶有 C++ 支持的 Visual Studio
# 從 https://visualstudio.microsoft.com/downloads/ 下載
```

#### 使用 Chocolatey

```powershell
# 首先安裝 Chocolatey，然後：
choco install rust

# 安裝 Visual Studio 構建工具
choco install visualstudio2019buildtools
```

## 配置

### 環境變量

設置環境變量以便於配置：

```bash
# 必需變量
export APP_ID="your-app-id"
export APOLLO_CONFIG_SERVICE="http://your-apollo-server:8080"

# 可選變量
export IDC="default"
export APOLLO_ACCESS_KEY_SECRET="your-secret-key"
export APOLLO_LABEL="production,canary"
export APOLLO_CACHE_DIR="/opt/apollo/cache"
```

### 配置文件

為您的應用程序創建一個配置文件：

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

### 常見問題

#### 網絡連接

```bash
# 測試 Apollo 服務器連接
curl -v http://your-apollo-server:8080/configs/your-app-id/default/application
```

#### 權限問題

```bash
# 修復緩存目錄權限
sudo mkdir -p /opt/apollo/cache
sudo chown -R $USER:$USER /opt/apollo/cache
```

#### 構建失敗

##### 缺少依賴項

```bash
# Linux: 安裝構建依賴項
sudo apt-get install build-essential pkg-config libssl-dev

# macOS: 安裝 Xcode 命令行工具
xcode-select --install
```

##### WebAssembly 構建問題

```bash
# 更新 wasm-pack
cargo install wasm-pack --force

# 清理並重建
cargo clean
wasm-pack build --target nodejs
```

### 調試模式

啟用調試日誌以進行故障排除：

```rust
// 設置環境變量
std::env::set_var("RUST_LOG", "apollo_rust_client=debug");

// 初始化記錄器
env_logger::init();
```

```javascript
// 在瀏覽器控制台中啟用調試日誌
localStorage.setItem("debug", "apollo-rust-client:*");
```

## 版本歷史

### 當前版本: 0.7.0

#### 新功能

- 具有自動格式檢測的類型化命名空間
- 用於實時配置更新的事件監聽器
- 具有全面錯誤類型的增強錯誤處理
- 改進的 WebAssembly 支持和更好的內存管理

#### 破壞性變更

- 命名空間 API 現在返回類型化枚舉而不是原始緩存
- 事件監聽器 API 已更改以提高類型安全性
- 為了清晰起見，重命名了一些配置選項

### 以前的版本

#### 0.4.x

- 基本命名空間支持
- 基於文件的緩存
- WebAssembly 支持

#### 0.3.x

- 初始發布
- 基本 Apollo 客戶端功能
- Properties 格式支持

## 下一步

安裝後，請繼續：

1. **[配置](Configuration.md)** - 設置您的客戶端配置
2. **[Rust 用法](Rust-Usage.md)** - 了解如何在 Rust 中使用該庫
3. **[JavaScript 用法](JavaScript-Usage.md)** - 了解如何在 JavaScript 中使用該庫
4. **[功能特性](Features.md)** - 探索所有可用功能

## 支持

如果在安裝過程中遇到問題：

1. 檢查 [GitHub Issues](https://github.com/qqiao/apollo-rust-client/issues)
2. 查看 [文檔](https://docs.rs/apollo-rust-client)
3. 加入社區討論
4. 如果需要，提交錯誤報告
