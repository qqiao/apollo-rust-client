[English](../en/Design-Overview.md) | [中文繁體](../zh-TW/Design-Overview.md)
[返回首页](Home.md)

# 设计概览

本文档提供了 `apollo-rust-client` 库架构的高级概览。该库旨在成为 Apollo 配置中心的强大客户端，支持原生 Rust 应用程序和 WebAssembly 环境。

## 核心模块

该库围绕一些核心模块/结构构建：

-   **`ClientConfig`**: 此结构包含 Apollo 客户端所需的所有必要设置。它包括 Apollo 服务器地址、应用程序 ID、集群名称、可选的密钥和缓存目录（针对非 WASM 目标）等详细信息。它作为客户端配置的单一事实来源。

-   **`Client`**: 这是与 Apollo 配置服务交互的主要入口点。它负责：
    *   管理不同配置命名空间的 `Cache` 实例。
    *   启动和管理一个后台任务（针对非 WASM 目标），该任务定期刷新所有请求命名空间的配置。
    *   提供访问特定命名空间缓存的方法。

-   **`Cache`**: 此结构负责存储和管理单个命名空间的配置。其主要职责包括：
    *   从 Apollo 服务器获取配置数据。
    *   解析获取的配置（目前支持 JSON，期望是字符串键到字符串值的映射）。
    *   将配置存储在内存缓存中。
    *   对于非 WASM 目标，它还处理基于文件的缓存以持久化配置并减少网络请求。
    *   提供检索单个配置属性的方法。

## 关键交互

1.  **`Client` 与 `ClientConfig`**:
    *   创建 `Client` 需要 `ClientConfig` 的实例。`Client` 存储此配置，并将其传递给它创建的 `Cache` 实例。

2.  **`Client` 与 `Cache` 管理**:
    *   当通过 `client.namespace("namespace_name").await` 请求特定命名空间时，`Client` 首先检查此命名空间的 `Cache` 是否已存在。
    *   如果存在，则返回对现有 `Cache` 的引用（通常包装在 `Arc` 中以实现共享所有权）。
    *   如果不存在，`Client` 会创建一个新的 `Cache` 实例，存储它以供将来请求，然后返回它。

3.  **配置请求的一般流程**:
    考虑调用：`client.namespace("application").await.get_property("some_key").await`
    *   `client.namespace("application").await`:
        *   `Client` 查找或创建 "application" 命名空间的 `Cache`。
        *   如果 `Cache` 是新的，或者其内部状态确定需要刷新（尽管通常是第一次属性访问触发加载），此操作可能涉及配置的初始获取。
    *   `.get_property("some_key").await`:
        *   此方法在 "application" 命名空间的 `Cache` 实例上调用。
        *   `Cache` 将首先检查其内存存储。
        *   如果在内存中未找到（并且对于非 WASM 目标），它可能会检查其文件缓存。
        *   如果仍未找到或缓存被认为已过期，`Cache` 将触发 `refresh()` 操作以从 Apollo 服务器获取最新配置。
        *   一旦配置可用，将检索 "some_key" 的值，解析为请求的类型 `T`，然后返回。
        *   如果正在运行（非 WASM），后台刷新任务也将定期在所有活动的 `Cache` 实例上调用 `refresh()`。
