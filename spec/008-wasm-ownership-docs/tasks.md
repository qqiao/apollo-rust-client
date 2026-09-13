# Tasks: WASM ownership guidance

Read the [plan](plan.md) and [common rules](../review-remediation/execution.md). Current examples only; preserve accurately labeled historical comparisons.

## T01 — Execute the canonical ownership contract

- [x] Complete T01.
- **Depends on:** none.
- **Files (2):** `scripts/wasm_api_smoke.js`, `docs/wiki/en/WASM-Memory-Management.md`.
- **Contracts:** FR-001–003, FR-005/006; AC-001–004; SC-001/003.
- **Work:** add the canonical marked snippet, strict extraction and optional generated-package path, retain export smoke, and implement the four no-network ownership cases. Correct the memory guide's try/finally and class patterns.
- **Acceptance:** corrected snippet executes from the actual doc; untransferred and consumed cases differ correctly; sentinel/application and constructor failures are not masked.
- **Verify:** `node --check scripts/wasm_api_smoke.js`; build a temporary Node/WASM package and run the plan's smoke command. Record old sequence's failure and corrected sequence's success.
- **Evidence:** Added `<!-- apollo-example: wasm-ownership -->` to `docs/wiki/en/WASM-Memory-Management.md`. Enhanced `scripts/wasm_api_smoke.js` with strict single-marker extraction, optional package dir `process.argv[2]`, export assertions, canonical snippet execution via `new Function(...)`, untransferred `config.free()`, consumed `config.free()` rejection assertion (`null pointer passed to rust`), application sentinel error preservation through `client.free()`, and constructor validation failure with consumed-config handling. Old sequence confirmed failing (`null pointer passed to rust`), corrected sequence passes 100%.

## T02 — Correct main entry-point examples

- [x] Complete T02.
- **Depends on:** T01.
- **Files (5):** `README.md`, `README_zh.md`, `docs/wiki/en/JavaScript-Usage.md`, `docs/wiki/zh-CN/JavaScript-Usage.md`, `docs/wiki/zh-TW/JavaScript-Usage.md`.
- **Contracts:** FR-001–004; AC-003/005/006.
- **Work:** remove consumed-config cleanup, use explicit ownership-transfer/exception-safe patterns, preserve Properties cleanup and ordinary data rules. Preserve bigint, synchronous getters, and async operation semantics.
- **Acceptance:** no entry-point sample frees transferred config; success/error cleanup releases only owned wrappers; translated meaning matches English.
- **Verify:** ownership search from plan; inspect each occurrence in context, `git diff --check`; rerun generated smoke only if its canonical block/helper changed.
- **Evidence:** Removed `clientConfig.free()` calls following `new Client(clientConfig)` in `README.md`, `README_zh.md`, and all `JavaScript-Usage.md` language variants. Updated memory management sections to clarify `ClientConfig` is consumed on transfer to `new Client(config)` and only untransferred configs should be freed. Verified with repo-wide regex search and `git diff --check`.

### Checkpoint A

- [x] Canonical generated-package ownership check and all primary usage examples agree.

## T03 — Align installation and home examples

- [x] Complete T03 in the three substeps below, recording each separately.
- **Depends on:** T02.
- **Substeps / files:** T03a: `docs/wiki/en/Installation.md`, `docs/wiki/en/Home.md`; T03b: corresponding `zh-CN` pair; T03c: corresponding `zh-TW` pair. Each substep is two files.
- **Contracts:** FR-001–004; AC-005/006.
- **Work:** correct only ownership and false exported-Cache cleanup in current examples, preserving installation instructions and runtime API semantics.
- **Acceptance:** each example either transfers config once or frees an untransferred config; no nonexistent Cache wrapper cleanup remains in current examples; no broad unrelated rewrite.
- **Verify:** contextual search, local links, `git diff --check` after each substep.
- **Evidence:**
  - T03a (`docs/wiki/en/Installation.md`, `docs/wiki/en/Home.md`): Removed `config.free()` after client creation in verification sample; removed `config.free()` and fixed comment in Home JS example.
  - T03b (`docs/wiki/zh-CN/Installation.md`, `docs/wiki/zh-CN/Home.md`): Removed `config.free()` in Installation sample; replaced nonexistent `cache.free()` with `properties.free()` and removed `config.free()` in Home JS sample.
  - T03c (`docs/wiki/zh-TW/Installation.md`, `docs/wiki/zh-TW/Home.md`): Removed `config.free()` in Installation sample; zh-TW Home contains index links only.

## T04 — Align secondary ownership references

- [x] Complete T04a.
- [x] Complete T04b.
- **Depends on:** T03.
- **Files:** T04a (4): `docs/wiki/en/Design-Client.md`, `docs/wiki/en/Design-WASM.md`, `docs/wiki/zh-CN/Design-Client.md`, `docs/wiki/zh-CN/Design-WASM.md`; T04b (2): `docs/wiki/zh-CN/WASM-Memory-Management.md`, `docs/wiki/zh-TW/WASM-Memory-Management.md`.
- **Contracts:** FR-004/006; AC-005/006; SC-002.
- **Work:** distinguish live/unconsumed ownership in prose and remove references to exported WASM Cache. Preserve current stop/drop limitations rather than promise all callbacks cease.
- **Acceptance:** language variants agree; constructor/getter signatures are unchanged; no invented generated API appears.
- **Verify:** final repository-wide ownership search; `scripts/test.sh integration --suite wasm`; inspect real format tests' Properties finally blocks. Do not introduce a mocked fetch in the real suite.
- **Evidence:**
  - T04a: In `Design-Client.md` and `Design-WASM.md` (EN & zh-CN), clarified that `Cache` is an internal private Rust struct and not exported to JS; updated `.free()` documentation to note `ClientConfig` consumption on transfer.
  - T04b: In `zh-CN` and `zh-TW` `WASM-Memory-Management.md`, clarified that `ClientConfig` is consumed by `new Client(config)` and must not be freed after transfer, while untransferred configs must be freed.
  - Repository-wide ownership search `rg -n 'new Client|ClientConfig|\.free\(|cache\.free' README.md README_zh.md docs/wiki` verified completely consistent.

## T05 — Record the ownership evidence

- [x] Complete T05.
- **Depends on:** T04a/T04b.
- **Files (4):** `spec/supporting/contracts.md`, `spec/supporting/research.md`, `spec/supporting/traceability.md`, this `tasks.md`.
- **Contracts:** all package AC/SC.
- **Work:** record generated package execution and mark the documentation discrepancy corrected at its actual revision; preserve historical observations and unrelated D-* policies.
- **Acceptance:** evidence names actual commands/cases; all current ownership occurrences were reviewed; no claims of measured leak freedom or browser certification.
- **Verify:** `git diff --check`; inspect completed FR/AC mappings and final scope.
- **Evidence:** Updated `spec/supporting/contracts.md`, `spec/supporting/research.md`, and `spec/supporting/traceability.md`. Verified full fast suite runs green (`scripts/test.sh fast`).

## Completion evidence

- **Generated package recipe and execution:**
  `wasm-pack build --target nodejs --dev --out-dir pkg`
  `node scripts/wasm_api_smoke.js pkg`
  Result: All WASM method export and ownership checks passed successfully.
- **Test cases in `scripts/wasm_api_smoke.js`:**
  1. Method exports: `namespace`, `add_listener`, `preload`, `refresh`, `start`, `stop`.
  2. Constructor exports: `Client`, `ClientConfig`.
  3. Canonical doc snippet extraction and execution: `<!-- apollo-example: wasm-ownership -->` from `docs/wiki/en/WASM-Memory-Management.md`.
  4. Untransferred `ClientConfig` freed via `config.free()`.
  5. Consumed `ClientConfig` throws if `config.free()` called; `client.free()` cleans up properly.
  6. Application sentinel error preserved through `try ... finally { if (client) client.free(); }`.
  7. Constructor validation failure with consumed-config handling preserved without masking.
- **Language audit:**
  All current occurrences of `new Client`, `ClientConfig`, and `.free()` verified across `README.md`, `README_zh.md`, and all `docs/wiki` pages (en, zh-CN, zh-TW).
- **Test suite validation:**
  `scripts/test.sh fast`: 49 unit tests + 4 public errors tests (native-tls), 49 unit tests + 4 public errors tests (rustls), 38 doc tests, 10 WASM unit tests, and 0 clippy warnings across all targets.
