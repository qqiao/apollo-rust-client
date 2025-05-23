[English](../en/Error-Handling.md) | [中文简体](../zh-CN/Error-Handling.md)
[返回首頁](Home.md)

## 錯誤處理

大多數客戶端操作在 Rust 中返回 `Result<T, apollo_rust_client::Error>`，或在 JavaScript 中返回一個可能因錯誤而拒絕的 Promise。常見錯誤包括設定獲取失敗（例如，網路問題、Apollo 伺服器錯誤）或本地快取問題。確保您的應用程式碼包含適當的錯誤處理邏輯（例如，在 Rust 中使用 `match` 或 `?`，或在 JavaScript 中使用 Promise 的 `.catch()`）。
