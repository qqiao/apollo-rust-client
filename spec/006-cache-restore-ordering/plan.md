# Plan: Cache restoration ordering

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

## Read first

Read the local [spec](spec.md), baseline 002/FR-004, FR-005, FR-011–012 and 003/FR-002–003. Inspect `Cache::get_value`, `load_persistent_item`, `replace_memory`, `perform_refresh`, `serve_cached_item`, and the existing tests in [src/cache.rs](../../src/cache.rs). `Client::cache` in [src/lib.rs](../../src/lib.rs) supplies shared namespace instances. Existing `TempDir`, `MockHttpsServer`, and `MockResponse` are sufficient for regression fixtures.

At baseline, successful restoration bypasses the generation check: `load_persistent_item().await` → unconditional `replace_memory` → return old candidate. Merely moving/checking the generation outside the memory write lock is still racy. Taking `refresh_lock` before returning stored values would undermine stale availability.

## Intended implementation

Introduce a private helper with a clear name such as `restore_memory_if_empty(&self, candidate: CacheItem) -> CacheItem`:

1. Acquire `self.memory.write().await`.
2. If `Some(current)`, clone current, mark `installed = false`, and leave memory unchanged.
3. Otherwise put the candidate into memory, retain its clone as the selected return item, and mark `installed = true`.
4. Release the memory guard in a lexical block.
5. If installed, snapshot listeners under their read lock, release that lock, and call `notify_listeners` with the installed item's config.
6. Return the selected item.

Change only the persistent-success branch of `get_value` to call this helper and pass its returned item to `serve_cached_item`. Keep `replace_memory` as the unconditional remote-refresh operation: converting it globally to “insert if empty” would prevent all future updates. Do not hold memory/listener-list locks while invoking callbacks. Existing load/refresh coordination locks may still be held; reentrancy is not being added.

No new generation rule, timestamp ordering, persistence write, public field, or feature flag is necessary. If choosing another equally small implementation, it must perform the check and assignment in one write-lock critical section and return the actual selected item.

## Deterministic behavioral regression

Use the real `get_value` path rather than testing the helper alone. Preferred portable native test seam:

- Add a test-only barrier field to `Cache`, under `#[cfg(all(test, not(target_arch = "wasm32")))]`, initialized to `None` by `Cache::new`.
- Define a private test-only barrier with two `tokio::sync::Notify` values: `loaded` and `release`. Store it behind `Arc` so Cache clones share it. No public ClientConfig setting or production field is allowed.
- In native `load_persistent_item`, after a valid item is read/parsed but before returning it, if the barrier is present, `loaded.notify_one()` then await `release.notified()`. `notify_one` retains a permit if the observer has not yet started waiting. Do not gate network refresh or persistence writes with this barrier.
- Normal tests leave the barrier `None`. Do not introduce scheduling sleeps or mocked values in the production path. Keep test-only code small and cfg-gated; an existing equivalent internal test seam may be reused.

Test `persistent_restore_does_not_overwrite_completed_refresh`:

1. Create an isolated TempDir and HTTPS server returning `{"value":"new"}`. Configure a long positive TTL and the existing test TLS override/client.
2. Construct Cache, persist `CacheItem { timestamp: now, config: {"value":"old"} }` with the existing file writer; keep memory empty. Attach the barrier and an ordered event recorder.
3. Spawn `get_value` on a cache clone. Await `loaded` under a bounded timeout.
4. Call `cache.refresh().await` under a bounded timeout. Assert its request succeeded, memory contains `new`, and the listener recorded `new`.
5. Release the storage barrier. Join the cold read; assert its returned value is `new`, memory and a subsequent read are `new`, event list is exactly `[new]`, and no extra fetch was needed while fresh.
6. Always release the barrier and abort/join the spawned task on assertion/timeout failure. A small test guard is acceptable; do not leak a suspended task or TempDir. Suggested per-phase timeout: 2 seconds, with one outer watchdog allowing slow CI startup.

Before the production fix, this test must fail on the old/new assertion, not on TLS, filesystem setup, timeout, or compilation. The review's FIFO test demonstrated the same ordering but is not required here; do not depend on `/tmp` review files or a platform `mkfifo` utility.

Add named companion behavioral tests:

- `persistent_restore_emits_one_initial_value` (AC-003): prewrite a fresh item, keep memory empty, read twice, assert exactly one population event and zero server requests.
- `stale_persistent_restore_returns_before_revalidation` (AC-004): prewrite an expired item, hold the server response with explicit readiness/release signaling, assert get_value completes with old before releasing the server and only one active refresh exists. Reuse the existing held-response fixture mechanism if available; a short arbitrary sleep is not proof.
- `concurrent_persistent_loads_emit_one_event` (AC-005): overlap several get_value callers behind the restoration barrier, release it, and assert all see the persisted value, one population event, and zero requests while fresh. Existing remote-only coalescing tests do not prove this persisted-event case.
- `failed_refresh_does_not_discard_pending_restore` (AC-006): suspend fresh valid stored data, complete an explicit failing HTTP refresh with memory still empty, release restoration, and assert stored data is usable. Preserve the existing explicit-refresh error event; do not silently redefine D-011.

Reuse `cancelled_initial_load_releases_single_flight` for AC-007 and the existing zero-TTL/coalescing regressions as additional compatibility evidence; extend rather than duplicate only where they actually cover the required path. Test-only changes must be excluded from normal native library and WASM builds.

## Verification and closure

Use `scripts/test.sh fast`; it runs both native configurations and WASM checks. Run `scripts/test.sh integration --suite native` and `--suite rustls` after the completed slice to detect public-client regressions. No Apollo fixture mutation beyond the normal harness is needed.

Update `docs/wiki/en/Design-Cache.md`, `docs/wiki/zh-CN/Design-Cache.md`, and baseline supporting design/traceability with conditional restoration semantics after implementation. Do not claim globally ordered callbacks or lock-free callbacks. Preserve D-003–D-007 policies. Task sequence and file-sized slices: [tasks.md](tasks.md).
