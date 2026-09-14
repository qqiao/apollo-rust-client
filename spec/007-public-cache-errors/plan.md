# Plan: Public cache error re-export

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

## Source and exact change

Inspect [src/lib.rs](../../src/lib.rs) (`mod cache;`, public `Error`) and [src/cache.rs](../../src/cache.rs) (public internal-module enum Error). Add a documented re-export near module declarations:

```rust
/// Errors produced by configuration retrieval and cache operations.
///
/// Match this enum inside [`Error::Cache`]. Coalesced refresh followers can
/// receive string snapshots instead of the original error variant.
pub use cache::Error as CacheError;
```

Keep `mod cache;` private. Do not change it to `pub mod cache`, re-export Cache, duplicate the enum, add accessors as a substitute for matching, rename existing variants, or add `#[non_exhaustive]` as unrelated API policy. Keep top-level `Error::Cache(#[from] cache::Error)` or spell its payload with the alias only if needed for rustdoc; either must remain the exact same type.

## External regression target

Create `tests/public_errors.rs`, gated with `#![cfg(not(target_arch = "wasm32"))]`. Unlike inline src unit tests, this is a consumer of the public library, so missing exports fail at compile time.

Use ordinary `#[test]` functions; no server, cache directory, async runtime, or dependency required. Tests:

1. Construct `CacheError::HttpStatus { status: 429, body: "rate limited".into() }`, convert into top-level Error through existing `From`, match it externally, assert exact fields.
2. Construct/match `Timeout { seconds: 10 }` and assert numeric value.
3. Construct `CoalescedRefresh("failure snapshot".into())`, convert/match it, and assert its existing Display includes its snapshot. Demonstrate that the alias is usable while preserving fallback handling.
4. Preserve the old `Error::Cache(inner)` pattern and `std::error::Error` use. Do not freeze every punctuation character of Display unless an existing contract requires it.

The initial red result must be unresolved import `CacheError`; the green result must compile and run outside the library. A source-string grep is not a replacement for this test. A separate compile-fail framework to test private Cache access is unnecessary: inspect the one-line export and leave the module declaration private.

The current fast path runs external integration-test targets on default native through `--all-targets`, but only library tests under Rustls. Change the Rustls test invocation in both `scripts/test.sh` and `scripts/apollo-test.sh` from `--lib` to `--all-targets`. Real-Apollo tests stay ignored in fast mode because they are `#[ignore]`; do not pass `--ignored`. This makes the new public boundary regression execute under both TLS configurations without adding a new CLI mode. Retain every existing lint/doctest/WASM command.

## Documentation and checks

Add a no-network rustdoc example on the alias or top-level Error showing a function matching HTTP status and timeout, plus a fallback. Document at least `CoalescedRefresh(String)` and `Error::Refresh(String)` limits rather than claiming all observers preserve typed original failures.

Update supporting/contracts.md's current consumer/error rows only after the alias exists; add the actual test names to traceability. Package 011 owns broad wiki/translation error-taxonomy correction; do not duplicate that large edit here.

Run `scripts/test.sh fast` and inspect both default/Rustls external-target summaries. Build/lint WASM through the unchanged fast command. Use `git diff --check` and verify public rustdoc links. This additive alias needs no new live-Apollo setup, version bump, Cargo.lock change, or generated JS declaration change.
