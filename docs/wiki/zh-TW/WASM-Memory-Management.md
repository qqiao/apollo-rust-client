[English](../en/WASM-Memory-Management.md) | [中文简体](../zh-CN/WASM-Memory-Management.md)
[返回首頁](Home.md)

## WASM 中的記憶體管理

為防止 WebAssembly 中的記憶體洩漏，請在不再使用活躍的 `Client` 和 `Properties` 實例後，顯式呼叫它們的 `free()` 方法。這將釋放在 WebAssembly 堆上由 Rust 分配的記憶體。

- **`Client`**: 在客戶端不再需要時呼叫 `client.free()`。
- **`Properties`**: 訪問 properties 格式命名空間時由 `client.namespace()` 返回，使用完畢後必須呼叫 `properties.free()`。
- **`ClientConfig`**: 傳給 `new Client(config)` 後所有權即被消費，**切勿**在傳參後呼叫 `config.free()`（會導致拋出錯誤）。僅在建立了 `ClientConfig` 但從未傳給 `new Client(config)` 時才需呼叫 `config.free()`。

其他命名空間格式（如 JSON、YAML 或 Text）返回的是普通的 JavaScript 物件或字串，由 JavaScript 垃圾回收機制自動管理，無需手動呼叫 `.free()` 釋放。
