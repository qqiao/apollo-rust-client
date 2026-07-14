# SDD Interface Design Viewpoint

This document details the external and internal interfaces of the `apollo-rust-client`, detailing how it binds to JS/WASM runtimes, communicates with Apollo Servers, and coordinates internally.

---

## 1. External Interfaces

### 1.1 WebAssembly & JavaScript Bindings (`wasm-bindgen`)

The client exports standard JavaScript classes to enable integration within Node.js and browser environments. The generated bindings interface with standard Promise structures.

#### Class `ClientConfig` (JavaScript API)
- **Constructor**:
  ```javascript
  constructor(appId: string, configServer: string, cluster: string);
  ```
- **Properties**:
  - `secret: string | null` (Getter/Setter)
  - `label: string | null` (Getter/Setter)
  - `ip: string | null` (Getter/Setter)
  - `allow_insecure_https: boolean | undefined` (Getter/Setter)
  - `cache_ttl: bigint | undefined` (Getter/Setter)
  - `refresh_interval: bigint | undefined` (Getter/Setter)
  - `request_timeout: bigint | undefined` (Getter/Setter)

#### Class `Client` (JavaScript API)
- **Constructor**:
  ```javascript
  constructor(config: ClientConfig);
  ```
- **Methods**:
  - `start(): void`: Spawns the non-blocking refresh loop (WASM), or throws if already running.
  - `stop(): void`: Stops the background loop.
  - `namespace(namespace: string): Promise<any>`: Retrieves the JavaScript representation of the `Namespace` variant (`Properties` class, plain JSON/YAML object, or text string).
  - `add_listener(namespace: string, callback: (data: any | undefined, error: string | undefined) => void): Promise<void>`: Registers an observer callback.

#### Class `Properties` (JavaScript API)
- **Methods**:
  - `get_string(key: string): string | null` (synchronous)
  - `get_int(key: string): number | null` (synchronous)
  - `get_float(key: string): number | null` (synchronous)
  - `get_bool(key: string): boolean | null` (synchronous)

> [!WARNING]
> **Manual Memory Management**: JS runtimes must call `.free()` on all instances of `ClientConfig`, `Client`, and `Properties` (when returned from `namespace()`) created from WASM once they are out of scope. If ignored, the underlying memory allocated in the WebAssembly heap will leak. Other returned namespace formats (plain JSON/YAML objects and text strings) are managed by JavaScript garbage collection.

---

### 1.2 Apollo HTTP Integration Protocol

The library retrieves configurations by directly polling Apollo Server JSON endpoints.

- **Request Type**: HTTP `GET`
- **Request URL Layout**:
  ```
  {config_server}/configfiles/json/{app_id}/{cluster}/{namespace}
  ```
- **Query Parameters**:
  - `ip` (Optional): Appended as `?ip={ip_val}` for targeted grayscale releases.
  - `label` (Optional): Appended as `?label={label_val}` for target canary rules.

#### Request Headers & Authentication Signature
If `secret` authentication is configured, the client generates custom HMAC-SHA1 headers on each request:

| Header | Description / Value Format |
| :--- | :--- |
| `timestamp` | Current Unix time in milliseconds (e.g., `1576478257344`). |
| `Authorization` | Signature format: `Apollo {app_id}:{signature}`. |

#### Signature Algorithm
The HMAC-SHA1 signature is constructed as follows:
1. Parse the request URL to extract the path and query string:
   `path_and_query = "/configs/{app_id}/{cluster}/{namespace}?ip={ip}"`
2. Formulate the signing input string:
   `input = "{timestamp_millis}\n{path_and_query}"`
3. Generate the HMAC-SHA1 digest using the `secret` key.
4. Encode the resulting digest byte array as standard **Base64** text.

```rust
// Signature generation signature (Internal)
fn sign(timestamp: i64, url: &str, secret: &str) -> Result<String, Error>;
```

### 1.3 Testing Mock Server

Native tests start a private, random-port HTTPS server in-process. It uses a generated self-signed certificate and models Apollo payloads, authentication headers, status errors, malformed bodies, delayed bodies, and concurrent request counts without Docker or a fixed port. WASM tests install a deterministic mocked global `fetch`, because Node's wasm test runtime cannot connect an in-module native server. The [scripts/test.sh](../scripts/test.sh) entry point runs native, Rustls, documentation, and WASM checks directly.

---

## 2. Internal Interfaces

The coordination between internal modules is asynchronous and thread-safe.

```mermaid
sequenceDiagram
    participant User as Consumer Code
    participant Client as Client Instance
    participant Registry as Namespace Registry (HashMap)
    participant Cache as Cache Instance
    participant Net as Apollo Server HTTP

    User->>Client: namespace("application.json")
    Client->>Registry: Acquire RwLock Write
    alt Cache does not exist
        Registry->>Cache: Create new Cache(Config, Namespace)
        Registry->>Registry: Insert into HashMap
    end
    Registry-->>Client: Return Arc<Cache>
    Client->>Cache: get_value()
    Cache->>Cache: Read in-memory cache
    alt Cache is Empty/Stale
        Cache->>Cache: Lock memory writer
        Cache->>Net: GET /configfiles/json/...
        Net-->>Cache: JSON Response
        Cache->>Cache: Write persistent cache (Native file / WASM localStorage)
        Cache->>Cache: Update in-memory cache
        Cache-->>User: Trigger Registered Observers (Events)
        Cache->>Cache: Unlock memory writer
    end
    Cache-->>Client: Return serde_json::Value
    Client-->>User: Return parsed Namespace (JSON enum)
```

### 2.1 Format Detection Flow
When converting a raw configuration response, `namespace::get_namespace` analyzes the name string:
1. Splitting the string on dot characters (`.`).
2. If no extensions are found, format returns `NamespaceType::Properties`.
3. If extensions exist, it parses the trailing substring:
   - `"json"` $\rightarrow$ `NamespaceType::Json`
   - `"yaml" | "yml"` $\rightarrow$ `NamespaceType::Yaml`
   - `"xml"` $\rightarrow$ `NamespaceType::Xml` (currently unsupported)
   - Other extension $\rightarrow$ `NamespaceType::Text` (default fallback)
