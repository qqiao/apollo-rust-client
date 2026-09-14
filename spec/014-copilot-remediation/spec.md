# Specification: GitHub Copilot Review Remediation

**Feature ID:** `014-copilot-remediation`  
**Status:** Implemented and verified.
**Source:** GitHub Copilot Review on PR [#133](https://github.com/qqiao/apollo-rust-client/pull/133) (commit `8ee392f387aaa51f53d232807c0b5c621a16a67a`).  
**Baseline contracts:**
- [009 — Observable test cleanup](../009-observable-test-cleanup/spec.md)
- [010 — Fixture body deadlines](../010-fixture-body-deadlines/spec.md)
- [011 — Executable public docs](../011-executable-public-docs/spec.md)
- [013 — Review follow-up](../013-review-followup/spec.md)

---

## 1. Scope and Context

This specification defines the scoped remediation plan addressing the findings generated across GitHub Copilot review rounds on PR #133: the initial baseline of eight findings (F1–F8: two inline comments and six review-detail findings) and subsequent follow-up findings and dispositions (F9–F18). All findings target test lifecycle supervision, fixture test timeout diagnostics, tooling error reporting, and Markdown documentation validation scripts.

No production Rust or WASM client library runtime code is modified. No external dependencies are introduced.

### Copilot Findings Inventory & Disposition

| ID | Location | Copilot Severity | Finding Summary | Proposed Disposition |
|---|---|---|---|---|
| **F1** | `scripts/apollo-fixtures.mjs:93-99` | Critical | Timeout diagnostic embeds raw URL, risking disclosure of userinfo credentials or sensitive query parameters. | **Accept**: Implement URL sanitization redacting userinfo and sensitive query params (`token`, `secret`, `key`, `password`, `auth`). |
| **F2** | `scripts/check-doc-links.mjs:62-75` | Moderate | Toggling a boolean on 3+ backticks closes a longer outer fence on a shorter inner fence, misclassifying code blocks as links. | **Accept**: Align with `scripts/check-doc-examples.mjs` fence tracking (matching delimiter character and minimum opening length). |
| **F3** | `scripts/check-doc-links.mjs:110-120` | Moderate | Angle-bracket destinations `<path with spaces.md>` are prematurely truncated at first space after removing outer brackets. | **Accept**: Parse angle-bracket destinations separately prior to optional title splitting. |
| **F4** | `scripts/check-doc-links.mjs:122` | Moderate | Protocol-relative URLs (`//docs.example.com/...`) are not recognized as external and are treated as broken local file paths. | **Accept**: Expand external URL regex to recognize protocol-relative `//` URLs. |
| **F5** | `scripts/apollo-test.sh:160-163` | Moderate | CLI recovery executes `exit $?` after `run_recovery_cleanup` sets `LIFECYCLE_PHASE="finished"`, dropping late `INTERRUPTED_STATUS`. | **Accept**: Capture recovery exit code and verify `INTERRUPTED_STATUS` before exiting. |
| **F6** | `scripts/apollo-test-lifecycle.sh:26-38` | Moderate | `run_with_timeout` and `cleanup` only check leader PID `kill -0 "$child_pid"`; orphaned descendants in process group can remain alive. | **Accept**: Probe process group `kill -0 -- "-$child_pid"` and ensure termination signals drain the group. |
| **F7** | `scripts/apollo-test-lifecycle.sh:267-285` | Moderate | In `run_recovery_cleanup`, a signal arriving after `final_status` is computed but before `LIFECYCLE_PHASE="finished"` loses signal exit code. | **Accept**: Atomic transition or evaluate `INTERRUPTED_STATUS` at the final transition point before returning. |
| **F8** | `scripts/apollo-test-lifecycle.sh:19-24` | Moderate | Race window between background spawn `"$@" &` and `CURRENT_CHILD_PID="$child_pid"` allows interrupt to skip group termination. | **Accept**: Defer signal handling across spawn-and-assign sequence or immediately register PID so cleanup terminates the child. |
| **F9** | `scripts/apollo-fixtures.mjs:52` (Inline) | Critical | Timeout message discloses common access secrets (`appSecret`, `clientSecret`, `apiKey`, `accessKey`, `authorization`). | **Accept**: Extend shared redaction policy to case/hyphen/underscore forms and preserve ordinary query params (R3-2). |
| **F10** | `scripts/check-doc-links.mjs:114` (Suppressed) | Moderate | Inline-link matcher stops at first `)`, truncating destinations with balanced parens (`guide_(v2).md`). | **Accept**: Implement linear scanning parser for balanced parens, escapes, angle brackets, and titles (R3-3). |
| **F11** | `scripts/check-doc-examples.mjs:287` (Suppressed) | Moderate | Compiler failures report generic error without mapping failing binary to source document, marker ID, or line. | **Accept**: Associate binary names with source example context and report on failure (R3-4). |
| **F12** | `spec/014-copilot-remediation/plan.md:173` (Suppressed) | Minor | Documented `LIFECYCLE_SPAWN_HOOK` does not match implemented `__lifecycle_test_spawn_hook` under `LIFECYCLE_TEST_MODE=1`. | **Accept**: Correct hook name and activation gate in plan.md (R3-5). |
| **F13** | `spec/014-copilot-remediation/spec.md:4` (Suppressed) | Minor | Handoff entry points stop at package 013, making package 014 undiscoverable. | **Accept**: Index package 014 in `spec/README.md` and `spec/review-remediation/README.md` (R3-5). |
| **F14** | `spec/014-copilot-remediation/tasks.md:20,39,58` (Suppressed) | Minor | Hard-coded test counts in completion records conflict with actual source test counts. | **Accept**: Replace stale counts with count-independent criteria (R3-5). |
| **F15** | `spec/verification-2026-09-13.md:7` (Suppressed) | Minor | Audit claims 43 Markdown files checked, omitting newly added package 014 files. | **Accept**: Explicitly scope the historical 43-document audit while noting package 014 documents (R3-5). |
| **F16** | `tests/apollo/README.md:235` (Suppressed) | Minor | Signal during spawning phase was documented as general deferral instead of aborting after child registration; `spawning` omitted from phase list. | **Accept**: Document spawning as deferred until registration followed by abort, and add `spawning` to phase inventory (R3-5). |
| **F17** | `scripts/check-doc-links.mjs` (CI Failure) | Critical | Six broken links from maintainer's intentional deletion of transient review files in `4ae64a6`. | **Accept**: Preserve deletions; repair references with durable documentation and historical commit links (R3-1). |
| **F18** | `spec/014-copilot-remediation/` (Suppressed) | Obsolete | Prior comments regarding `HANDOFF.md` reconciliation rendered obsolete by intentional deletion of `HANDOFF.md` in `4ae64a6`. | **Obsolete**: Preserved deletion of `HANDOFF.md`; package 014 indexed in remaining durable docs (R3-1, R3-5). |

---

## 2. Stories and Acceptance Scenarios

### Story 1: Redact Sensitive Credentials in Fixture Timeout Diagnostics (P1)
*(Addresses Finding F1)*

**Independent Test:** Run unit tests against `requestJson` timeout logic with mock URLs containing basic auth userinfo and sensitive query parameters.

- **AC-001:** Given a fixture request URL containing basic auth credentials (`http://user:secret@host/path`), when the request times out, then the diagnostic error message contains the URL with credentials redacted (e.g. `http://***:***@host/path` or userinfo removed).
- **AC-002:** Given a fixture request URL containing sensitive query parameters (such as `token`, `secret`, `key`, `password`, or `auth`), when the request times out, then the query parameter values are replaced with `REDACTED` in the diagnostic message.
- **AC-003:** Given an ordinary fixture request URL without credentials or sensitive query parameters, when the request times out, then the protocol, host, path, and non-sensitive query parameters remain intact and accurately reported.

---

### Story 2: CommonMark-Compliant Documentation Link Parsing (P1)
*(Addresses Findings F2, F3, F4)*

**Independent Test:** Run `tests/tooling/doc-links.test.mjs` against synthetic Markdown fixtures containing nested code fences, angle-bracket links with spaces, and protocol-relative links.

- **AC-004:** Given a Markdown document with an outer code fence of $N$ backticks or tildes containing inner code fences of $M < N$ characters, when `extractLinks` runs, then code lines inside the outer fence are skipped, and links after the matching $N$-character close fence are parsed normally.
- **AC-005:** Given a link destination enclosed in angle brackets with spaces (e.g. text `guide` with destination `<docs/Getting Started.md>` or `<docs/Getting Started.md> "Guide Title"`), when `checkFileLinks` runs, then the destination path is resolved as `docs/Getting Started.md` without space truncation, correctly finding the file if it exists.
- **AC-006:** Given an external protocol-relative link (e.g. text `example` with destination `//docs.example.com/guide` or `<//docs.example.com/guide>`), when `checkFileLinks` runs, then it is recognized as external and excluded from local filesystem checks.

---

### Story 3: Comprehensive Process Group Supervision and Race-Free Signal Handling (P1)
*(Addresses Findings F5, F6, F7, F8)*

**Independent Test:** Run `tests/tooling/apollo-lifecycle.test.mjs` with stubbed Docker and spawned descendant processes under SIGINT and SIGTERM conditions.

- **AC-007:** Given a supervised command initiated by `run_with_timeout`, when SIGINT or SIGTERM arrives in the window during process spawning before `CURRENT_CHILD_PID` assignment, then the signal is handled safely, the child PID is registered, and teardown cleanup terminates the spawned process group without leaving orphaned processes.
- **AC-008:** Given a supervised command whose leader process exits prematurely while leaving a background descendant running in its process group, when the timeout or cleanup loop runs, then `run_with_timeout` and `cleanup` detect the surviving process group (`-$child_pid`), terminate it with SIGTERM/SIGKILL, and ensure no orphaned processes survive.
- **AC-009:** Given `run_recovery_cleanup` executing teardown, when SIGINT or SIGTERM arrives at any point before return, then `INTERRUPTED_STATUS` (130 or 143) is captured and preserved as the final exit status.
- **AC-010:** Given CLI recovery `scripts/apollo-test.sh cleanup --run-dir ...`, when SIGINT or SIGTERM arrives before exit, then the process exits with `INTERRUPTED_STATUS` (130 or 143) rather than exit code 0.

---

## 3. Functional Requirements

- **FR-001:** Fixture HTTP request timeout errors MUST NOT leak basic authentication userinfo or sensitive query parameter values.
- **FR-002:** Markdown code fence tracking in `scripts/check-doc-links.mjs` MUST conform to CommonMark fence matching rules (matching opening character, minimum opening length, and ignoring inner fences).
- **FR-003:** Angle-bracket Markdown link destinations MUST support embedded spaces and separate title attributes per CommonMark specifications.
- **FR-004:** The link checker MUST recognize protocol-relative external URLs beginning with `//` and exclude them from local file validation.
- **FR-005:** Signal trapping during child process launch MUST eliminate unmonitored windows where a process could be spawned without its group being tracked.
- **FR-006:** Process supervision MUST probe the entire process group (`-$child_pid`) to detect and terminate orphaned descendants whose group leader has exited.
- **FR-007:** `run_recovery_cleanup` and the CLI recovery caller MUST enforce first-signal exit status precedence (130 for SIGINT, 143 for SIGTERM) over successful recovery status.
- **FR-008:** All changes MUST preserve existing test results, timeout budgets, ownership safety checks, and zero-leak requirements.

---

## 4. Success Criteria

- **SC-001:** All tooling unit tests pass (`node --test tests/tooling/*.test.mjs`), including regression test cases covering F1 through F8 and post-review remediations.
- **SC-002:** Fast check suite `scripts/test.sh fast` passes with zero regressions across Rust, Clippy, WASM, and Node tooling.
- **SC-003:** Real Apollo native integration suite `scripts/test.sh integration --suite native` passes with clean setup and complete teardown.
- **SC-004:** `git diff --check` reports zero whitespace or formatting issues.
