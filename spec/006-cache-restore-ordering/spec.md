# Feature Specification: Preserve refresh results during cache restoration

**Feature ID:** `006-cache-restore-ordering`
**Created:** 2026-09-13
**Status:** Specified; implementation not started.
**Review:** R1 / P1. Depends on [002](../002-retain-configuration/spec.md) and [003](../003-observe-updates/spec.md).

## Purpose and scope

An application starting from persisted configuration must not revert to an older stored response after a concurrent remote refresh has successfully populated that same client namespace. This is a narrow correction to retained-state ordering, not a new freshness or global version-ordering policy.

In scope: one cache instance, overlapping cold restoration and explicit/periodic refresh, returned values, retained memory, and notifications caused by discarded restoration. Out of scope: ordering independent remote responses across clients, timestamp comparison as a server version, cross-process cache coordination, detached task cancellation, global callback ordering, new storage formats, and eager content validation.

## User stories and acceptance scenarios

### Story 1 — Keep the successful refresh (P1)

As an application developer, I want a cold read overlapping refresh to preserve the response already installed in memory.

**Independent test:** Hold a persistent read after it has obtained `old`, complete a remote refresh that installs `new`, then release the persistent read.

- **AC-001:** Given memory initially empty and a suspended restoration of `old`, when refresh finishes installing `new` before restoration commits, then the cold read returns `new` and subsequent reads retain `new`.
- **AC-002:** Given the same overlap with a listener registered beforehand, when the stored candidate is discarded, then it produces no `old` notification; the refresh's `new` notification is preserved.
- **AC-003:** Given a persisted response and no competing memory population, when the cold read completes, then it returns/stores that response and emits the ordinary first-population notification once.

### Story 2 — Preserve existing availability and concurrency (P1)

As an application developer, I want the ordering fix to preserve prompt stale reads and cancellation recovery.

**Independent test:** Restore stale data while holding remote revalidation open; then exercise duplicate cold reads, failed remote refresh, and cancellation.

- **AC-004:** Given a stale stored response and no memory winner, when read, then the stale response returns before revalidation completes and one active revalidation is shared.
- **AC-005:** Given multiple cold readers and a stored response, when they overlap, then they share the installed response without duplicate first-population notifications or unnecessary remote reads for a fresh item.
- **AC-006:** Given a refresh failure while restoration is pending, when valid persisted data is released and memory is still empty, then it is usable; a failed generation must not by itself discard the only retained response.
- **AC-007:** Given a cancelled initial reader, when another caller reads the same namespace, then it can complete; no coordination state stays permanently occupied.

## Functional requirements

- **FR-001:** Restoration MUST populate memory only if no item is present at the atomic restoration-commit decision.
- **FR-002:** If memory already contains an item at that decision, the read MUST use that item, including its timestamp/freshness behavior; it MUST NOT return the discarded persisted candidate.
- **FR-003:** A discarded candidate MUST NOT mutate retained state or produce a population/change notification.
- **FR-004:** A candidate installed into empty memory MUST retain ordinary first-population notification and fresh/stale/zero-TTL semantics from feature 002.
- **FR-005:** The fix MUST preserve cancellation-safe coalescing and MUST NOT require a retained read to wait for remote request completion.
- **FR-006:** Restoration MUST NOT interpret wall-clock timestamps as globally ordered Apollo release versions or suppress valid persisted fallback solely because a failed refresh completed.

## Key entities

- **Persisted candidate:** the stored item obtained before the atomic installation decision.
- **Memory winner:** the item already present at that decision, regardless of how it was populated.
- **Restoration commit:** the indivisible empty-check and optional installation.

## Success criteria

- **SC-001:** The controlled AC-001/002 interleaving deterministically returns/retains `new`, with exactly the expected `new` event and no discarded `old` event.
- **SC-002:** AC-003–007 and existing stale-read, zero-TTL, coalescing, listener, cancellation, persistence-failure, and unchanged-refresh regressions pass.
- **SC-003:** Native default/Rustls and WASM compile/lint/test checks pass without a new public API, dependency, or persistent format.

## Assumptions

“Winner” is determined by memory occupancy at commit, not an arbitrary timestamp comparison. A later legitimate refresh can still replace memory after restoration; no cross-operation event serialization is added. Detailed mechanisms and test scheduling live in [plan.md](plan.md); execution is in [tasks.md](tasks.md).
