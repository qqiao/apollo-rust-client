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

  // Get properties namespace (exposed as JS class Properties)
  const properties = await client.namespace("application");
 
  // Example: Retrieve properties synchronously
  const stringVal = properties.get_string("some_key");
  if (stringVal !== undefined) {
    console.log("Property 'some_key':", stringVal);
  } else {
    console.log("Property 'some_key' not found.");
  }
 
  const intVal = properties.get_int("meaningOfLife");
  if (intVal !== undefined) {
    console.log("Property 'meaningOfLife':", intVal);
  } else {
    console.log("Property 'meaningOfLife' not found or not an integer.");
  }
 
  const floatVal = properties.get_float("pi");
  if (floatVal !== undefined) {
    console.log("Property 'pi':", floatVal);
  } else {
    console.log("Property 'pi' not found or not a float.");
  }
 
  const boolVal = properties.get_bool("debug_enabled");
  if (boolVal !== undefined) {
    console.log("Property 'debug_enabled':", boolVal);
  } else {
    console.log("Property 'debug_enabled' not found or not a boolean.");
  }
 
  // IMPORTANT: Release Rust memory for WASM objects when they are no longer needed
  properties.free();
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
 
  // Register an event listener (at the client level)
  await client.add_listener("application", (data, error) => {
    if (error) {
      console.error("Configuration update error:", error);
    } else {
      console.log("Configuration updated:", data);
 
      // data is a Properties instance for Properties format, or raw JS object/string for JSON/YAML/Text formats
      if (data && typeof data.get_string === "function") {
        console.log("Updated some_key:", data.get_string("some_key"));
      }
    }
  });
 
  // Start background polling to trigger updates
  await client.start();
 
  // Your application logic here
 
  // Cleanup
  client.stop();
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

  // Properties namespace (default format) - Returns Properties class
  const props = await client.namespace("application");
  const stringValue = props.get_string("app.name");
  console.log("App name:", stringValue);
 
  // JSON namespace (detected by .json extension) - Returns raw JS object
  const jsonConfig = await client.namespace("config.json");
  console.log("Database Host:", jsonConfig.database?.host);
 
  // Text namespace (detected by .txt extension) - Returns raw JS string
  const textContent = await client.namespace("readme.txt");
  console.log("Readme length:", textContent.length);
 
  // Cleanup - Only Properties class instance needs to be freed
  props.free();
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
  let properties = null;
  let clientConfig = null;

  try {
    clientConfig = new ClientConfig(
      "your_app_id",
      "http://your-apollo-server:8080",
      "default"
    );

    client = new Client(clientConfig);
    await client.start();

    properties = await client.namespace("application");
 
    const value = properties.get_string("some_key");
    if (value !== undefined) {
      console.log("Retrieved value:", value);
    }
  } catch (error) {
    console.error("Apollo client error:", error);
  } finally {
    // Always cleanup WASM objects
    if (properties) properties.free();
    if (client) {
      client.stop();
      client.free();
    }
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
properties.free(); // Properties instances (returned by client.namespace() for properties format)
```

The `free()` method releases the memory allocated by Rust on the WebAssembly heap. Other formats (like JSON, YAML, or Text) are returned as raw JS objects or strings, and do not need to be freed manually.

## Configuration Options

The `ClientConfig` constructor accepts the following parameters:

- `app_id` (required): Your application ID in Apollo
- `config_server` (required): The Apollo config server URL
- `cluster` (required): The cluster name (usually "default")

Optional properties that can be set after construction:

- `secret`: Secret key for authentication
- `label`: Label for grayscale releases
- `ip`: IP address for grayscale releases
- `allow_insecure_https`: Accepted for API parity but ignored by browser TLS enforcement
- `cache_ttl`: Memory/localStorage TTL in seconds (default 600)
- `refresh_interval`: Periodic polling interval in seconds (default 30)

Note: `cache_dir` is not applicable in browser environments and is automatically handled.
