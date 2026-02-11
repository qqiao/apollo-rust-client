[English](../en/Home.md) | [中文繁體](../zh-TW/Home.md)

# Apollo Rust 客户端

一个强大的 Apollo 配置中心 Rust 客户端，支持 WebAssembly，可用于浏览器和 Node.js 环境。

[![Crates.io](https://img.shields.io/crates/v/apollo-rust-client.svg)](https://crates.io/crates/apollo-rust-client)
[![Docs.rs](https://docs.rs/apollo-rust-client/badge.svg)](https://docs.rs/apollo-rust-client)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/qqiao/apollo-rust-client/blob/main/LICENSE)
[![Rust CI](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml/badge.svg)](https://github.com/qqiao/apollo-rust-client/actions/workflows/rust.yml)

## 快速开始

- **[安装](Installation.md)** - Rust 或 WebAssembly 入门
- **[Rust 使用](Rust-Usage.md)** - 原生 Rust 应用程序和示例
- **[JavaScript 使用](JavaScript-Usage.md)** - 浏览器和 Node.js 环境
- **[配置](Configuration.md)** - 客户端配置选项和设置

## 主要特性

### 配置管理

- **多种配置格式**: 支持 Properties、JSON、文本格式，自动检测
- **类型安全 API**: 编译时保证和运行时类型转换
- **自动格式检测**: 基于命名空间文件扩展名
- **自定义类型反序列化**: 支持将 JSON 反序列化为自定义结构

### 跨平台支持

- **原生 Rust**: 完整功能集，支持文件缓存和后台任务
- **WebAssembly**: 为浏览器环境优化，仅内存缓存
- **JavaScript 互操作**: 与 JavaScript 和 TypeScript 无缝集成

### 实时更新

- **后台轮询**: 可配置间隔的自动配置刷新
- **事件监听器**: 订阅配置变更通知
- **手动刷新**: 按需配置更新
- **优雅错误处理**: 即使在网络问题时也能继续运行

### 缓存与性能

- **多级缓存**: 内存和基于文件的缓存（原生目标）
- **并发访问**: 具有异步锁定的线程安全操作
- **缓存失效**: 配置更改时自动缓存更新
- **WASM 优化**: 为浏览器环境优化的纯内存缓存

### 安全与认证

- **基于密钥的认证**: HMAC-SHA1 签名支持
- **灰度发布**: 基于 IP 和标签的配置定向
- **安全连接**: 支持 Apollo 服务器的 HTTPS 通信
- **基于环境的配置**: 安全的凭据管理

## 配置格式

库根据命名空间名称自动检测配置格式：

| 格式           | 命名空间示例                           | 描述                             |
| -------------- | -------------------------------------- | -------------------------------- |
| **Properties** | `"application"`, `"config.properties"` | 键值对（默认格式）               |
| **JSON**       | `"config.json"`, `"settings.json"`     | 结构化 JSON 数据，支持自定义类型 |
| **Text**       | `"readme.txt"`, `"content.txt"`        | 纯文本内容                       |
| **YAML**       | `"config.yaml"`, `"config.yml"`        | YAML 格式，支持结构化数据        |
| **XML**        | `"config.xml"`                         | XML 格式（计划中）               |

## 文档章节

### 用户指南

- **[功能特性](Features.md)** - 全面的功能概述和能力
- **[安装](Installation.md)** - Rust 和 WebAssembly 的安装说明
- **[配置](Configuration.md)** - 客户端配置选项和环境变量
- **[Rust 使用](Rust-Usage.md)** - 原生 Rust 示例、模式和最佳实践
- **[JavaScript 使用](JavaScript-Usage.md)** - JavaScript 环境中的 WebAssembly 使用
- **[错误处理](Error-Handling.md)** - 全面的错误处理指南和最佳实践
- **[WASM 内存管理](WASM-Memory-Management.md)** - WebAssembly 环境的内存管理
- **[升级指南](Upgrade-Guide.md)** - 在不同版本之间升级

### 库设计与架构

- **[设计概览](Design-Overview.md)** - 高级架构和设计概念
- **[ClientConfig 详情](Design-ClientConfig.md)** - 配置结构和选项
- **[Client 详情](Design-Client.md)** - 主要客户端实现和生命周期
- **[Cache 详情](Design-Cache.md)** - 缓存系统和命名空间管理
- **[WASM 设计考虑](Design-WASM.md)** - WebAssembly 特定的设计决策

## API 概览

### Rust API

```rust
use apollo_rust_client::{Client, client_config::ClientConfig};

// 创建和配置客户端
let config = ClientConfig { /* ... */ };
let mut client = Client::new(config);

// 启动后台刷新
client.start().await?;

// 获取类型化命名空间
let namespace = client.namespace("application").await?;

// 处理不同格式
match namespace {
    Namespace::Properties(props) => {
        let value = props.get_string("key").await;
    }
    Namespace::Json(json) => {
        let config: MyConfig = json.to_object()?;
    }
    Namespace::Yaml(yaml) => {
        let config: MyConfig = yaml.to_object()?;
    }
    Namespace::Text(text) => {
        println!("内容: {}", text);
    }
}
```

### JavaScript API

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

// 创建和配置客户端
const config = new ClientConfig(
  "app-id",
  "http://apollo-server:8080",
  "default"
);
const client = new Client(config);

// 启动后台刷新
await client.start();

// 获取缓存并检索值
const cache = await client.namespace("application");
const value = await cache.get_string("key");

// 在 WASM 中始终释放内存
cache.free();
client.free();
config.free();
```

## 平台差异

### 原生 Rust 功能

- 具有自动持久化的基于文件的缓存
- 具有完整线程支持的后台刷新任务
- 环境变量配置
- 完整的 async/await 支持和所有标准库功能

### WebAssembly 功能

- 为浏览器环境优化的纯内存缓存
- 使用 spawn_local 的单线程执行任务
- 具有自动类型转换的 JavaScript 互操作
- 使用 free() 方法的显式内存管理

## 版本信息

**当前版本**: 0.7.0

### 最新变更 (v0.7.0)

- **连接复用**: 复用 `reqwest::Client` 实例以减少连接开销并提高性能。
- **依赖更新**: 将依赖项更新为最新的稳定版本。

## 获取帮助

- **[GitHub Issues](https://github.com/qqiao/apollo-rust-client/issues)** - 错误报告和功能请求
- **[文档](https://docs.rs/apollo-rust-client)** - docs.rs 上的 API 文档
- **[示例](https://github.com/qqiao/apollo-rust-client/tree/main/examples)** - 示例代码和使用模式

## 贡献

欢迎贡献！请查看我们的[贡献指南](https://github.com/qqiao/apollo-rust-client/blob/main/CONTRIBUTING.md)了解如何：

- 报告错误和请求功能
- 提交拉取请求
- 改进文档
- 添加新的配置格式支持

## 许可证

本项目根据 Apache 许可证版本 2.0 进行许可。有关详细信息，请参阅 [LICENSE](https://github.com/qqiao/apollo-rust-client/blob/main/LICENSE) 文件。
