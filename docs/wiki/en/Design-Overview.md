[中文简体](../zh-CN/Design-Overview.md) | [中文繁體](../zh-TW/Design-Overview.md)
[Back to Home](Home.md)

# Design Overview

This document provides a high-level overview of the `apollo-rust-client` library's architecture. The library is designed to be a robust client for the Apollo Configuration Centre, with support for both native Rust applications and WebAssembly environments.

## Core Modules

The library is built around a few core modules/structs:

-   **`ClientConfig`**: This struct holds all the necessary settings for the Apollo client. It includes details such as the Apollo server address, application ID, cluster name, optional secret key, and cache directory (for non-WASM targets). It serves as the single source of truth for client configuration.

-   **`Client`**: This is the main entry point for interacting with the Apollo configuration service. It is responsible for:
    *   Managing instances of `Cache` for different configuration namespaces.
    *   Initiating and managing a background task (for non-WASM targets) that periodically refreshes configuration for all requested namespaces.
    *   Providing methods to access namespace-specific caches.

-   **`Cache`**: This struct is responsible for storing and managing the configuration for a single namespace. Its key responsibilities include:
    *   Fetching configuration data from the Apollo server.
    *   Parsing the fetched configuration (currently supports JSON, expecting a map of string keys to string values).
    *   Storing the configuration in an in-memory cache.
    *   For non-WASM targets, it also handles a file-based cache to persist configuration and reduce network requests.
    *   Providing methods to retrieve individual configuration properties.

## Key Interactions

1.  **`Client` and `ClientConfig`**:
    *   An instance of `ClientConfig` is required to create a `Client`. The `Client` stores this configuration and passes it down to `Cache` instances it creates.

2.  **`Client` and `Cache` Management**:
    *   When a specific namespace is requested via `client.namespace("namespace_name").await`, the `Client` first checks if a `Cache` for this namespace already exists.
    *   If it exists, a reference to the existing `Cache` is returned (often wrapped in an `Arc` for shared ownership).
    *   If it doesn't exist, the `Client` creates a new `Cache` instance, stores it for future requests, and then returns it.

3.  **General Flow of a Configuration Request**:
    Consider the call: `client.namespace("application").await.get_property("some_key").await`
    *   `client.namespace("application").await`:
        *   The `Client` looks up or creates a `Cache` for the "application" namespace.
        *   This operation might involve an initial fetch of the configuration if the `Cache` is new or if its internal state determines a refresh is needed (though typically the first property access triggers the load).
    *   `.get_property("some_key").await`:
        *   This method is called on the `Cache` instance for the "application" namespace.
        *   The `Cache` will first check its in-memory store.
        *   If not found in memory (and for non-WASM targets), it might check its file cache.
        *   If still not found or if the cache is considered stale, the `Cache` will trigger a `refresh()` operation to fetch the latest configuration from the Apollo server.
        *   Once the configuration is available, the value for "some_key" is retrieved, parsed into the requested type `T`, and returned.
        *   The background refresh task, if running (non-WASM), will also periodically call `refresh()` on all active `Cache` instances.
