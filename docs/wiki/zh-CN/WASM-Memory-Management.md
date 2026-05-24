[English](../en/WASM-Memory-Management.md) | [中文繁體](../zh-TW/WASM-Memory-Management.md)
[返回首页](Home.md)

## WASM 中的内存管理
为防止 WebAssembly 中的内存泄漏，请在不再使用 `ClientConfig`、`Client` 和 `Properties` 实例后，显式调用它们的 `free()` 方法。这将释放在 WebAssembly 堆上由 Rust 分配的内存。

其他命名空间格式（如 JSON、YAML 或 Text）返回的是普通的 JavaScript 对象或字符串，由 JavaScript 垃圾回收机制自动管理，无需手动调用 `.free()` 释放。
