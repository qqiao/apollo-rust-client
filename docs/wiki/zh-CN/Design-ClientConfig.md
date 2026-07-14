[English](../en/Design-ClientConfig.md) | [中文繁體](../zh-TW/Design-ClientConfig.md)
[返回首页](Home.md)

# ClientConfig 详情

`ClientConfig` 结构体是 `apollo-rust-client` 库的基石，封装了客户端与 Apollo 配置中心交互所需的所有必要设置。

## 字段

-   **`app_id: String`**:
    要获取其配置的应用程序 ID。这是一个必填字段。

-   **`cluster: String`**:
    要从中获取配置的 Apollo 集群的名称。在许多情况下默认为 `"default"`，但应明确提供。

-   **`cache_dir: Option<String>`**:
    *(仅限非 WASM)* 本地缓存的可选基础目录。默认使用平台标准应用缓存目录，版本化哈希文件名隔离完整请求身份。WASM 浏览器改用 localStorage。

-   **`config_server: String`**:
    Apollo 配置服务器的基本 URL（例如 `http://localhost:8080`）。这是一个必填字段。

-   **`secret: Option<String>`**:
    一个可选的密钥，如果 Apollo 命名空间需要身份验证，则用于签署请求。

-   **`label: Option<String>`**:
    当前实例的可选逗号分隔标签列表。这用于灰度发布规则。

-   **`ip: Option<String>`**:
    客户端计算机的可选 IP 地址。这也用于灰度发布规则。

## 实例化

有几种方法可以创建 `ClientConfig` 实例：

1.  **验证构建器（Rust，推荐）:**
    ```rust
    use apollo_rust_client::client_config::ClientConfig;
    let config = ClientConfig::builder("my_app", "http://apollo-server:8080")
        .cluster("my_cluster")
        .secret("my_secret_key")
        .cache_dir("/tmp/apollo_cache")
        .label("version1,regionA")
        .ip("192.168.1.100")
        .request_timeout(10)
        .build()?;
    ```

2.  **`ClientConfig::from_env()` (非 WASM):**
    此关联函数允许从环境变量创建 `ClientConfig`。这对于服务器端应用程序特别有用。
    -   `APP_ID`: 对应于 `app_id`。
    -   `APOLLO_CLUSTER` 或 `IDC`: 对应于 `cluster`（如果两者均未设置，则默认为 "default"）。
    -   `APOLLO_CONFIG_SERVICE`: 对应于 `config_server`。
    -   `APOLLO_ACCESS_KEY_SECRET`: 对应于 `secret`。
    -   `APOLLO_LABEL`: 对应于 `label`。
    -   `APOLLO_CACHE_DIR`: 对应于 `cache_dir`。
    `ip` 字段不会通过 `from_env()` 设置。

3.  **WASM 特定构造函数:**
    对于 WebAssembly 环境，向 JavaScript 公开了一个特定的构造函数：
    `new ClientConfig(app_id: String, config_server: String, cluster: String)`
    此构造函数更简单，因为像 `cache_dir` 这样的字段在典型的浏览器 WASM 上下文中不适用，并且默认为 `None`。可选字段（如 `secret`、`label` 和 `ip`）可以在构造后从 JavaScript 在实例上设置。

    ```javascript
    // 在 JavaScript 中，从 WASM 包导入后
    const clientConfig = new ClientConfig("my_app_id", "http://apollo-server:8080", "default_cluster");
    clientConfig.secret = "my_secret_key"; // 如果需要，设置可选字段
    ```

## 角色

`ClientConfig` 作为 `Client` 的中心且不可变的配置部分。一旦使用 `ClientConfig` 初始化 `Client`，此配置将用于其所有操作，包括传递给它创建的任何 `Cache` 实例。

## `get_cache_dir()` (非 WASM)

此方法（在非 WASM 目标上可用）确定要使用的实际缓存目录路径。
-   如果 `self.cache_dir`（`ClientConfig` 中的字段）是 `Some(path)`，则使用该路径。
-   如果 `self.cache_dir` 是 `None`，使用平台标准应用缓存目录。

此逻辑确保即使在用户未明确指定的情况下，缓存配置文件也有一个可预测的位置。
