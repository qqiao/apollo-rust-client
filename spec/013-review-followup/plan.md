# Plan: Close implementation review gaps

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

Read [specification](spec.md), [tasks](tasks.md), [review evidence](../review-remediation/re-review.md), repository instructions, and the relevant 009/011 contracts before editing. This package is the current work assignment; earlier package plans are baseline context.

## Current implementation and failure mechanisms

- `scripts/apollo-test-lifecycle.sh`: `lifecycle_exit_handler` disables EXIT before calling `cleanup`; `handle_signal` unconditionally exits. A signal received during cleanup therefore leaves `CURRENT_CHILD_PID` unsupervised. `scripts/apollo-test.sh` also disables EXIT before `run_recovery_cleanup`. Existing signal tests signal an idle work loop, not an active teardown.
- `docs/wiki/en/Error-Handling.md`: the fence beginning near line 44 remains open at the `public-errors` marker. Locate these symbols afresh instead of patching historical line numbers.
- `scripts/check-doc-examples.mjs`: `extractSnippets` scans for markers without tracking surrounding unmarked code fences. It accepts a marker that readers see as literal code.
- Current completion claims have changed independently in some entry points. Inspect current contents and preserve other agents' changes. The review describes its dated observation, not a command to restore old wording.

## V1: Lifecycle design

Prefer a small explicit lifecycle phase (work, cleanup/recovery, finished) alongside the existing cached result and first signal status. Do not add an external process supervisor or dependency.

1. On the first INT/TERM, store 130/143. During normal work, transfer control to the normal cleanup path as today. During cleanup/recovery, record the signal and return to bounded supervision instead of calling `exit` recursively. Subsequent signals preserve the first status.
2. Enter the protected phase before diagnostic collection or teardown begins, including explicit recovery. Do not equate “cleanup entered” with “cleanup finished”: reentry must not return a default successful cached result while cleanup is still active.
3. Let the existing bounded command complete, or deliberately terminate and reap its tracked process group. Deferring signals through the bounded teardown is the recommended policy. Never restart its timer or run `down` again in response to another signal.
4. Audit interrupted `sleep` and `wait` under Bash 3.2, `set -e`, and conditional function calls. A trapped signal may interrupt a wait without the child having completed. Do not clear `CURRENT_CHILD_PID` until the actual child is reaped or conclusively gone. Explicitly capture nonzero helper results so `errexit` cannot skip final status selection.
5. Use one final-status selector for automatic cleanup and recovery if that reduces duplication: first signal, original stage, teardown, diagnostic, zero. Retain the original 30-second teardown deadline and two-second TERM/KILL grace; existing bounded diagnostics may add their documented time. Do not create an unbounded retry or suppress capture failures.
6. Preserve invalid/missing ownership safeguards and shell-safe recovery messages. Report interruption/failure accurately even if a deferred teardown succeeds. Never print whole-run success when final status is nonzero.

### Deterministic signal regression recipe

Extend `tests/tooling/apollo-lifecycle.test.mjs`; source the actual helper for automatic paths and spawn the actual `scripts/apollo-test.sh cleanup --run-dir ...` for recovery.

- Create a temporary run directory and `ownership.json` with an `apollo-test-` project. Prepend a temporary stub `docker` to PATH. Never contact real Docker in these tests.
- Stub distinguishes `ps`, `logs`, and `down`; make ordinary diagnostics finish promptly. For the target stage, record its PID and a descendant PID, emit a ready file, and block. Hold commands inside their normal process group; do not intentionally daemonize or escape it.
- Start the runner with captured stdout/stderr. Register exit/close listeners before signaling. Wait for the ready file with a bounded polling loop that also detects premature exit. Do not infer readiness from a fixed sleep.
- Signal the runner with INT or TERM. For repeated-signal cases, begin in a work-ready phase, send the first signal, wait for cleanup-stage readiness, then send the second signal. Test mixed INT→TERM and TERM→INT to establish first-signal precedence.
- Use a short existing `TEARDOWN_TIMEOUT` override for hung teardown. Exercise diagnostics separately with a releasable stub so their production 15/30-second budgets do not make tests slow. Signals must not extend the configured budget.
- Assert expected 130/143, a single `down`, no success banner, preserved diagnostics, and no live recorded child/descendant after runner completion. Record measured duration with a scheduling tolerance (for example a one-second teardown deadline plus two-second grace plus at most three seconds tolerance when diagnostics are immediate).
- Put an independent watchdog around every scenario. On all assertion failures, timeout, or premature exits, release/terminate and reap the runner and each recorded process group in `finally`; await stream closure before deleting files. A test that hangs or leaks on the old implementation is not a usable regression.

Required matrix: automatic teardown × INT/TERM; recovery teardown × INT/TERM; signal-initiated cleanup with mixed repeated signals; repeated signal during diagnostic collection; unchanged existing no-signal matrix. Establish a red result with the current implementation, then the green result. The review's original probe used a stub that exec'd `sleep 30`: runner 143 with its child still alive is the failure to detect.

## V2: Fence-aware extraction

Repair the missing closing fence before the `public-errors` marker. Keep the marker immediately before its intended standalone Rust fence, with blank lines allowed. Preserve the four required `(document, ID)` pairs and actual source compilation.

Within `extractSnippets`, track ordinary fenced blocks as well as marked examples. A top-level fence uses at least three identical backticks or tildes (zero to three leading spaces); a matching close uses the same character, at least the opening length, and whitespace only afterward. Thus a language-tagged opening line cannot close an existing block, and a shorter or different delimiter cannot do so. Preserve CRLF support.

Only a marker outside a fence can introduce an executable example. Ignore markers used as literal fenced text; required-inventory validation must then fail if the only required marker was hidden. Continue rejecting duplicate real markers, missing immediate fences, non-Rust languages, empty snippets, and unclosed marked snippets. Preserve document/example/line context in errors. No general Markdown dependency is needed; do not expand scope to nested lists, HTML rendering, or every wiki example.

Add fixture tests for: exact nested-marker defect; a valid ordinary fence showing a literal marker; mismatched character; shorter closing delimiter; language suffix on a purported close; longer valid close; tilde fence; CRLF; and the existing malformed/duplicate/inventory/compile-negative cases. Test the malformed shape through `validateInventory` using a temporary repository fixture, not just the low-level parser. The corresponding repaired fixture must pass extraction and inventory. Do not replace compilation with regex validation of Rust.

The link checker has separate parsing limitations. Change it only if the new fixtures expose a directly related false result needed for this correction; document the reason and keep it in a separately bounded task. General Markdown parser redesign is out of scope.

## V3: Evidence and status reconciliation

The planning handoff makes current entry points point here immediately. The executor must finish the detailed audit after fixes are verified:

- Mark 006/007/008/010/012 implemented with dated baseline verification, without inventing new runs. Mark 009/011 implemented with open 013 follow-up until V1/V2 pass.
- Review the scoped amendment paragraphs in 001–005 and current supporting contracts/research/traceability; distinguish pending original policy decisions from already implemented narrow corrections. Preserve requirement IDs and historical evidence.
- Append dated resolution evidence to the review rather than deleting findings or changing what was observed originally. Include current commit/tree reference and relevant exact commands and outcomes.
- Update 013 status only after all required tasks pass; a remaining environment limitation remains an explicit gap. No reviewer approval or CI execution may be inferred from local tests.

## Verification and delivery

Focused commands already supported:

```sh
bash -n scripts/apollo-test-lifecycle.sh scripts/apollo-test.sh
node --test tests/tooling/apollo-lifecycle.test.mjs
node --test tests/tooling/doc-examples.test.mjs
node scripts/check-doc-examples.mjs
node scripts/check-doc-links.mjs
scripts/test.sh fast
scripts/test.sh integration --suite native
git diff --check
```

Run focused tests after their changes, then fast once after both fixes. One real native integration validates normal teardown after the lifecycle change. Record its exact owned project and teardown log outcome. Full three-runtime integration already passed in the review; repeat broader suites only if new changes justify it. Use `scripts/test.sh` for Rust tests, Clippy instead of cargo check. Respect actual sandbox failures and report them separately from code failures.

No commit, push, publish, merge, dependency addition, or external service modification is part of this handoff. Optional review suggestions (Cache rustdoc placement and commit titles) are not completion blockers for this package and must not obscure V1–V3.
