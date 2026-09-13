# Feature Specification: Observable, bounded integration cleanup

**Feature ID:** `009-observable-test-cleanup`
**Created:** 2026-09-13
**Status:** Specified; implementation not started.
**Review:** R4 / P2. Baseline: [005](../005-real-apollo-testing/spec.md), 005/FR-010–012 and 005/AC-017.

## Purpose and scope

Contributors must know whether their disposable test stack was removed, and a cleanup failure must not erase a prior test failure or be reported as total success. Both automatic teardown and explicit recovery need bounded outcomes and actionable diagnostics.

In scope: teardown logs, status precedence, catchable signal handling, explicit recovery, and Docker-independent fault tests. Out of scope: global Docker cleanup, guaranteed cleanup after SIGKILL/daemon loss, new ownership formats, concurrent fallback-pointer redesign, changing test fixtures or image pins, and retaining databases between runs.

## Stories and acceptance scenarios

### Story 1 — Know when cleanup fails (P1)

As a contributor, I want a failed teardown to be visible with the exact owned project and recovery command.

**Independent test:** Use an isolated stub Docker command that exits 42 with a diagnostic; never affect real Docker resources.

- **AC-001:** Given successful test stages and successful teardown, when the runner exits, then it returns 0 and reports completion accurately.
- **AC-002:** Given successful stages and teardown exit 42, when the runner exits, then it returns nonzero, retains teardown output, reports cleanup failure/project/run-directory/log path, and prints a scoped recovery command. It must not claim the whole run completed successfully.
- **AC-003:** Given an original stage exit 7 and teardown failure, when the runner exits, then it preserves exit 7 while additionally reporting/persisting cleanup failure.
- **AC-004:** Given a teardown that never exits, when its deadline expires, then supervision terminates that child/process group, retains a timeout diagnostic, and returns nonzero within the documented bound plus termination grace.

### Story 2 — Preserve scope and lifecycle correctness (P1)

As a contributor, I want cleanup and recovery to be safe on interrupts and repeated invocation.

**Independent test:** Run a stubbed lifecycle, interrupt it, repeat cleanup, and exercise invalid/missing ownership inputs.

- **AC-005:** Given SIGINT or SIGTERM during a run, when cleanup executes, then the overall result remains 130 or 143 respectively, even if teardown also fails; cleanup is attempted at most once for that automatic exit.
- **AC-006:** Given cleanup already attempted, when an automatic handler is invoked again, then it does not repeat destructive commands and retains the recorded result.
- **AC-007:** Given setup never created owned resources, when the run exits, then cleanup does not call Docker to delete arbitrary resources and preserves the setup error.
- **AC-008:** Given explicit recovery with valid ownership, when teardown succeeds/fails/times out, then it is bounded and reports the corresponding result and log/recovery information; invalid ownership still refuses deletion.

## Functional requirements

- **FR-001:** Automatic and explicit recovery teardown MUST retain stdout/stderr in a run-owned diagnostic file where writable and surface failures/timeouts to stderr.
- **FR-002:** Exit-status precedence MUST be: catchable signal status, otherwise original nonzero stage status, otherwise nonzero teardown status, otherwise diagnostic-capture failure status 1, otherwise 0. Cleanup success MUST NOT erase original failure.
- **FR-003:** Teardown MUST remain scoped to the validated/owned Compose project and use bounded process-group supervision.
- **FR-004:** Automatic cleanup MUST be idempotent within a run, including EXIT and signal-handler interaction.
- **FR-005:** Cleanup failures MUST identify project, run directory, diagnostic file when available, and a shell-safe explicit `cleanup --run-dir` recovery command.
- **FR-006:** Diagnostic capture failure MUST NOT suppress the teardown attempt or manufacture success; if the log file cannot be written, an explicit stderr fallback MUST remain.
- **FR-007:** Fast mode MUST run the new lifecycle fault checks without requiring/contacting a real Docker daemon; all old checks MUST remain.

## Status table

| Original state | Teardown | Final status |
|---|---|---|
| Success | Success / nothing owned | 0 |
| Success | Failure 42 | 42 |
| Success | Deadline exceeded | 124 |
| Success | Teardown succeeded but diagnostic capture failed | 1 |
| Stage failure 7 | Any | 7 |
| SIGINT | Any | 130 |
| SIGTERM | Any | 143 |

If failure to prepare diagnostics prevents capture, surface it and still attempt deletion. A missing/unwritable log must not be silently ignored; do not let it obscure a more informative existing stage/teardown status.

## Entities and success criteria

**Entities:** original stage status, signal status, teardown result, owned project/run directory, teardown log, recovery command.

- **SC-001:** Automated fault tests establish every status-table row and AC-005–008 against stubs, including at-most-once calls.
- **SC-002:** An actual disposable native integration run still passes and records successful teardown; no unrelated Docker project is targeted.
- **SC-003:** Timeout tests terminate within configured test deadline plus explicit TERM/KILL grace and watchdog tolerance; no live subprocess remains.

## Assumptions

Use the existing 30-second teardown deadline in normal operation. Process scheduling/TERM grace is additional documented overhead, not a second unbounded wait. No permissions/CI event changes are required. See [plan](plan.md) and [tasks](tasks.md).
