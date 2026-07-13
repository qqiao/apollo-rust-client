[English](../en/Design-Client.md) | [中文繁體](../zh-TW/Design-Client.md)
[返回首页](Home.md)

# Client 详情

`Client` 结构体是与 Apollo 配置服务交互的主要面向用户的接口。它管理配置设置、特定于命名空间的缓存以及后台刷新任务。

## 字段

-   **`client_config: ClientConfig`**:
    一个 `ClientConfig` 实例，包含此客户端的所有配置设置。此配置被克隆并传递给客户端创建的每个 `Cache` 实例。

-   **`namespaces: Arc<RwLock<HashMap<String, Arc<Cache>>>>`**:
    一个线程安全的、共享的 `Cache` 实例集合。
    -   `HashMap`: 将命名空间名称 (String) 映射到其对应的 `Cache`。
    -   `Arc<Cache>`: 每个 `Cache` 都包装在 `Arc` 中，以允许在 `Client` 和可能持有缓存的其他应用程序部分之间共享所有权。
    -   `RwLock`: 提供具有读写锁的内部可变性，允许多个线程并发读取 `HashMap` 或一个线程写入它（例如，在添加新命名空间时）。
    -   `Arc<RwLock<...>>`: 整个 `HashMap` 包装在 `Arc` 中，以便后台刷新任务也可以安全地访问和迭代命名空间。

-   **`handle: Option<tokio::task::JoinHandle<()>>`**:
    *(仅限非 WASM)* 定期刷新任务的句柄。`stop()` 和 `Drop` 会立即中止该任务。WASM 使用 `futures::future::AbortHandle`。

-   **`running: Arc<AtomicBool>`**:
    一个标志，用于控制后台刷新任务的执行。
    -   `Arc<RwLock<...>>`: 在 `Client` 和后台任务之间共享。`Client` 将其设置为 `false` 以通知任务停止，任务读取它以了解何时退出。

## 核心方法

-   **`new(client_config: ClientConfig) -> Result<Self, Error>`**:
    `Client` 的构造函数。它将 `namespaces` 映射初始化为空，并将 `handle` 设置为 `None`，`running` 设置为 `false`（初始状态）。

-   **`namespace(&mut self, name: &str) -> impl Future<Output = Result<CACHE_RETURN_TYPE, Error>> + Send`**:
    检索给定命名空间名称的 `Cache`。
    -   它首先检查命名空间的 `Cache` 是否已存在于 `namespaces` 映射中。
    -   如果是，则返回现有的 `Cache`。
    -   如果否，它会使用客户端的 `client_config` 和给定的命名空间名称创建一个新的 `Cache` 实例，将其存储在 `namespaces` 映射中，然后返回它。
    -   **返回类型差异**:
        -   **非 WASM**: 返回 `Arc<Cache>`。这允许调用者共享缓存的所有权。
        -   **WASM**: 直接返回 `Cache`。在 WASM 上下文中，与 Rust 的直接共享所有权不太常见，JavaScript 管理 `Cache` 对象的生命周期。`Cache` 本身在内部对某些字段使用 `Arc`。

-   **`start(&mut self) -> impl Future<Output = Result<(), Error>> + Send`**:
    启动用于配置更新的后台轮询机制。
    -   如果已在运行，则不执行任何操作。
    -   将 `self.running` 设置为 `true`。
    -   **生成任务**:
        -   **非 WASM (原生)**: 使用 `tokio::spawn`。
        -   **WASM**: 使用 `wasm_bindgen_futures::spawn_local` 和可取消 future。
    -   **后台任务逻辑**:
        1.  进入一个循环，只要 `*self.running.read().unwrap()` 为 `true`，该循环就会继续。
        2.  休眠固定间隔（当前为 30 秒）。
        3.  以最多四个并发请求刷新命名空间。
        4.  失败时采用有上限的指数退避和抖动。
    -   该方法本身在生成任务后迅速返回。

-   **`stop(&mut self)`**:
    通知后台任务终止。
    -   将 `*self.running.write().unwrap()` 设置为 `false`。
    -   *(仅限非 WASM)*: 如果 `self.handle` 是 `Some`，则调用 `handle.cancel().await` 来停止后台任务。然后将 `self.handle` 设置为 `None`。

## 线程安全与并发

-   `Arc` 和 `RwLock` 的使用对于管理对 `namespaces` 和 `running` 标志的共享访问至关重要，尤其是在主应用程序线程（调用 `Client` 方法）和后台刷新任务（在非 WASM 环境中）之间。
-   `Cache` 实例也设计为其操作在内部是线程安全的。
