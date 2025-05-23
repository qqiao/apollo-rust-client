[中文简体](../zh-CN/JavaScript-Usage.md) | [中文繁體](../zh-TW/JavaScript-Usage.md)
[Back to Home](Home.md)

## WebAssembly (JavaScript) Usage

This example shows how to use the client in a JavaScript environment (e.g., browser or Node.js) via WebAssembly. It covers `ClientConfig` initialization, starting the client, fetching a namespace, and getting properties.

```javascript
import { Client, ClientConfig } from '@qqiao/apollo-rust-client';

async function main() {
  // Basic configuration: app_id, config_server URL, cluster name
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // Set optional properties directly on the instance if needed
  clientConfig.secret = "your_apollo_secret"; // Example: if your Apollo namespace requires a secret
  // clientConfig.label = "your_instance_label"; // For grayscale releases
  // clientConfig.ip = "client_ip_address"; // For grayscale releases
  // clientConfig.cache_dir = "/custom/cache/path"; // Note: cache_dir is less common in browser environments

  const client = new Client(clientConfig);

  // Optional: Start background polling for configuration updates.
  // This is a non-blocking operation.
  await client.start();

  // Get configuration for the "application" namespace
  const namespace = await client.namespace("application");

  // Example: Retrieve a string property
  const stringVal = await namespace.get_string("some_key");
  if (stringVal !== undefined) {
    console.log("Property 'some_key':", stringVal);
  } else {
    console.log("Property 'some_key' not found.");
  }

  // Example: Retrieve an integer property using get_int
  const intVal = await namespace.get_int("meaningOfLife");
  if (intVal !== undefined) {
    console.log("Property 'meaningOfLife':", intVal);
  } else {
    console.log("Property 'meaningOfLife' not found or not an integer.");
  }

  // IMPORTANT: Release Rust memory for WASM objects when they are no longer needed
  namespace.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```
Properties like `secret`, `label`, `ip`, and `cache_dir` can be set on the `clientConfig` instance directly after construction if needed. `cache_dir` is typically not used in browser environments due to filesystem limitations.
