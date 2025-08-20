# 升级指南

本指南提供了在不同版本的 Apollo Rust Client 之间升级的详细步骤说明。

## 目录

- [从 v0.5.x 升级到 v0.6.0](#从-v05x-升级到-v060)
- [从 v0.4.x 升级到 v0.5.0](#从-v04x-升级到-v050)
- [从 v0.3.x 升级到 v0.4.0](#从-v03x-升级到-v040)

## 从 v0.5.x 升级到 v0.6.0

### ⚠️ 破坏性变更

这是一个**次要版本升级** (0.6.0)，由于 API 的重大破坏性变更。

#### 1. 命名空间获取方法（最重要）

所有命名空间类型中的 `get_*` 方法现在都是**同步的**而不是异步的：

```rust
// 之前 (0.5.x) - 所有方法都是异步的
let app_name = properties.get_string("app.name").await;
let port = properties.get_int("server.port").await;
let debug = properties.get_bool("debug.enabled").await;
let custom = properties.get_property::<MyType>("config").await;

// 之后 (0.6.0) - 所有方法现在都是同步的
let app_name = properties.get_string("app.name");
let port = properties.get_int("server.port");
let debug = properties.get_bool("debug.enabled");
let custom = properties.get_property::<MyType>("config");
```

#### 2. JSON 和 YAML 反序列化

`to_object<T>()` 方法现在是同步的：

```rust
// 之前 (0.5.x) - 方法是异步的
let config: DatabaseConfig = json_namespace.to_object().await?;
let settings: ServerConfig = yaml_namespace.to_object().await?;

// 之后 (0.6.0) - 方法现在是同步的
let config: DatabaseConfig = json_namespace.to_object()?;
let settings: ServerConfig = yaml_namespace.to_object()?;
```

#### 3. API 变更

- `Client::new()` 构造函数参数从 `client_config` 重命名为 `config`
- `Client` 结构体字段从 `client_config` 重命名为 `config`

```rust
// 之前
let client = Client::new(client_config);

// 之后
let client = Client::new(config);
```

#### 4. 函数可见性变更

几个内部函数现在是 `pub(crate)` 而不是公开的：

- `get_namespace()` 函数现在是 `pub(crate)`
- `Cache` 结构体现在是 `pub(crate)`

### 迁移步骤

1. **更新 Cargo.toml**：

   ```toml
   [dependencies]
   apollo-rust-client = "0.6.0"
   ```

2. **从所有命名空间获取方法中移除 `.await`**：

   ```rust
   // 搜索并替换所有出现的内容：
   .get_string("key").await → .get_string("key")
   .get_int("key").await → .get_int("key")
   .get_bool("key").await → .get_bool("key")
   .get_property::<Type>("key").await → .get_property::<Type>("key")
   ```

3. **从反序列化方法中移除 `.await`**：

   ```rust
   // 搜索并替换所有出现的内容：
   .to_object().await → .to_object()
   ```

4. **更新构造函数调用**（如果使用旧的参数名）：

   ```rust
   // 搜索并替换：
   Client::new(client_config) → Client::new(config)
   ```

5. **移除内部类型的直接导入**：

   ```rust
   // 如果存在，移除这些导入：
   use apollo_rust_client::cache::Cache;
   use apollo_rust_client::namespace::get_namespace;

   // 使用公共 API 代替：
   use apollo_rust_client::{Client, ClientConfig};
   ```

### 迁移示例

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
        // ... 更多代码
    }
    apollo_rust_client::namespace::Namespace::Json(json) => {
        let config: DatabaseConfig = json.to_object().await?;
        // ... 更多代码
    }
    _ => {}
}

// 之后 (0.6.0)
use apollo_rust_client::{Client, client_config::ClientConfig};

let config = ClientConfig::from_env()?;
let mut client = Client::new(config);

let namespace = client.namespace("application").await?;
match namespace {
    apollo_rust_client::namespace::Namespace::Properties(properties) => {
        let app_name = properties.get_string("app.name");
        let port = properties.get_int("server.port");
        // ... 更多代码
    }
    apollo_rust_client::namespace::Namespace::Json(json) => {
        let config: DatabaseConfig = json.to_object()?;
        // ... 更多代码
    }
    _ => {}
}
```

## 从 v0.4.x 升级到 v0.5.0

### 新增功能

- **多种配置格式**：支持 JSON、YAML 和文本格式
- **缓存 TTL**：缓存过期的时间到活机制
- **增强的命名空间系统**：统一的命名空间处理，支持格式检测
- **改进的错误处理**：更好的错误类型和处理

### 新增配置字段

- `cache_ttl: Option<u64>` - 缓存过期时间（秒）（默认：600）
- `APOLLO_CACHE_TTL` 环境变量支持

### 迁移步骤

1. **更新 Cargo.toml**：

   ```toml
   [dependencies]
   apollo-rust-client = "0.5.0"
   ```

2. **添加缓存 TTL 配置**（可选）：

   ```rust
   let config = ClientConfig {
       app_id: "my-app".to_string(),
       config_server: "http://apollo-server:8080".to_string(),
       cluster: "default".to_string(),
       secret: None,
       cache_dir: None,
       label: None,
       ip: None,
       cache_ttl: Some(300), // 5 分钟，或使用 APOLLO_CACHE_TTL 环境变量
   };
   ```

3. **无需代码更改** - 所有新字段都是可选的且向后兼容

4. **环境变量支持**（可选）：
   ```bash
   export APOLLO_CACHE_TTL=300  # 5 分钟
   ```

### 迁移示例

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

// 之后 (0.5.0) - 添加 cache_ttl 字段
let config = ClientConfig {
    app_id: "my-app".to_string(),
    config_server: "http://apollo-server:8080".to_string(),
    cluster: "default".to_string(),
    secret: None,
    cache_dir: None,
    label: None,
    ip: None,
    cache_ttl: Some(600), // 10 分钟默认值
};
```

## 从 v0.3.x 升级到 v0.4.0

### 破坏性变更

- `cache_dir` 字段类型从 `PathBuf` 改为 `Option<String>`

### 迁移步骤

1. **更新 Cargo.toml**：

   ```toml
   [dependencies]
   apollo-rust-client = "0.4.0"
   ```

2. **更新 cache_dir 字段类型**：

   ```rust
   // 旧版本 (0.3.x)
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

## 通用升级技巧

### 1. 增量升级

建议进行增量升级：

- v0.3.x → v0.4.x → v0.5.x → v0.6.0

这允许您测试每个版本并及早发现问题。

### 2. 每次升级后测试

每次升级后：

1. 运行您的测试套件
2. 在暂存环境中测试
3. 验证所有功能按预期工作

### 3. 依赖更新

某些升级可能需要更新其他依赖：

```bash
cargo update
cargo build
cargo test
```

### 4. 破坏性变更摘要

| 版本            | 破坏性变更               | 迁移工作量 |
| --------------- | ------------------------ | ---------- |
| v0.3.x → v0.4.0 | `cache_dir` 类型变更     | 低         |
| v0.4.x → v0.5.0 | 无（仅新功能）           | 无         |
| v0.5.x → v0.6.0 | 异步到同步方法，API 变更 | 高         |

### 5. 获取帮助

如果在升级过程中遇到问题：

1. 查看 [CHANGELOG.md](../../../CHANGELOG.md) 了解详细的变更信息
2. 查看 [错误处理指南](Error-Handling.md) 进行故障排除
3. 在 GitHub 上提出 issue，详细描述您的具体问题

## 版本兼容性矩阵

| 功能             | v0.3.x | v0.4.x | v0.5.x | v0.6.0 |
| ---------------- | ------ | ------ | ------ | ------ |
| Properties 格式  | ✅     | ✅     | ✅     | ✅     |
| JSON 格式        | ❌     | ❌     | ✅     | ✅     |
| YAML 格式        | ❌     | ❌     | ✅     | ✅     |
| 文本格式         | ❌     | ❌     | ✅     | ✅     |
| 缓存 TTL         | ❌     | ❌     | ✅     | ✅     |
| 异步获取方法     | ✅     | ✅     | ✅     | ❌     |
| 同步获取方法     | ❌     | ❌     | ❌     | ✅     |
| 多种命名空间格式 | ❌     | ❌     | ✅     | ✅     |
