# Tasks: Public cache errors

All tasks complete. Each references the local spec and [plan](plan.md).

## T01 — Expose the existing enum and prove external matching

- [x] Complete T01.
- **Depends on:** none.
- **Files (2):** `src/lib.rs`, new `tests/public_errors.rs`.
- **Contracts:** FR-001–004; AC-001–004; SC-001/002.
- **Work:** write external import/match regressions, observe missing alias, add the documented re-export and compiler-checked rustdoc example. Preserve enum/type identity.
- **Acceptance:** HttpStatus/Timeout numeric fields match externally; old wrapping/conversion pattern works; Cache/module remain private.
- **Verify:** `scripts/test.sh fast`; record the intended unresolved-import failure before the export and subsequent pass. Inspect WASM Clippy success.

## T02 — Execute consumer tests under Rustls too

- [x] Complete T02.
- **Depends on:** T01.
- **Files (2):** `scripts/test.sh`, `scripts/apollo-test.sh`.
- **Contracts:** SC-001/002; AC-003/004.
- **Work:** replace the Rustls `--lib` test selector with `--all-targets` in both existing fast paths; retain flags and all checks. Do not run ignored real-server tests in fast mode.
- **Acceptance:** new external tests actually run in both native configurations; fast mode still never calls Docker; existing native unit/doc/WASM/lint checks remain.
- **Verify:** `sh -n scripts/test.sh`; `bash -n scripts/apollo-test.sh`; `scripts/test.sh fast`; inspect target/test names rather than relying on exit status alone.

### Checkpoint A

- [x] Public alias is additive, and external tests run for default native and Rustls.

## T03 — Document the boundary and evidence

- [x] Complete T03.
- **Depends on:** T02.
- **Files (3):** `spec/supporting/contracts.md`, `spec/supporting/traceability.md`, this `tasks.md`.
- **Contracts:** FR-005; AC-005; SC-003.
- **Work:** change pending alias text to the implemented signature, record tests/commands, and retain limitations of coalesced/listener snapshots. Point package 011 at the now-available alias.
- **Acceptance:** no claim every failure has original typed status; rustdoc example compiles; this package's scope is complete without broad guide edits.
- **Verify:** `git diff --check`, review relative links and API diff. Update spec/index status only after criteria are met.

## Completion evidence

- **Platform**: macOS ARM64 (Apple Silicon), Rust 1.85+ stable.
- **Reproduction (RED step)**:
  Created external test target `tests/public_errors.rs` importing `apollo_rust_client::{CacheError, Error}`.
  Running `cargo test --test public_errors` produced the expected compile error:
  ```
  error[E0432]: unresolved import `apollo_rust_client::CacheError`
   --> tests/public_errors.rs:3:26
    |
  3 | use apollo_rust_client::{CacheError, Error};
    |                          ^^^^^^^^^^ no `CacheError` in the root
  ```
- **Implementation (GREEN step)**:
  Added `pub use cache::Error as CacheError;` in `src/lib.rs` with documentation and a compiling rustdoc example. `mod cache;` remains private.
- **External consumer coverage (`tests/public_errors.rs`)**:
  - `match_http_status_externally` (AC-001): matches `Error::Cache(CacheError::HttpStatus { status, body })` and asserts status 429 and body without parsing strings.
  - `match_timeout_externally` (AC-002): matches `Error::Cache(CacheError::Timeout { seconds })` and asserts numeric timeout value (10).
  - `match_coalesced_refresh_externally` (AC-005): matches `Error::Cache(CacheError::CoalescedRefresh(ref snapshot))` and asserts display includes failure snapshot.
  - `preserve_existing_error_cache_matching_and_std_error` (AC-003): verifies `Error::Cache(inner)` matching and `&dyn std::error::Error` coercion.
- **Dual native TLS execution**:
  Updated `scripts/test.sh` and `scripts/apollo-test.sh` Rustls test invocation from `--lib` to `--all-targets`. Both default native (`native-tls`) and `rustls` run and pass all 4 tests in `tests/public_errors.rs`.
- **Verification outcomes**:
  - `scripts/test.sh fast`: 49 unit tests + 4 `public_errors` integration tests under native-tls, 49 unit tests + 4 `public_errors` integration tests under rustls, 38 doctests (including the new `CacheError` match example), 10 WASM unit tests, and Clippy across all 3 targets passed cleanly.
- **Documentation**:
  - Updated `spec/supporting/contracts.md` and `spec/supporting/traceability.md` to document the public alias and limitations of coalesced/listener snapshot errors. Package 011 is directed to use this public alias.
