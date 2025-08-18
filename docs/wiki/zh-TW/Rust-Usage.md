[English](../en/Rust-Usage.md) | [中文简体](../zh-CN/Rust-Usage.md)
[返回首頁](Home.md)

## Rust 用法

以下範例演示了如何在 Rust 應用程式中初始化 `ClientConfig`、啟動客戶端、獲取命名空間以及檢索設定屬性。

```rust
use apollo_rust_client::Client;
use apollo_rust_client::client_config::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig {
        app_id: "your_app_id".to_string(), // 您的應用ID
        cluster: "default".to_string(), // 叢集名稱
        secret: Some("your_apollo_secret".to_string()), // 您的 Apollo 金鑰
        config_server: "http://your-apollo-server:8080".to_string(), // 設定伺服器位址
        cache_dir: None, // 本地快取目錄
        label: None, // 實例標籤
        ip: None, // 應用IP位址
        allow_insecure_https: None, // 是否允許不安全的HTTPS連接
    };
    let mut client = Client::new(client_config);
    // 啟動後台輪詢以獲取設定更新。
    client.start().await?;

    // 獲取 "application" 命名空間的設定。
    let cache = client.namespace("application").await;

    // 範例：檢索字串屬性。
    match cache.get_property::<String>("some_key").await {
        Some(value) => println!("屬性 'some_key': {}", value),
        None => println!("未找到屬性 'some_key'"),
    }

    // 範例：檢索整數屬性。
    match cache.get_property::<i64>("meaningOfLife").await {
        Some(value) => println!("屬性 'meaningOfLife': {}", value),
        None => println!("未找到屬性 'meaningOfLife'"),
    }
    Ok(())
}
```

### 通過環境變數設定 (Rust)

`ClientConfig` 也可以使用 `ClientConfig::from_env()` 從環境變數初始化。這對於伺服器端應用程式非常有用，因為設定通常通過環境傳遞。識別以下變數：

- `APP_ID`: 您的應用 ID。
- `APOLLO_ACCESS_KEY_SECRET`: 您命名空間的金鑰（如果適用）。
- `IDC`: 叢集名稱（如果未設定，則預設為 "default"）。
- `APOLLO_CONFIG_SERVICE`: Apollo 設定服務的 URL。
- `APOLLO_LABEL`: 用于灰度規則的以逗號分隔的標籤列表。
- `APOLLO_CACHE_DIR`: 用于儲存本地快取的目錄。
- `APOLLO_ALLOW_INSECURE_HTTPS`: 是否允許不安全的 HTTPS 連接（可選，預設為 false）。

注意：`ip` 欄位不會通過 `from_env()` 設定。
