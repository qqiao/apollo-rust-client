# Review of package 013 implementation

**Date:** 2026-09-13. **Scope:** working-tree implementation of package 013 over HEAD `0a5e321`; changes remain uncommitted. This review did not modify implementation or its existing completion records.

**Verdict: request a narrow correction to Markdown verification.** The earlier signal-during-cleanup defect and actual missing documentation fence are fixed in the exercised paths. The standard checks pass. Two input shapes still bypass the intended document validation, so package 013's blanket completion claim is premature.

## Required finding: P2 — Fence recognition still permits false success

**Locations:** `scripts/check-doc-examples.mjs:39`, `:64`, and `:101` (`extractSnippets`). **Contracts:** 013/FR-004/006, AC-006–008; T03 explicitly requires CRLF and malformed-fence coverage.

### A. CRLF line endings bypass ordinary-fence tracking

Splitting on LF alone leaves a trailing carriage return on each CRLF line. The ordinary opening-fence regex ends in `(.*)$`; JavaScript's dot does not consume that carriage return. The opening fence is not recognized, so a marker inside it is subsequently accepted as an executable example.

**Verified through `validateInventory`**, using a temporary fixture directory and the required pair `{doc: 'example.md', id: 'public-errors'}`:

````text
```rust
let old = 1;
<!-- apollo-example: public-errors -->
```rust
let inner = 2;
```
````

With LF line endings the inventory correctly rejects this document as missing the required example. Converting the identical text to CRLF makes the inventory accept `public-errors` and extract `let inner = 2;`. The malformed rendered structure is the same in both cases. This reproduces the original hidden-marker failure under the explicitly required CRLF input.

### B. Marked opening fences use weaker validation than ordinary fences

The ordinary-fence branch rejects a backtick inside a backtick fence's info string. The marked-example branch at line 101 uses a prefix regex and does not perform that validation. It accepts the following opener (three initial backticks, `rust`, then an extra backtick):

````text
<!-- apollo-example: invalid-opener -->
```rust`
let x = 1;
```
````

This is not a valid opening code fence, yet `extractSnippets` returned one snippet and `compileSnippets` compiled it successfully under **both native TLS and Rustls**. The checker is again compiling an extracted fragment that is not the real Markdown code block it claims to verify.

### Minimal correction and acceptance

1. Normalize line endings before both ordinary and marked fence parsing, preserving one-based line numbers. At minimum support LF and CRLF consistently; avoid globally trimming indentation or code content.
2. Use a common opening-fence recognizer for both paths. Validate the full delimiter/info-string shape, including backtick restrictions, before selecting the Rust language. Keep the existing matched closing-delimiter rules and required inventory.
3. Add an end-to-end temporary `validateInventory` fixture test for the exact hidden-marker document above under LF and CRLF. Both must reject the required example. A repaired standalone marked Rust block must pass in both forms.
4. Add an invalid marked-opener regression for the second example; it must fail before compilation. Keep valid tilde/backtick, longer-close, mismatch, empty, duplicate, and unavailable-symbol controls. Do not substitute regex checking for actual Rust compilation.
5. Rerun focused document tests, actual example compilation, link checking, and the fast entry point. Correct T03/completion claims using new evidence; no new lifecycle or real Apollo run is necessary for a parser-only follow-up unless additional changes affect those paths.

Treat A and B as one small parser correction, rather than another general documentation rewrite. No new dependency or Markdown renderer is needed.

## Verification performed

| Check | Result |
|---|---|
| Bash syntax for lifecycle helper and main runner | Passed |
| Focused lifecycle and documentation tests | 32 passed: 21 lifecycle + 11 documentation |
| `scripts/test.sh fast` | Passed |
| Native unit tests | 49 passed in each default/Rustls configuration |
| Public error consumer tests | 4 passed in each native configuration |
| Doctests / WASM unit tests | 38 / 10 passed |
| Three Clippy configurations | Passed |
| All Node tooling tests | 43 passed |
| Four actual canonical examples | Compiled under both native configurations |
| `scripts/test.sh integration --suite native` | 6 real Apollo tests passed; normal teardown completed |
| `git diff --check` | Passed |
| Additional CRLF required-inventory probe | Reproduced false acceptance |
| Additional invalid marked-opener compile probe | Reproduced false acceptance and successful compilation |

The integration project was `apollo-test-1789284259-df077cb9`. Local review logs: `/tmp/apollo-review3-focused.log`, `/tmp/apollo-review3-fast.log`, `/tmp/apollo-review3-native.log`. Temporary logs are not required to understand or reproduce the findings above. Tests needing local sockets/Docker ran with the necessary access.

The new lifecycle regressions passed for INT/TERM during automatic teardown, mixed first/second signals, interruption during diagnostic collection, and the actual recovery CLI. These exercise the original failures; I found no further blocker in those exercised lifecycle paths. Full Rustls/WASM real-server suites were not repeated because this follow-up did not change the client and the prior review already ran them.

## Non-blocking cleanup and evidence feedback

- T02 says deferred/repeated-signal behavior was documented in `tests/apollo/README.md`, but that file has no implementation diff and its signal paragraph still describes only the earlier unified EXIT behavior. Add a concise explanation of deferral and first-signal-wins, or correct the evidence claim.
- New lifecycle tests register `waitForExit` after sending a signal, and their `finally` blocks kill without awaiting runner closure. Attach the exit promise immediately after spawn and await shutdown on failure paths to avoid missed fast exits or cleanup races. Read recorded PID files in `finally` even if readiness waiting failed, so failures before PID assignment do not leave an untracked stub group.
- The top-level records now mostly agree about implementation status. They need only narrow updates for the parser finding; do not undo completed baseline work or rewrite the prior review's historical observations.

No claim is made about Linux CI, browser/Windows certification, external URLs, heading anchors, every unmarked example, or deferred D-* policies.


## Final resolution verified (2026-09-13)

Both required parser cases above are corrected: line splitting normalizes CRLF, and ordinary/marked fences share `parseOpeningFence`. Independent `validateInventory` probes reject hidden required markers under LF and CRLF, accept repaired examples, and reject the invalid marked opener before compilation. The full fast suite passes with 45 tooling tests; all 19 real Apollo tests and fresh generated ownership smoke pass. No blocker remains from this review. Historical findings and non-blocking suggestions above are retained; see the [current audit](../verification-2026-09-13.md).
