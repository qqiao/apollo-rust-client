const path = require("path");
const fs = require("fs");

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

console.log("[wasm_api_smoke] All WASM method export and ownership checks passed successfully.");
