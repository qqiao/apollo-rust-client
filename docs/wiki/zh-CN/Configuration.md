[English](../en/Configuration.md) | [中文繁體](../zh-TW/Configuration.md)
[返回首页](Home.md)

## 配置

客户端通过 `ClientConfig` 结构体/类支持以下配置选项：

- `app_id`: 您在 Apollo 中的应用ID。
- `cluster`: 集群名称 (默认为 "`default`")。
- `secret`: 给定 `app_id` 的可选密钥。
- `config_server`: 配置服务器的地址。
- `cache_dir`: 用于存储本地缓存的目录。
    - 对于非 WASM 目标（本机 Rust 应用程序）：如果未指定，则默认为构造为 `/opt/data/{app_id}/config-cache` 的路径。
    - 对于 WASM 目标：此字段是可选的，默认为 `None`；文件系统缓存在浏览器环境中通常不适用或不使用。
- `label`: 当前实例的标签。用于在灰度发布中标识当前实例。
- `ip`: 您应用程序的 IP 地址。用于在灰度发布中标识当前实例。
