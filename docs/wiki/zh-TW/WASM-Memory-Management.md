[English](../en/WASM-Memory-Management.md) | [中文简体](../zh-CN/WASM-Memory-Management.md)
[返回首頁](Home.md)

## WASM 中的記憶體管理
為防止 WebAssembly 中的記憶體洩漏，請在不再使用 `ClientConfig`、`Client` 和 `Cache` 實例後，顯式呼叫它們的 `free()` 方法。這將釋放在 WebAssembly 堆上由 Rust 分配的記憶體。
