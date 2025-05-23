[English](../en/Design-Cache.md) | [中文繁體](../zh-TW/Design-Cache.md)
[返回首页](Home.md)

# Cache 详情

`Cache` 结构体负责管理单个 Apollo 命名空间的配置。它处理获取、解析、存储和刷新此配置。

## 字段

-   **`client_config: ClientConfig`**:
    `ClientConfig` 的副本，包含服务器 URL、应用 ID 等设置。

-   **`namespace: String`**:
    此缓存实例负责的 Apollo 命名空间的名称（例如 "application", "FX.Properties"）。

-   **`loading: Arc<RwLock<bool>>`**:
    一个标志，用于防止对同一 `Cache` 实例进行并发 `refresh()` 操作。如果刷新已在进行中，其他对 `refresh()`（或触发它的方法）的调用将等待或提前返回。

-   **`checking_cache: Arc<RwLock<bool>>`**:
    *(仅限非 WASM)* 一个与 `loading` 类似的标志，但专门用于首次请求属性时从文件缓存进行的初始检查/加载。这可以防止多个并发尝试读取和解析同一文件缓存。

-   **`memory_cache: Arc<RwLock<Option<serde_json::Value>>>`**:
    命名空间已解析配置的内存存储。
    -   `serde_json::Value`: 配置通常是一个 JSON 对象（字符串键到字符串值的映射），因此 `Value` 可以表示此结构。
    -   `Option<...>`: 如果缓存尚未填充或获取失败，则为 `None`。
    -   `Arc<RwLock<...>>`: 允许线程安全的内部可变性，用于读取和更新内存缓存。

-   **`file_path: std::path::PathBuf`**:
    *(仅限非 WASM)* 此命名空间配置被缓存到的本地文件的完整路径。它通常构造为 `{cache_dir}/{app_id}_{cluster}_{namespace}.json`，其中 `cache_dir` 由 `client_config.get_cache_dir()` 确定。对于 WASM，此字段仍然存在，但未主动用于文件 I/O。

## 核心方法和逻辑

-   **`new(client_config: ClientConfig, namespace: String) -> Self`**:
    构造函数。它将 `loading` 和 `checking_cache` 初始化为 `false`，`memory_cache` 初始化为 `None`，并根据提供的配置构造 `file_path`。

-   **`get_property<T: DeserializeOwned + Send + Sync + 'static>(&self, key: &str) -> impl Future<Output = Option<T>> + Send`**:
    (以及 `get_value(key)`，它类似但返回 `Option<serde_json::Value>`)
    这是检索配置值的主要方法。它遵循分层查找逻辑：
    1.  **检查内存缓存**: 首先尝试从 `self.memory_cache` 获取值。
    2.  **检查文件缓存 (非 WASM)**:
        -   如果内存中没有并且在非 WASM 目标上，它会尝试从 `self.file_path` 加载配置。
        -   这涉及获取对 `self.checking_cache` 的锁。
        -   如果文件存在，则读取其内容，解析为 JSON，然后更新 `self.memory_cache`。然后从（现已填充的）内存缓存中检索该值。
    3.  **必要时刷新**:
        -   如果仍然在内存缓存中找不到该值（无论是最初不存在，还是文件缓存加载失败或不包含该键），它会调用 `self.refresh().await`。
        -   `refresh()` 完成后，它会再次尝试从 `self.memory_cache` 获取该值。
    -   检索到的 `serde_json::Value` 然后被克隆，并尝试将其反序列化为请求的类型 `T`。

-   **`refresh(&self) -> impl Future<Output = Result<(), Error>> + Send`**:
    从 Apollo 服务器获取最新配置，并更新内存和文件缓存。
    1.  **锁定 `loading`**: 获取对 `self.loading` 的写锁。如果已锁定，则等待。这可以防止并发刷新操作。
    2.  **构造 URL**: 构建 Apollo 服务器的请求 URL：
        `{config_server}/configs/{app_id}/{cluster}/{namespace_encoded}?ip={ip}&label={label}`
        -   `namespace_encoded`: 命名空间名称经过 URL 编码。
        -   `ip` 和 `label` 从 `client_config` 中获取，如果存在则作为查询参数添加。
    3.  **身份验证 (签名)**:
        -   如果 `client_config.secret` 是 `Some(secret)`，则请求需要签名。
        -   生成一个时间戳。
        -   使用时间戳、相对 URL 路径（例如 `/configs/...`）和密钥调用 `sign()` 辅助函数。
        -   `sign()` 函数计算 HMAC-SHA1 签名。
        -   签名和时间戳作为 `Authorization` 和 `Timestamp` 标头添加到 HTTP 请求中。
    4.  **HTTP GET 请求**: 使用像 `reqwest` 这样的库向构造的 URL 发出异步 HTTP GET 请求。
    5.  **处理响应**:
        -   如果请求成功（例如 HTTP 200 OK）：
            -   响应体被解析为 JSON，成为 `serde_json::Value`。
            -   `*self.memory_cache.write().unwrap() = Some(parsed_value)`。
            -   *(仅限非 WASM)*: 解析后的 JSON 被序列化回字符串并写入 `self.file_path`。这涉及如果缓存目录不存在则创建它。
        -   如果请求返回 HTTP 304 Not Modified：这意味着自上次获取以来配置未更改（Apollo 使用类似 ETag 的机制，尽管此客户端未明确管理 ETag，但如果客户端轮询频繁，服务器可能会响应 304）。缓存保持不变。
        -   其他 HTTP 错误或网络错误会导致返回 `Error`。
    6.  **解锁 `loading`**: 方法退出时释放锁。

## 辅助函数 `sign(timestamp: &str, url_path: &str, secret: &str) -> String`

-   此内部静态函数负责生成 Apollo 认证请求所需的签名。
-   **输入**: 时间戳字符串、URL 路径（例如 `/configs/appId/cluster/namespace`）和密钥。
-   **过程**:
    1.  创建要签名的字符串：`"{timestamp}\n{url_path}"`。
    2.  使用 HMAC-SHA1 实现（来自 `hmac` 和 `sha1` 包）。
    3.  使用密钥计算要签名的字符串的 HMAC-SHA1 哈希值。
    4.  以 Base64 编码生成的哈希值。
-   **输出**: Base64 编码的签名字符串。

这种结构化方法可确保有效地检索、缓存和更新配置，同时还处理潜在的并发问题和身份验证要求。
