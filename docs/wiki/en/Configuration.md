[中文简体](../zh-CN/Configuration.md) | [中文繁體](../zh-TW/Configuration.md)
[Back to Home](Home.md)

## Configuration

The client supports the following configuration options via the `ClientConfig` struct/class:

- `app_id`: Your application ID in Apollo.
- `cluster`: The cluster name (default: "`default`").
- `secret`: The optional secret for the given `app_id`.
- `config_server`: The address of the configuration server.
- `cache_dir`: Directory to store local cache.
    - For non-WASM targets (native Rust applications): If not specified, it defaults to a path constructed as `/opt/data/{app_id}/config-cache`.
    - For WASM targets: This field is optional and defaults to `None`; filesystem caching is generally not applicable or used in browser environments.
- `label`: The label of the current instance. Used to identify the current instance for a grayscale release.
- `ip`: The IP address of your application. Used to identify the current instance for a grayscale release.
