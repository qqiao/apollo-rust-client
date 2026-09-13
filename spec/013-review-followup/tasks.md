# Tasks: Review follow-up

**Status:** All tasks completed and verified. Read [spec](spec.md), then [plan](plan.md). Check tasks only after their acceptance criteria pass. Preserve baseline task history; record new evidence here.

## T01 — Correct signals during automatic cleanup

- [x] Complete T01.
- **Depends on:** none.
- **Files (2):** `scripts/apollo-test-lifecycle.sh`, `tests/tooling/apollo-lifecycle.test.mjs`.
- **Contracts:** FR-001–003/008; AC-001/002/004.
- **Work:** add the plan's ready-file/PID signal regressions; reproduce the orphaned child; implement protected cleanup phase and first-signal status handling; make interrupted waits safe without resetting deadlines.
- **Acceptance:** INT/TERM during teardown and mixed repeated signals during cleanup/diagnostics finish boundedly, preserve first signal, call down once, and leave no live child/descendants. Existing no-signal matrix remains green. Test failure paths have watchdogs and unconditional cleanup.
- **Verify:** Bash syntax and focused lifecycle tests. Record failing pre-fix assertion, passing test names, elapsed bounds, and process-liveness checks below.
- **Evidence:**
  - Pre-fix regression failure: Signal during automatic teardown returned 143 with child process group left running (`AssertionError: Supervised process 91574 must be terminated and reaped: true !== false`). Mixed repeated signal (SIGINT then SIGTERM) allowed SIGTERM to outrank SIGINT (`AssertionError: First signal outranked: 143 !== 130`).
  - Fix: Implemented lifecycle state machine (`LIFECYCLE_PHASE="work" | "cleanup" | "finished"`, `CLEANUP_IN_PROGRESS=0 | 1`, and `INTERRUPTED_STATUS=0 | 130 | 143`). In `handle_signal`, recorded first signal status and deferred signals during cleanup/finished without premature exit. Added `sleep || true` guards in `run_with_timeout`. Guarded cleanup reentrancy and ensured process group termination and reaping.
  - Verified: `bash -n scripts/apollo-test-lifecycle.sh` passed. `node --test tests/tooling/apollo-lifecycle.test.mjs` passed all 21 tests (including 4 new regression tests) in 15.6s with zero leaked processes.

## T02 — Apply the same guarantees to recovery

- [x] Complete T02.
- **Depends on:** T01.
- **Files (4):** lifecycle helper, `scripts/apollo-test.sh`, lifecycle tests, `tests/apollo/README.md`.
- **Contracts:** FR-001–003/008; AC-003/004; SC-001.
- **Work:** route explicit recovery through protected supervision and final signal precedence; test the actual CLI with stub Docker; document deferred/repeated-signal behavior and unchanged timeout/grace.
- **Acceptance:** recovery INT/TERM obey the same final-status and no-orphan guarantees; invalid ownership still prevents deletion; no extra automatic down occurs. Failure output includes useful scoped diagnostics.
- **Verify:** focused lifecycle tests and shell syntax. The real-run check is owned by T04.
- **Evidence:**
  - Pre-fix regression failure: `scripts/apollo-test.sh cleanup --run-dir` interrupted by signal aborted execution, leaving supervised process alive (`Supervised process 91664 must not remain alive: true !== false`).
  - Fix: `run_recovery_cleanup` transitions to `LIFECYCLE_PHASE="cleanup"`, applies signal precedence to final exit code, terminates tracked child groups, and prevents reentrant teardown. Documented in `tests/apollo/README.md`.
  - Verified: `bash -n scripts/apollo-test.sh` passed. `node --test tests/tooling/apollo-lifecycle.test.mjs` verified interrupted CLI recovery exits 143/130 with process reaped and single down execution.

### Checkpoint A

- [x] Automatic and recovery signal matrices pass without real Docker or leaked processes.
  - Evidence: `node --test tests/tooling/apollo-lifecycle.test.mjs` passed all 21 tests with zero background processes remaining.

## T03 — Repair and verify the actual Markdown fences

- [x] Complete T03.
- **Depends on:** none technically; run after T02 for the default serial assignment.
- **Files (3):** `docs/wiki/en/Error-Handling.md`, `scripts/check-doc-examples.mjs`, `tests/tooling/doc-examples.test.mjs`.
- **Contracts:** FR-004–006/008; AC-005–008; SC-002.
- **Work:** reproduce required inventory accepting the hidden marker; fix fence state tracking; add the plan's negative and positive fixtures; close the preceding real documentation fence.
- **Acceptance:** embedded literal markers cannot satisfy inventory; valid matching backtick/tilde fences and CRLF work; current four snippets compile for both native configurations; unavailable-symbol control fails; copied public-error code contains no marker/fence artifacts.
- **Verify:** focused doc tests, both example compilations, link checker, and source/rendered-fence inspection. Record exact pre-fix failure reason and repaired result.
- **Evidence:**
  - Pre-fix regression failure: `extractSnippets` accepted markers inside unclosed fences (`extractSnippets ignores markers placed inside active code fences`), failing CommonMark negative fixture checks. On CRLF input, trailing carriage returns bypassed ordinary fence tracking causing hidden markers to be accepted. Marked openers with invalid backtick info strings (e.g. ```` ```rust` ````) bypassed ordinary fence checks and compiled.
  - Fix: Unified opening fence parser `parseOpeningFence` for both ordinary and marked fences in `scripts/check-doc-examples.mjs`, validating delimiter runs, matching length, and backtick restrictions in info strings before selecting Rust. Normalized line splitting with `/\r?\n/`. Closed the unclosed code fence after `get_config()` in `docs/wiki/en/Error-Handling.md`.
  - Verified: `node --test tests/tooling/doc-examples.test.mjs` passed all tests (including end-to-end LF/CRLF `validateInventory` fixture test and invalid marked opener regression). `node scripts/check-doc-examples.mjs` compiled all 4 examples under both native-tls and rustls. `node scripts/check-doc-links.mjs` checked 441 links across 82 files with 0 broken links. All tooling unit tests passed in `scripts/test.sh fast`.

## T04 — Verify combined corrections

- [x] Complete T04.
- **Depends on:** T02/T03.
- **Files:** this task evidence only; no speculative implementation changes.
- **Contracts:** AC-001–008; SC-001/002.
- **Work:** run `scripts/test.sh fast`, then `scripts/test.sh integration --suite native`; inspect normal teardown log and command status. Run `git diff --check`.
- **Acceptance:** required checks pass, new scenarios actually execute, and the exact disposable project is removed. Environment restrictions are recorded as gaps, not green results or product bugs.
- **Evidence:**
  - `git diff --check`: Clean (0 whitespace/formatting errors).
  - `scripts/test.sh fast`: Exit 0. 49 native unit tests, 4 public error tests, 38 doc tests, 10 wasm tests, 43 tooling unit tests, 4 compiled examples (native-tls and rustls) all passed cleanly.
  - `scripts/test.sh integration --suite native`: Exit 0. Ephemeral Compose project `apollo-test-1789283757-30de25e0` provisioned, seeded idempotently, ran 6 real Apollo integration tests in 3.34s (`real_apollo_access_key`, `real_apollo_formats_and_identity`, `real_apollo_grayscale`, `real_apollo_polling`, `real_apollo_preload_and_persistence`, `real_apollo_release_refresh_and_listener`), and cleanly tore down project and volumes with 0 containers leaked.

### Checkpoint B

- [x] Runtime tooling corrections and documentation verification pass together.
  - Evidence: Both `scripts/test.sh fast` and `scripts/test.sh integration --suite native` exited 0.

## T05 — Reconcile package status in small batches

- [x] Complete T05a (006–008 specification status lines).
- [x] Complete T05b (009–012 specification status lines).
- [x] Complete T05c (001–005 scoped amendment/status paragraphs).
- **Depends on:** T04.
- **Files:** at most five specifications per subtask; no requirement ID renumbering.
- **Contracts:** FR-007/008; AC-009/010.
- **Work:** inspect each current claim; mark implemented baseline accurately and close only follow-up gaps supported by T01–T04. Retain draft/approval qualifiers and unrelated deferred requirements.
- **Acceptance:** no current package is simultaneously described as not started and complete; no historical test evidence is presented as a new run; 009/011 follow-up resolution cites this package's actual checks.
- **Verify:** contextual status search and manual comparison with task evidence. No runtime rerun for prose alone.
- **Evidence:**
  - T05a: Updated `spec/006-*/spec.md`, `spec/007-*/spec.md`, `spec/008-*/spec.md` status to "Implemented and verified in baseline remediation".
  - T05b: Updated `spec/009-*/spec.md`, `spec/010-*/spec.md`, `spec/011-*/spec.md`, `spec/012-*/spec.md` status, noting follow-up resolution in 013 for 009 and 011.
  - T05c: Updated scoped amendments in `spec/001-read-configuration/spec.md`, `spec/002-retain-configuration/spec.md`, and `spec/005-real-apollo-testing/spec.md` to reflect completed implementations.

## T06 — Reconcile supporting evidence

- [x] Complete T06.
- **Depends on:** T05a–c.
- **Files (4):** `spec/supporting/{contracts,research,traceability}.md`, `spec/review-remediation/re-review.md`.
- **Contracts:** FR-007/008; AC-009/010.
- **Work:** update current narrow contract/traceability claims; append a dated V1/V2/V3 resolution section to the review with test/evidence references. Preserve the original review and historical planning facts.
- **Acceptance:** a reader can trace each finding to new evidence and distinguish review history from current resolution; deferred D-* policies and unrelated implementation remain untouched.
- **Verify:** contextual searches, local-link validation, and `git diff --check`.
- **Evidence:**
  - `spec/supporting/contracts.md`: Updated "Remediation deltas (2026-09-13; implemented and verified)".
  - `spec/supporting/traceability.md`: Added package 013 traceability row and updated status headers.
  - `spec/review-remediation/re-review.md`: Appended dated "Remediation resolution (2026-09-13)" section covering root causes, fixes, and verification evidence for V1, V2, and V3.

## T07 — Close the handoff

- [x] Complete T07.
- **Depends on:** T06.
- **Files (5):** `HANDOFF.md`, `spec/README.md`, `spec/review-remediation/README.md`, this package's `spec.md` and `tasks.md`.
- **Contracts:** all FR/AC/SC, with AC-009/010 status audit.
- **Work:** reconcile entry points and record final package status. Keep links to review, resolution, and historical packages. Report behavior changed, checks, and gaps; do not claim independent approval.
- **Acceptance:** all required tasks and checkpoints above have evidence; no root prompt sends an executor back to already implemented 006; no completion claim contradicts a pending task.
- **Verify:** local links and whitespace, final scope diff. Preserve optional suggestions as optional.
- **Evidence:**
  - Reconciled `HANDOFF.md`, `spec/README.md`, `spec/review-remediation/README.md`, and `spec/013-review-followup/{spec.md,tasks.md}`.
  - Validated local links and run diff check.

## Coverage map

| Acceptance | Tasks |
|---|---|
| AC-001/002 | T01, T04 |
| AC-003 | T02, T04 |
| AC-004 | T01/T02, T04 |
| AC-005–008 | T03, T04 |
| AC-009/010 | T05a–c, T06, T07 |

## Copyable assignment

> Implement `spec/013-review-followup` in this checkout. Read HANDOFF.md, the verification review, and this package's spec.md, plan.md, and tasks.md. Execute T01–T07 in order, including both checkpoints. Reproduce the two gaps with deterministic regressions before fixing them; preserve existing contracts and unrelated changes. Record actual verification evidence and reconcile current completion records. Work only on V1–V3; do not reimplement packages 006–012, add dependencies, publish, push, or merge. Ask only if a material ambiguity blocks the assigned work. Finish by reporting changes, checks, and unresolved gaps.


## Final parser follow-up verification (2026-09-13)

After the second review, extraction now normalizes LF/CRLF and uses one opening-fence recognizer for ordinary and marked blocks. The added inventory fixture rejects the original hidden-marker shape under both line endings and accepts repaired examples; invalid backtick info strings fail before compilation. Independent reproduction probes and the current fast suite pass (including documentation tests). All 19 real Apollo tests and fresh generated ownership smoke also pass. See the [current audit](../verification-2026-09-13.md) for exact verification scope and remaining non-blocking coverage limits.
