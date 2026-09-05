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

## Execution status

The previous documentation attempt invoked `scripts/test.sh` at this same source revision. It exited 101 in the first Clippy dependency-download stage because `mirrors.aliyun.com` did not resolve while fetching `log` 0.4.34. Subsequent stages did not run. That is an environment/dependency-fetch failure, not a test assertion failure or a passing runtime baseline.

This regeneration changes documentation only. It uses source inspection, requirement/scenario mapping, structural/link checks, and an independent specification review; the blocked runtime suite has not been rerun or claimed as passing. These documents expose missing acceptance coverage rather than pretending it was supplied by the documentation change.
