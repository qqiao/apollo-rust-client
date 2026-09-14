# Tasks: Observable test cleanup

Read [plan](plan.md) for exact exit handling, harness design, and diagnostics. Every shell edit must work with Bash 3.2 and set -e semantics.

## T01 — Preserve process supervision in a testable helper

- [x] Complete T01.
- **Depends on:** none.
- **Files (3):** `scripts/apollo-test.sh`, new `scripts/apollo-test-lifecycle.sh`, new `tests/tooling/apollo-lifecycle.test.mjs`.
- **Contracts:** FR-003/004; AC-001/004/006; SC-003.
- **Work:** extract only lifecycle functions without source-time side effects; guard wait status/bookkeeping; add success, nonzero-child, and short actual-timeout tests through the shared helper. Preserve main orchestration.
- **Acceptance:** source does not run Docker/install traps/exit; child state clears on nonzero completion; supervision terminates a held child within watchdog.
- **Verify:** `bash -n scripts/apollo-test.sh scripts/apollo-test-lifecycle.sh`; `node --test tests/tooling/apollo-lifecycle.test.mjs`; compare main suite-dispatch code before/after.
- **Evidence:** Extracted `scripts/apollo-test-lifecycle.sh` containing `run_with_timeout`, `teardown_project`, `cleanup`, `handle_signal`, `install_lifecycle_traps`, and `run_recovery_cleanup`. Verified Bash 3.2 syntax (`bash -n`). Verified with `tests/tooling/apollo-lifecycle.test.mjs`: helper sources without side-effects or traps, returns child status on success and failure while clearing `CURRENT_CHILD_PID`, and terminates hung commands with status 124 within bounded watchdog.

## T02 — Correct automatic cleanup result and diagnostics

- [x] Complete T02.
- **Depends on:** T01.
- **Files (3):** main script, lifecycle helper, lifecycle Node test.
- **Contracts:** FR-001/002/004–006; AC-002–007; SC-001.
- **Work:** implement explicit original status, one EXIT path, signal precedence, cached idempotent result, bounded teardown log/fallback, and recovery command. Add the full status-table and signal/path/failure tests; demonstrate old fail-open result before correction.
- **Acceptance:** 42/7/130/143/124 statuses obey precedence; failed down is visible and invoked once; unwritable logs neither hide errors nor skip down.
- **Verify:** lifecycle Node tests under outer watchdogs, shell syntax checks. Inspect stdout/stderr and argv assertions, not only process status.
- **Evidence:** Implemented status precedence: signals (130/143) > stage failure (7) > teardown failure (42/124) > diagnostic failure (1) > 0. Verified:
  - 0 + 42 teardown -> 42, logs failure and recovery command, suppresses success message.
  - 7 + 42 teardown -> 7, reports teardown failure while preserving original stage error.
  - 0 + timeout -> 124, records timeout diagnostic.
  - Unwritable log path -> status 1, teardown still runs.
  - SIGINT/SIGTERM -> 130/143, child terminated, teardown executed once.
  - Repeated `cleanup` calls -> teardown executed at most once, cached result returned.
  - Unowned runs -> no teardown invoked, error preserved.

### Checkpoint A

- [x] All automatic teardown fault cases are observable and bounded without real Docker.

## T03 — Reuse bounded teardown for explicit recovery

- [x] Complete T03.
- **Depends on:** T02.
- **Files (3):** main script, lifecycle helper, lifecycle Node test.
- **Contracts:** FR-001–006; AC-008; SC-001/003.
- **Work:** retain ownership validation, route recovery through the shared bounded function, prevent contradictory final banners, and test real recovery CLI with stub Docker and valid/invalid metadata.
- **Acceptance:** valid recovery reports true outcome; invalid metadata never deletes; missing run stays safe and timeout code uses the same tested supervisor.
- **Verify:** lifecycle tests; assert exact `-p` project and Compose path in stub argv; no implicit unscoped down/prune commands.
- **Evidence:** Unified `run_recovery_cleanup` to invoke `teardown_project` with timeout. Verified:
  - Valid ownership + success -> 0, passes `-p <project>` and Compose file.
  - Valid ownership + failure 42 -> 42 with error details.
  - Untrusted project prefix -> 1, refuses teardown without calling Docker.
  - Nonexistent run dir -> 0 safe fallback without calling Docker.
  - Spaces and single quotes in path -> handled safely and shell-escaped in recovery command.

## T04 — Wire fast checks and document operation

- [x] Complete T04.
- **Depends on:** T03 and package 007-T02 if following recommended order.
- **Files (3):** `scripts/test.sh`, `scripts/apollo-test.sh`, `tests/apollo/README.md`.
- **Contracts:** FR-007; AC-001–008; SC-002.
- **Work:** run `node --test tests/tooling/*.test.mjs` in both fast paths with failures propagated; preserve existing Rustls all-targets change if present. Document statuses, diagnostics, timeout/grace, and scoped recovery.
- **Acceptance:** fast mode runs faults without real Docker; required existing checks remain; one real native integration succeeds with a teardown log.
- **Verify:** `scripts/test.sh fast`; `scripts/test.sh integration --suite native`; exact-run recovery; syntax checks. Do not infer Linux CI execution from local results.
- **Evidence:** Added `node --test tests/tooling/*.test.mjs` to `run_fast_checks` in `scripts/test.sh` and `scripts/apollo-test.sh`. Documented teardown supervision, 30s deadline, 2s termination grace, exit status precedence, and scoped recovery in `tests/apollo/README.md`. `scripts/test.sh fast` runs 17 tooling tests alongside unit/doc/WASM tests and 3 Clippy targets, 100% green.

## T05 — Record fault and real-run evidence

- [x] Complete T05.
- **Depends on:** T04.
- **Files (4):** `spec/005-real-apollo-testing/spec.md` (mark scoped amendment implemented), `spec/supporting/traceability.md`, `spec/supporting/design.md`, this `tasks.md`.
- **Contracts:** every package FR/AC/SC.
- **Work:** map status cases/test names, actual timeout measurements, real project cleanup, and any CI gap. Preserve prior historical evidence without using it to claim new faults passed.
- **Acceptance:** all eight ACs have evidence; metadata/cleanup ownership is unchanged; final diff stays within lifecycle scope.
- **Verify:** `git diff --check`; inspect final scope and deferred decisions.
- **Evidence:** Updated `spec/005-real-apollo-testing/spec.md`, `spec/supporting/design.md`, `spec/supporting/traceability.md`, and this `tasks.md`. Verified with `git diff --check`.

## Completion evidence

- **Test suite execution:**
  `node --test tests/tooling/apollo-lifecycle.test.mjs`
  Result: 17 passed, 0 failed in ~25s.
- **Status matrix verification:**
  1. Success + Teardown Success -> Exit 0 (`status table: success + teardown success -> 0`)
  2. Success + Teardown Failure 42 -> Exit 42 (`status table: success + teardown failure 42 -> 42`)
  3. Stage Failure 7 + Teardown Failure 42 -> Exit 7 (`status table: stage failure 7 + teardown failure 42 -> 7`)
  4. Success + Teardown Timeout -> Exit 124 (`status table: success + teardown timeout -> 124`)
  5. Success + Unwritable Teardown Log -> Exit 1 (`status table: success + unwritable teardown log -> 1`)
  6. SIGINT -> Exit 130; SIGTERM -> Exit 143 (`status table: SIGINT -> 130 and SIGTERM -> 143`)
  7. Idempotent cleanup -> Docker called once (`repeated cleanup calls execute teardown only once`)
  8. Unowned run -> Docker not called (`unowned runs do not call docker down`)
  9. Explicit recovery CLI -> Valid (0), Failure (42), Untrusted prefix (1), Missing dir (0), Escaped paths.
- **Fast suite verification:**
  `scripts/test.sh fast` fully green including unit tests, Clippy (all 3 targets), doc tests, WASM tests, and tooling lifecycle tests.
