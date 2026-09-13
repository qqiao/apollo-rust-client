# Implementation and specification verification — 2026-09-13

## Result and scope

Features 001–013 have implementations in this checkout. The latest parser corrections resolve both reproduced defects from [the package-013 review](review-remediation/re-review-013.md). No new blocking implementation defect was found in this verification. Implementation status is separate from complete acceptance-test coverage and maintainer acceptance of retrospective requirements.

This audit covers all 43 Markdown documents in `spec/`: feature requirements, plans, task lists, supporting contracts/design/research/traceability, the checklist, handoffs, and historical review records. It checks current status against source/test evidence, references, and available runtime verification. Historical planning instructions and review findings remain dated history; they are not active implementation backlogs.

Reviewed tree: HEAD `0a5e321` plus the coding agent's uncommitted follow-up changes. This verification changed changelog/specification/handoff documentation only and preserved the agent's implementation. The last published version is not inferred from this working tree; recent changes remain under Unreleased.

## Feature disposition

| Feature | Implementation status and decisive evidence | Acceptance limits |
|---|---|---|
| 001 — Read configuration | Implemented: builder/environment validation, request identity/signing, typed formats, public errors and complete request deadlines. Native/Rustls tests and real identity/auth/grayscale/format suites pass. | Some optional-setting combinations, exact TLS warning/certificate controls, and malformed-content variants remain source-inspected rather than individually tested. |
| 002 — Retain configuration | Implemented: memory/persistent reads, stale revalidation, coalescing, cancellation, preload, refresh, and atomic restore. Persisted-path and stale/cancellation tests pass. | Identical-content freshness renewal and some corrupt/unreadable storage combinations lack dedicated acceptance cases. Real two-client persistence smoke does not independently simulate server-unavailable restart. |
| 003 — Observe updates | Implemented: registration/notifications, bounded-concurrency polling, failure backoff, start/stop/drop. Native/WASM lifecycle and real release/polling tests pass. | Exact listener ordering/panic/format-error combinations are partly source-only. D-011 explicitly leaves the overlapping initial-read failure notification policy unresolved. |
| 004 — JavaScript client | Implemented: generated API, format values, bigint Properties access, host capabilities, lifecycle and ownership. Real Node/WASM suite and fresh generated ownership smoke pass. | No comprehensive browser-host matrix, integer-boundary matrix, emitted-warning check, JS throwing-callback matrix, or separate WASM stalled-body regression is established. Source paths and related tests support implementation, not exhaustive certification. |
| 005 — Real Apollo testing | Implemented: disposable stack, fixture seeding/idempotency, three real runtimes, diagnostics/recovery and CI workflow configuration. All 19 real tests and normal teardown pass now. | Remote PR/main/fork workflow execution and downloadable failure artifacts were not verified here; SC-003 and Linux evidence in SC-006 remain open. Prior concurrent-run evidence is historical, not rerun in this audit. |
| 006 — Cache restore ordering | Implemented and regression-verified: conditional empty-memory restoration returns the winner, preserves stale availability and avoids discarded notifications. | No global version/event ordering or cross-process policy is added. |
| 007 — Public cache errors | Implemented and verified: alias keeps enum identity and private cache internals; four external consumer tests pass under each native configuration. | Coalesced/listener snapshot limitations remain documented. |
| 008 — WASM ownership guidance | Implemented and verified: corrected current examples and generated-package success/failure ownership smoke. | No quantitative leak-free or detached-work cancellation guarantee. |
| 009 — Observable cleanup | Implemented and verified with 013: status/log/timeout/recovery tests and signal-during-cleanup regressions pass. | SIGKILL/daemon-loss cleanup remains outside the guarantee. |
| 010 — Fixture body deadlines | Implemented and verified: unified fetch/body abort lifetime, seven focused HTTP cases, actual fixture seed/verify/idempotency. | Does not add production payload-size or confidentiality policies. |
| 011 — Executable documentation | Implemented and verified with final 013 parser fixes: four real Markdown examples compile under both native configurations; scoped local-file links pass. | Not every wiki fence, external URL, or heading anchor is checked. |
| 012 — Polling documentation | Implemented: source-aligned post-round sleep, failure-only jitter, retry eligibility and latency wording. | No scheduler change or new freshness SLO. |
| 013 — Review follow-up | Implemented and verified: protected cleanup phases/first-signal status; fence-aware extraction; final LF/CRLF hidden-marker and malformed-opener regressions. | Remaining non-blocking test-harness suggestions from the historical review are not represented as production fixes. |

## Commands and observed results

| Verification | Result |
|---|---|
| `scripts/test.sh fast` | Exit 0; all required stages completed |
| Native unit tests | 49 default TLS + 49 Rustls passed |
| External public-error tests | 4 default TLS + 4 Rustls passed |
| Rust doctests | 38 passed |
| WASM library tests | 10 passed |
| Clippy | Native, Rustls and wasm32 configurations passed with warnings denied |
| Node tooling tests | 45 passed: lifecycle, fixture HTTP, document extraction and links |
| Canonical Rust Markdown examples | Four examples compiled through Clippy under native TLS and Rustls |
| `scripts/test.sh integration` | Exit 0; 6 native + 6 Rustls + 7 Node/WASM real Apollo tests passed |
| Fixture setup and cleanup | Repeated seeding idempotent; owned project teardown completed successfully |
| `node scripts/wasm_api_smoke.js <fresh-integration-wasm-directory>` | Generated exports and all documented ownership cases passed |
| Additional original-defect probes | Hidden required marker rejected under LF and CRLF; repaired examples accepted under both; invalid marked opener rejected before compilation |
| `cargo fmt --all -- --check` | Passed |

The disposable project was `apollo-test-1789285692-f611ff3d`. Local logs were written to `/tmp/apollo-final-fast.log` and `/tmp/apollo-final-integration.log`; the run-owned diagnostics contain successful teardown. Temporary files may expire; this record contains the results needed to interpret status without relying on them. Runtime verification used the necessary loopback/Docker access. Documentation-only edits after these passes do not require repeating the runtime suites.

## Specification corrections made

- Marked 001–004 implemented while retaining pending maintainer acceptance and explicit partial/source-only coverage.
- Corrected feature-005 plan/task headers that still said unstarted/planning-only despite completed implementation. Reopened remote CI acceptance checkboxes that lack inspected run evidence, rather than calling workflow configuration runtime proof.
- Corrected stale planned labels for implemented 001/003/004 amendments and supporting traceability. Kept requirement/scenario IDs stable.
- Replaced traceability references to removed unit tests with their actual real-integration replacements; recorded generated ownership and bigint evidence without claiming exhaustive coverage.
- Marked retained implementation plans and planning validation as historical design/checklist artifacts with links to current status.
- Appended final resolution evidence to the earlier parser review and package 013; removed the root handoff's instruction to redo completed work.
- Retained unresolved D-001–D-011, maintainer acceptance, missing acceptance coverage, and platform/CI verification as open work. No requirement was weakened to claim completion.

## Remaining work classification

**Implemented with incomplete verification:** feature 005's remote CI/fork/failure-artifact and Linux checks; source-only/partial scenario coverage listed in [traceability](supporting/traceability.md). These are not a reason to label the implemented features “unstarted.”

**Product decisions, not an implementation backlog:** D-001–D-011 in [research](supporting/research.md), including stale-age/backoff policy, cancellation/reentrancy, cross-process durability, credential isolation, resource limits, large timing values and diagnostic redaction.

**Approval:** maintainer acceptance of retrospective requirements/priorities remains separate from verification. No local result establishes remote CI execution, browser/Windows certification, production performance, or approval.


## Final document checks

All thirteen feature specifications have unique feature-local FR/AC/SC declarations. All 43 specification Markdown documents have balanced fenced blocks. Named test references in current traceability resolve to source/tests/scripts. The repository local-file link checker and `git diff --check` pass after the status and changelog edits. Remaining unstarted wording is confined to dated review history or acceptance instructions, not current feature status.
