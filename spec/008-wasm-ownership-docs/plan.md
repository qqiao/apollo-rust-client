# Plan: Executable WASM ownership guidance

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

## Read and inventory

Inspect `Client::new(config: ClientConfig)` and WASM conversion in [src/lib.rs](../../src/lib.rs), `ClientConfig::new` in [src/client_config.rs](../../src/client_config.rs), [scripts/wasm_api_smoke.js](../../scripts/wasm_api_smoke.js), [scripts/build.sh](../../scripts/build.sh), and ownership handling in [tests/apollo/wasm.cjs](../../tests/apollo/wasm.cjs).

Search current docs before editing:

```sh
rg -n 'new Client|ClientConfig|\.free\(|cache\.free' README.md README_zh.md docs/wiki
```

Affected families include both READMEs, JavaScript-Usage, WASM-Memory-Management, Installation, Home, Design-Client, and Design-WASM in the languages where they exist. Do not create missing translated design pages merely to update ownership; package 011 removes their broken navigation links.

## Canonical example and generated-package smoke

Add one short, standalone JavaScript fence to `docs/wiki/en/WASM-Memory-Management.md`, immediately following the exact marker `<!-- apollo-example: wasm-ownership -->`. The block must require only injected `Client` and `ClientConfig`, perform no network request or module import, and be executable as a function body. It should construct a config, transfer it to Client, and free Client in finally; explain in a comment why config is now consumed and must not be freed. A separate tiny block documents `config.free()` **before any transfer**. Do not remove all config.free text indiscriminately.

Enhance `scripts/wasm_api_smoke.js`:

1. Preserve all existing constructor/method export assertions.
2. Accept an optional package directory as `process.argv[2]`, resolved absolutely, defaulting to the current `../pkg` directory relative to the script. Require the generated `apollo_rust_client.js` file there. Existing `node scripts/wasm_api_smoke.js` behavior remains compatible.
3. Read the canonical Markdown file relative to `__dirname`. Require exactly one marked JavaScript fence, fail on missing/duplicate/unterminated marker, and execute it with `new Function("Client", "ClientConfig", snippet)(...)`. This executes repository-owned documentation, never downloaded or user-supplied text.
4. Add no-network assertions for freeing an untransferred config, preserving a sentinel application error through Client cleanup, and invalid-settings constructor failure with consumed-config handling. Assert observable behavior/exception identity or type, not wasm-bindgen's exact diagnostic wording or private pointer fields.
5. Add a clear successful summary; any failed assertion exits nonzero. Do not catch failures and merely print them.

The pre-fix ownership sequence is confirmed to throw; demonstrate that when developing the regression, then keep the corrected documented sequence in the final suite. Do not modify the generated package to make double-free work.

Use the build job's existing smoke invocation; no extra CI job is necessary. The no-network smoke must not replace the separate real-Apollo Node suite or import its fault setup. Do not override global fetch in `tests/apollo/wasm.cjs`.

## Patterns for larger examples

Prefer consuming temporary configuration when no setters are needed:

```javascript
const client = new Client(new ClientConfig(appId, serverUrl, "default"));
try {
  // await operations; free each returned Properties wrapper in its own finally
} finally {
  client.free();
}
```

When a larger finally block tracks config before transfer, clear the ownership variable **before** calling Client:

```javascript
let config = null;
let client = null;
try {
  config = new ClientConfig(appId, serverUrl, "default");
  config.secret = secret;
  const transferred = config;
  config = null; // ownership leaves this scope when Client receives the valid wrapper
  client = new Client(transferred);
  // application work
} finally {
  if (client) client.free();
  if (config) config.free(); // only a wrapper that never reached Client
}
```

This handles Rust validation rejection after transfer. Avoid catching that rejection and assuming config remains owned. For classes storing wrappers, stop storing consumed ClientConfig as an owned resource; clear `this.client`/Properties references after freeing if cleanup can be invoked twice. Do not build a generic ownership framework.

Properties are snapshots; a class caching Properties forever must not claim to expose continuously updated values. Do not broaden this work into changing namespace caching; simplify such examples or accurately label their snapshot behavior.

## Verification

From repository root, after the optional path support lands:

```sh
task_package_dir="$(mktemp -d)"
wasm-pack build --target nodejs --dev --out-dir "$task_package_dir"
node scripts/wasm_api_smoke.js "$task_package_dir"
scripts/test.sh integration --suite wasm
```

Remove only the directory created by that command after verification. The normal `scripts/build.sh` also executes the smoke using its default path. Run `node --check scripts/wasm_api_smoke.js` and `git diff --check`. Documentation corrections alone do not require unrelated native tests repeatedly. Update contracts/traceability after actual package execution, preserving deferred detached-work lifetime policy.
