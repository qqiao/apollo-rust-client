[English](../en/WASM-Memory-Management.md) | [中文繁體](../zh-TW/WASM-Memory-Management.md)
[返回首页](Home.md)

## WASM 中的内存管理

为防止 WebAssembly 中的内存泄漏，请在不再使用活跃的 `Client` 和 `Properties` 实例后，显式调用它们的 `free()` 方法。这将释放在 WebAssembly 堆上由 Rust 分配的内存。

- **`Client`**: 在客户端不再需要时调用 `client.free()`。
- **`Properties`**: 访问 properties 格式命名空间时由 `client.namespace()` 返回，使用完毕后必须调用 `properties.free()`。
- **`ClientConfig`**: 传给 `new Client(config)` 后所有权即被消费，**切勿**在传参后调用 `config.free()`（会导致抛出错误）。仅在创建了 `ClientConfig` 但从未传给 `new Client(config)` 时才需调用 `config.free()`。

其他命名空间格式（如 JSON、YAML 或 Text）返回的是普通的 JavaScript 对象或字符串，由 JavaScript 垃圾回收机制自动管理，无需手动调用 `.free()` 释放。
