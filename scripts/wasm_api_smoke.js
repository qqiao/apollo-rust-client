const path = require("path");
const fs = require("fs");
const assert = require("node:assert/strict");

// 1. Resolve package directory
const pkgDir = process.argv[2]
  ? path.resolve(process.argv[2])
  : path.resolve(__dirname, "../pkg");

const bindings = require(path.join(pkgDir, "apollo_rust_client.js"));

// 2. Export assertions
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

// Configuration accessors preserve the public API when Copy fields use copying
// getters and owned strings use cloning getters (004/FR-001, 008/FR-006).
const accessorConfig = new bindings.ClientConfig("app_id", "http://server:8080", "default");
try {
  const cases = {
    app_id: ["app_id", "updated-app"],
    cluster: ["default", "production"],
    config_server: ["http://server:8080", "https://example.com"],
    cache_dir: [undefined, "cache/配置", "", undefined, null],
    secret: [undefined, "secret-value", "", undefined, null],
    label: [undefined, "canary,β", "", undefined, null],
    ip: [undefined, "127.0.0.1", "", undefined, null],
    allow_insecure_https: [undefined, true, false, undefined, null],
    cache_ttl: [undefined, 0n, 600n, (1n << 64n) - 1n, undefined, null],
    refresh_interval: [undefined, 0n, 30n, (1n << 64n) - 1n, undefined, null],
    request_timeout: [undefined, 0n, 10n, (1n << 64n) - 1n, undefined, null],
  };
  for (const [field, values] of Object.entries(cases)) {
    const descriptor = Object.getOwnPropertyDescriptor(bindings.ClientConfig.prototype, field);
    assert.equal(typeof descriptor?.get, "function", `${field} getter`);
    assert.equal(typeof descriptor?.set, "function", `${field} setter`);
    for (const value of values) {
      accessorConfig[field] = value;
      const expected = value === null ? undefined : value;
      assert.equal(accessorConfig[field], expected, `${field} round trip`);
      assert.equal(accessorConfig[field], expected, `${field} repeated read`);
    }
  }
  accessorConfig.app_id = "original-配置";
  const retainedString = accessorConfig.app_id;
  accessorConfig.app_id = "replacement";
  assert.equal(retainedString, "original-配置", "string reads remain independently owned values");
} finally {
  accessorConfig.free();
}

// 3. Extract and execute canonical ownership snippet from doc
const docPath = path.resolve(
  __dirname,
  "../docs/wiki/en/WASM-Memory-Management.md"
);
const docContent = fs.readFileSync(docPath, "utf8");

const marker = "<!-- apollo-example: wasm-ownership -->";
const markerIndex = docContent.indexOf(marker);
if (markerIndex === -1) {
  throw new Error(`Missing ${marker} in ${docPath}`);
}
if (docContent.indexOf(marker, markerIndex + marker.length) !== -1) {
  throw new Error(`Duplicate ${marker} in ${docPath}`);
}

const afterMarker = docContent.slice(markerIndex + marker.length);
const codeBlockMatch = afterMarker.match(
  /^\s*```javascript\r?\n([\s\S]*?)\r?\n```/
);
if (!codeBlockMatch) {
  throw new Error(
    `Could not find javascript code block immediately following ${marker}`
  );
}
const canonicalSnippet = codeBlockMatch[1];

// Execute canonical doc snippet
try {
  const docFn = new Function("Client", "ClientConfig", canonicalSnippet);
  docFn(bindings.Client, bindings.ClientConfig);
} catch (err) {
  throw new Error(`Canonical ownership documentation snippet failed: ${err.message}`);
}

// 4. No-network ownership cases:
// 4a. Untransferred ClientConfig can be freed safely
const untransferredConfig = new bindings.ClientConfig(
  "app_id",
  "http://server:8080",
  "default"
);
untransferredConfig.free();

// 4b. Consumed ClientConfig throws if freed; Client frees cleanly
const consumedConfig = new bindings.ClientConfig(
  "app_id",
  "http://server:8080",
  "default"
);
const client = new bindings.Client(consumedConfig);
let configFreeThrew = false;
try {
  consumedConfig.free();
} catch (err) {
  configFreeThrew = true;
}
if (!configFreeThrew) {
  client.free();
  throw new Error("Expected consumed config.free() to throw after transfer to Client");
}
client.free();

// 4c. Application error preservation during cleanup
const sentinel = new Error("application sentinel error");
let caught = null;
let appClient = null;
try {
  appClient = new bindings.Client(
    new bindings.ClientConfig("app_id", "http://server:8080", "default")
  );
  throw sentinel;
} catch (err) {
  caught = err;
} finally {
  if (appClient) appClient.free();
}
if (caught !== sentinel) {
  throw new Error("Application error was not preserved during Client cleanup");
}

// 4d. Constructor validation failure with consumed-config handling
let invalidConfig = null;
let failedClient = null;
let validationError = null;
try {
  invalidConfig = new bindings.ClientConfig(
    "app_id",
    "invalid-url",
    "default"
  );
  const transferred = invalidConfig;
  invalidConfig = null; // ownership transferred to Client constructor
  failedClient = new bindings.Client(transferred);
} catch (err) {
  validationError = err;
} finally {
  if (failedClient) failedClient.free();
  if (invalidConfig) invalidConfig.free();
}
if (!validationError) {
  throw new Error("Expected Client constructor to fail with invalid URL");
}

console.log("[wasm_api_smoke] All WASM method export, configuration accessor, and ownership checks passed successfully.");
