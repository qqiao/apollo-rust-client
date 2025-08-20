# apollo-rust-client

一个强大的 Apollo 配置中心 Rust 客户端，支持 WebAssembly，可用于浏览器和 Node.js 环境。

[![Crates.io](https://img.shields.io/crates/v/apollo-rust-client.svg)](https://crates.io/crates/apollo-rust-client)
[![Docs.rs](https://docs.rs/apollo-rust-client/badge.svg)](https://docs.rs/apollo-rust-client)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust CI](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml/badge.svg)](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml)

## 功能特性

- **多种配置格式**: 支持 Properties、JSON、YAML、文本格式，计划支持 XML
- **自动格式检测**: 基于命名空间文件扩展名自动检测
- **类型安全的配置管理**: 编译时保证和运行时类型转换
- **跨平台支持**: 原生 Rust 和 WebAssembly 目标
- **实时更新**: 可配置间隔的后台轮询和事件监听器
- **全面缓存**: 多级缓存，支持文件持久化（原生）和内存缓存（WASM）
- **Async/Await 支持**: 完整的异步 API，实现非阻塞操作
- **错误处理**: 详细的错误诊断和全面的错误类型
- **灰度发布支持**: 基于 IP 和标签的配置定向
- **灵活配置**: 直接实例化或环境变量配置
- **内存管理**: 自动清理，WASM 环境显式控制

## 安装

### Rust

将以下内容添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
apollo-rust-client = "0.6.0"
```

或者，您可以使用 `cargo add`：

```bash
cargo add apollo-rust-client
```

### WebAssembly (npm)

```bash
npm install @qqiao/apollo-rust-client
```

## 快速开始

### Rust 用法

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建客户端配置
    let client_config = ClientConfig {
        app_id: "your_app_id".to_string(),
        cluster: "default".to_string(),
        config_server: "http://your-apollo-server:8080".to_string(),
        secret: Some("your_apollo_secret".to_string()),
        cache_dir: None, // 使用默认值: /opt/data/{app_id}/config-cache
        label: None,     // 用于灰度发布
        ip: None,        // 用于灰度发布
        #[cfg(not(target_arch = "wasm32"))]
cache_ttl: None, // 使用默认值: 600 秒（10 分钟）
    };

    let mut client = Client::new(client_config);

    // 启动后台轮询配置更新
    client.start().await?;

    // 获取不同命名空间格式的配置
    let namespace = client.namespace("application").await?;

    // 处理不同的配置格式
    match namespace {
        apollo_rust_client::namespace::Namespace::Properties(properties) => {
            // Properties 格式（默认）- 键值对
            if let Some(app_name) = properties.get_string("app.name") {
                println!("应用名称: {}", app_name);
            }

            if let Some(port) = properties.get_int("server.port") {
                println!("服务器端口: {}", port);
            }

            if let Some(debug) = properties.get_bool("debug.enabled") {
                println!("调试启用: {}", debug);
            }
        }
        apollo_rust_client::namespace::Namespace::Json(json) => {
            // JSON 格式 - 结构化数据
            #[derive(serde::Deserialize)]
            struct Config {
                name: String,
                version: String,
            }

            let config: Config = json.to_object()?;
            println!("配置: {} v{}", config.name, config.version);
        }
        apollo_rust_client::namespace::Namespace::Text(text) => {
            // 文本格式 - 纯文本内容
            println!("文本内容: {}", text);
        }
    }

    // 添加配置变更事件监听器
    client.add_listener("application", std::sync::Arc::new(|result| {
        match result {
            Ok(namespace) => println!("配置已更新: {:?}", namespace),
            Err(e) => eprintln!("配置更新错误: {}", e),
        }
    })).await;

    Ok(())
}
```

#### 通过环境变量配置

```rust
use apollo_rust_client::client_config::ClientConfig;

// 从环境变量初始化
let client_config = ClientConfig::from_env()?;
```

**环境变量:**

- `APP_ID`: 您的应用 ID（必需）
- `APOLLO_CONFIG_SERVICE`: Apollo 配置服务器 URL（必需）
- `IDC`: 集群名称（可选，默认为 "default"）
- `APOLLO_ACCESS_KEY_SECRET`: 认证密钥（可选）
- `APOLLO_LABEL`: 灰度规则的标签列表，用逗号分隔（可选）
- `APOLLO_CACHE_DIR`: 本地缓存目录（可选）
- `APOLLO_CACHE_TTL`: 缓存生存时间，以秒为单位（可选，默认为 600，仅限原生目标）

### JavaScript/WebAssembly 用法

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  // 创建客户端配置
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // 设置可选属性
  clientConfig.secret = "your_apollo_secret";
  clientConfig.label = "production";
  clientConfig.ip = "192.168.1.100";

  const client = new Client(clientConfig);

  // 启动后台轮询
  await client.start();

  // 获取配置缓存
  const cache = await client.namespace("application");

  // 检索不同数据类型
  const appName = cache.get_string("app.name");
  const serverPort = cache.get_int("server.port");
  const debugEnabled = cache.get_bool("debug.enabled");
  const timeout = cache.get_float("timeout.seconds");

  console.log(`应用: ${appName}, 端口: ${serverPort}, 调试: ${debugEnabled}`);

  // 添加配置变更事件监听器
  await cache.add_listener((data, error) => {
    if (error) {
      console.error("配置更新错误:", error);
    } else {
      console.log("配置已更新:", data);
    }
  });

  // 重要：使用完毕后释放内存
  cache.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```

## 配置格式

库根据命名空间名称自动检测配置格式：

### Properties 格式（默认）

- **命名空间**: `"application"`, `"config.properties"`
- **格式**: 键值对
- **示例**: `{"app.name": "MyApp", "server.port": "8080"}`

### JSON 格式

- **命名空间**: `"config.json"`, `"settings.json"`
- **格式**: 结构化 JSON 数据
- **示例**: `{"database": {"host": "localhost", "port": 5432}}`

### 文本格式

- **命名空间**: `"readme.txt"`, `"content.txt"`
- **格式**: 纯文本内容
- **示例**: 原始文本内容

### YAML 格式

- **命名空间**: `"config.yaml"`, `"config.yml"`
- **格式**: 结构化 YAML 数据
- **示例**: 复杂配置结构和嵌套对象

```rust
#[derive(serde::Deserialize)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
}

#[derive(serde::Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(serde::Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    ssl: bool,
}

if let apollo_rust_client::namespace::Namespace::Yaml(yaml) = namespace {
    let config: AppConfig = yaml.to_object()?;
    println!("服务器: {}:{}", config.server.host, config.server.port);
    println!("数据库: {}:{}", config.database.host, config.database.port);
}
```

### 计划中的格式

- **XML**: `"config.xml"`

## 配置选项

### ClientConfig 字段

- **`app_id`**: 您在 Apollo 中的应用 ID（必需）
- **`config_server`**: Apollo 配置服务器 URL（必需）
- **`cluster`**: 集群名称（必需，通常为 "default"）
- **`secret`**: 认证的可选密钥
- **`cache_dir`**: 本地缓存文件目录（仅原生）
  - 默认值: `/opt/data/{app_id}/config-cache`
  - WASM: 始终为 `None`（仅内存缓存）
- **`label`**: 灰度发布的标签（可选）
- **`ip`**: 灰度发布的 IP 地址（可选）

## 错误处理

库提供全面的错误处理：

```rust
use apollo_rust_client::Error;

match client.namespace("application").await {
    Ok(namespace) => {
        // 处理成功的配置检索
    }
    Err(Error::Cache(cache_error)) => {
        // 处理缓存相关错误（网络、解析等）
        eprintln!("缓存错误: {}", cache_error);
    }
    Err(Error::Namespace(namespace_error)) => {
        // 处理命名空间相关错误（格式检测等）
        eprintln!("命名空间错误: {}", namespace_error);
    }
    Err(e) => {
        // 处理其他错误
        eprintln!("错误: {}", e);
    }
}
```

## 内存管理（WASM）

对于 WebAssembly 环境，需要显式的内存管理：

```javascript
// 使用完毕后始终对 WASM 对象调用 free()
cache.free();
client.free();
clientConfig.free();
```

这通过释放 WebAssembly 堆上的 Rust 分配内存来防止内存泄漏。

## 高级用法

### 事件监听器

```rust
// Rust - 添加事件监听器
client.add_listener("application", std::sync::Arc::new(|result| {
    match result {
        Ok(namespace) => println!("配置已更新: {:?}", namespace),
        Err(e) => eprintln!("更新错误: {}", e),
    }
})).await;
```

```javascript
// JavaScript - 添加事件监听器
await cache.add_listener((data, error) => {
  if (error) {
    console.error("更新错误:", error);
  } else {
    console.log("配置已更新:", data);
  }
});
```

### 灰度发布

```rust
let client_config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: Some("canary,beta".to_string()),  // 多个标签
    ip: Some("192.168.1.100".to_string()),  // 客户端 IP
    #[cfg(not(target_arch = "wasm32"))]
    cache_ttl: None,
};
```

### 自定义 JSON 反序列化

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    ssl: bool,
}

if let apollo_rust_client::namespace::Namespace::Json(json) = namespace {
    let db_config: DatabaseConfig = json.to_object()?;
    println!("数据库: {}:{}", db_config.host, db_config.port);
}
```

## v0.6.0 API 变更

- **类型化命名空间**: 支持多种配置格式和自动检测
- **事件监听器**: 实时配置变更通知
- **增强的错误处理**: 全面的错误类型和更好的错误报告
- **WASM 改进**: 更好的内存管理和 JavaScript 互操作
- **后台轮询**: 可配置的自动配置刷新

## 文档

有关完整文档，请访问我们的 [wiki](docs/wiki/zh-CN/Home.md)：

- **[安装指南](docs/wiki/zh-CN/Installation.md)** - 安装说明
- **[Rust 用法](docs/wiki/zh-CN/Rust-Usage.md)** - 原生 Rust 示例
- **[JavaScript 用法](docs/wiki/zh-CN/JavaScript-Usage.md)** - JavaScript 中的 WASM 使用
- **[配置](docs/wiki/zh-CN/Configuration.md)** - 配置选项
- **[功能特性](docs/wiki/zh-CN/Features.md)** - 功能概述
- **[错误处理](docs/wiki/zh-CN/Error-Handling.md)** - 错误处理指南
- **[设计概览](docs/wiki/zh-CN/Design-Overview.md)** - 架构文档

## 贡献

欢迎贡献！请随时提交拉取请求。

## 许可证

本项目根据 Apache 许可证版本 2.0 进行许可 - 有关详细信息，请参阅 [LICENSE](LICENSE) 文件。
