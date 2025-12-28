[English](../en/Installation.md) | [中文繁體](../zh-TW/Installation.md)
[返回首页](Home.md)

# 安装

## Rust

将以下内容添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
apollo-rust-client = "0.6.3"
```

或者，您可以使用 `cargo add`：

```bash
cargo add apollo-rust-client
```

## WebAssembly (npm)

```bash
npm install @qqiao/apollo-rust-client
```

WebAssembly 版本提供与原生 Rust 版本相同的功能，但编译为在浏览器和 Node.js 等 JavaScript 环境中运行。
