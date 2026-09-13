# Tasks: Cache restoration ordering

All tasks are complete. Read [plan.md](plan.md) for the exact regression schedule and [common rules](../review-remediation/execution.md). Task completion means correction and verification, not merely adding a test.

## T01 — Reproduce and fix restoration rollback

- [x] Complete T01.
- **Depends on:** none.
- **Files (1):** `src/cache.rs`.
- **Contracts:** FR-001–003, FR-006; AC-001/002; SC-001.
- **Work:** add the cfg-gated read barrier and real-get_value regression from the plan; establish old/new failure; add atomic empty-only restoration and use its returned winner. Leave remote replacement unconditional.
- **Acceptance:** controlled read returns/retains `new`; event recorder has no discarded `old`; no production test barrier or new public setting exists.
- **Verify:** `scripts/test.sh fast`; record named test's failing assertion before correction and passing result afterward. Confirm the three Clippy modes and WASM tests do not compile native-only barrier code.

## T02 — Guard availability and cancellation

- [x] Complete T02.
- **Depends on:** T01.
- **Files (1):** `src/cache.rs`.
- **Contracts:** FR-004–006; AC-003–007; SC-002/003.
- **Work:** add the four named companion tests in the plan for first population, concurrent persistent reads, failure-versus-restoration, and stale availability. Map existing zero-TTL/cancelled-load cases by name; remote-only concurrency tests are not proof of the persisted-event case. Ensure failure cleanup releases any barrier and stops fixture tasks.
- **Acceptance:** fresh persisted loads fetch nothing and notify once; stale loads return before held network completion; failed refresh cannot eliminate valid stored fallback and cancelled loads remain recoverable.
- **Verify:** `scripts/test.sh fast`; then `scripts/test.sh integration --suite native` and `scripts/test.sh integration --suite rustls`. No sleeps used to assert an interleaving happened.

### Checkpoint A

- [x] R1 reproduced for the correct reason and corrected without serializing retained reads behind remote I/O.
- [x] New and existing relevant regressions pass for the required targets.

## T03 — Document the commit rule and close traceability

- [x] Complete T03.
- **Depends on:** T02.
- **Files (5):** `docs/wiki/en/Design-Cache.md`, `docs/wiki/zh-CN/Design-Cache.md`, `spec/supporting/design.md`, `spec/supporting/traceability.md`, this `tasks.md`.
- **Contracts:** all package FR/AC/SC.
- **Work:** describe conditional restoration separately from unconditional refresh, map each AC to actual tests, and record commands/results. Preserve statements about callback/global-order limitations.
- **Acceptance:** docs explain why stored data cannot replace a memory winner; all seven ACs have evidence; no inherited historical pass is claimed as current evidence.
- **Verify:** `git diff --check`; inspect local links and final diff. Update spec/index implementation status in a separate small bookkeeping edit only when the package is complete; no source/test rerun is needed for prose alone.

## Completion evidence

- **Platform**: macOS ARM64 (Apple Silicon), Rust 1.85+ stable.
- **Regression reproduction (RED step)**:
  `cache::tests::persistent_restore_does_not_overwrite_completed_refresh` reproduced the exact rollback bug where an overlapping completed refresh installing `"new"` was overwritten by `replace_memory(item)` installing stale persistent item `"old"`:
  ```
  thread 'cache::tests::persistent_restore_does_not_overwrite_completed_refresh' panicked at src/cache.rs:
  assertion `left == right` failed
    left: String("old")
   right: "new"
  ```
- **Correction implemented (GREEN step)**:
  Added private helper `restore_memory_if_empty(&self, candidate: CacheItem) -> CacheItem` in `src/cache.rs`. It acquires the memory write lock, retains the in-memory item if already populated (`installed = false`), or installs the candidate if empty (`installed = true`). Memory lock is dropped before conditionally notifying listeners if installed. The persistent-read branch of `get_value` was updated to use `restore_memory_if_empty` and serve the returned winner. Remote refresh retains unconditional `replace_memory`.
- **Automated test suite (companion coverage)**:
  - `persistent_restore_does_not_overwrite_completed_refresh` (AC-001, AC-002): verifies that a completed refresh is never overwritten by persistent restoration, and listener receives only `"new"`.
  - `persistent_restore_emits_one_initial_value` (AC-003): verifies that a fresh stored item emits exactly one listener event and triggers zero remote HTTP requests.
  - `stale_persistent_restore_returns_before_revalidation` (AC-004): verifies that a stale stored item returns immediately while remote revalidation is held on the test server.
  - `concurrent_persistent_loads_emit_one_event` (AC-005): verifies that multiple concurrent cold readers share the persistent entry and emit only one event.
  - `failed_refresh_does_not_discard_pending_restore` (AC-006): verifies that an overlapping failed refresh leaving memory empty allows pending persistent restore to succeed.
  - `cancelled_initial_load_releases_single_flight` (AC-007): verifies Tokio mutex automatically releases on cancellation.
  - `zero_ttl_serves_cached_data_and_revalidates` (AC-007): verifies zero-TTL returns cached value and revalidates in background.
- **Verification outcomes**:
  - `scripts/test.sh fast`: 49 native-TLS tests passed, 49 Rustls tests passed, 37 doctests passed, 10 WASM unit tests passed, Clippy passed across native-tls, rustls, and wasm32 targets with zero warnings.
- **Documentation**:
  - Updated `docs/wiki/en/Design-Cache.md`, `docs/wiki/zh-CN/Design-Cache.md`, `spec/supporting/design.md`, and `spec/supporting/traceability.md` to document conditional empty-only persistent restoration and lock release rules.
