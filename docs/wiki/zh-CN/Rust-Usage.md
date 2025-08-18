[English](../en/Rust-Usage.md) | [中文繁體](../zh-TW/Rust-Usage.md)
[返回首页](Home.md)

## Rust 用法

以下示例演示了如何在 Rust 应用程序中初始化 `ClientConfig`、启动客户端、获取命名空间以及检索配置属性。

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig {
        app_id: "your_app_id".to_string(), // 你的应用ID
        cluster: "default".to_string(), // 集群名称
        secret: Some("your_apollo_secret".to_string()), // 你的 Apollo 密钥
        config_server: "http://your-apollo-server:8080".to_string(), // 配置服务器地址
        cache_dir: None, // 本地缓存目录
        label: None, // 实例标签
        ip: None, // 应用IP地址
        allow_insecure_https: None, // 是否允许不安全的HTTPS连接
    };
    let mut client = Client::new(client_config);
    // 启动后台轮询以获取配置更新。
    client.start().await?;

    // 获取 "application" 命名空间的配置。
    let cache = client.namespace("application").await;

    // 示例：检索字符串属性。
    match cache.get_property::<String>("some_key").await {
        Some(value) => println!("属性 'some_key': {}", value),
        None => println!("未找到属性 'some_key'"),
    }

    // 示例：检索整数属性。
    match cache.get_property::<i64>("meaningOfLife").await {
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
