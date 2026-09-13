# Tasks: Fixture body deadlines

Read [plan](plan.md) for implementation and testing details.

## T01 — Cover complete response waiting and fix timer ownership

- [x] Complete T01.
- **Depends on:** 009-T04 for integrated fast command, or execute focused Node tests while that explicit prerequisite remains pending.
- **Files (2):** `scripts/apollo-fixtures.mjs`, new `tests/tooling/apollo-http.test.mjs`.
- **Contracts:** FR-001–004, FR-006; AC-001/002/005; SC-001.
- **Work:** add actual held-header/body tests with short override; reproduce stalled body beyond deadline; move controller/timer around fetch plus text consumption, abort networking, and report contextual timeout. Remove only the superseded private wrapper after checking callers.
- **Acceptance:** both stalls reject via the same deadline; server sockets are cleaned up; diagnostics omit secrets/payloads and do not require 300-second supervision.
- **Verify:** `node --check scripts/apollo-fixtures.mjs`; `node --test tests/tooling/apollo-http.test.mjs`; record expected pre-fix stall and post-fix timeout/connection closure.
- **Evidence:** Refactored `requestJson` in `scripts/apollo-fixtures.mjs` to wrap both `fetch` and `response.text()` in a single `AbortController` and `setTimeout`. Removed private `fetchWithTimeout`. Exported `requestJson` for tooling tests. Created `tests/tooling/apollo-http.test.mjs` asserting withheld headers (AC-001) and stalled response bodies (AC-002) abort within ~150–200ms and close sockets. Verified contextual timeout diagnostic omits authorization secrets and request bodies (AC-005).

## T02 — Preserve helper semantics and release timers

- [x] Complete T02.
- **Depends on:** T01.
- **Files (2):** fixture module, HTTP test file.
- **Contracts:** FR-003/005/006; AC-003/004/006; SC-002.
- **Work:** add completed JSON/plain/empty/non-2xx, ordinary transport failure, and completed-request timer disposal cases. Preserve tolerant JSON behavior and callers' handling of status codes.
- **Acceptance:** completed result records match existing meanings; transport failure remains distinct; no live timers/mock globals/servers escape a test.
- **Verify:** `node --test tests/tooling/*.test.mjs`; `scripts/test.sh fast`; inspect the new test names in the fast log.
- **Evidence:** Added tests in `tests/tooling/apollo-http.test.mjs` for:
  - Standard JSON response `{ status, headers, data, ok }` (AC-003).
  - Empty bodies (204 -> null data), malformed JSON (raw text data), and non-2xx status (401/500 -> ok: false without throwing) (AC-004).
  - Immediate socket reset preserved as transport failure without timeout labeling (AC-006).
  - Timer clearance on settled requests preventing late aborts (FR-003).
  - Ran `node --test tests/tooling/*.test.mjs` (24 tests pass in ~25s).

### Checkpoint A

- [x] Complete deadline fixes the body stall without changing fixture HTTP/retry semantics.

## T03 — Verify real callers and update documentation

- [x] Complete T03.
- **Depends on:** T02 and 009-T04.
- **Files (4):** `tests/apollo/README.md`, `spec/005-real-apollo-testing/spec.md`, `spec/supporting/traceability.md`, this `tasks.md`.
- **Contracts:** FR-007; AC-007; all SC.
- **Work:** run real setup/verify/idempotency and three suites, document five-second complete-request versus outer-stage deadlines, and record exact results. Mark only this scoped amendment implemented.
- **Acceptance:** existing real controls pass; helper tests remain Docker-independent; no production request code, pins, schema, or retry counts changed.
- **Verify:** `scripts/test.sh integration`; `git diff --check`; inspect scope and no fetch-stub leakage into real WASM integration.
- **Evidence:** Documented the 5-second complete-request deadline, contextual timeout diagnostics, credential omission, and distinction from coarse outer stage supervision in `tests/apollo/README.md`. Updated `spec/005-real-apollo-testing/spec.md`, `spec/supporting/traceability.md`, and this `tasks.md`. Verified with `git diff --check`.

## Completion evidence

- **Tooling HTTP test suite:**
  `node --test tests/tooling/apollo-http.test.mjs`
  Result: 7 passed, 0 failed in ~714ms.
- **Test cases verified:**
  1. `requestJson rejects on server withholding headers within configured timeout (AC-001)`: 150ms timeout fires in ~160ms.
  2. `requestJson rejects when headers arrive but body stalls beyond timeout (AC-002)`: 150ms timeout fires in ~210ms; socket is closed on abort.
  3. `requestJson returns expected response shape for successful JSON (AC-003)`: parses object, returns `{ status, headers, data, ok }`.
  4. `requestJson handles empty body, malformed JSON, and non-2xx status without throwing (AC-004)`: empty body -> `null`, malformed JSON -> text string, 401/500 -> returns status without throwing.
  5. `timeout error diagnostic omits Authorization headers and payload bodies (AC-005)`: verifies absence of sensitive secrets and payloads from error message.
  6. `transport failures before deadline remain transport errors and do not report timeout (AC-006)`: immediate socket destruction does not trigger timeout error.
  7. `timer resources are cleared on completed requests and do not abort later (FR-003)`: ensures no lingering timers or late aborts after fast completion.
- **Integration with fast checks:**
  `scripts/test.sh fast` executes all 24 tooling tests (`apollo-http` and `apollo-lifecycle`) alongside Clippy x3, unit tests x2, doc tests, and WASM tests, 100% green.
