[English](../en/Error-Handling.md) | [中文繁體](../zh-TW/Error-Handling.md)
[返回首页](Home.md)

## 错误处理

大多数客户端操作在 Rust 中返回 `Result<T, apollo_rust_client::Error>`，或在 JavaScript 中返回一个可能因错误而拒绝的 Promise。常见错误包括配置获取失败（例如，网络问题、Apollo 服务器错误）或本地缓存问题。确保您的应用程序代码包含适当的错误处理逻辑（例如，在 Rust 中使用 `match` 或 `?`，或在 JavaScript 中使用 Promise 的 `.catch()`）。
