# Tasks: GitHub Copilot Review Remediation

**Status:** Plan awaiting HumanReviewer approval before implementation. Read [spec](spec.md), then [plan](plan.md).  
Tasks below reflect the proposed implementation sequence upon approval.

---

## T01 — Redact sensitive credentials in fixture timeout diagnostics (F1)

- [ ] Complete T01.
- **Depends on:** none.
- **Files (2):** `scripts/apollo-fixtures.mjs`, `tests/tooling/apollo-http.test.mjs`.
- **Contracts:** FR-001; AC-001, AC-002, AC-003.
- **Work:**
  - Implement `redactUrl(rawUrl)` in `scripts/apollo-fixtures.mjs` to sanitize userinfo (HTTP basic credentials) and sensitive query parameters (`token`, `secret`, `key`, `password`, `auth`).
  - Use `redactUrl` in `requestJson` when formatting `TimeoutError` messages.
  - Add unit tests in `tests/tooling/apollo-http.test.mjs` verifying redaction of basic auth userinfo and sensitive query values, while preserving standard URLs, paths, and benign parameters.
- **Acceptance:**
  - Timeout diagnostics omit passwords and tokens without hiding request method, host, path, or non-sensitive query attributes.
  - `node --test tests/tooling/apollo-http.test.mjs` passes cleanly.

---

## T02 — Robust CommonMark link checking and URL parsing (F2, F3, F4)

- [ ] Complete T02.
- **Depends on:** none.
- **Files (2):** `scripts/check-doc-links.mjs`, `tests/tooling/doc-links.test.mjs`.
- **Contracts:** FR-002, FR-003, FR-004; AC-004, AC-005, AC-006.
- **Work:**
  - In `scripts/check-doc-links.mjs`, replace boolean `inCodeBlock` toggle with matching fence delimiter tracking (char and minimum length), aligning with `scripts/check-doc-examples.mjs`.
  - Parse `<destination>` angle-bracket links prior to title splitting to support embedded spaces (e.g. `<docs/Getting Started.md>`).
  - Include protocol-relative URLs (`//...`) in external URL ignore regex `/^(https?:|mailto:|ftp:|data:|\/\/)/i`.
  - Add test cases in `tests/tooling/doc-links.test.mjs` verifying nested fence skipping, angle-bracket links with spaces, and protocol-relative URLs.
- **Acceptance:**
  - Four-backtick code blocks containing three-backtick examples do not leak inner links.
  - Valid local links in angle brackets with spaces resolve to existing files without false positives.
  - Protocol-relative URLs are excluded from local file existence checks.
  - `node --test tests/tooling/doc-links.test.mjs` passes cleanly.

---

## T03 — Process group supervision and signal race elimination (F5, F6, F7, F8)

- [ ] Complete T03.
- **Depends on:** none.
- **Files (3):** `scripts/apollo-test-lifecycle.sh`, `scripts/apollo-test.sh`, `tests/tooling/apollo-lifecycle.test.mjs`.
- **Contracts:** FR-005, FR-006, FR-007, FR-008; AC-007, AC-008, AC-009, AC-010.
- **Work:**
  - Add transient `spawning` phase state in `run_with_timeout` to eliminate signal race during child process spawn and registration.
  - Probe both leader and process group (`kill -0 "$child_pid" || kill -0 -- "-$child_pid"`) in `run_with_timeout` and `cleanup` to detect and terminate surviving descendants.
  - In `run_recovery_cleanup`, evaluate `final_status` at the `finished` phase boundary to ensure late signals preserve first-signal status.
  - In `scripts/apollo-test.sh:160-163`, capture recovery return code and check `INTERRUPTED_STATUS` before exiting so interrupted recovery exits 130 or 143.
  - Add regression test cases in `tests/tooling/apollo-lifecycle.test.mjs` verifying process group descendant reaping and recovery interrupt exit codes.
- **Acceptance:**
  - No background subprocess is orphaned if group leader exits prematurely.
  - Signals during child spawn or recovery cleanup guarantee non-zero signal exit codes (130/143) and reaped process groups.
  - `node --test tests/tooling/apollo-lifecycle.test.mjs` passes all tests cleanly with zero process leaks.

---

### Checkpoint A

- [ ] All focused tooling tests pass:
  - `node --test tests/tooling/apollo-http.test.mjs`
  - `node --test tests/tooling/doc-links.test.mjs`
  - `node --test tests/tooling/apollo-lifecycle.test.mjs`

---

## T04 — Fast and integration test verification

- [ ] Complete T04.
- **Depends on:** T01, T02, T03.
- **Files:** No new code; test verification only.
- **Contracts:** SC-001, SC-002, SC-003, SC-004.
- **Work:**
  - Run `scripts/test.sh fast` (Clippy, Rust unit/doc tests, WASM unit tests, tooling tests, doc links, doc examples).
  - Run `scripts/test.sh integration --suite native` to verify real Apollo test execution, idempotency, and clean teardown.
  - Run `git diff --check` to ensure no whitespace errors.
- **Acceptance:**
  - `scripts/test.sh fast` exits 0.
  - `scripts/test.sh integration --suite native` exits 0 with 0 container or volume leaks.
  - `git diff --check` is clean.

---

### Checkpoint B

- [ ] Full local verification passes without regressions.

---

## T05 — Status audit and specification index reconciliation

- [ ] Complete T05.
- **Depends on:** T04.
- **Files (2):** `spec/README.md`, `spec/014-copilot-remediation/spec.md`.
- **Contracts:** FR-008.
- **Work:**
  - Update `spec/README.md` review remediation section to record package 014 and its verified scope.
  - Update `spec/014-copilot-remediation/spec.md` and `tasks.md` status with concrete test evidence upon completion.
