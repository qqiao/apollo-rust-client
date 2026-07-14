[English](../en/Configuration.md) | [中文繁體](../zh-TW/Configuration.md)
[返回首页](Home.md)

## 配置

客户端通过 `ClientConfig` 结构体/类支持以下配置选项：

- `app_id`: 您在 Apollo 中的应用 ID。
- `cluster`: 集群名称 (默认为 "`default`")。
- `secret`: 给定 `app_id` 的可选密钥。
- `config_server`: 配置服务器的地址。
- `cache_dir`: 用于存储本地缓存的目录。
  - 原生目标默认使用平台标准应用缓存目录；版本化哈希文件名隔离完整请求身份。WASM 浏览器使用带 TTL 的 localStorage。
  - 对于 WASM 目标：此字段是可选的，默认为 `None`；文件系统缓存在浏览器环境中通常不适用或不使用。
- `label`: 当前实例的标签。用于在灰度发布中标识当前实例。
- `ip`: 您应用程序的 IP 地址。用于在灰度发布中标识当前实例。
- `allow_insecure_https`: 是否允许不安全的 HTTPS 连接（自签名证书）。用于公司内部网络或开发环境。
- `cache_ttl`: 内存和持久缓存 TTL（默认 600 秒；`0` 表示始终后台重新验证）。
- `refresh_interval`: 后台轮询间隔（默认 30 秒，必须大于零）。
- `request_timeout`: 完整请求和响应体超时（默认 10 秒，必须大于零）。
