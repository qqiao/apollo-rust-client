# Plan: Document the actual polling model

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

## Source anchors and facts

Read [src/lib.rs](../../src/lib.rs) `refresh_loop`, `platform_sleep`, `start_background`; [src/cache.rs](../../src/cache.rs) `refresh_delay_seconds`, `is_backing_off`, `perform_refresh`, `schedule_revalidation`, `is_fresh`; and [003](../003-observe-updates/spec.md).

Facts to preserve:

1. Starting polling schedules an immediate first round; it does not invent unused namespaces. Reads/preload/refresh/listener registration add namespace caches.
2. Each round snapshots currently eligible namespaces, runs up to four refresh futures concurrently, awaits their completion, and sleeps the configured interval. Backing-off namespaces are excluded at snapshot time.
3. Healthy platform_sleep has no random jitter. For R = round duration and I = interval, next-round start is approximately previous start + R + I, plus scheduling overhead.
4. No application-wide freshness bound follows from I alone. Server-side propagation, request/body waits, queued waves beyond concurrency four, persistence, synchronous callbacks, runtime scheduling, and backoff all matter. Request timeout covers network waiting, not a full refresh round or callbacks/storage.
5. Failure backoff belongs to each Cache namespace; only polling checks eligibility. Explicit refresh and stale-read revalidation bypass it. Success resets the failure state.
6. TTL governs whether a read triggers revalidation, separately from periodic work. Zero TTL retains cached availability while always making a read eligible for revalidation; it does not disable caching or impose a polling interval.

If documenting the numeric formula, use the current exact one: nominal delay `min(saturating(b * 2^min(n,4)), max(b,300))`, where b is base refresh interval and n consecutive failures; integer jitter is applied within `floor(delay / 10)` seconds on either side. Do not claim all configurations cap at exactly 300 seconds, or that the jittered result cannot exceed 300. With b=30, nominal first four failure delays are 60,120,240,300 seconds; jitter ranges are 54–66,108–132,216–264,270–330. Further failures remain at that nominal cap. Recheck source if it changed since review.

Avoid adding this formula to every user page. Keep one technical explanation in supporting/design or Design-Cache and a short accurate overview in the READMEs.

## Suggested replacement summary

Use equivalent natural wording in each language:

> The client periodically reads Apollo's cached configfiles endpoint. Each round refreshes eligible registered namespaces with up to four concurrent operations, then waits refresh_interval after the round completes. Healthy polling has no intentional jitter. Namespace failures use bounded exponential retry backoff with jitter; successful refresh resets that delay. Update observation can take longer than refresh_interval because round duration, server propagation, and retries also contribute. Cache TTL governs read-triggered revalidation separately from this periodic schedule.

Where discussing request rate, state that manual refresh and stale revalidation bypass periodic backoff; do not promise a global rate ceiling. Preserve the existing explicit absence of notification long-polling/releaseKey. Do not replace false latency guarantees with new universal numbers.

## Documents and discovery

Start with README.md/README_zh.md Update Model and CHANGELOG.md Unreleased. Search all current guides and source rustdoc:

```sh
rg -n -i 'jitter|抖动|抖動|latency|延迟|延遲|refresh_interval|backoff|退避' README.md README_zh.md CHANGELOG.md docs/wiki spec/supporting/design.md src/lib.rs src/cache.rs
```

Classify each match as correct current statement, incorrect current claim, or explicitly historical. Existing source comments already accurately mention failure backoff in several places; do not change every mention of jitter. Edit only inaccurate statements and their needed immediate context. Package 011 owns API/error/link correctness; preserve its edits.

## Verification

This is documentation-only. No new scheduler tests or runtime changes are needed. Use package 011's `node scripts/check-doc-examples.mjs`, `node scripts/check-doc-links.mjs`, plus `git diff --check`. If a rustdoc example is changed, run `scripts/test.sh fast`; ordinary polling prose changes do not require another Docker run.

In tasks.md record a compact claim → source symbol → document table for AC-001–006, final search dispositions, and the diff review proving no executable behavior changed. Mark the old Unreleased jitter claim corrected; preserve released history unless actual source-history evidence justifies an additional edit. Do not claim the review's old green tests verify a newly changed algorithm, because no algorithm change is intended.
