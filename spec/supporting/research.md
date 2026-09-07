# Reconstruction research and decision register

## Scope and method

Initial phase direction: regenerate retrospective specifications (001–004) from scratch in `spec/`, using Spec Kit-style documentation. Reconstruct existing product capabilities without redesigning the client. (Feature 005 subsequently added real Apollo test infrastructure and test migrations without modifying production client contracts).

Evidence baseline: commit `4473cffe7a8a45c86978053da23ef36553b0ea3d`, package manifest 0.7.0 plus Unreleased changes. Sources inspected include [README](../../README.md), [CHANGELOG](../../CHANGELOG.md), [client API](../../src/lib.rs), [configuration](../../src/client_config.rs), [cache](../../src/cache.rs), [format adapters](../../src/namespace/mod.rs), embedded tests, [test entry point](../../scripts/test.sh), and the previous design/wiki documentation.

Interpretation rules:

1. Explicit user direction determines deliverable scope and format.
2. Public documentation and changelog establish evidence of consumer intent; tests provide concrete expected examples.
3. Code determines what is currently implemented, but a code-only quirk is not automatically a desired requirement.
4. When those disagree, specify the supported outcome only where evidence is sufficient; record uncertainty or a gap separately. Do not invent why a historical choice was made.

Format reference: the [official Spec Kit template](https://github.com/github/spec-kit/blob/main/templates/spec-template.md), inspected 2026-09-06. The four specs adapt its story/requirement/outcome organization to a retrospective library specification. A `Feature ID` replaces a fictional feature branch; technical design and evidence are separate. The project was not scaffolded with Specify and no generated implementation plan is claimed.

## Reconstruction assumptions

| Assumption | Basis and disposition |
|---|---|
| Primary users are native/JavaScript application developers and service operators. | README and public APIs; explicit personas in feature stories. |
| Retrospective scope includes Unreleased behavior at HEAD. | The user requested the existing codebase; manifest version alone does not identify shipped behavior. |
| Core capabilities are read, retain, observe, and JavaScript consumption. | User journeys, not a proposed module decomposition; shared dependencies are explicit. |
| P1/P2 priorities express functional dependency and consumer value. | Reconstructed for review; no evidence of historical roadmap approval is claimed. |
| No new operational SLO, coverage percentage, version matrix, or payload limit is imposed. | No established values were found. Outcomes use controlled behavioral fixtures instead. |
| Existing code preserved during retrospective task. | Initial phase scope was documentation; Feature 005 later added real integration tests and CI orchestration without changing public client behavior. |

## Open decisions and implementation gaps

None of these rows authorizes an implementation change. They delimit what these draft feature contracts do and do not settle. No speculative requirement is hidden behind a generic TODO placeholder.

| ID | Evidence / current observation | Decision or gap |
|---|---|---|
| D-001 | Stale responses have no maximum age. Only periodic polling consults retry backoff; stale reads and manual refresh bypass it (`schedule_revalidation`, `refresh_loop`). | Decide whether reads should honor failure backoff and whether bounded staleness needs a separate mode. Baseline retains availability behavior without claiming a maximum request rate. |
| D-002 | JSON validates on namespace conversion; native YAML defers syntax parsing until typed conversion. Preload/refresh accept outer JSON without universal typed-content validation. WASM YAML/serialization failures log and return null. | Decide whether to require eager validation and consistent JS rejection. Current specs define known conversion boundaries, not a promise that every successful retrieval contains valid typed data. |
| D-003 | Stop/drop aborts the owned poller; `schedule_revalidation` creates detached work with its own cache reference. | Decide whether all already scheduled work/callbacks must be cancelled on stop/release. The current specification guarantees owned-poller cancellation only. |
| D-004 | Callback memory/listener-list locks are released but refresh/load locks can remain held. | Decide callback reentrancy and slow-callback policy. Current synchronous delivery must not be described as universally lock-free or safely reentrant. |
| D-005 | Temporary-file cleanup checks filename only, not age or active writer. Writes flush/rename without fsync. | Establish cross-process sharing and crash-durability requirements before promising them; same-process writer tests do not prove them. |
| D-006 | Timing validation rejects zero interval/timeout only; TTL/deadline arithmetic uses signed casts; future persisted timestamps can appear fresh. WASM timer milliseconds are capped. | Define upper limits and clock-skew policy. Do not freeze overflow or future-timestamp behavior as intended requirements. |
| D-007 | Cache identity omits credentials, only trims trailing server slashes, and uses architecture-sized length prefixes. Persistent configuration is plaintext. | Decide portability/credential-isolation/confidentiality needs. Server authorization is not a cache access-control guarantee. |
| D-008 | There is no namespace eviction or listener removal; preload has no concurrency ceiling. | Define a dynamic-namespace workload and resource limits before adding such requirements. |
| D-009 | Trace logging can include response/config content; HTTP errors include full bodies. | Define redaction and response-size policy if needed. The spec promises diagnostics, not confidentiality or bounded diagnostic volume. |
| D-010 | No explicit minimum Rust/runtime matrix, numeric coverage goal, or production performance SLO is established. Empty namespace and standalone dot-segment handling lacks an agreed validation policy. | Obtain deployment/product constraints when these areas become change scope. Do not infer them from tests or latest-stable guidance. |
| D-011 | An explicit refresh joining a cold initial read can reuse its failed result without a listener error: `coalesced_refresh` returns early after a generation change, while the initial read used `notify_error = false`. | Decide the notification obligation for the overlapping case. Feature 003 scopes FR-004 to retrievals initiated by refresh and leaves this case pending, rather than claiming comprehensive notification coverage. |

## Conflicting descriptions at the source baseline

| Description | Reconciled evidence |
|---|---|
| [Interface design](../../specs/interface_design.md) routes unknown suffixes to Text. | Namespace tests and code use Properties; Text requires `.txt`. |
| Its sequence diagram holds the memory writer through remote I/O. | Current code has distinct load/refresh coordination and avoids holding the memory write lock through that I/O. |
| Listener documentation says all internal locks are released. | Only memory/listener-list locks are released; see D-004. |
| Older JS table omits preload/refresh and describes signed integers as numbers. | Exports include those methods and signed Properties integers use bigint. |
| [Memory guide](../../docs/wiki/en/WASM-Memory-Management.md) suggests freeing ClientConfig after passing it to Client. | Constructor accepts configuration by value; consuming ownership must be distinguished from a live unconsumed wrapper. Generated ownership smoke coverage remains incomplete. |
| Changelog says polling uses per-client jitter; some test prose implies Rustls runtime tests. | Code applies jitter to namespace failure delays, not healthy polling sleep. The script lints Rustls but runs native tests with default features. |

The initial retrospective regeneration wrote only `spec/`. During final verification, concurrent documentation-only edits appeared in `src/namespace/mod.rs`, `specs/interface_design.md`, and three English wiki files (`Design-Overview`, `Features`, `Rust-Usage`). They correct the unknown-suffix Properties description and do not change executable behavior. Those edits were inspected and preserved. The table records discrepancies at the baseline revision; the unknown-suffix discrepancy is already corrected in the current working tree.

## Independent content review

A reviewer with fresh context inspected the four primary specifications against the requested format and source. Six findings were accepted and corrected: server-owned authorization was separated from client behavior; retained-data preconditions were added to JS failure/deadline scenarios; a coalesced error-notification gap was recorded as D-011; circular conversion language was replaced by explicit error cases; retry language was corrected for capped/jittered delays; and an unchanged-response freshness-renewal scenario was added.

The user chose to finish with this independent review and declined an additional external cross-model review. No external review run is claimed. A follow-up check of the corrected artifact is recorded in the checklist.
