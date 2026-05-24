[中文简体](../zh-CN/WASM-Memory-Management.md) | [中文繁體](../zh-TW/WASM-Memory-Management.md)
[Back to Home](Home.md)

# WASM Memory Management

When using the Apollo Rust client in WebAssembly environments, proper memory management is crucial to prevent memory leaks.

## Critical: Always Call `free()`

WebAssembly objects created by the Apollo client must be explicitly freed when no longer needed:

```javascript
// These objects MUST be freed:
clientConfig.free(); // ClientConfig instances
client.free(); // Client instances
properties.free(); // Properties instances (returned by client.namespace() for properties format)
```

## Memory Management Pattern

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function useApolloClient() {
  let clientConfig = null;
  let client = null;
  let properties = null;

  try {
    // Create objects
    clientConfig = new ClientConfig("app_id", "http://server:8080", "default");
    client = new Client(clientConfig);
    properties = await client.namespace("application");

    // Use the client
    const value = properties.get_string("some_key");
    console.log("Retrieved:", value);
  } catch (error) {
    console.error("Error:", error);
  } finally {
    // ALWAYS cleanup Properties, Client, and ClientConfig
    if (properties) properties.free();
    if (client) client.free();
    if (clientConfig) clientConfig.free();
  }
}
```

## Event Listener Cleanup

Event listeners are automatically cleaned up when the cache is freed, but you should still follow proper cleanup patterns:

```javascript
async function useWithEventListeners() {
  let clientConfig = null;
  let client = null;

  try {
    clientConfig = new ClientConfig("app_id", "http://server:8080", "default");
    client = new Client(clientConfig);

    // Register event listener at client level
    await client.add_listener("application", (data, error) => {
      if (error) {
        console.error("Config error:", error);
      } else {
        console.log("Config updated:", data);
      }
    });

    await client.start();

    // Your application logic here
  } finally {
    // Cleanup client and config
    if (client) client.free();
    if (clientConfig) clientConfig.free();
  }
}
```

## Class-Based Pattern

For long-lived applications, consider wrapping the client in a class:

```javascript
class ApolloConfigManager {
  constructor(appId, serverUrl, cluster) {
    this.clientConfig = new ClientConfig(appId, serverUrl, cluster);
    this.client = new Client(this.clientConfig);
    this.propertiesNamespaces = new Map();
  }

  async getProperties(name) {
    if (!this.propertiesNamespaces.has(name)) {
      const props = await this.client.namespace(name);
      this.propertiesNamespaces.set(name, props);
    }
    return this.propertiesNamespaces.get(name);
  }

  async start() {
    await this.client.start();
  }

  // CRITICAL: Call this when shutting down
  cleanup() {
    // Free all properties namespaces
    for (const props of this.propertiesNamespaces.values()) {
      props.free();
    }
    this.propertiesNamespaces.clear();

    // Free client and config
    if (this.client) this.client.free();
    if (this.clientConfig) this.clientConfig.free();
  }
}

// Usage
const manager = new ApolloConfigManager(
  "app_id",
  "http://server:8080",
  "default"
);
await manager.start();

// Use the manager...

// IMPORTANT: Always cleanup when done
manager.cleanup();
```

## Memory Leak Prevention

1. **Never ignore cleanup**: Always call `free()` even if an error occurs
2. **Use try-finally blocks**: Ensure cleanup happens regardless of success/failure
3. **Track object lifetimes**: Keep references to objects that need cleanup
4. **Avoid circular references**: Don't store WASM objects in closures that might outlive them

## Browser Integration

For browser applications, consider cleanup on page unload:

```javascript
let apolloManager = null;

window.addEventListener("beforeunload", () => {
  if (apolloManager) {
    apolloManager.cleanup();
  }
});

// Or for single-page applications
window.addEventListener("beforeunload", () => {
  // Cleanup all Apollo resources
  if (cache) cache.free();
  if (client) client.free();
  if (clientConfig) clientConfig.free();
});
```

## Common Mistakes

❌ **Don't do this:**

```javascript
// Memory leak: Properties object never freed
const client = new Client(config);
const props = await client.namespace("app");
// Objects leaked when function exits (client and props must be freed)
```

✅ **Do this instead:**

```javascript
// Proper cleanup
const client = new Client(config);
try {
  const props = await client.namespace("app");
  // Use props synchronously...
  const value = props.get_string("key");
  props.free();
} finally {
  client.free();
}
```

Remember: The `free()` method releases memory allocated by Rust on the WebAssembly heap. Without it, WASM classes (Client, ClientConfig, and Properties) will accumulate and eventually cause performance issues or crashes. Other returned namespace formats like JSON, YAML, or Text are standard JS objects or strings managed automatically by JS garbage collection and do not need to be freed manually.
