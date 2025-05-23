[English](../en/JavaScript-Usage.md) | [中文简体](../zh-CN/JavaScript-Usage.md)
[返回首頁](Home.md)

## WebAssembly (JavaScript) 用法

此範例顯示了如何通過 WebAssembly 在 JavaScript 環境（例如，瀏覽器或 Node.js）中使用客戶端。它涵蓋了 `ClientConfig` 初始化、啟動客戶端、獲取命名空間和獲取屬性。

```javascript
import { Client, ClientConfig } from '@qqiao/apollo-rust-client';

async function main() {
  // 基本設定：app_id、config_server URL、叢集名稱
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // 如果需要，可以直接在實例上設定可選屬性
  clientConfig.secret = "your_apollo_secret"; // 範例：如果您的 Apollo 命名空間需要金鑰
  // clientConfig.label = "your_instance_label"; // 用于灰度發布
  // clientConfig.ip = "client_ip_address"; // 用于灰度發布
  // clientConfig.cache_dir = "/custom/cache/path"; // 注意：cache_dir 在瀏覽器環境中不太常用

  const client = new Client(clientConfig);

  // 可選：啟動後台輪詢以獲取設定更新。
  // 這是一個非阻塞操作。
  await client.start();

  // 獲取 "application" 命名空間的設定
  const namespace = await client.namespace("application");

  // 範例：檢索字串屬性
  const stringVal = await namespace.get_string("some_key");
  if (stringVal !== undefined) {
    console.log("屬性 'some_key':", stringVal);
  } else {
    console.log("未找到屬性 'some_key'");
  }

  // 範例：使用 get_int 檢索整數屬性
  const intVal = await namespace.get_int("meaningOfLife");
  if (intVal !== undefined) {
    console.log("屬性 'meaningOfLife':", intVal);
  } else {
    console.log("未找到屬性 'meaningOfLife' 或該屬性不是整數。");
  }

  // 重要提示：當不再需要 WASM 物件時，請釋放 Rust 記憶體
  namespace.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```
如果需要，可以在建構 `clientConfig` 實例後直接設定 `secret`、`label`、`ip` 和 `cache_dir` 等屬性。由於檔案系統限制，`cache_dir` 通常不在瀏覽器環境中使用。
