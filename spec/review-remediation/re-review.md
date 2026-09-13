# Implementation verification review

Reviewed on 2026-09-13 against commit `3fa78be` (`After gemini work`), comparing the implementation with baseline `9f18e39` and remediation packages 006–012. The working tree was clean when review started.

**Verdict: request changes.** The main client corrections are implemented and the standard verification passes. Two reproduced gaps remain in cleanup supervision and executable documentation. Completion metadata also contradicts the completed task lists. No implementation fixes were made during this review.

## Findings

### V1 — P2: A signal during teardown abandons the supervised process

**Location:** `scripts/apollo-test-lifecycle.sh`, `handle_signal` (lines 174–183) and `lifecycle_exit_handler` (186–191); the explicit recovery dispatch in `scripts/apollo-test.sh` also removes its EXIT trap.

**Contract:** 009/FR-002–004, AC-004/005/008, SC-003.

The EXIT handler removes the EXIT trap before calling `cleanup`. While cleanup is waiting for diagnostic collection or `docker compose down`, INT/TERM still invoke `handle_signal`, which calls `exit` immediately. There is then no handler to terminate or wait for `CURRENT_CHILD_PID`. This also bypasses the timeout loop that would eventually stop that process group. The same signal path is installed for explicit recovery, where the EXIT trap is deliberately removed before running recovery.

**Reproduced:** a temporary stub `docker` recorded its PID and executed `sleep 30`. A shell sourced the actual lifecycle helper, installed its actual traps, configured an owned temporary run, and exited normally. After the teardown stub started, SIGTERM was sent to the runner. The runner returned **143 while the teardown child was still alive**. The review explicitly killed the probe's process group afterward; no real Docker resource was involved.

The existing signal test sends INT/TERM while the harness is in its work loop, before cleanup begins. The timeout test does not interrupt cleanup. Consequently both tests pass without exercising this failure.

**Required correction:** make signals during cleanup/recovery preserve 130/143 without abandoning supervision. Track the cleanup phase explicitly, retain or safely terminate/reap the current process group, and complete bounded cleanup/reporting without starting a second teardown. A signal handler should not unconditionally exit an active cleanup handler. Apply the same lifecycle behavior to explicit recovery.

**Acceptance:** add stub-based tests for INT and TERM after a readiness signal emitted from teardown, and a repeated signal during cleanup initiated by an earlier interrupt. Cover automatic EXIT and the real recovery CLI. Assert final signal status, at most one automatic `down`, bounded completion, no live tracked child/descendants, and useful failure/recovery diagnostics. Give tests an outer watchdog and release/kill every spawned child in `finally`.

### V2 — P2: The public error example is embedded inside an unclosed Rust fence

**Location:** `docs/wiki/en/Error-Handling.md:60–63`; `scripts/check-doc-examples.mjs`, `extractSnippets`.

**Contract:** 011/FR-001/004, AC-001/004; current documentation correctness.

The Rust fence opened at line 44 is not closed before the new `public-errors` marker at line 62. A subsequent line containing ```rust cannot close that fence: a closing fence cannot have a language suffix. The rendered code block therefore includes the HTML marker and another opening fence as literal Rust text. Copying the displayed block produces invalid Rust.

The new checker scans for markers without tracking an already-open unmarked fence. It extracts only the inner fragment and successfully compiles that fragment, so its green result does not verify the code block that readers actually see.

**Reproduced:** `extractSnippets` accepted a marker inside an existing unclosed fence and returned it as an executable snippet. The actual four-snippet checker also passed on this malformed document. This is a false-positive verification result, not a Rust compiler failure.

**Required correction:** close the first example before the marker. Make extraction track Markdown fence state and accept executable markers only outside code fences. Handle fence delimiters and closing syntax consistently; reject or ignore embedded markers so that the required inventory fails when a marker is not a real document marker. Keep the compiler invocation tied to the actual visible code fence.

**Acceptance:** add a regression using the exact preceding-unclosed-fence shape, plus a marker shown as literal text in a valid fenced example. The required `public-errors` inventory must not pass on the malformed form. Verify the repaired Markdown structure and rerun tooling tests and both native consumer compilations.

### V3 — P3: Handoff completion status is contradictory

**Location:** `HANDOFF.md:3`, `spec/README.md` remediation introduction, `spec/review-remediation/README.md:3`, and the status line in each `spec/006-*` through `spec/012-*/spec.md`.

The root entry says fixes have not been implemented, the shared index says implementation has not started, and all seven feature specifications still say “Specified; implementation not started.” Their task lists are checked complete. An agent following the requested handoff cannot reliably identify remaining work and may implement completed packages again.

**Required bookkeeping:** reconcile the current entry points and package statuses with actual implementation and verification. Record V1/V2 as outstanding, and distinguish implemented code from accepted completion. Preserve historical planning validation as dated history rather than rewriting it to imply it verified implementation. Link current review evidence from the handoff. Do not mark all work verified merely because the standard tests passed.

## Verification performed in this review

| Check | Observed result |
|---|---|
| `scripts/test.sh fast` | Passed with local loopback access enabled |
| Default native unit tests | 49 passed |
| Rustls unit tests | 49 passed |
| External public error tests | 4 passed under each native configuration |
| Rust doctests | 38 passed |
| WASM library tests | 10 passed |
| Native, Rustls, and WASM Clippy | All three passed with warnings denied |
| Node tooling tests | 36 passed |
| Documentation link checker | Reported 416 extracted links across 77 documents, zero broken scoped file targets |
| Canonical Rust example compilation | Four extracted snippets compiled under both native configurations; V2 limits what this proves |
| `scripts/test.sh integration` | Passed: 6 native, 6 Rustls, 7 Node/WASM real Apollo tests |
| Real fixture lifecycle | Setup, verification, idempotency, and normal teardown completed successfully |
| Generated WASM ownership smoke | Passed against the fresh package produced by this integration run |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |
| Additional teardown interrupt probe | Reproduced V1 despite the green standard suite |
| Additional embedded-marker probe | Reproduced V2 despite the green example checker |

Initial sandboxed attempts could not bind local test sockets or access Docker; those failures were environmental. Both suites were rerun successfully with the necessary access. They are not reported as code regressions.

Local logs from this review are `/tmp/apollo-rereview-fast-unrestricted.log` and `/tmp/apollo-rereview-integration-unrestricted.log`. The disposable integration project was `apollo-test-1789282065-ebd697bd`; its teardown log records removal of its three containers, volume, and network. These local logs may expire; the table records the observed results rather than requiring future agents to retrieve temporary files.

## Assessment by package

| Package | Assessment |
|---|---|
| 006 | Atomic empty-check/install returns the existing memory winner and suppresses discarded restoration notifications. Five new persisted-path regressions and existing cancellation/stale/zero-TTL tests pass. Remote replacement remains unconditional. |
| 007 | The public re-export preserves enum identity and private cache implementation. External matching executes under both native configurations. |
| 008 | Generated-package ownership checks pass; transferred-config cleanup was corrected across the inspected current examples and language guides. Real WASM format coverage passes. |
| 009 | Ordinary failure precedence, timeout, and normal real teardown pass. Signal delivery after cleanup starts remains incorrect (V1). |
| 010 | The abort timer covers headers and response-body consumption; held-header/body tests, response compatibility tests, and real fixture callers pass. |
| 011 | Public imports, example content, broken language links, and concrete API claims were substantially corrected. The Markdown fence and extraction gap remains (V2). |
| 012 | Revised polling descriptions agree with the post-round sleep and namespace failure-backoff implementation. No new healthy jitter or interval-only latency guarantee was introduced. |

The runtime design change is appropriately narrow: conditional restoration is separate from unconditional refresh, and the public error name adds no wrapper hierarchy or dependency. The larger verification additions use the actual lifecycle helper and fixture request helper. Their main remaining weakness is scenario coverage and faithful Markdown parsing, rather than the core client correction.

## Optional maintenance feedback

- In `src/cache.rs`, `TestRestoreBarrier` was inserted between the existing Cache rustdoc and `struct Cache`. That documentation now attaches to the test-only barrier and disappears with it in non-test builds. Move the helper above the documentation or into the test-support area so the comment again documents Cache.
- The commit title `After gemini work` does not explain the changes. Future commits should identify the behavior corrected and carry verification notes; separate package commits would make regressions easier to trace.

## Limits

This review compared all seven packages with their contracts and inspected the source, tests, scripts, and documentation changes relative to the previously reviewed baseline. It does not establish Linux CI execution, browser-host certification, Windows support, external-link validity, heading-anchor validity, every unmarked wiki example, production load behavior, or resolution of the explicitly deferred D-001–D-011 policy decisions. Those remain outside this remediation's acceptance scope.
