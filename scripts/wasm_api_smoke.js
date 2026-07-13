const bindings = require("../pkg/apollo_rust_client.js");

const clientMethods = Object.getOwnPropertyNames(bindings.Client.prototype);
for (const method of [
  "namespace",
  "add_listener",
  "preload",
  "refresh",
  "start",
  "stop",
]) {
  if (!clientMethods.includes(method)) {
    throw new Error(`WASM Client binding is missing ${method}()`);
  }
}

for (const constructor of ["Client", "ClientConfig"]) {
  if (typeof bindings[constructor] !== "function") {
    throw new Error(`WASM binding is missing ${constructor}`);
  }
}
