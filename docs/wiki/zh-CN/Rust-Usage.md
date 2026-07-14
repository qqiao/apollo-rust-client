[English](../en/Rust-Usage.md) | [中文繁體](../zh-TW/Rust-Usage.md)
[返回首页](Home.md)

## Rust 用法

以下示例演示了如何在 Rust 应用程序中初始化 `ClientConfig`、启动客户端、获取命名空间以及检索配置属性。

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig::builder(
        "your_app_id", "http://your-apollo-server:8080"
    ).secret("your_apollo_secret").build()?;
    let mut client = Client::new(client_config)?;
    // 启动后台轮询以获取配置更新。
    client.start().await?;

    // 获取 "application" 命名空间的配置。
    let namespace = client.namespace("application").await?;

    // 示例：检索字符串属性。
    if let apollo_rust_client::namespace::Namespace::Properties(properties) = namespace {
        println!("属性: {:?}", properties.get_string("some_key"));
    }

    // 示例：检索整数属性。
    match cache.get_property::<i64>("meaningOfLife") {
        Some(value) => println!("属性 'meaningOfLife': {}", value),
        None => println!("未找到属性 'meaningOfLife'"),
    }
    Ok(())
}
```

### 通过环境变量配置 (Rust)

`ClientConfig` 也可以使用 `ClientConfig::from_env()` 从环境变量初始化。这对于服务器端应用程序非常有用，因为配置通常通过环境传递。识别以下变量：

- `APP_ID`: 您的应用 ID。
- `APOLLO_ACCESS_KEY_SECRET`: 您命名空间的密钥（如果适用）。
- `IDC`: 集群名称（如果未设置，则默认为 "default"）。
- `APOLLO_CONFIG_SERVICE`: Apollo 配置服务的 URL。
- `APOLLO_LABEL`: 用于灰度规则的以逗号分隔的标签列表。
- `APOLLO_CACHE_DIR`: 用于存储本地缓存的目录。
- `APOLLO_ALLOW_INSECURE_HTTPS`: 是否允许不安全的 HTTPS 连接（可选，默认为 false）。

注意：`ip` 字段不会通过 `from_env()` 设置。
