# Codebase review handoff

All remediation packages 006–012 and follow-up package [013 — Review follow-up](spec/013-review-followup/spec.md) are fully implemented and verified in this checkout. Findings V1–V3 from the [verification review](spec/review-remediation/re-review.md) have been reproduced with deterministic regressions, resolved, and verified (see [remediation resolution](spec/review-remediation/re-review.md#remediation-resolution-2026-09-13)).

Current verification and remaining acceptance limits are recorded in the [status audit](spec/verification-2026-09-13.md). Implemented does not mean every platform or acceptance clause has independent runtime coverage.

## Start here

1. Read [repository instructions](AGENTS.md) and the applicable `.agent/**/*.md` rules.
2. Read the [handoff index](spec/review-remediation/README.md) for findings, priorities, dependencies, scope, and review evidence.
3. Read the [execution guide](spec/review-remediation/execution.md) and the selected package's specification, plan, and tasks, in that order.
4. Use the [validation map](spec/review-remediation/validation.md) to check acceptance coverage. The [specification index](spec/README.md) links the existing contracts that the fixes must preserve.

## Implemented baseline packages

| Order | Finding and intended outcome | Specification | Plan | Tasks |
|---|---|---|---|---|
| 006 | P1: Prevent disk restoration from overwriting newer memory state | [spec](spec/006-cache-restore-ordering/spec.md) | [plan](spec/006-cache-restore-ordering/plan.md) | [tasks](spec/006-cache-restore-ordering/tasks.md) |
| 007 | P2: Expose cache errors for Rust consumers to match | [spec](spec/007-public-cache-errors/spec.md) | [plan](spec/007-public-cache-errors/plan.md) | [tasks](spec/007-public-cache-errors/tasks.md) |
| 008 | P2: Correct and test WASM ownership documentation | [spec](spec/008-wasm-ownership-docs/spec.md) | [plan](spec/008-wasm-ownership-docs/plan.md) | [tasks](spec/008-wasm-ownership-docs/tasks.md) |
| 009 | P2: Report bounded test teardown failures accurately | [spec](spec/009-observable-test-cleanup/spec.md) | [plan](spec/009-observable-test-cleanup/plan.md) | [tasks](spec/009-observable-test-cleanup/tasks.md) |
| 010 | P2: Keep fixture request deadlines active through body completion | [spec](spec/010-fixture-body-deadlines/spec.md) | [plan](spec/010-fixture-body-deadlines/plan.md) | [tasks](spec/010-fixture-body-deadlines/tasks.md) |
| 011 | P2: Make public examples and documentation references verifiable | [spec](spec/011-executable-public-docs/spec.md) | [plan](spec/011-executable-public-docs/plan.md) | [tasks](spec/011-executable-public-docs/tasks.md) |
| 012 | P2: Document actual polling intervals, jitter, and latency limits | [spec](spec/012-accurate-polling-docs/spec.md) | [plan](spec/012-accurate-polling-docs/plan.md) | [tasks](spec/012-accurate-polling-docs/tasks.md) |
| 013 | P1: Resolve cleanup signals, doc code fences, and status records | [spec](spec/013-review-followup/spec.md) | [plan](spec/013-review-followup/plan.md) | [tasks](spec/013-review-followup/tasks.md) |

The table records baseline packages (006–012) and follow-up package 013; all are completed and verified.

## Next work

No implementation redo is required for the reviewed findings. Consult the status audit for remaining verification and product-decision work before assigning a new task.

## Availability and status

All linked artifacts are repository files; execution does not depend on the original temporary review probes. New follow-up artifacts are local changes until committed or otherwise transferred. Agents using this same checkout can read them immediately. Separate worktrees or clones need the changes transferred or committed before they can access them.

Maintain detailed requirements and status in the linked documents rather than copying them into this entry point. Historical passing tests in the review are baseline evidence, not a substitute for verification of a later change.
