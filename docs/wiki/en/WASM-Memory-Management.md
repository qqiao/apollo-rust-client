[中文简体](../zh-CN/WASM-Memory-Management.md) | [中文繁體](../zh-TW/WASM-Memory-Management.md)
[Back to Home](Home.md)

# WASM Memory Management

When using the Apollo Rust client in WebAssembly environments, proper memory management is crucial to prevent memory leaks.

## Critical: Ownership and Calling `free()`

WebAssembly objects allocated on the Rust heap must be explicitly freed when no longer needed to prevent memory leaks:

- **Client**: Must be freed via `client.free()` when the client is no longer needed.
- **Properties**: Returned by `client.namespace()` when accessing properties format; must be freed via `properties.free()`.
- **ClientConfig**:
  - If passed to `new Client(config)`, ownership of `ClientConfig` is transferred to `Client` and consumed by Rust. **Do NOT call `config.free()` after passing it to `new Client(config)`** — attempting to do so will throw an error.
  - If a `ClientConfig` is created but never transferred to a `Client`, it must be freed via `config.free()`.

Other returned namespace formats like JSON, YAML, or Text are standard JavaScript objects or strings managed automatically by JavaScript garbage collection and do not need to be freed manually.

<!-- apollo-example: wasm-ownership -->
```javascript
const client = new Client(new ClientConfig("app_id", "http://server:8080", "default"));
try {
  // Use client. Config is consumed by Client construction and must not be freed.
} finally {
  client.free();
}
```

```javascript
// An untransferred ClientConfig must be freed if not passed to Client:
const config = new ClientConfig("app_id", "http://server:8080", "default");
config.free();
```

## Memory Management Pattern

```javascript
import { Client, ClientConfig } from "@qqiao/apollo-rust-client";

async function useApolloClient() {
  let config = null;
  let client = null;
  let properties = null;

  try {
    // Create objects
    config = new ClientConfig("app_id", "http://server:8080", "default");
    const transferred = config;
    config = null; // Ownership transferred to Client
    client = new Client(transferred);
    properties = await client.namespace("application");

    // Use the client
    const value = properties.get_string("some_key");
    console.log("Retrieved:", value);
  } catch (error) {
    console.error("Error:", error);
  } finally {
    // ALWAYS cleanup live Properties and Client; cleanup config only if never transferred
    if (properties) properties.free();
    if (client) client.free();
    if (config) config.free();
  }
}
```

## Event Listener Cleanup

Event listeners are automatically cleaned up when the client is freed, but you should still follow proper cleanup patterns:

```javascript
async function useWithEventListeners() {
  let config = null;
  let client = null;

  try {
    config = new ClientConfig("app_id", "http://server:8080", "default");
    const transferred = config;
    config = null; // Ownership transferred to Client
    client = new Client(transferred);

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
    // Cleanup client; cleanup config only if never transferred
    if (client) client.free();
    if (config) config.free();
  }
}
```

## Class-Based Pattern

For long-lived applications, consider wrapping the client in a class:

```javascript
class ApolloConfigManager {
  constructor(appId, serverUrl, cluster) {
    const config = new ClientConfig(appId, serverUrl, cluster);
    this.client = new Client(config);
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

    // Free client
    if (this.client) {
      this.client.free();
      this.client = null;
    }
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

1. **Never ignore cleanup**: Always call `free()` on live `Client` and `Properties` even if an error occurs
2. **Use try-finally blocks**: Ensure cleanup happens regardless of success/failure
3. **Track object lifetimes**: Keep references to objects that need cleanup, and clear them once transferred
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
  // Cleanup live Apollo client resources
  if (client) {
    client.free();
    client = null;
  }
});
```

## Common Mistakes

❌ **Don't do this:**

```javascript
// Error: Trying to free config after passing it to Client
const config = new ClientConfig("app_id", "http://server:8080", "default");
const client = new Client(config);
config.free(); // THROWS: config was already consumed by Client!
```

❌ **Don't do this:**

```javascript
// Memory leak: Properties object never freed
const client = new Client(new ClientConfig("app_id", "http://server:8080", "default"));
const props = await client.namespace("app");
// Objects leaked when function exits (client and props must be freed)
```

✅ **Do this instead:**

```javascript
// Proper cleanup
const client = new Client(new ClientConfig("app_id", "http://server:8080", "default"));
try {
  const props = await client.namespace("app");
  // Use props synchronously...
  const value = props.get_string("key");
  props.free();
} finally {
  client.free();
}
```

Remember: The `free()` method releases memory allocated by Rust on the WebAssembly heap. Without it, WASM classes (`Client`, untransferred `ClientConfig`, and `Properties`) will accumulate and eventually cause performance issues or crashes. Other returned namespace formats like JSON, YAML, or Text are standard JS objects or strings managed automatically by JS garbage collection and do not need to be freed manually.
