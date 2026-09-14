# Review remediation: start here

**Status:** Packages 006–014 are implemented and verified. The historical verification review findings V1–V3 are resolved in [013 — Review follow-up](../013-review-followup/spec.md), and GitHub Copilot review findings are addressed in [014 — Copilot remediation](../014-copilot-remediation/spec.md) (prior review record preserved at [commit 1ca6871](https://github.com/qqiao/apollo-rust-client/blob/1ca68717e268a5298b814c97cd0703cd950554fa/spec/review-remediation/re-review.md)).
**Source review:** 2026-09-12, checkout `9f18e3942008d5280a63809655aa9a073c3f0507`.
**User request:** Prepare a detailed, accurate handoff for a smaller model to fix the review findings. This planning task changes specification documents only. A later instruction to implement a package supplies execution authorization; do not demand repeated approval of ordinary steps within that authorized package.

Current verification and remaining acceptance limits: [status audit](../verification-2026-09-13.md).

## Historical baseline package map

Each row has a separate specification, plan, and task list. This was the original implementation order; packages 006–014 are now completed and verified. Requirements use qualified IDs such as `006-cache-restore-ordering/FR-001`; task IDs are local to their package.

| Review finding | Package / outcome | Specification | Plan | Tasks | Implementation dependencies |
|---|---|---|---|---|---|
| R1, P1 | 006 — Prevent storage restoration from undoing a refresh | [spec](../006-cache-restore-ordering/spec.md) | [plan](../006-cache-restore-ordering/plan.md) | [tasks](../006-cache-restore-ordering/tasks.md) | None; baseline 002/003 contracts |
| R2, P2 | 007 — Let Rust consumers match cache errors | [spec](../007-public-cache-errors/spec.md) | [plan](../007-public-cache-errors/plan.md) | [tasks](../007-public-cache-errors/tasks.md) | None; baseline 001 contract |
| R3, P2 | 008 — Correct and verify WASM ownership examples | [spec](../008-wasm-ownership-docs/spec.md) | [plan](../008-wasm-ownership-docs/plan.md) | [tasks](../008-wasm-ownership-docs/tasks.md) | None; baseline 004 contract |
| R4, P2 | 009 — Surface bounded teardown failures accurately | [spec](../009-observable-test-cleanup/spec.md) | [plan](../009-observable-test-cleanup/plan.md) | [tasks](../009-observable-test-cleanup/tasks.md) | None; baseline 005 contract |
| R5, P2 | 010 — Bound fixture requests through body completion | [spec](../010-fixture-body-deadlines/spec.md) | [plan](../010-fixture-body-deadlines/plan.md) | [tasks](../010-fixture-body-deadlines/tasks.md) | Land after 009 to reuse its Node test entry point |
| R6, P2 | 011 — Make current public examples and references reliable | [spec](../011-executable-public-docs/spec.md) | [plan](../011-executable-public-docs/plan.md) | [tasks](../011-executable-public-docs/tasks.md) | 007 public alias; 008 ownership text; 010 runner edits |
| R7, P2 | 012 — Describe actual polling timing and jitter | [spec](../012-accurate-polling-docs/spec.md) | [plan](../012-accurate-polling-docs/plan.md) | [tasks](../012-accurate-polling-docs/tasks.md) | Land after 011 to avoid overlapping guide edits |
| V1–V3, P1 | 013 — Review follow-up | [spec](../013-review-followup/spec.md) | [plan](../013-review-followup/plan.md) | [tasks](../013-review-followup/tasks.md) | Follow-up for 009, 011, and status reconciliation |
| Copilot F1–F18 | 014 — Copilot remediation | [spec](../014-copilot-remediation/spec.md) | [plan](../014-copilot-remediation/plan.md) | [tasks](../014-copilot-remediation/tasks.md) | Follow-up for 009, 010, 011, and PR #133 Copilot review rounds (baseline F1–F8 and follow-ups F9–F18) |

006 and 007 can be implemented independently, but both touch `src/lib.rs`/cache documentation indirectly. 009 and 010 share test scripts; 008/011/012 share documentation. Default to serial execution. This map describes dependencies, not authorization to launch other agents or tasks.

## How to execute one package

1. Read repository `AGENTS.md`, every applicable `.agent/**/*.md`, this index, and [execution rules](execution.md).
2. Read the selected package's spec, then plan, then tasks. Read only its listed baseline contracts and source symbols initially; expand context when evidence requires it.
3. Check git status and preserve unrelated work. Locate symbols afresh: source line numbers in the review are navigation hints, not stable patch coordinates.
4. Work through unchecked tasks in dependency order. A behavioral task includes reproduction, correction, and green verification; do not commit an intentionally failing intermediate suite.
5. Record actual commands, outcomes, and remaining gaps in that package's `tasks.md`. Check a task only when its acceptance criteria are met. Planning checkboxes are not implementation evidence.
6. Keep shared contracts/descriptive docs current without claiming that pending siblings have shipped. No requirement can be weakened merely to match the implementation.
7. Report the selected package ID, completed tasks, behavior changed, checks run, and unresolved limitations. Stop at the authorized scope; do not silently implement the next package.

The historical executor prompt for package 013 is preserved in [013/tasks.md](../013-review-followup/tasks.md#copyable-assignment) for provenance. Packages 006–014 are complete, and subsequent review remediations are tracked in [014/tasks.md](../014-copilot-remediation/tasks.md).

## Scope boundary

The seven numbered findings are the implementation backlog. Package 011 also covers the review's concrete broken local links and adjacent false current API/tooling claims because they are part of the same documentation consistency defect. Formatting should be clean for changed code; existing repository-wide format drift is recorded, not a mandate for an unrelated reformat commit.

The following remain **deferred decisions**, not tasks disguised as requirements: payload redaction/limits, encrypted or credential-isolated caches, active-writer-aware startup cleanup, maximum stale age, read-triggered backoff, detached-work cancellation, callback reentrancy, global event ordering, listener removal/namespace eviction, huge-duration/clock-skew policy, feature-flag redesign, dependency upgrades, Windows/browser certification, and production load SLOs. See [research](../supporting/research.md), D-001–D-011. Correct false documentation about an existing limitation without claiming to resolve its policy.

## Evidence baseline and provenance

The previous review ran `scripts/test.sh fast`: 44 default-native + 44 Rustls tests, 37 doctests, 10 WASM tests, and three Clippy checks passed. It also ran all real-Apollo suites: 6 default-native + 6 Rustls + 7 Node/WASM tests passed. Formatting checks failed in five Rust files; 237 local Markdown links included eight broken links. These are historical review results, **not verification of future fixes** and not hard-coded required test totals.

Confirmed probes: delayed persistent read rolled memory from `new` back to `old`; external cache-error import failed E0603; freeing consumed config threw `null pointer passed to rust`; stub teardown exit 42 was suppressed; a stalled fixture body remained pending with an un-aborted signal after 5.2 seconds. The original probes lived in `/tmp`; the plans below give self-contained reproducible recipes and must not depend on those ephemeral files.

## Planning validation

See [current verification and status audit](../verification-2026-09-13.md) for the acceptance-to-task coverage map, checked assumptions, and verification limits (historical planning validation preserved at [commit 1ca6871](https://github.com/qqiao/apollo-rust-client/blob/1ca68717e268a5298b814c97cd0703cd950554fa/spec/review-remediation/validation.md)).

## Program completion

Packages 006–014 are now implemented and verified (historical instruction: after all seven baseline packages were implemented, follow [execution.md](execution.md#final-integration-checkpoint)). Do not infer completion from the green baseline or automatically mark the earlier feature-005 historical checklist as proof of these new fault cases.
