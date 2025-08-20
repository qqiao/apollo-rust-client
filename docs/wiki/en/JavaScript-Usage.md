[中文简体](../zh-CN/JavaScript-Usage.md) | [中文繁體](../zh-TW/JavaScript-Usage.md)
[Back to Home](Home.md)

# JavaScript Usage

The following example demonstrates how to use the Apollo Rust client in a JavaScript environment (browser or Node.js).

## Basic Usage

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  // Basic configuration: app_id, config_server URL, cluster name
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  // Set optional properties directly on the instance if needed
  clientConfig.secret = "your_apollo_secret"; // Example: if your Apollo namespace requires a secret
  clientConfig.label = "your_instance_label"; // For grayscale releases
  clientConfig.ip = "client_ip_address"; // For grayscale releases

  const client = new Client(clientConfig);

  // Optional: Start background polling for configuration updates.
  // This is a non-blocking operation.
  await client.start();

  // Get configuration for the "application" namespace
  const cache = await client.namespace("application");

  // Example: Retrieve a string property
  const stringVal = cache.get_string("some_key");
  if (stringVal !== undefined) {
    console.log("Property 'some_key':", stringVal);
  } else {
    console.log("Property 'some_key' not found.");
  }

  // Example: Retrieve an integer property
  const intVal = cache.get_int("meaningOfLife");
  if (intVal !== undefined) {
    console.log("Property 'meaningOfLife':", intVal);
  } else {
    console.log("Property 'meaningOfLife' not found or not an integer.");
  }

  // Example: Retrieve a float property
  const floatVal = cache.get_float("pi");
  if (floatVal !== undefined) {
    console.log("Property 'pi':", floatVal);
  } else {
    console.log("Property 'pi' not found or not a float.");
  }

  // Example: Retrieve a boolean property
  const boolVal = cache.get_bool("debug_enabled");
  if (boolVal !== undefined) {
    console.log("Property 'debug_enabled':", boolVal);
  } else {
    console.log("Property 'debug_enabled' not found or not a boolean.");
  }

  // IMPORTANT: Release Rust memory for WASM objects when they are no longer needed
  cache.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```

## Event Listeners

You can register event listeners to be notified when configuration changes:

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  const client = new Client(clientConfig);
  const cache = await client.namespace("application");

  // Register an event listener
  await cache.add_listener((data, error) => {
    if (error) {
      console.error("Configuration update error:", error);
    } else {
      console.log("Configuration updated:", data);

      // You can access specific properties from the updated data
      if (data && data.some_key) {
        console.log("Updated some_key:", data.some_key);
      }
    }
  });

  // Start background polling to trigger updates
  await client.start();

  // Your application logic here

  // Cleanup
  cache.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```

## Working with Different Namespace Types

The library automatically detects the namespace format based on the namespace name:

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  const clientConfig = new ClientConfig(
    "your_app_id",
    "http://your-apollo-server:8080",
    "default"
  );

  const client = new Client(clientConfig);

  // Properties namespace (default format)
  const propsCache = await client.namespace("application");
  const stringValue = propsCache.get_string("app.name");
  console.log("App name:", stringValue);

  // JSON namespace (detected by .json extension)
  const jsonCache = await client.namespace("config.json");
  // JSON namespaces in WASM currently return raw configuration data
  // You can access properties directly from the cache methods

  // Text namespace (detected by .txt extension)
  const textCache = await client.namespace("readme.txt");
  // Text namespaces contain plain text content

  // Cleanup
  propsCache.free();
  jsonCache.free();
  textCache.free();
  client.free();
  clientConfig.free();
}

main().catch(console.error);
```

## Error Handling

Always wrap your Apollo client calls in try-catch blocks:

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function main() {
  let client = null;
  let cache = null;
  let clientConfig = null;

  try {
    clientConfig = new ClientConfig(
      "your_app_id",
      "http://your-apollo-server:8080",
      "default"
    );

    client = new Client(clientConfig);
    await client.start();

    cache = await client.namespace("application");

    const value = cache.get_string("some_key");
    if (value !== undefined) {
      console.log("Retrieved value:", value);
    }
  } catch (error) {
    console.error("Apollo client error:", error);
  } finally {
    // Always cleanup WASM objects
    if (cache) cache.free();
    if (client) client.free();
    if (clientConfig) clientConfig.free();
  }
}

main().catch(console.error);
```

## Memory Management

**Critical for WASM**: Always call `free()` on Apollo client objects when you're done with them to prevent memory leaks:

```javascript
// These objects need to be freed:
clientConfig.free(); // ClientConfig instances
client.free(); // Client instances
cache.free(); // Cache instances (returned by client.namespace())
```

The `free()` method releases the memory allocated by Rust on the WebAssembly heap. Failure to call `free()` will result in memory leaks in your application.

## Configuration Options

The `ClientConfig` constructor accepts the following parameters:

- `app_id` (required): Your application ID in Apollo
- `config_server` (required): The Apollo config server URL
- `cluster` (required): The cluster name (usually "default")

Optional properties that can be set after construction:

- `secret`: Secret key for authentication
- `label`: Label for grayscale releases
- `ip`: IP address for grayscale releases

Note: `cache_dir` is not applicable in browser environments and is automatically handled.
