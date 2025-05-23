[中文简体](../zh-CN/Design-ClientConfig.md) | [中文繁體](../zh-TW/Design-ClientConfig.md)
[Back to Home](Home.md)

# ClientConfig Details

The `ClientConfig` struct is a cornerstone of the `apollo-rust-client` library, encapsulating all the necessary settings required by the client to interact with the Apollo Configuration Centre.

## Fields

-   **`app_id: String`**:
    The application ID for which configurations are to be fetched. This is a mandatory field.

-   **`cluster: String`**:
    The name of the Apollo cluster to fetch configurations from. Defaults to `"default"` in many scenarios but should be explicitly provided.

-   **`cache_dir: Option<std::path::PathBuf>`**:
    *(Non-WASM only)* An optional path to a directory where configuration files will be cached locally. If `None`, a default path is typically constructed by `get_cache_dir()` (e.g., `/opt/data/{app_id}/config-cache`). For WASM targets, this is always `None` as filesystem access is restricted.

-   **`config_server: String`**:
    The base URL of the Apollo Configuration server (e.g., `http://localhost:8080`). This is a mandatory field.

-   **`secret: Option<String>`**:
    An optional secret key used for signing requests if the Apollo namespace requires authentication.

-   **`label: Option<String>`**:
    An optional comma-separated list of labels for the current instance. This is used for grayscale release rules.

-   **`ip: Option<String>`**:
    An optional IP address of the client machine. This is also used for grayscale release rules.

## Instantiation

There are several ways to create a `ClientConfig` instance:

1.  **Manual Creation (Rust):**
    As a standard Rust struct, you can instantiate it directly:
    ```rust
    use apollo_rust_client::client_config::ClientConfig;
    use std::path::PathBuf;

    let config = ClientConfig {
        app_id: "my_app".to_string(),
        cluster: "my_cluster".to_string(),
        config_server: "http://apollo-server:8080".to_string(),
        secret: Some("my_secret_key".to_string()),
        cache_dir: Some(PathBuf::from("/tmp/apollo_cache")),
        label: Some("version1,regionA".to_string()),
        ip: Some("192.168.1.100".to_string()),
    };
    ```

2.  **`ClientConfig::from_env()` (Non-WASM):**
    This associated function allows creating a `ClientConfig` from environment variables. This is particularly useful for server-side applications.
    -   `APP_ID`: Corresponds to `app_id`.
    -   `APOLLO_CLUSTER` or `IDC`: Corresponds to `cluster` (defaults to "default" if neither is set).
    -   `APOLLO_CONFIG_SERVICE`: Corresponds to `config_server`.
    -   `APOLLO_ACCESS_KEY_SECRET`: Corresponds to `secret`.
    -   `APOLLO_LABEL`: Corresponds to `label`.
    -   `APOLLO_CACHE_DIR`: Corresponds to `cache_dir`.
    The `ip` field is not set via `from_env()`.

3.  **WASM-Specific Constructor:**
    For WebAssembly environments, a specific constructor is exposed to JavaScript:
    `new ClientConfig(app_id: String, config_server: String, cluster: String)`
    This constructor is simpler because fields like `cache_dir` are not applicable in a typical browser WASM context and default to `None`. Optional fields like `secret`, `label`, and `ip` can be set on the instance from JavaScript after construction.

    ```javascript
    // In JavaScript, after importing from the WASM package
    const clientConfig = new ClientConfig("my_app_id", "http://apollo-server:8080", "default_cluster");
    clientConfig.secret = "my_secret_key"; // Set optional fields if needed
    ```

## Role

`ClientConfig` acts as the central and immutable piece of configuration for the `Client`. Once a `Client` is initialized with a `ClientConfig`, this configuration is used for all its operations, including being passed down to any `Cache` instances it creates.

## `get_cache_dir()` (Non-WASM)

This method, available on non-WASM targets, determines the actual cache directory path to be used.
-   If `self.cache_dir` (the field in `ClientConfig`) is `Some(path)`, that path is used.
-   If `self.cache_dir` is `None`, it constructs a default path: `/opt/data/{self.app_id}/config-cache`.

This logic ensures that there's a predictable location for cached configuration files even if not explicitly specified by the user.
