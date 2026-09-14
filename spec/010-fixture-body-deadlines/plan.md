# Plan: Fixture request body deadline

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

## Exact ownership change

Inspect `fetchWithTimeout`, `requestJson`, and `probeWithRetry` in [scripts/apollo-fixtures.mjs](../../scripts/apollo-fixtures.mjs). Search all references before editing. At baseline only requestJson calls fetchWithTimeout; exported higher-level helpers eventually call requestJson.

Move the AbortController and timer lifetime into `requestJson` so one try/finally encloses **both** `await fetch(...)` and `await response.text()`. After proving there are no other callers, delete the now-unnecessary private fetchWithTimeout wrapper as part of this targeted refactor; do not retain duplicate timeout layers.

Implementation shape:

1. Read `timeoutMs` with default `REQUEST_TIMEOUT_MS` (5000). Keep it out of the options forwarded to fetch. Use local headers/body variables; preserve JSON serialization and Content-Type/Accept behavior.
2. Create controller and timer immediately before starting fetch. Pass its signal. Keep method/URL available for diagnostics without copying headers/body into errors.
3. Await fetch, then complete `response.text()`, under the same try/finally. Preserve existing tolerant JSON parse and `{ status, headers, data, ok }` return shape.
4. In catch, if this controller's deadline fired, throw a contextual timeout Error with original cause and method/URL/duration; otherwise rethrow the original transport/body error. Do not classify every AbortError as this deadline unless its controller actually fired.
5. In finally, clear the timer. Do not add a Promise.race that leaves fetch active. Do not start a new five-second window after headers.

It is acceptable to mark `requestJson` as a named export so internal tooling tests can call the actual helper with a short timeout; the module already exports fixture functions. This is a tooling-module export, not a Rust or npm client API. No new module/dependency/CLI option is necessary. Preserve any existing internal override callers found during inspection. Do not add a broad generic HTTP client abstraction.

## Tests against actual Node networking

Add `tests/tooling/apollo-http.test.mjs` using node:test/assert/http and import the actual helper. Listen on port 0 on 127.0.0.1; no fixed port or Docker. Suggested override: 150 ms, outer test watchdog 2.5 seconds (increase only for demonstrated host scheduling needs). Do not assert an exact millisecond completion or very tight lower bound; assert timeout category/context and completion well before the outer watchdog.

Server modes:

- Never send headers (AC-001).
- `writeHead` + `flushHeaders`, optionally write partial JSON, never end (AC-002).
- Complete valid JSON, empty body, plain malformed JSON, HTTP 401 and HTTP 500 (AC-003/004).
- Close/reset the connection before body completion (AC-006).

Track accepted sockets and destroy them in finally; close the server. A timeout test must fail before the fix because body consumption survives the deadline, not because the server did not start. Record that original failure explicitly. Observe connection closure for the aborted held response; do not use a mere Promise.race assertion as proof of underlying cancellation.

For timer disposal, a separate isolated test may use Node's scoped mock of global fetch to capture the signal, return a completed Response, wait past the short deadline, and assert signal was not later aborted. Restore the mock in finally and run this case serially; never import such setup in `tests/apollo/wasm.cjs`. Also assert ordinary failures are not reported as timeout. Do not include secrets in fixture names/assertion output.

## Wiring and compatibility

Package 009 introduces `node --test tests/tooling/*.test.mjs` in both fast paths. This new file is automatically selected; confirm selection and do not duplicate the whole command. If implementing 010 independently before 009, either first land its narrow test-command prerequisite or record the dependency as pending; do not silently omit regression checks from the final fast path.

Run Node test/syntax commands, `scripts/test.sh fast`, and `scripts/test.sh integration` once after helper changes. Real seed/verify/idempotency/auth/gray controls exercise all helper callers. The production Rust/WASM request implementation is untouched.

Update tests/apollo/README.md with the complete request deadline and its distinction from coarse stage bounds; update 005's scoped amendment and traceability after tests pass. No image/schema/fixture or retry-count changes are needed to make this test green.
