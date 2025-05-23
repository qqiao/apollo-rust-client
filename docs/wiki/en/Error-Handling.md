[中文简体](../zh-CN/Error-Handling.md) | [中文繁體](../zh-TW/Error-Handling.md)
[Back to Home](Home.md)

## Error Handling

Most client operations return a `Result<T, apollo_rust_client::Error>` in Rust, or a Promise that may reject with an error in JavaScript. Common errors include configuration fetch failures (e.g., network issues, Apollo server errors) or issues with the local cache. Ensure your application code includes appropriate error handling logic (e.g., using `match` or `?` in Rust, or `.catch()` with Promises in JavaScript).
