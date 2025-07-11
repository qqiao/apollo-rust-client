[中文简体](../zh-CN/Error-Handling.md) | [中文繁體](../zh-TW/Error-Handling.md)
[Back to Home](Home.md)

# Error Handling

The Apollo Rust client provides comprehensive error handling through Rust's `Result` type system and JavaScript Promise rejection patterns.

## Error Types

### Client Errors

- **`AlreadyRunning`**: Attempting to start a client that's already running
- **`Namespace`**: Errors related to namespace operations and format detection
- **`Cache`**: Errors from cache operations (loading, refreshing, file I/O)

### Cache Errors

- **`AlreadyLoading`**: Concurrent refresh attempts on the same cache
- **`AlreadyCheckingCache`**: Concurrent cache initialization attempts
- **`NamespaceNotFound`**: Requested namespace doesn't exist on the server
- **`Reqwest`**: Network communication errors with Apollo server
- **`UrlParse`**: Invalid URL format in configuration
- **`Serde`**: JSON parsing and serialization errors
- **`Io`**: File system operations (native targets only)

### Namespace Errors

- **`Json`**: JSON namespace processing errors
- **`ContentNotFound`**: Missing content in JSON namespace data
- **`DeserializeError`**: Failed to deserialize JSON to custom types

## Error Handling Patterns

### Rust

```rust
use apollo_rust_client::{Client, Error};

// Pattern 1: Using ? operator for early returns
async fn get_config() -> Result<String, Error> {
    let client = create_client().await?;
    let namespace = client.namespace("application").await?;

    match namespace {
        apollo_rust_client::namespace::Namespace::Properties(props) => {
            Ok(props.get_string("app.name").await.unwrap_or_default())
        }
        _ => Ok(String::new())
    }
}

// Pattern 2: Explicit error handling with match
async fn handle_errors() {
    let client = create_client().await.unwrap();

    match client.namespace("application").await {
        Ok(namespace) => {
            // Handle successful namespace retrieval
            println!("Successfully retrieved namespace");
        }
        Err(Error::Cache(cache_error)) => {
            eprintln!("Cache error: {}", cache_error);
        }
        Err(Error::Namespace(ns_error)) => {
            eprintln!("Namespace error: {}", ns_error);
        }
        Err(e) => {
            eprintln!("Other error: {}", e);
        }
    }
}

// Pattern 3: Graceful degradation
async fn get_config_with_fallback() -> String {
    let client = create_client().await.unwrap();

    match client.namespace("application").await {
        Ok(namespace) => {
            match namespace {
                apollo_rust_client::namespace::Namespace::Properties(props) => {
                    props.get_string("app.name").await.unwrap_or_else(|| "DefaultApp".to_string())
                }
                _ => "DefaultApp".to_string()
            }
        }
        Err(_) => "DefaultApp".to_string()
    }
}
```

### JavaScript

```javascript
// Pattern 1: Using try-catch with async/await
async function getConfig() {
  let client = null;
  let cache = null;

  try {
    const clientConfig = new ClientConfig(
      "app_id",
      "http://server:8080",
      "default"
    );
    client = new Client(clientConfig);
    await client.start();

    cache = await client.namespace("application");
    const value = await cache.get_string("app.name");

    return value || "DefaultApp";
  } catch (error) {
    console.error("Configuration error:", error);
    return "DefaultApp";
  } finally {
    // Always cleanup WASM objects
    if (cache) cache.free();
    if (client) client.free();
  }
}

// Pattern 2: Using Promise.catch()
function getConfigWithPromise() {
  const clientConfig = new ClientConfig(
    "app_id",
    "http://server:8080",
    "default"
  );
  const client = new Client(clientConfig);

  return client
    .start()
    .then(() => client.namespace("application"))
    .then((cache) => {
      const result = cache.get_string("app.name");
      cache.free();
      return result;
    })
    .catch((error) => {
      console.error("Configuration error:", error);
      return "DefaultApp";
    })
    .finally(() => {
      client.free();
    });
}
```

## Best Practices

1. **Always handle errors**: Don't ignore potential failures in configuration retrieval
2. **Provide fallback values**: Have sensible defaults for missing configuration
3. **Log errors appropriately**: Use different log levels for different error types
4. **Cleanup resources**: Always free WASM objects in JavaScript, even on errors
5. **Retry logic**: Implement exponential backoff for transient network errors
6. **Graceful degradation**: Continue operation with default values when possible

## Common Error Scenarios

### Network Issues

- Server unreachable
- DNS resolution failures
- Connection timeouts
- HTTP error responses

### Configuration Issues

- Invalid namespace names
- Missing required configuration
- Malformed JSON in JSON namespaces
- Authentication failures

### Resource Issues

- File system permissions (native targets)
- Memory allocation failures
- Concurrent access conflicts
