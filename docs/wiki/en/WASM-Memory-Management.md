[中文简体](../zh-CN/WASM-Memory-Management.md) | [中文繁體](../zh-TW/WASM-Memory-Management.md)
[Back to Home](Home.md)

## Memory Management in WASM

To prevent memory leaks in WebAssembly, explicitly call the `free()` method on `ClientConfig`, `Client`, and `Cache` instances once they are no longer in use. This releases the memory allocated by Rust on the WebAssembly heap.
