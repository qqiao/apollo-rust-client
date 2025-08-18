[English](../en/Installation.md) | [中文简体](../zh-CN/Installation.md)
[返回首頁](Home.md)

# 安裝

## Rust

將以下內容添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
apollo-rust-client = "0.5.2"
```

或者，您可以使用 `cargo add`：

```bash
cargo add apollo-rust-client
```

## WebAssembly (npm)

```bash
npm install @qqiao/apollo-rust-client
```

WebAssembly 版本提供與原生 Rust 版本相同的功能，但編譯為在瀏覽器和 Node.js 等 JavaScript 環境中運行。
