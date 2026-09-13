# Requirement traceability and existing evidence

The feature specs define desired outcomes for review; this table records reconstruction evidence. IDs are qualified by the feature containing them. `AC-*` references point to acceptance scenarios in that feature.

**Evidence labels**: **Test + source** means existing tests cover examples of the behavior and code was inspected; it does not claim every scenario clause has automated coverage. **Source** means no dedicated acceptance test was established. **Partial** highlights a specific material gap. Existing tests were not rewritten into requirements: the scenarios can be implemented with another suitable fixture.

Sources: [configuration tests](../../src/client_config.rs), [cache tests](../../src/cache.rs), [client tests](../../src/lib.rs), [namespace routing](../../src/namespace/mod.rs), [Properties](../../src/namespace/properties.rs), [JSON](../../src/namespace/json.rs), [YAML](../../src/namespace/yaml.rs), [generated JS smoke](../../scripts/wasm_api_smoke.js).

## 001 — Read typed configuration

| Requirement | Acceptance | Evidence and remaining coverage |
|---|---|---|
| FR-001 | AC-001, AC-003 | Test + source: `test_string_value`, `properties_public_and_text_namespaces_use_the_correct_types`; source `Client::namespace`. |
| FR-002 | AC-002, AC-004 | Test + source: `builder_applies_consistent_defaults_and_validates`, `from_lookup_is_deterministic_and_complete`; not every optional setter/environment mapping has a dedicated case. |
| FR-003 | AC-005, AC-006 | Test + source: `invalid_optional_environment_values_are_reported`, `builder_applies_consistent_defaults_and_validates`; non-Unicode fixture is synthetic and does not mutate the OS environment. |
| FR-004 | AC-007, AC-008 | Test + source: `authenticated_request_contains_required_headers`, `test_sign_with_path`, `test_sign_url`, `test_bool_value_with_grayscale_ip`, `test_bool_value_with_grayscale_label`; unsigned rejection depends on the server fixture, not client authorization. |
| FR-005 | AC-009 | Test + source: `cache_identity_and_url_segments_are_isolated_and_safe`; not a guarantee for unvalidated standalone dot segments. |
| FR-006 | AC-003, AC-011, AC-012 | Test + source: `test_get_namespace_type_properties`, `test_get_namespace_type_json`, `test_get_namespace_type_yaml`, `test_get_namespace_type_xml`, `test_get_namespace_type_text`, `test_get_namespace_type_unsupported_extensions`. |
| FR-007 | AC-010 | Test + source: `getters_accept_string_number_and_boolean_scalars`, `test_missing_value`, `test_int_value`, `test_float_value`, `test_bool_value`; complete conversion matrices are not claimed. |
| FR-008 | AC-011, AC-013 | Test + source: JSON/YAML `test_json_to_object`, `test_yaml_to_object`, `test_namespace_to_object`, `preserves_yaml_1_1_boolean_scalars`; malformed-content/type-mismatch branches also source-inspected. |
| FR-009 | AC-012, AC-013 | Partial: `properties_public_and_text_namespaces_use_the_correct_types` covers successful text; missing/non-string content and exact preservation branches source-inspected. |
| FR-010 | AC-005, AC-013, AC-014 | Test + source: `mock_https_status_and_malformed_responses_are_typed_errors`, `rejects_http_errors_without_caching_them`; format/configuration cases above supplement these. |
| FR-011 | AC-015 | Test + source: `request_timeout_has_a_typed_error`, `outer_timeout_also_bounds_custom_http_clients`. |
| FR-012 | AC-016, AC-017 | Partial: `test_custom_http_client_injection` and outer-timeout tests; default certificate rejection and override-precedence warning need focused acceptance assertions. |

## 002 — Retain configuration

| Requirement | Acceptance | Evidence and remaining coverage |
|---|---|---|
| FR-001 | AC-001 | Test + source: cache/read tests and `get_value`/`is_fresh`; isolated fresh-read request-count acceptance is not separately established. |
| FR-002 | AC-002, AC-004 | Test + source: `stale_reads_are_immediate_and_schedule_one_revalidation`, `readers_are_not_blocked_by_refresh_io_and_ttl_applies_to_memory`. |
| FR-003 | AC-003 | Test + source: `stale_persistent_value_falls_back_and_reports_refresh_error`, `rejects_http_errors_without_caching_them`; separate stale outer-JSON failure fixture not established. |
| FR-004 | AC-004, AC-016 | Partial: `zero_ttl_serves_cached_data_and_revalidates` covers replacement; timestamp renewal on identical content is source-inspected. AC-016 is a newly written acceptance scenario, not a new automated test. |
| FR-005 | AC-005 | Test + source: `stale_persistent_value_falls_back_and_reports_refresh_error`, WASM `test_wasm_local_storage_caching`; native end-to-end two-client restart case is not independently established. |
| FR-006 | AC-006, AC-007 | Partial: `persistence_failure_is_nonfatal`; corrupt/unreadable read branches source-inspected. |
| FR-007 | AC-008 | Test + source: `cache_identity_and_url_segments_are_isolated_and_safe`, `test_wasm_cache_key_isolation`; identity-key checks are not a complete cross-client isolation matrix. |
| FR-008 | AC-009, AC-010 | Test + source: `preload_supports_parallel_duplicates_and_propagates_http_errors`; empty-list and partial-success consequences source-inspected. |
| FR-009 | AC-011 | Test + source: manual-refresh tests including `listeners_receive_changes_and_errors_but_not_unchanged_values`; `Client::refresh`. |
| FR-010 | AC-012 | Test + source: `zero_ttl_serves_cached_data_and_revalidates`. |
| FR-011 | AC-013, AC-014 | Test + source: `test_concurrent_get_value`, `concurrent_refresh_callers_share_one_request`, `readers_are_not_blocked_by_refresh_io_and_ttl_applies_to_memory`; cold-failure follower behavior is source-only. |
| FR-012 | AC-015 | Test + source: `cancelled_initial_load_releases_single_flight`. |

## 003 — Observe updates

| Requirement | Acceptance | Evidence and remaining coverage |
|---|---|---|
| FR-001 | AC-002, AC-007 | Test + source: `test_add_listener_and_notify_on_refresh`; append-only registration behavior source-inspected. |
| FR-002 | AC-001, AC-002 | Test + source: `listeners_receive_changes_and_errors_but_not_unchanged_values`; no-replay branch source-inspected. |
| FR-003 | AC-003 | Source: listener snapshot/invocation loop and `invoke_listener`; dedicated ordering/panic-isolation acceptance coverage not established. |
| FR-004 | AC-004 | Test + source: `listeners_receive_changes_and_errors_but_not_unchanged_values`, `read_path_failures_do_not_notify_listeners`, `stale_persistent_value_falls_back_and_reports_refresh_error`. Cold-read/explicit-refresh overlap is unresolved D-011. |
| FR-005 | AC-005 | Source: `get_namespace` and `notify_listeners`; exact per-format error-delivery scenarios need dedicated acceptance tests. |
| FR-006 | AC-006, AC-007 | Test + source: `test_custom_refresh_interval`, `wasm_background_polling_performs_refresh_requests`; newly registered namespace polling source-inspected. |
| FR-007 | AC-007, AC-008 | Partial: `test_custom_refresh_interval`; no invented namespace and delay-after-round semantics source-inspected. |
| FR-008 | AC-009 | Test + source: `test_decoupled_failures_in_refresh_loop`. |
| FR-009 | AC-009, AC-010 | Test + source: `refresh_backoff_uses_bounded_symmetric_jitter`, `test_cache_backoff_and_skipping`; complete failure-growth/saturation/recovery sequence not established. |
| FR-010 | AC-011, AC-012, AC-013 | Test + source: `lifecycle_is_idempotent_prompt_and_cancelled_on_drop`, `test_wasm_lifecycle_starts_stops_and_rejects_duplicate_start`; repeated-stop/retained-registration clauses also source-inspected. |
| FR-011 | AC-012 | Test + source: `lifecycle_is_idempotent_prompt_and_cancelled_on_drop` uses a short completion budget with a long interval. |

## 004 — JavaScript client

| Requirement | Acceptance | Evidence and remaining coverage |
|---|---|---|
| FR-001 | AC-001, AC-004, AC-007 | Partial: generated JS smoke asserts constructor/method names; Rust WASM tests exercise behaviors but do not cover every generated JS argument/return shape. |
| FR-002 | AC-002, AC-007 | Test + source: `namespace_wasm_preserves_properties_class_api`, WASM lifecycle tests; generated declarations were not rebuilt during this regeneration. |
| FR-003 | AC-002 | Partial: `namespace_wasm_preserves_properties_class_api`, `wasm_conversion_returns_a_structured_value`; bigint mapping follows binding type contract, with no fresh generated JS integer-range assertion. |
| FR-004 | AC-003, AC-004 | Source: Client constructor/error conversion and `preload_wasm` input validation; generated JS rejection/index tests not established. |
| FR-005 | AC-005, AC-006 | Partial: `test_add_listener_wasm_and_notify` covers plain data; JS throwing-listener and subsequent-delivery behavior source-inspected. |
| FR-006 | AC-008 | Test + source: `node_environment_lookup_reads_process_env` uses synthetic globals; actual process/browser integration is not a version certification. |
| FR-007 | AC-009 | Test + source: `test_wasm_local_storage_caching`, `test_wasm_cache_key_isolation`; hostile getters/quota/write failures source-inspected. |
| FR-008 | AC-010 | Partial: `test_wasm_allow_insecure_https_warning` constructs successfully; emitted-warning and host TLS enforcement are not independently verified by that test. |
| FR-009 | AC-011 | Partial: `wasm_request_timeout_bounds_hung_fetches` covers a hung fetch; distinct hung-body acceptance coverage in WASM remains to be established. |
| FR-010 | AC-012 | Source/binding ownership contract: by-value `ClientConfig`, generated wrapper conventions, and Client Drop; no fresh generated-package ownership or memory-leak test run. |

## 005 — Test against a real Apollo server

| Requirement | Acceptance | Evidence and verified coverage |
|---|---|---|
| FR-001 | AC-001, AC-014 | Test + source: `tests/apollo_integration.rs` (native + rustls) and `tests/apollo/wasm.cjs` (wasm) connect exclusively to live Apollo ConfigService/AdminService instances provisioned via Docker Compose; synthetic mocks are quarantined to offline unit/fault suites. |
| FR-002 | AC-001, AC-002, AC-003 | Test + source: `scripts/test.sh` and `scripts/apollo-test.sh` manage the full lifecycle (preflight, container spinup, dynamic loopback port assignment, health readiness, fixture seeding, multi-runtime execution, diagnostics capture, and scoped cleanup). |
| FR-003 | AC-004, AC-006 | Test + source: `scripts/apollo-fixtures.mjs` declarative seed/verify pipeline provisions apps, app namespaces, access keys, grayscale child branches/rules, and publishes initial releases; ConfigService verification confirms client visibility before test execution. |
| FR-004 | AC-003, AC-005 | Test + source: Dual-pass seed verification in `scripts/apollo-test.sh` validates 100% idempotency with 0 diff between `state.json` and `state-second.json`; repeated clean runs pass deterministically. |
| FR-005 | AC-007, AC-008, AC-009 | Test + source: Real Apollo multi-format parsing (Properties, JSON, YAML, Text), cluster/app/namespace selection, public namespace inheritance, enforced access key authentication (valid HMAC-SHA1 signature accepted, unsigned/wrong secret rejected with HTTP 401), and grayscale branch targeting (IP, label, combination rules) verified across `native`, `rustls`, and `wasm`. |
| FR-006 | AC-010, AC-011 | Test + source: `real_apollo_release_refresh_and_listener` and `real_apollo_polling` verify that unpublished modifications remain invisible, published releases appear upon explicit `refresh()`, event listeners receive notifications, and background polling picks up new releases within bounded intervals. |
| FR-007 | AC-012 | Test + source: `real_apollo_preload_and_persistence` in Rust and `real_apollo_wasm_preload` in Node/WASM verify parallel batch namespace preloading and file/in-memory persistence against live Apollo endpoints. |
| FR-008 | AC-014 | Test + source: `scripts/apollo-test.sh` orchestrates three separate execution environments: native default TLS (`native-tls`), native rustls (`rustls`), and Node.js WebAssembly (`wasm`). |
| FR-009 | AC-013 | Test + source: `.github/workflows/rust.yml` executes the exact same `scripts/test.sh` lifecycle as local development, preflighting tools, running on public runners without private secrets, and preserving diagnostic artifacts. |
| FR-010 | AC-002, AC-010, AC-015 | Test + source: Every stage has bounded timeouts (pull 600s, health 300s, seed 300s, suite 300s, teardown 30s); missing Docker, broken seeds, assertion failures, or 0 test matches fail with non-zero exit codes. |
| FR-011 | AC-016, AC-017 | Test + source: Full concurrent isolation verified by running simultaneous instances with dynamic ephemeral ports, unique project names (`apollo-test-<timestamp>-<rand>`), independent MySQL volumes, and unique run directories. |
| FR-012 | AC-015, AC-017 | Test + source: On failure or interrupt, `docker compose ps -a` and `docker compose logs` are captured to `<run-dir>/logs/` and symlinked at `target/apollo-test-latest`; scoped recovery cleanup (`scripts/apollo-test.sh cleanup`) targets only owned projects via `ownership.json`. |
| FR-013 | AC-018 | Test + source: `scripts/test.sh fast` runs completely Docker-free, retaining all Clippy checks, doctests, and deterministic fault/network/caching unit tests (`MockHttpsServer` and scoped WASM stubs). |
| FR-014 | AC-001, AC-013 | Test + source: Container images (`mysql:8.4.11`, `apollo-configservice:2.5.2`, `apollo-adminservice:2.5.2`) pinned by multi-arch index digest; SQL schema provenance recorded; upgrade procedures and contributor workflow documented in `tests/apollo/README.md`, `README.md`, `README_zh.md`, and `spec/supporting/design.md`. |

## Execution status

- **Environment**: macOS ARM64 (Apple Silicon), Rust 1.85+ stable, wasm-pack 0.13+, Node.js 24 LTS (`v24.2.0`), Docker 28.0+ with Docker Compose v2.33+.
- **Pinned Image Provenance**:
  - `mysql:8.4.11` (`sha256:b3b90af2a6552ae30c266fdb7d5dd55f3afb72404bb78d37fe8a23eb857fd3fb`)
  - `apolloconfig/apollo-configservice:2.5.2` (`sha256:a5e4bb5755688fdfc77418e3e34c87095ecb9cf36c3618d05c838bec21f008e8`)
  - `apolloconfig/apollo-adminservice:2.5.2` (`sha256:a7884c10d3fdef2a79c03f3d069fc10843837c9dec6e566a9d68a86d3db0dd95`)
  - Schema: `tests/apollo/sql/apolloconfigdb.sql` (SHA-256: `7b725d81410d502c7a6ead3a16b6b4daf3b4434b3fa9c57829e67a87ff47ab26`)
  - Fixtures: `tests/apollo/fixtures.json` (SHA-256: `9c580e003559101f7f3630904edfac393f532bed3d9a215b2a39fca4d17dbdcd`)
- **Verified Outcomes**:
  - `scripts/test.sh fast`: 44 native unit tests, 44 rustls unit tests, 37 doc tests, 10 wasm unit tests, and 3 Clippy targets passed without Docker.
  - `scripts/test.sh integration` (2 consecutive clean cycles): Dynamic ports, MySQL health readiness, idempotent seed (0 diff), and all 19 real-server integration tests (6 native, 6 rustls, 7 wasm) passed cleanly; Compose projects and volumes cleanly torn down.
  - Concurrent isolation: Two simultaneous integration runs (`native` and `rustls`) ran in parallel on distinct dynamic ports without interference.
  - Failure/interrupt safety: Signal trapping (`SIGINT` -> 130), diagnostic preservation (`compose-services.log`, `compose-ps.txt`, `state.json`), and safe recovery cleanup verified.
  - **CI & Platform Status**: Local macOS ARM64 (Apple Silicon) runs verified 100% clean across all modes. GitHub Actions workflow (`.github/workflows/rust.yml`) is fully configured and verified locally; Pull request created at [https://github.com/qqiao/apollo-rust-client/pull/132](https://github.com/qqiao/apollo-rust-client/pull/132); remote CI execution and Linux amd64 hosted runner results are monitored there.

## Planned review remediation traceability (2026-09-13)

**No remediation task is implemented or verified yet.** Historical entries above describe earlier work; in particular, happy-path cleanup and coarse stage limits did not establish visible teardown failures or response-body request deadlines. The review found those gaps. Follow the linked plans for new negative controls and record actual post-fix results later.

| Package | Baseline relationship | New scenario coverage | Planned evidence |
|---|---|---|---|
| [006](../006-cache-restore-ordering/tasks.md) | 002/FR-004, FR-011, planned FR-013/AC-017; 003/FR-002 | 006/AC-001–007 | **Verified**: `restore_memory_if_empty` prevents memory overwrite. Automated coverage in `src/cache.rs`: `persistent_restore_does_not_overwrite_completed_refresh` (AC-001, AC-002), `persistent_restore_emits_one_initial_value` (AC-003), `stale_persistent_restore_returns_before_revalidation` (AC-004), `concurrent_persistent_loads_emit_one_event` (AC-005), `failed_refresh_does_not_discard_pending_restore` (AC-006), `cancelled_initial_load_releases_single_flight`, `zero_ttl_serves_cached_data_and_revalidates` (AC-007). Verified passing via `scripts/test.sh fast`. |
| [007](../007-public-cache-errors/tasks.md) | 001/FR-010, planned FR-013/AC-018 | 007/AC-001–005 | **Verified**: `apollo_rust_client::CacheError` re-exported at crate root while `cache` module remains private. External consumer coverage in `tests/public_errors.rs`: `match_http_status_externally` (AC-001), `match_timeout_externally` (AC-002), `match_coalesced_refresh_externally` (AC-005), `preserve_existing_error_cache_matching_and_std_error` (AC-003). Compiling rustdoc example (AC-004). Verified across native-TLS and Rustls via `scripts/test.sh fast`. |
| [008](../008-wasm-ownership-docs/tasks.md) | 004/FR-010, AC-012, planned AC-013 | 008/AC-001–006 | **Verified**: Executable canonical ownership snippet in `docs/wiki/en/WASM-Memory-Management.md` executed against generated WASM package via `scripts/wasm_api_smoke.js` (AC-001). No-network tests verify: untransferred config cleanup (AC-002), error preservation across client cleanup (AC-003), constructor validation rejection with consumed-config handling (AC-004), Properties wrapper cleanup and ordinary JS value rules (AC-005), and language audit across READMEs and all wiki translations (AC-006). |
| [009](../009-observable-test-cleanup/tasks.md) | 005/FR-010/FR-012, planned FR-015/AC-019 | 009/AC-001–008 | **Verified**: Shared lifecycle helper in `scripts/apollo-test-lifecycle.sh` with process supervision and teardown precedence. 17 automated tests in `tests/tooling/apollo-lifecycle.test.mjs` verify: 0+success->0 (AC-001), 0+42->42 with teardown log and scoped recovery command (AC-002), 7+42->7 preserving stage failure (AC-003), 0+hang->124 bounded timeout (AC-004), SIGINT (130) and SIGTERM (143) precedence (AC-005), at-most-once idempotent teardown (AC-006), unowned run safety (AC-007), and CLI explicit recovery with valid/invalid ownership (AC-008). Integrated into `scripts/test.sh fast` and `scripts/apollo-test.sh`. |
| [010](../010-fixture-body-deadlines/tasks.md) | 005/FR-010, planned FR-016/AC-020 | 010/AC-001–007 | **Verified**: Exported `requestJson` in `scripts/apollo-fixtures.mjs` encloses fetch and response text reading in a unified AbortController and timeout lifecycle. 7 automated tests in `tests/tooling/apollo-http.test.mjs` verify: headers withheld deadline rejection (AC-001), stalled body abortion (AC-002), JSON response shape compatibility (AC-003), empty/malformed/non-2xx response handling (AC-004), credential omission in timeout diagnostic (AC-005), transport failure distinction (AC-006), and timer clearance on settled requests (FR-003). |
| [011](../011-executable-public-docs/tasks.md) | 001 public configuration/error docs; 004 ownership; 005 tooling guidance | 011/AC-001–008 | **Verified**: Mechanical Markdown extraction and compilation in `scripts/check-doc-examples.mjs` against temporary consumer crate under native-tls and rustls (AC-001–004). Negative controls in `tests/tooling/doc-examples.test.mjs` verify duplicate marker rejection, malformed fences, missing inventory items, and compilation failure diagnostics for unavailable symbols. Local Markdown link verification in `scripts/check-doc-links.mjs` (418 links across 77 files, 0 broken) and tests in `tests/tooling/doc-links.test.mjs` (AC-006, AC-008). 8 broken zh-TW design language switches repaired. Discrepancy checklist closed (AC-005, AC-007). Integrated into `scripts/test.sh fast` and `scripts/apollo-test.sh`. |
| [012](../012-accurate-polling-docs/tasks.md) | 003/FR-007/FR-009, AC-008–010 | 012/AC-001–006 | **Verified**: Replaced false healthy ±10% jitter and interval-only latency bounds in `README.md`, `README_zh.md`, `CHANGELOG.md`, `spec/supporting/design.md`, and wiki design/feature documents with accurate completed-round cadence (up to 4 concurrent refreshes, post-round exact sleep) and scoped per-namespace failure backoff jitter (AC-001–003). Documented that manual refresh and stale revalidation bypass backoff, and TTL controls read freshness separately from polling (AC-004–005). Audit verified across English, Simplified Chinese, and Traditional Chinese with no executable runtime behavior modified (AC-006). Verified with doc checkers and fast suite. |

Acceptance/success criteria are qualified by package ID; numeric IDs repeat intentionally across features. Completion evidence belongs in each new tasks.md and this table must not be relabeled passed merely because the baseline suite remains green.
