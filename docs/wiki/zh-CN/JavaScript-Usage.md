[English](../en/JavaScript-Usage.md) | [中文繁體](../zh-TW/JavaScript-Usage.md)
[返回首页](Home.md)

## WebAssembly (JavaScript) 用法

此示例显示了如何通过 WebAssembly 在 JavaScript 环境（例如，浏览器或 Node.js）中使用客户端。它涵盖了 `ClientConfig` 初始化、启动客户端、获取命名空间和获取属性。

```javascript
import { Client, ClientConfig } from '@qqiao/apollo-rust-client';

async function main() {
  // 基本配置：app_id、config_server URL、集群名称
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // 如果需要，可以直接在实例上设置可选属性
  clientConfig.secret = "your_apollo_secret"; // 示例：如果您的 Apollo 命名空间需要密钥
  // clientConfig.label = "your_instance_label"; // 用于灰度发布
  // clientConfig.ip = "client_ip_address"; // 用于灰度发布
  // clientConfig.cache_dir = "/custom/cache/path"; // 注意：cache_dir 在浏览器环境中不太常用

  const client = new Client(clientConfig);

  // 可选：启动后台轮询以获取配置更新。
  // 这是一个非阻塞操作。
  await client.start();

  // 获取 "application" 命名空间的配置
  const namespace = await client.namespace("application");

  // 示例：检索字符串属性
  const stringVal = await namespace.get_string("some_key");
  if (stringVal !== undefined) {
    console.log("属性 'some_key':", stringVal);
  } else {
    console.log("未找到属性 'some_key'");
  }

  // 示例：使用 get_int 检索整数属性
  const intVal = await namespace.get_int("meaningOfLife");
  if (intVal !== undefined) {
    console.log("属性 'meaningOfLife':", intVal);
  } else {
    console.log("未找到属性 'meaningOfLife' 或该属性不是整数。");
  }

  // 重要提示：当不再需要 WASM 对象时，请释放 Rust 内存
  namespace.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```
如果需要，可以在构造 `clientConfig` 实例后直接设置 `secret`、`label`、`ip` 和 `cache_dir` 等属性。由于文件系统限制，`cache_dir` 通常不在浏览器环境中使用。
