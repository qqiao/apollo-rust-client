[English](../en/WASM-Memory-Management.md) | [中文繁體](../zh-TW/WASM-Memory-Management.md)
[返回首页](Home.md)

## WASM 中的内存管理
为防止 WebAssembly 中的内存泄漏，请在不再使用 `ClientConfig`、`Client` 和 `Cache` 实例后，显式调用它们的 `free()` 方法。这将释放在 WebAssembly 堆上由 Rust 分配的内存。
