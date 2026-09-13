# Codebase review handoff

The review findings have detailed specifications, implementation plans, and task lists in this checkout. **All seven review remediation packages have been fully implemented, tested via TDD, and verified.** Use the package task lists below as the canonical completion record.

## Start here

1. Read [repository instructions](AGENTS.md) and the applicable `.agent/**/*.md` rules.
2. Read the [handoff index](spec/review-remediation/README.md) for findings, priorities, dependencies, scope, and review evidence.
3. Read the [execution guide](spec/review-remediation/execution.md) and the selected package's specification, plan, and tasks, in that order.
4. Use the [validation map](spec/review-remediation/validation.md) to check acceptance coverage. The [specification index](spec/README.md) links the existing contracts that the fixes must preserve.

## Implementation packages

| Order | Finding and intended outcome | Specification | Plan | Tasks |
|---|---|---|---|---|
| 006 | P1: Prevent disk restoration from overwriting newer memory state | [spec](spec/006-cache-restore-ordering/spec.md) | [plan](spec/006-cache-restore-ordering/plan.md) | [tasks](spec/006-cache-restore-ordering/tasks.md) |
| 007 | P2: Expose cache errors for Rust consumers to match | [spec](spec/007-public-cache-errors/spec.md) | [plan](spec/007-public-cache-errors/plan.md) | [tasks](spec/007-public-cache-errors/tasks.md) |
| 008 | P2: Correct and test WASM ownership documentation | [spec](spec/008-wasm-ownership-docs/spec.md) | [plan](spec/008-wasm-ownership-docs/plan.md) | [tasks](spec/008-wasm-ownership-docs/tasks.md) |
| 009 | P2: Report bounded test teardown failures accurately | [spec](spec/009-observable-test-cleanup/spec.md) | [plan](spec/009-observable-test-cleanup/plan.md) | [tasks](spec/009-observable-test-cleanup/tasks.md) |
| 010 | P2: Keep fixture request deadlines active through body completion | [spec](spec/010-fixture-body-deadlines/spec.md) | [plan](spec/010-fixture-body-deadlines/plan.md) | [tasks](spec/010-fixture-body-deadlines/tasks.md) |
| 011 | P2: Make public examples and documentation references verifiable | [spec](spec/011-executable-public-docs/spec.md) | [plan](spec/011-executable-public-docs/plan.md) | [tasks](spec/011-executable-public-docs/tasks.md) |
| 012 | P2: Document actual polling intervals, jitter, and latency limits | [spec](spec/012-accurate-polling-docs/spec.md) | [plan](spec/012-accurate-polling-docs/plan.md) | [tasks](spec/012-accurate-polling-docs/tasks.md) |

Execute in numeric order by default. Package 010 depends on 009; 011 depends on 007, 008, and 010; 012 follows 011. Consult the handoff index before dividing work because some packages edit shared files.

## Copyable executor prompt

Replace the package ID when assigning another package:

> Implement package `006-cache-restore-ordering` in this repository. Start with `HANDOFF.md` and `spec/review-remediation/README.md`, then read the execution guide and the package's spec.md, plan.md, and tasks.md. Follow the tasks in order, preserve existing contracts and unrelated changes, and implement the specified regression coverage. Use scripts/test.sh as the Rust test entry point. Record actual verification commands and outcomes in the package's tasks.md; check tasks only after their acceptance criteria pass. Implement this package only. Do not add dependencies, change unrelated policies, publish, or merge. If a plan detail conflicts with the code, explain the evidence and make the smallest contract-preserving adjustment. Ask only when a material product decision blocks the authorized scope.

## Availability and status

All linked artifacts are repository files; execution does not depend on the original temporary review probes. At handoff creation, these documents are local, uncommitted changes. Agents using this same checkout can read them immediately. Separate worktrees or clones need the changes transferred or committed before they can access them.

Maintain detailed requirements and status in the linked documents rather than copying them into this entry point. Historical passing tests in the review are baseline evidence, not proof that these pending fixes work.
