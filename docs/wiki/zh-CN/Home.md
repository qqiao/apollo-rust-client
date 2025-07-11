[English](../en/Home.md) | [中文繁體](../zh-TW/Home.md)

# Apollo Rust Client

一个强大的 Apollo 配置中心 Rust 客户端，支持 WebAssembly，可用于浏览器和 Node.js 环境。

## 快速开始

- **安装**: [Rust 或 WebAssembly 入门](Installation.md)
- **Rust 使用**: [原生 Rust 应用程序](Rust-Usage.md)
- **JavaScript 使用**: [浏览器和 Node.js 环境](JavaScript-Usage.md)

## 文档章节

### 用户指南

- [功能特性](Features.md) - 核心功能和支持的格式
- [安装](Installation.md) - Rust 和 WASM 的安装说明
- [配置](Configuration.md) - 客户端配置选项
- [Rust 使用](Rust-Usage.md) - 原生 Rust 示例和模式
- [JavaScript 使用](JavaScript-Usage.md) - JavaScript 中的 WebAssembly 使用
- [错误处理](Error-Handling.md) - 全面的错误处理指南
- [WASM 内存管理](WASM-Memory-Management.md) - WebAssembly 的内存管理

### 库设计

- [设计概览](Design-Overview.md) - 高级架构和概念
- [ClientConfig 详情](Design-ClientConfig.md) - 配置结构和选项
- [Client 详情](Design-Client.md) - 主要客户端实现
- [Cache 详情](Design-Cache.md) - 缓存系统和命名空间管理
- [WASM 设计考虑](Design-WASM.md) - WebAssembly 特定的设计决策

## 主要特性

- **多种配置格式**: Properties、JSON、文本，支持自动检测
- **类型安全 API**: 编译时保证和运行时类型转换
- **跨平台**: 原生 Rust 和 WebAssembly 支持
- **实时更新**: 后台轮询和事件监听器
- **全面缓存**: 多级缓存和文件持久化
- **灰度支持**: 基于 IP 和标签的配置定向
