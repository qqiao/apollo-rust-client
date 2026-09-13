# Feature Specification: Public typed cache errors

**Feature ID:** `007-public-cache-errors`
**Created:** 2026-09-13
**Status:** Specified; implementation not started.
**Review:** R2 / P2. Baseline: [001](../001-read-configuration/spec.md), 001/FR-010 and 001/AC-014.

## Purpose and scope

Rust application developers must be able to inspect cache errors with ordinary enum matching, including HTTP status and timeout details, without importing private implementation modules or parsing error display strings.

In scope: a public name for the existing enum, consumer tests, and API documentation. Out of scope: making Cache public, adding a new error hierarchy, changing variants/display/source chains, preserving original typed errors for coalesced followers, changing listener error snapshots, or altering JavaScript Error conversion.

## User stories and acceptance scenarios

### Story 1 — Handle specific retrieval failures (P1)

As a Rust consumer, I want to distinguish HTTP rejection from deadline expiry using stable public paths.

**Independent test:** Compile and execute an integration-test target that imports only the public crate API and constructs/matches representative existing errors.

- **AC-001:** Given an external Rust module, when it imports `apollo_rust_client::{CacheError, Error}`, then it can match `Error::Cache(CacheError::HttpStatus { status, body })` and obtain the numeric status/body without string parsing.
- **AC-002:** Given `Error::Cache(CacheError::Timeout { seconds })`, when matched externally, then the configured duration is available as a numeric field.
- **AC-003:** Given existing code that matches `Error::Cache(inner)` and prints or uses its standard error behavior, when rebuilt, then it continues compiling and its conversion/display semantics remain unchanged.

### Story 2 — Keep implementation encapsulated (P2)

As a maintainer, I want the error available without making cache state an external API.

**Independent test:** Inspect exports and compile normal native/WASM library targets.

- **AC-004:** Given the public error alias, when the crate is built for existing supported configurations, then the alias denotes the same existing enum while the `cache` module and Cache implementation remain private.
- **AC-005:** Given a coalesced follower or listener error, when inspected, then its existing snapshot representation remains documented rather than promising a typed original HTTP status for every error path.

## Functional requirements

- **FR-001:** The crate MUST publicly re-export the existing cache error enum as `apollo_rust_client::CacheError`.
- **FR-002:** All existing enum variants/fields MUST remain matchable through that name; the alias MUST NOT be a new wrapper enum.
- **FR-003:** The change MUST preserve `Error::Cache` payload identity, conversions, display/source behavior, and existing JS error mapping.
- **FR-004:** Cache implementation modules/types MUST remain private except for the error re-export.
- **FR-005:** Public documentation MUST demonstrate typed matching and explain the existing coalesced/listener snapshot limits.

## Entities and success criteria

**Entities:** public `CacheError`, top-level `Error::Cache`, existing `HttpStatus`, `Timeout`, and `CoalescedRefresh` variants.

- **SC-001:** External consumer tests prove AC-001–003 and compile in both native feature configurations.
- **SC-002:** Existing native/WASM Clippy, Rust tests, and doc tests pass without new dependencies or protocol/representation changes.
- **SC-003:** Rustdoc exposes the alias with a useful enum reference and a compiling match example; cache implementation is not accidentally exported.

## Assumptions

This is an additive API exposure. No promise of identical typed errors for coalesced callers is introduced. The public name is fixed here to prevent executor guesswork. See [plan](plan.md) and [tasks](tasks.md).
