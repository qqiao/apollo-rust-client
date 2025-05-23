[English](../en/Design-WASM.md) | [中文繁體](../zh-TW/Design-WASM.md)
[返回首页](Home.md)

# WASM 设计考量

`apollo-rust-client` 旨在无缝地在 WebAssembly (WASM) 环境中工作，主要针对浏览器和 Node.js。由于 WASM 及其与 JavaScript 交互的特性，这需要特定的设计选择和调整。

## `cfg_if!` 与条件编译

`cfg_if!` 宏在整个代码库中广泛用于管理特定于平台的逻辑。这使得该库能够拥有一个单一的代码库，可以针对 `wasm32` 目标与原生目标进行不同的编译。

应用条件编译的关键领域：

-   **任务生成:**
    -   原生: 使用 `async_std::task::spawn` 在单独的操作系统线程中运行后台刷新任务。
    -   WASM: 使用 `async_std::task::spawn_local`，因为 WASM 环境（尤其是浏览器）通常是单线程的，`spawn` 可能不可用或不适用。

-   **`Client::namespace()` 中的 `Cache` 返回类型:**
    -   原生: 返回 `Arc<Cache>` 以允许共享缓存的所有权。
    -   WASM: 直接返回 `Cache`。由 `wasm-bindgen` 生成的 JavaScript 绑定处理 JS 端的对象生命周期。

-   **`ClientConfig` 构造函数:**
    -   原生: 提供 `ClientConfig::from_env()` 以方便服务器端使用。
    -   WASM: 向 JavaScript 公开一个特定的 `ClientConfig::new(app_id, config_server, cluster)` 构造函数，因为环境变量通常不在浏览器 WASM 中使用，并且文件系统缓存被禁用。

-   **文件系统缓存:**
    -   所有与磁盘上缓存配置相关的 文件 I/O 操作都针对 WASM 目标进行了条件编译排除。`ClientConfig` 中的 `cache_dir` 字段和 `Cache` 中的 `file_path` 字段在 WASM 中实际上未使用。

## `wasm-bindgen`

`wasm-bindgen` 工具和属性对于创建库的 JavaScript 接口至关重要。

-   **`#[wasm_bindgen]`**: 此属性用于结构体 (`ClientConfig`, `Client`, `Cache`) 及其方法，以使其可从 JavaScript 访问。
    -   对于结构体，它通常生成 JavaScript 类。
    -   对于方法，它在这些类上生成相应的 JavaScript 方法。

-   **`constructor`**:
    -   特定方法标记为 `#[wasm_bindgen(constructor)]`，以便在从 JavaScript 创建对象时充当构造函数（例如 `new ClientConfig(...)`，`new Client(...)`）。

-   **Getter 和 Setter**:
    -   需要从 JavaScript 访问的字段通常通过 getter 方法（例如 `#[wasm_bindgen(getter_with_clone)] pub fn app_id(&self) -> String;`）或 setter 方法（如果可变）公开。`getter_with_clone` 用于 String 字段以向 JavaScript 返回副本。

-   **方法公开**:
    -   打算从 JavaScript 使用的公共方法用 `#[wasm_bindgen]` 标记。这包括 `Client::namespace()`、`Client::start()`、`Cache::get_string()`、`Cache::get_int()` 等。

## 内存管理

-   **`free()` 方法**:
    -   `wasm-bindgen` 在 JavaScript 端为暴露给 WASM 且非 `Copy` 类型的 Rust 结构体生成一个 `free()` 方法。当不再需要 Rust 对象（`ClientConfig`、`Client`、`Cache`）时，JavaScript 代码调用此 `free()` 方法至关重要。
    -   这将释放在 WebAssembly 堆上由 Rust 分配的内存。否则可能导致 WASM 模块中的内存泄漏。
    -   *(注意: `free()` 方法本身并未在此库的 Rust 代码中明确定义；`wasm-bindgen` 为 JS 对象释放时提供了必要的绑定和 JavaScript 粘合代码以进行内存回收。)*

## API 差异

-   **`ClientConfig` 实例化**: 如前所述，WASM 使用简化的 `new ClientConfig(app_id, config_server, cluster)` 构造函数。可选字段（如 `secret`、`label` 和 `ip`）在实例化后从 JavaScript 直接设置。
-   **`Cache` 值检索**: 为了在 JavaScript 中方便使用，`Cache` 公开了诸如 `get_string(key: &str) -> Option<String>` 和 `get_int(key: &str) -> Option<i64>` 之类的方法。这些是泛型 `get_property::<T>()` 方法的包装器，为常见用例提供直接类型转换。由于 `wasm-bindgen` 中泛型类型的复杂性，泛型 `get_property` 未直接向 WASM 公开。
-   **错误处理**: 来自 `async` Rust 方法（返回 `Result<T, E>`）的错误通常会转换为 JavaScript Promise，该 Promise 会因错误而拒绝 (reject)。

## 文件系统

-   如前所述，基于文件系统的配置缓存对于 WASM 目标完全禁用。`Cache` 结构体仅依赖其内存存储和从 Apollo 服务器获取。

这些设计考量确保 `apollo-rust-client` 可以在广泛的 JavaScript 和 WebAssembly 应用程序中有效使用，提供一致的核心功能，同时适应 WASM 环境的特定约束和模式。
