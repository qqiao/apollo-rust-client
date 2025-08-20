# 升級指南

本指南提供了在不同版本的 Apollo Rust Client 之間升級的詳細步驟說明。

## 目錄

- [從 v0.5.x 升級到 v0.6.0](#從-v05x-升級到-v060)
- [從 v0.4.x 升級到 v0.5.0](#從-v04x-升級到-v050)
- [從 v0.3.x 升級到 v0.4.0](#從-v03x-升級到-v040)

## 從 v0.5.x 升級到 v0.6.0

### ⚠️ 破壞性變更

這是一個**次要版本升級** (0.6.0)，由於 API 的重大破壞性變更。

#### 1. 命名空間獲取方法（最重要）

所有命名空間類型中的 `get_*` 方法現在都是**同步的**而不是異步的：

```rust
// 之前 (0.5.x) - 所有方法都是異步的
let app_name = properties.get_string("app.name").await;
let port = properties.get_int("server.port").await;
let debug = properties.get_bool("debug.enabled").await;
let custom = properties.get_property::<MyType>("config").await;

// 之後 (0.6.0) - 所有方法現在都是同步的
let app_name = properties.get_string("app.name");
let port = properties.get_int("server.port");
let debug = properties.get_bool("debug.enabled");
let custom = properties.get_property::<MyType>("config");
```

#### 2. JSON 和 YAML 反序列化

`to_object<T>()` 方法現在是同步的：

```rust
// 之前 (0.5.x) - 方法是異步的
let config: DatabaseConfig = json_namespace.to_object().await?;
let settings: ServerConfig = yaml_namespace.to_object().await?;

// 之後 (0.6.0) - 方法現在是同步的
let config: DatabaseConfig = json_namespace.to_object()?;
let settings: ServerConfig = yaml_namespace.to_object()?;
```

#### 3. API 變更

- `Client::new()` 構造函數參數從 `client_config` 重命名為 `config`
- `Client` 結構體字段從 `client_config` 重命名為 `config`

```rust
// 之前
let client = Client::new(client_config);

// 之後
let client = Client::new(config);
```

#### 4. 函數可見性變更

幾個內部函數現在是 `pub(crate)` 而不是公開的：

- `get_namespace()` 函數現在是 `pub(crate)`
- `Cache` 結構體現在是 `pub(crate)`

### 遷移步驟

1. **更新 Cargo.toml**：

   ```toml
   [dependencies]
   apollo-rust-client = "0.6.0"
   ```

2. **從所有命名空間獲取方法中移除 `.await`**：

   ```rust
   // 搜索並替換所有出現的內容：
   .get_string("key").await → .get_string("key")
   .get_int("key").await → .get_int("key")
   .get_bool("key").await → .get_bool("key")
   .get_property::<Type>("key").await → .get_property::<Type>("key")
   ```

3. **從反序列化方法中移除 `.await`**：

   ```rust
   // 搜索並替換所有出現的內容：
   .to_object().await → .to_object()
   ```

4. **更新構造函數調用**（如果使用舊的參數名）：

   ```rust
   // 搜索並替換：
   Client::new(client_config) → Client::new(config)
   ```

5. **移除內部類型的直接導入**：

   ```rust
   // 如果存在，移除這些導入：
   use apollo_rust_client::cache::Cache;
   use apollo_rust_client::namespace::get_namespace;

   // 使用公共 API 代替：
   use apollo_rust_client::{Client, ClientConfig};
   ```

### 遷移示例

```rust
// 之前 (0.5.x)
use apollo_rust_client::{Client, client_config::ClientConfig};

let client_config = ClientConfig::from_env()?;
let mut client = Client::new(client_config);

let namespace = client.namespace("application").await?;
match namespace {
    apollo_rust_client::namespace::Namespace::Properties(properties) => {
        let app_name = properties.get_string("app.name").await;
        let port = properties.get_int("server.port").await;
        // ... 更多代碼
    }
    apollo_rust_client::namespace::Namespace::Json(json) => {
        let config: DatabaseConfig = json.to_object().await?;
        // ... 更多代碼
    }
    _ => {}
}

// 之後 (0.6.0)
use apollo_rust_client::{Client, client_config::ClientConfig};

let config = ClientConfig::from_env()?;
let mut client = Client::new(config);

let namespace = client.namespace("application").await?;
match namespace {
    apollo_rust_client::namespace::Namespace::Properties(properties) => {
        let app_name = properties.get_string("app.name");
        let port = properties.get_int("server.port");
        // ... 更多代碼
    }
    apollo_rust_client::namespace::Namespace::Json(json) => {
        let config: DatabaseConfig = json.to_object()?;
        // ... 更多代碼
    }
    _ => {}
}
```

## 從 v0.4.x 升級到 v0.5.0

### 新增功能

- **多種配置格式**：支持 JSON、YAML 和文本格式
- **緩存 TTL**：緩存過期的時間到活機制
- **增強的命名空間系統**：統一的命名空間處理，支持格式檢測
- **改進的錯誤處理**：更好的錯誤類型和處理

### 新增配置字段

- `cache_ttl: Option<u64>` - 緩存過期時間（秒）（默認：600）
- `APOLLO_CACHE_TTL` 環境變量支持

### 遷移步驟

1. **更新 Cargo.toml**：

   ```toml
   [dependencies]
   apollo-rust-client = "0.5.0"
   ```

2. **添加緩存 TTL 配置**（可選）：

   ```rust
   let config = ClientConfig {
       app_id: "my-app".to_string(),
       config_server: "http://apollo-server:8080".to_string(),
       cluster: "default".to_string(),
       secret: None,
       cache_dir: None,
       label: None,
       ip: None,
       cache_ttl: Some(300), // 5 分鐘，或使用 APOLLO_CACHE_TTL 環境變量
   };
   ```

3. **無需代碼更改** - 所有新字段都是可選的且向後兼容

4. **環境變量支持**（可選）：
   ```bash
   export APOLLO_CACHE_TTL=300  # 5 分鐘
   ```

### 遷移示例

```rust
// 之前 (0.4.x)
let config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: None,
    ip: None,
};

// 之後 (0.5.0) - 添加 cache_ttl 字段
let config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: None,
    ip: None,
    cache_ttl: Some(600), // 10 分鐘默認值
};
```

## 從 v0.3.x 升級到 v0.4.0

### 破壞性變更

- `cache_dir` 字段類型從 `PathBuf` 改為 `Option<String>`

### 遷移步驟

1. **更新 Cargo.toml**：

   ```toml
   [dependencies]
   apollo-rust-client = "0.4.0"
   ```

2. **更新 cache_dir 字段類型**：

   ```rust
   // 舊版本 (0.3.x)
   use std::path::PathBuf;

   let config = ClientConfig {
       app_id: "my-app".to_string(),
       config_server: "http://apollo-server:8080".to_string(),
       cluster: "default".to_string(),
       secret: None,
       cache_dir: Some(PathBuf::from("/opt/cache")),
       label: None,
       ip: None,
   };

   // 新版本 (0.4.0+)
   let config = ClientConfig {
       app_id: "my-app".to_string(),
       config_server: "http://apollo-server:8080".to_string(),
       cluster: "default".to_string(),
       secret: None,
       cache_dir: Some("/opt/cache".to_string()),
       label: None,
       ip: None,
   };
   ```

## 通用升級技巧

### 1. 增量升級

建議進行增量升級：

- v0.3.x → v0.4.x → v0.5.x → v0.6.0

這允許您測試每個版本並及早發現問題。

### 2. 每次升級後測試

每次升級後：

1. 運行您的測試套件
2. 在暫存環境中測試
3. 驗證所有功能按預期工作

### 3. 依賴更新

某些升級可能需要更新其他依賴：

```bash
cargo update
cargo build
cargo test
```

### 4. 破壞性變更摘要

| 版本            | 破壞性變更               | 遷移工作量 |
| --------------- | ------------------------ | ---------- |
| v0.3.x → v0.4.0 | `cache_dir` 類型變更     | 低         |
| v0.4.x → v0.5.0 | 無（僅新功能）           | 無         |
| v0.5.x → v0.6.0 | 異步到同步方法，API 變更 | 高         |

### 5. 獲取幫助

如果在升級過程中遇到問題：

1. 查看 [CHANGELOG.md](../../../CHANGELOG.md) 了解詳細的變更信息
2. 查看 [錯誤處理指南](Error-Handling.md) 進行故障排除
3. 在 GitHub 上提出 issue，詳細描述您的具體問題

## 版本兼容性矩陣

| 功能             | v0.3.x | v0.4.x | v0.5.x | v0.6.0 |
| ---------------- | ------ | ------ | ------ | ------ |
| Properties 格式  | ✅     | ✅     | ✅     | ✅     |
| JSON 格式        | ❌     | ❌     | ✅     | ✅     |
| YAML 格式        | ❌     | ❌     | ✅     | ✅     |
| 文本格式         | ❌     | ❌     | ✅     | ✅     |
| 緩存 TTL         | ❌     | ❌     | ✅     | ✅     |
| 異步獲取方法     | ✅     | ✅     | ✅     | ❌     |
| 同步獲取方法     | ❌     | ❌     | ❌     | ✅     |
| 多種命名空間格式 | ❌     | ❌     | ✅     | ✅     |
