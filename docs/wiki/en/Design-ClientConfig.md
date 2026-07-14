[中文简体](../zh-CN/Design-ClientConfig.md) | [中文繁體](../zh-TW/Design-ClientConfig.md)
[Back to Home](Home.md)

# ClientConfig Details

The `ClientConfig` struct is a cornerstone of the `apollo-rust-client` library, encapsulating all the necessary settings required by the client to interact with the Apollo Configuration Centre.

## Fields

-   **`app_id: String`**:
    The application ID for which configurations are to be fetched. This is a mandatory field.

-   **`cluster: String`**:
    The name of the Apollo cluster to fetch configurations from. Defaults to `"default"` in many scenarios but should be explicitly provided.

-   **`cache_dir: Option<String>`**:
    *(Non-WASM only)* An optional base path for configuration files. If `None`, the client uses the platform-standard application cache directory. Versioned hashed filenames isolate all request identity fields.

-   **`config_server: String`**:
    The base URL of the Apollo Configuration server (e.g., `http://localhost:8080`). This is a mandatory field.

-   **`secret: Option<String>`**:
    An optional secret key used for signing requests if the Apollo namespace requires authentication.

-   **`label: Option<String>`**:
    An optional comma-separated list of labels for the current instance. This is used for grayscale release rules.

-   **`ip: Option<String>`**:
    An optional IP address of the client machine. This is also used for grayscale release rules.

-   **`cache_ttl`, `refresh_interval`, `request_timeout`: Option<u64>**:
    Independent cache-expiry, polling, and complete-request timeout controls. `cache_ttl = 0` is an always-revalidate mode; the other two values must be greater than zero.

## Instantiation

There are several ways to create a `ClientConfig` instance:

1.  **Validated Builder (Rust, preferred):**
    ```rust
    use apollo_rust_client::client_config::ClientConfig;

    let config = ClientConfig::builder("my_app", "http://apollo-server:8080")
        .cluster("my_cluster")
        .secret("my_secret_key")
        .cache_dir("/tmp/apollo_cache")
        .label("version1,regionA")
        .ip("192.168.1.100")
        .build()?;
    ```

2.  **`ClientConfig::from_env()`:**
    This associated function reads the native process environment. On WASM it reads `globalThis.process.env` when running under Node.js and returns an environment error in browsers.
    -   `APP_ID`: Corresponds to `app_id`.
    -   `IDC`: Corresponds to `cluster` (defaults to "default" if not set).
    -   `APOLLO_CONFIG_SERVICE`: Corresponds to `config_server`.
    -   `APOLLO_ACCESS_KEY_SECRET`: Corresponds to `secret`.
    -   `APOLLO_LABEL`: Corresponds to `label`.
    -   `APOLLO_CACHE_DIR`: Corresponds to `cache_dir`.
    -   `APOLLO_ALLOW_INSECURE_HTTPS`: Corresponds to `allow_insecure_https` ("true" to bypass cert verification).
    -   `APOLLO_CACHE_TTL`: Corresponds to `cache_ttl` in seconds (defaults to 600).
    -   `APOLLO_REFRESH_INTERVAL`: Corresponds to `refresh_interval` in seconds (defaults to 30).
    -   `APOLLO_REQUEST_TIMEOUT`: Corresponds to `request_timeout` in seconds (defaults to 10).
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
-   If `self.cache_dir` is `None`, it uses the platform-standard application cache directory (XDG cache, macOS `~/Library/Caches`, or Windows Local AppData).

This logic ensures that there's a predictable location for cached configuration files even if not explicitly specified by the user.
