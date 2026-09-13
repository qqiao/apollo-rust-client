# Specification: Close implementation review gaps

**Feature ID:** `013-review-followup`
**Status:** Implemented and verified (see tasks.md).
**Source:** Historical verification review (findings V1–V3; reference preserved at [commit 1ca6871](https://github.com/qqiao/apollo-rust-client/blob/1ca68717e268a5298b814c97cd0703cd950554fa/spec/review-remediation/re-review.md)).
**Baseline contracts:** [009 cleanup](../009-observable-test-cleanup/spec.md) and [011 documentation](../011-executable-public-docs/spec.md). Existing identifiers and obligations remain in force.

## Scope

Complete the remaining cleanup, Markdown verification, and completion-record corrections after packages 006–012. Do not reimplement those packages. No production client behavior, dependency, Docker ownership format, image pin, or deferred policy changes are required.

## Stories and acceptance scenarios

### Story 1: Interrupt cleanup without abandoning its children (P1)

Independent test: use the real lifecycle helper and a stub Docker process that announces teardown readiness before waiting. Signal only after readiness.

- **AC-001:** Given automatic EXIT cleanup is executing teardown, when SIGINT or SIGTERM arrives, then the runner finishes with 130 or 143, respectively, within the existing bounded cleanup budget, with no live supervised child or descendant left behind.
- **AC-002:** Given a signal has already initiated cleanup, when further INT/TERM signals arrive during diagnostics or teardown, then cleanup is not restarted, its deadlines are not extended, and the first received signal determines the final signal status.
- **AC-003:** Given explicit `cleanup --run-dir` recovery is executing teardown for valid ownership, when interrupted, then the same signal-status and process-supervision guarantees hold, without an automatic second teardown.
- **AC-004:** Given no signal, when ordinary success, stage failure, teardown failure, timeout, diagnostic failure, repeated cleanup, or invalid ownership occurs, then the complete existing 009 status/safety matrix remains unchanged.

### Story 2: Compile the documentation readers actually see (P1)

Independent test: introduce a required example marker inside a preceding unclosed Rust fence and verify the required inventory fails.

- **AC-005:** Given the repaired error-handling guide, when rendered or copied, then the first Rust example and the marked public-error example are separate valid fences, with no literal marker or opening fence embedded in the copied public-error code.
- **AC-006:** Given a marker embedded in an ordinary fenced example, including the previously unclosed-fence shape, when extraction and required-inventory validation run, then that embedded marker does not satisfy the inventory.
- **AC-007:** Given supported backtick or tilde fences, when scanning, then closing delimiters match the opening character, meet its minimum length, and contain only trailing whitespace. A language-tagged line inside a fence is not a closing delimiter. Existing required Rust snippets, duplicate-marker rejection, and invalid-language/empty/missing/unclosed-example diagnostics remain effective.
- **AC-008:** Given all current canonical examples and a controlled unavailable-symbol example, when checked, then the current examples compile under native TLS and Rustls and the invalid example fails. Errors identify the document and example involved.

### Story 3: Hand off accurate remaining work (P2)

Independent test: follow the root handoff through the specification index to the package task lists and compare every current completion claim with recorded evidence.

- **AC-009:** Given implemented baseline packages and open follow-up findings, when another agent reads the handoff, then it can distinguish implemented baseline work, pending corrections, historical verification, and current verification without being directed to reimplement 006.
- **AC-010:** Given follow-up checks complete or encounter a real limitation, when completion is recorded, then only satisfied tasks are checked, commands/outcomes and gaps are recorded, historical review findings remain dated history, and no Linux CI/browser/production certification or maintainer approval is invented.

## Functional requirements

- **FR-001:** Catchable signals MUST preserve supervision throughout automatic cleanup and explicit recovery.
- **FR-002:** Repeated signals MUST NOT reset cleanup deadlines or duplicate teardown. First signal wins; any signal outranks stage, teardown, and diagnostic status.
- **FR-003:** Existing normal-operation deadlines, termination grace, ownership validation, diagnostics, and no-signal status precedence MUST remain intact.
- **FR-004:** The example checker MUST recognize markers only outside existing fenced blocks and compile the corresponding actual Rust fence.
- **FR-005:** Current documentation MUST render the canonical public-error example as a separate copyable Rust block.
- **FR-006:** Required inventory and negative controls MUST prevent missing, hidden, malformed, or uncompilable examples from producing false success.
- **FR-007:** Current handoff/spec/task status MUST distinguish implementation, verification, unresolved findings, and historical evidence.
- **FR-008:** Changes MUST stay within these findings and preserve all earlier accepted contracts and unrelated work.

## Entities and success criteria

Entities: lifecycle phase, first signal status, supervised process group, teardown attempt, active fence, document marker, required example inventory, dated verification record.

- **SC-001:** AC-001–004 pass through stub tests using actual shared functions and the actual recovery CLI; one real native integration run completes normal teardown.
- **SC-002:** AC-005–008 pass in focused tooling checks and the full fast entry point, with explicit negative controls.
- **SC-003:** AC-009/010 have a source-to-status audit and every requirement maps to completed tasks with actual evidence before package completion is claimed.

## Assumptions

Signals during cleanup may be deferred while already-bounded teardown completes. SIGKILL and escaped external processes are outside the contract. First-signal-wins makes repeated mixed INT/TERM behavior deterministic. Markdown support concerns repository top-level fenced examples, not implementation of a general Markdown renderer. Detailed decisions and tests live in [plan.md](plan.md); work state lives in [tasks.md](tasks.md).
