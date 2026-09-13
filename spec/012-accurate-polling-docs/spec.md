# Feature Specification: Accurate polling and freshness guidance

**Feature ID:** `012-accurate-polling-docs`
**Created:** 2026-09-13
**Status:** Specified; implementation not started.
**Review:** R7 / P2. Baseline: [003](../003-observe-updates/spec.md), 003/FR-007/FR-009 and 003/AC-008–010. Finish 011 first to avoid shared-document conflicts.

## Purpose and scope

Operators must be able to interpret refresh_interval, cache_ttl, request_timeout, and failure backoff without relying on nonexistent healthy-poll jitter or an incorrect maximum update-latency guarantee.

In scope: correction of current READMEs, current design/configuration/feature prose where it makes the same claim, and Unreleased changelog wording. Out of scope: implementing healthy jitter, changing poll timing/concurrency/backoff, adopting long polling/releaseKey, independent per-namespace schedulers, benchmarks, or new production freshness SLOs.

## Stories and acceptance scenarios

### Story 1 — Understand when periodic work runs (P1)

As an operator, I want polling timing described as the client actually schedules it.

**Independent test:** Compare the current documentation with refresh_loop and platform_sleep and the baseline contract.

- **AC-001:** Given a started client with eligible registered namespaces, when reading current polling guidance, then it explains that a round refreshes eligible namespaces with concurrency at most four, waits for that round to finish, and then sleeps refresh_interval before the next round.
- **AC-002:** Given normal successful polling, when reading jitter guidance, then it states that the healthy sleep uses the exact interval and has no intentional per-client ±10% jitter.
- **AC-003:** Given request, persistence, callback, scheduling, server-propagation, and retry delays, when reading latency guidance, then it does not promise update latency is bounded by refresh_interval alone.

### Story 2 — Distinguish freshness and retry controls (P2)

As an application developer, I want to know which controls apply to cached reads, periodic refresh, and explicit operations.

**Independent test:** Compare the control descriptions with is_fresh, is_backing_off, refresh_delay_seconds, schedule_revalidation, and refresh.

- **AC-004:** Given repeated refresh failures, when reading retry guidance, then jitter is scoped to the affected namespace's failure delay, bounded backoff is described accurately, and success resets failure state.
- **AC-005:** Given stale reads or explicit refresh, when reading guidance, then it notes that those paths bypass periodic backoff and that TTL controls read-triggered freshness separately from polling cadence.
- **AC-006:** Given equivalent translated/current source documentation, when read, then these distinctions agree; no runtime code has been changed to make old documentation true.

## Functional requirements

- **FR-001:** Current docs MUST describe refresh_interval as the sleep after a completed eligible refresh round, not a fixed-start cadence or maximum update latency.
- **FR-002:** Current docs MUST distinguish deterministic healthy sleep from jittered per-namespace failure backoff.
- **FR-003:** Current docs MUST state that request/round duration and other delays can extend observation latency; they MUST NOT introduce an unverified freshness SLO.
- **FR-004:** Current docs MUST distinguish cache TTL, request timeout, explicit refresh, stale revalidation, and periodic retry eligibility.
- **FR-005:** Unreleased claims and edited language variants MUST agree with the source; historical released behavior MUST not be rewritten without evidence.
- **FR-006:** This package MUST NOT modify executable Rust/JS/shell behavior, tests' behavior expectations, dependencies, or fixture values.

## Entities and success criteria

**Entities:** eligible polling round, round duration, post-round sleep, retained-value TTL, complete network deadline, per-namespace retry timestamp.

- **SC-001:** All six ACs are mapped to source symbols and corrected documents in the final task evidence.
- **SC-002:** A contextual search finds no current assertion of healthy per-client jitter or refresh_interval-only maximum latency in the defined doc scope; any remaining historical mention is explicitly justified.
- **SC-003:** Document example/link checks and diff whitespace checks pass; source-code diff contains no behavioral change.

## Assumptions

For a round taking R seconds and configured interval I, the next round starts roughly R + I seconds after the previous one, plus scheduling overhead. This is an explanatory relationship, not a bound on server publication or a numeric SLO. No change to the actual algorithm is authorized by this documentation finding. See [plan](plan.md) and [tasks](tasks.md).
