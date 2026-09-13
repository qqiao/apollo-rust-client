# Feature Specification: Complete fixture request deadlines

**Feature ID:** `010-fixture-body-deadlines`
**Created:** 2026-09-13
**Status:** Specified; implementation not started.
**Review:** R5 / P2. Baseline: [005](../005-real-apollo-testing/spec.md), 005/FR-010; execution follows package 009.

## Purpose and scope

Contributors running fixture setup, verification, or mutations must receive a bounded request failure when Apollo sends headers but never finishes its body. A five-second fixture request deadline must not silently become a 300-second stage timeout.

In scope: the Node fixture HTTP helper, timer lifetime, timeout diagnostics, response-shape compatibility, deterministic local fault tests, and fast-check integration. Out of scope: changing production Rust/WASM deadlines, adding retries, changing Apollo protocol/authentication/fixtures, introducing response-size or CPU parsing limits, and changing overall stage deadlines.

## Stories and acceptance scenarios

### Story 1 — Bound a complete network response (P1)

As a contributor, I want fixture commands to stop waiting on stalled headers or bodies.

**Independent test:** A local Node HTTP server deliberately withholds headers or sends/flushed headers and withholds the rest of the body; call the actual fixture request helper.

- **AC-001:** Given a server withholding headers and deadline T, when a fixture request runs, then it rejects through that deadline without waiting for the server to finish.
- **AC-002:** Given headers received before T and a body that never completes, when T elapses from request start, then the request rejects and the underlying fetch/body operation is aborted. Headers do not restart or cancel the deadline.
- **AC-003:** Given a body completing within T, when consumed, then the helper returns the same status/headers/data/ok representation as before and clears its timer.
- **AC-004:** Given malformed JSON, an empty body, or non-success HTTP status completing within T, when read, then existing semantics remain: malformed JSON is returned as text, empty data is null, and non-success status is returned for callers to interpret rather than automatically retried or thrown merely for its status.

### Story 2 — Diagnose timeout without leaking credentials (P2)

As a contributor, I want the error to identify the stalled operation without exposing signing headers or request payloads.

**Independent test:** Exercise timeout and ordinary transport failure and inspect errors and final cleanup.

- **AC-005:** Given a fixture timeout, when reported, then its diagnostic identifies method, URL, and configured duration while omitting Authorization headers, access secrets, and request bodies.
- **AC-006:** Given an ordinary transport/body-read failure before the deadline, when reported, then it remains a transport failure rather than being relabeled timeout; timers are cleared on both success and failure.
- **AC-007:** Given existing seed/verify/mutation operations, when run against disposable real Apollo, then their existing behavior and positive/negative controls still pass with the complete deadline.

## Functional requirements

- **FR-001:** One deadline MUST cover asynchronous fetch waiting from request start through complete response text consumption, including headers and body.
- **FR-002:** The deadline MUST abort the actual request/body operation, not only reject a competing Promise while leaving networking alive.
- **FR-003:** Timeout resources MUST be cleared on every settled path; an already completed request MUST NOT be aborted later by a leftover timer.
- **FR-004:** Default deadline MUST remain 5000 ms, with existing internal `timeoutMs` override available for bounded fault tests; no new user-facing CLI timing option is required.
- **FR-005:** Response classification and fixture retry policies MUST remain unchanged except that stalled body reads now terminate.
- **FR-006:** Timeout diagnostics MUST be contextual and must not print authentication headers or payloads; ordinary non-timeout failures MUST retain their cause/category.
- **FR-007:** Fast mode MUST run these HTTP fault tests without Docker and without leaking fetch replacements into real Node/WASM integration.

## Entities and success criteria

**Entities:** complete request operation, AbortController, single deadline, returned response record, timeout diagnostic.

- **SC-001:** AC-001/002 reject via a short test override well before the server's external watchdog; held connections are closed during cleanup.
- **SC-002:** AC-003–006 preserve result/error semantics, do not leave timers/servers alive, and expose no fixture secret in timeout diagnostics.
- **SC-003:** AC-007 passes through the existing integration orchestration without changing image/fixture versions or production request code.

## Assumptions

Timers cannot preempt synchronous CPU parsing; this is a network-wait deadline. Individual requests are bounded while aggregate retry/seed stages retain their existing outer deadlines. Only valid internal positive finite timeout values are needed; if validating invalid overrides, reject explicitly rather than silently disable the deadline. See [plan](plan.md) and [tasks](tasks.md).
