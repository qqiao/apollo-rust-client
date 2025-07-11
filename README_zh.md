# apollo-rust-client

一个强大的 Apollo 配置中心 Rust 客户端，支持 WebAssembly，可用于浏览器和 Node.js 环境。

[![Crates.io](https://img.shields.io/crates/v/apollo-rust-client.svg)](https://crates.io/crates/apollo-rust-client)
[![Docs.rs](https://docs.rs/apollo-rust-client/badge.svg)](https://docs.rs/apollo-rust-client)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust CI](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml/badge.svg)](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml)

## 功能特性

- 支持多个命名空间
- 类型安全的配置管理
- 支持 Async/await
- 详细的错误处理和诊断
- 通过后台轮询实现实时配置更新（需要通过 `client.start()` 手动启动）。
- 灵活的客户端配置，包括直接实例化或从环境变量（Rust）初始化。

## 安装

### Rust

将以下内容添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
apollo-rust-client = "0.5.0" # 请确认并使用最新版本
```

或者，您可以使用 `cargo add`：

```bash
cargo add apollo-rust-client
```

### WebAssembly (npm)

```bash
npm install @qqiao/apollo-rust-client
```

## 使用方法

### Rust 用法

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig {
        app_id: "your_app_id".to_string(),
        cluster: "default".to_string(),
        secret: Some("your_apollo_secret".to_string()),
        config_server: "http://your-apollo-server:8080".to_string(),
        cache_dir: None,
        label: None,
        ip: None,
    };
    let mut client = Client::new(client_config);

    // 启动后台轮询以获取配置更新
    client.start().await?;

    // 获取 "application" 命名空间的配置
    let namespace = client.namespace("application").await?;

    // 命名空间现在根据从命名空间名称检测到的格式进行类型化
    match namespace {
        apollo_rust_client::namespace::Namespace::Properties(properties) => {
            // 示例：检索字符串属性
            match properties.get_string("some_key").await {
                Some(value) => println!("属性 'some_key': {}", value),
                None => println!("属性 'some_key' 未找到"),
            }

            // 示例：检索整数属性
            match properties.get_int("meaningOfLife").await {
                Some(value) => println!("属性 'meaningOfLife': {}", value),
                None => println!("属性 'meaningOfLife' 未找到"),
            }
        }
        apollo_rust_client::namespace::Namespace::Json(json) => {
            // 对于 JSON 命名空间，您可以反序列化为自定义类型
            // let config: MyConfig = json.to_object()?;
            println!("收到 JSON 命名空间");
        }
        apollo_rust_client::namespace::Namespace::Text(text) => {
            println!("文本命名空间内容: {}", text);
        }
    }

    Ok(())
}
```

#### 通过环境变量配置 (Rust)

`ClientConfig` 也可以使用 `ClientConfig::from_env()` 从环境变量初始化。这对于服务器端应用程序非常有用，因为配置通常通过环境传递。识别以下变量：

- `APP_ID`: 您的应用 ID。
- `APOLLO_ACCESS_KEY_SECRET`: 您命名空间的密钥（如果适用）。
- `IDC`: 集群名称（如果未设置，则默认为 "default"）。
- `APOLLO_CONFIG_SERVICE`: Apollo 配置服务的 URL。
- `APOLLO_LABEL`: 用于灰度规则的以逗号分隔的标签列表。
- `APOLLO_CACHE_DIR`: 用于存储本地缓存的目录。

注意：`ip` 字段不会通过 `from_env()` 设置。

### WebAssembly (JavaScript) 用法

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  // 基本配置：app_id、config_server URL、集群名称
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // 如果需要，可以直接在实例上设置可选属性
  clientConfig.secret = "your_apollo_secret"; // 示例：如果您的 Apollo 命名空间需要密钥
  // clientConfig.label = "your_instance_label"; // 用于灰度发布
  // clientConfig.ip = "client_ip_address"; // 用于灰度发布
  // clientConfig.cache_dir = "/custom/cache/path"; // 注意：cache_dir 在浏览器环境中不太常用

  const client = new Client(clientConfig);

  // 可选：启动后台轮询以获取配置更新。
  // 这是一个非阻塞操作。
  await client.start();

  // 获取 "application" 命名空间的配置
  const namespace = await client.namespace("application");

  // 示例：检索字符串属性
  const stringVal = await namespace.get_string("some_key");
  if (stringVal !== undefined) {
    console.log("属性 'some_key':", stringVal);
  } else {
    console.log("未找到属性 'some_key'");
  }

  // 示例：使用 get_int 检索整数属性
  const intVal = await namespace.get_int("meaningOfLife");
  if (intVal !== undefined) {
    console.log("属性 'meaningOfLife':", intVal);
  } else {
    console.log("未找到属性 'meaningOfLife' 或该属性不是整数。");
  }

  // 重要提示：当不再需要 WASM 对象时，请释放 Rust 内存
  namespace.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```

如果需要，可以在构造 `clientConfig` 实例后直接设置 `secret`、`label`、`ip` 和 `cache_dir` 等属性。由于文件系统限制，`cache_dir` 通常不在浏览器环境中使用。

#### WASM 中的内存管理

为防止 WebAssembly 中的内存泄漏，请在不再使用 `ClientConfig`、`Client` 和 `Cache` 实例后，显式调用它们的 `free()` 方法。这将释放在 WebAssembly 堆上由 Rust 分配的内存。

## 配置

客户端支持以下配置选项：

- `app_id`: 您在 Apollo 中的应用 ID。
- `cluster`: 集群名称 (默认为 "`default`")。
- `secret`: 给定 `app_id` 的可选密钥。
- `config_server`: 配置服务器的地址。
- `cache_dir`: 用于存储本地缓存的目录。对于非 WASM 目标（本机 Rust 应用程序），如果未指定，则默认为构造为 `/opt/data/{app_id}/config-cache` 的路径。对于 WASM 目标，此字段是可选的，默认为 `None`；文件系统缓存在浏览器环境中通常不适用或不使用。
- `label`: 当前实例的标签。用于在灰度发布中标识当前实例。
- `ip`: 您应用程序的 IP 地址。用于在灰度发布中标识当前实例。

## 错误处理

大多数客户端操作在 Rust 中返回 `Result<T, apollo_rust_client::Error>`，或在 JavaScript 中返回一个可能因错误而拒绝的 Promise。常见错误包括配置获取失败（例如，网络问题、Apollo 服务器错误）或本地缓存问题。确保您的应用程序代码包含适当的错误处理逻辑（例如，在 Rust 中使用 `match` 或 `?`，或在 JavaScript 中使用 Promise 的 `.catch()`）。

## v0.5.0 主要 API 变更

- **类型化命名空间**: 库现在支持多种配置格式（Properties、JSON、Text），并基于命名空间名称自动检测格式
- **事件监听器**: 添加了事件监听器支持以响应配置更改
- **改进的错误处理**: 增强的错误类型和更好的错误报告
- **WASM 改进**: 为 WebAssembly 目标提供更好的内存管理和 JavaScript 互操作

## 命名空间格式检测

库根据命名空间名称自动检测配置格式：

- 无扩展名或 `.properties` → Properties 格式（键值对）
- `.json` → JSON 格式（结构化数据）
- `.txt` → 文本格式（纯文本内容）
- `.yaml` 或 `.yml` → YAML 格式（计划中）
- `.xml` → XML 格式（计划中）

## TODO

- 更新 `Cargo.toml` 以实现更好的条件编译。尽管目前一切正常，但由于所有依赖项都是扁平列出的，自动化和 IDE 的速度明显减慢。
- 添加对其他命名空间格式的支持，例如 YAML、JSON 等。

## 贡献

欢迎贡献！请随时提交拉取请求。

## 许可证

本项目根据 Apache 许可证版本 2.0 进行许可 - 有关详细信息，请参阅 [LICENSE](LICENSE) 文件。
