# Tasks: Real Apollo integration testing

**Feature**: [005-real-apollo-testing](spec.md)
**Technical contract**: [plan.md](plan.md)
**Status**: Unstarted implementation backlog. This session is planning only; stop after delivering these documents.

All `AC-*`, `FR-*`, and `SC-*` below refer to feature **005-real-apollo-testing** unless another feature is explicitly named. Each task should be a focused agent session. Load this feature's spec/plan and only the source files needed for the assigned task. Read `AGENTS.md`, `.agent/**/*.md`, and relevant existing feature contracts first.

Do not treat this checklist as permission to code in the planning session. When the maintainer delegates implementation, preserve their accepted decisions and resolve only material newly discovered conflicts. Update affected specifications before changing client behavior; no public API or production implementation change is currently planned. Keep verification evidence with the task/plan, and never check a box based only on source inspection when runtime proof is required.

## Task Index and Dependencies

| Task | Deliverable | Depends on | Scope |
|---|---|---|---|
| T01 | Pinned stack and first real release proof | None | M, 3 files |
| T02 | Declarative basic fixtures and idempotent initializer | T01 | M, 3 files |
| T03 | Enforced auth and real grayscale fixtures | T02 | M, 3 files |
| T04 | Owned lifecycle and test-command dispatch | T01–T03 | M, 4 files |
| T05 | Native format/identity/auth/grayscale compatibility | T04 | M, 3 files |
| T06 | Real native publication/refresh/polling/preload | T05 | M, 3 files |
| T07 | Generated Node/WASM real-network suite | T03, T04, T06 | M, 3 files |
| T08 | Native mock migration without losing fault coverage | T05, T06 | M, 4 files |
| T09 | WASM mock boundary and fast-suite completion | T07, T08 | M, 3 files |
| T10 | GitHub Actions lifecycle and diagnostics | T04–T09 | M, 2 files |
| T11a | Contributor/provenance documentation | T10 | M, 4 files |
| T11b | Traceability and final runtime evidence | T11a | M, 3 files |

No task assumes another agent's uncommitted changes are disposable. T08 and T09 both touch `src/lib.rs` and must run sequentially. Scripts/fixture helpers have one owner at a time. Future native/Node authors can work independently only after agreeing on run state and helper commands; use separate Docker projects for execution.

## T01 — Prove the pinned real-server stack

- [x] **T01 complete**

**Purpose**: Resolve deployment uncertainty before rewriting existing tests. Implement the three-service Compose definition, pin the schema/images, and demonstrate one real publish/read cycle using source-verified AdminService calls.

**Requirements/scenarios**: FR-001, FR-003, FR-014; AC-001, AC-004, AC-006; contributes to SC-001/SC-006.

**Dependencies**: None.

**Files (3)**: `tests/apollo/compose.yaml`, `tests/apollo/sql/apolloconfigdb.sql`, `tests/apollo/README.md`.

**Work**: Inspect candidate Apollo 2.5.2 and MySQL 8.4.11 manifest indexes on the registry; choose fixed compatible digests for amd64/arm64. Vendor the exact release schema with license and hash. Configure shared JDBC settings, database-discovery profiles, authenticated schema readiness, dynamic loopback ports, and project-scoped storage. Run a disposable proof: create app, find default namespace ID, create a string item, publish, read the ConfigService JSON endpoint. Record exact payloads/returned fields and startup resource/time observations. No Portal or SQL release insertion.

**Acceptance**:

- [x] All image references are immutable with verified architectures; schema/source/hash and effective profiles are recorded; no latest tags or placeholder digests remain.
- [x] A clean volume initializes and an AdminService-created **published** value is returned by ConfigService on the host's discovered port; saved-only data is not falsely counted as published.
- [x] Services use only owned resources and loopback host ports; proof cleanup removes that project and volume only.

**Verification**: `docker compose -f tests/apollo/compose.yaml config --quiet`; `docker buildx imagetools inspect <each selected image>`; explicit uniquely named Compose up/probe/down proof with request/response evidence. This is infrastructure qualification, not a substitute invocation of Rust tests. Run the proof on available ARM64 and Linux amd64 environments, or explicitly record the missing architecture for T11b. If the chosen DB/image fails, update the pinned candidate and repeat before proceeding.

## T02 — Initialize and verify the basic published fixtures

- [x] **T02 complete**

**Purpose**: Convert the first publish proof into a repeatable data initializer for formats, application/cluster identity, public namespace association, and reserved update namespaces.

**Requirements/scenarios**: FR-003–005, FR-014; AC-004–007, AC-012.

**Dependencies**: T01.

**Files (3)**: `scripts/apollo-fixtures.mjs`, `tests/apollo/fixtures.json`, `tests/apollo/README.md`.

**Work**: Implement the manifest from plan §3 and Node built-in-only HTTP/JSON/error handling. Validate current-run loopback URLs, audit fields, namespace IDs, item IDs, content types, and returned names. Reconcile before creating/updating/publishing. Add `seed`, `verify`, `set-item`, `publish`, and `set-and-publish` operations with mutable-namespace restrictions; mutation commands may be completed as needed in T06. Include per-run marker and resolved IDs in state. Verify through independent ConfigService requests, not only AdminService item reads.

**Acceptance**:

- [x] Every basic manifest value, cluster identity, format, and actual public association is published and correctly probed; JSON/YAML content is a string and Text preserves final newlines.
- [x] Two pre-mutation seed passes leave entity counts and active release IDs stable, with no broad “ignore conflict” error handling.
- [x] Corrupt/missing required fixture values, an incorrect endpoint, or failed publishing produce an identified nonzero error within a finite deadline.

**Verification**: Run `node --check scripts/apollo-fixtures.mjs`; seed/verify twice against a uniquely owned T01 stack using the explicit run environment. Compare entity counts and latest releases, inspect all ConfigService values, then deliberately change one required expected value in a temporary manifest copy and verify failure. Preserve/delete only test-owned resources.

## T03 — Prove authentication enforcement and grayscale routing

- [x] **T03 complete**

**Purpose**: Make it impossible for server-compatibility tests to pass because the server ignores authentication or simply returns the same response to every audience.

**Requirements/scenarios**: FR-003–005, FR-010; AC-004–006, AC-008–009.

**Dependencies**: T02.

**Files (3)**: `scripts/apollo-fixtures.mjs`, `tests/apollo/fixtures.json`, `tests/apollo/README.md`.

**Work**: Create/reconcile a protected app key in enabled FILTER mode. Independently sign ConfigService probes with Node crypto. Create real branches under plain and protected apps; capture generated names/IDs, edit/publish branch content, install/read back IP/label rules with the correct release IDs, and wait for client-visible convergence. Add correct-secret, no-secret, wrong-secret, signed-targeting, IP/label matches, nonmatches, and no-target controls to fixture verification.

**Acceptance**:

- [x] Correct signed requests succeed; missing/wrong-secret requests fail with real 401 responses after enforcement converges; repeated seed does not accumulate keys.
- [x] Matching IP or label receives gray=true; nonmatches and no targeting receive gray=false; protected-app targeted requests also succeed with correct signatures.
- [x] Rule/key mismatches fail initialization with useful diagnostics; wildcard substitution, observer-only keys, and SQL release/rule insertion are absent.

**Verification**: Execute initializer/verification twice on a new project and compare branch/rule/key identities. Temporarily remove a required rule or disable the protected key via that disposable instance and demonstrate that verification detects the missing behavior. Restore by starting a fresh run. Record HTTP evidence and propagation duration.

### Checkpoint A — The server is a trustworthy fixture

- [x] T01–T03 evidence establishes real published values, enforced authentication, and server-selected gray responses.
- [x] Image/schema provenance and the admin API payload corrections are recorded in the plan/fixture README.
- [x] Deployment failures or unsupported candidates are resolved before any compatibility test is removed.

## T04 — Implement the owned lifecycle and test entry modes

- [x] **T04 complete**

**Purpose**: Provide one portable orchestration path that local and CI runs can use and that cannot silently skip integration setup.

**Requirements/scenarios**: FR-002–004, FR-010–014; AC-001–003, AC-015–018.

**Dependencies**: T01–T03.

**Files (4)**: `scripts/apollo-test.sh`, `scripts/test.sh`, `scripts/apollo-fixtures.mjs`, `tests/apollo/README.md`.

**Work**: Implement all/fast/integration mode validation, selected-suite/filter dispatch contract, mandatory tool preflight, unique run/project state, immutable-image pulls, dynamic port discovery, seed/verify phases, finite deadlines, child supervision, diagnostic capture, signal handling, and scoped cleanup/recovery. Keep the integration command explicitly unsuccessful until its required suite executors exist; missing tests are not a successful stub. Retain all existing fast checks and use Clippy rather than cargo check. Fast mode must not require or query Docker. Generated WASM paths belong to this run.

**Acceptance**:

- [x] No-argument full mode and explicit subset modes have clear behavior; missing prerequisites/setup/suite executors and unmatched filters fail visibly, never fall back or skip to green.
- [x] Consecutive and concurrent provisioning runs own different ports, volumes, state/cache/package paths; cleanup validates ownership and preserves unrelated resources.
- [x] Success/failure/INT/TERM/deadline paths terminate supervised children, capture applicable logs first, attempt bounded cleanup once, and preserve failed status.

**Verification**: `bash -n scripts/apollo-test.sh`; `sh -n scripts/test.sh`; `node --check scripts/apollo-fixtures.mjs`; `scripts/test.sh fast` with Docker deliberately unavailable through a scoped test PATH/environment. Exercise provisioning with missing Docker, partial startup failure, failed seed, and catchable interrupt. Use an unrelated sentinel project to prove cleanup scope. At this stage explicitly record absent integration executors as expected failures, not completed AC-001 evidence.

## T05 — Add real native compatibility cases

- [x] **T05 complete**

**Purpose**: Test the shipped native public API against Apollo using both native TLS configurations.

**Requirements/scenarios**: FR-001, FR-005, FR-008, FR-010; AC-007–009, AC-014.

**Dependencies**: T04.

**Files (3)**: `tests/apollo_integration.rs`, `tests/apollo/support/mod.rs`, `scripts/apollo-test.sh`.

**Work**: Add the planned real formats/identity, access-key, and grayscale cases. Use public Client/ClientConfig APIs; create separate temporary cache guards per auth variant. Gate this test target off WASM compilation, mark real cases ignored by ordinary fast test invocation, and make the orchestrator select them explicitly. Assert actual values, selected identity, types, real unauthorized error status, and gray controls. Check selected test names/count before execution. Do not import the library's private mock fixtures.

**Acceptance**:

- [x] Both native runtime feature configurations pass all format/identity/auth/grayscale cases, with no cached-success shortcut in negative cases.
- [x] Required missing environment or zero selected tests fails; ordinary fast mode still runs without a server.
- [x] Integration tests compile/lint for applicable targets without new production API exposure or unreviewed dependencies.

**Verification**: For each of `native` and `rustls`, run `scripts/test.sh integration --suite <suite> --filter <case>` for `real_apollo_formats_and_identity`, `real_apollo_access_key`, and `real_apollo_grayscale`. These are explicitly partial development runs. An unmatched filter must fail; an unfiltered native run must report the missing T06 cases until T06 is implemented. Run the three existing Clippy commands via `scripts/test.sh fast`.

## T06 — Add actual release, refresh, polling, and preload cases

- [ ] **T06 complete**

**Purpose**: Prove that edits, releases, and client update behavior interact correctly with Apollo rather than a changing response closure.

**Requirements/scenarios**: FR-006–007, FR-010–011; AC-010–012, AC-016.

**Dependencies**: T05.

**Files (3)**: `tests/apollo_integration.rs`, `tests/apollo/support/mod.rs`, `scripts/apollo-fixtures.mjs`.

**Work**: Complete narrow mutation-helper commands; invoke them through bounded blocking-task supervision from Rust. Implement the plan §5 publication sequence, distinguish first-load events from changed events, and test unchanged refresh suppression. Test periodic observation without manually refreshing the observed client. Add duplicate-preload/known-value assertions and a native same-identity completed-persistence smoke. Use separate `updates-native`/`updates-rustls` namespaces and serial tests. Do not turn request counters, stale-read timing, or outage regressions into nondeterministic live-server checks.

**Acceptance**:

- [x] Saved-but-unpublished edits remain invisible; a real release becomes visible through explicit refresh with the expected callback; identical refresh adds no changed-content callback.
- [x] A further real release reaches the polling client without a manual refresh and every test stops/drops clients on success and failure.
- [x] Preload/persistence smoke assertions pass with test-local storage, isolated runtime mutation namespaces, and bounded state observation.

**Verification**: `scripts/test.sh integration --suite native` and `scripts/test.sh integration --suite rustls`, followed by a second clean native run. Inspect release IDs and value transitions in diagnostics. Ensure test names/listing includes all six native groups and a nonexistent release/value fails at its deadline rather than retrying the whole test.

### Checkpoint B — Native local lifecycle works

- [x] A full native lifecycle succeeds from a fresh volume for both TLS configurations.
- [x] Authentication and publication have negative controls; real polling observes changes.
- [x] Setup/test failure evidence and cleanup have been exercised; retained fast regressions remain intact.

## T07 — Exercise generated WASM through real Node networking

- [x] **T07 complete**

**Purpose**: Test the consumer-facing generated JavaScript package in a process unaffected by WASM unit-test fetch replacements.

**Requirements/scenarios**: FR-005–008, FR-010–011; AC-007–014, AC-016.

**Dependencies**: T03, T04, T06; use finalized T06 publication-helper commands.

**Files (3)**: `tests/apollo/wasm.cjs`, `scripts/apollo-test.sh`, `scripts/apollo-fixtures.mjs`.

**Work**: Build Node bindings into the run-owned absolute package directory; inspect generated API types; run Node built-in tests with concurrency one and real built-in fetch. Implement format/identity, auth negative/positive, gray controls, publication/listener/polling, and preload scenarios with `updates-wasm`. Use bigint integer assertions and correct Properties/direct-listener representation. Manage wrapper ownership in finally blocks. Enforce nonzero test selection and finite process timeout.

**Acceptance**:

- [x] Node integration makes actual ConfigService requests for the mandatory cases; no global fetch replacement or imported unit setup exists on this path.
- [x] Generated API values/callbacks/timing methods behave as specified, with wrappers released and polling stopped on failure as well as success.
- [x] Running two WASM integrations concurrently does not overwrite generated packages or share fixture/cache state.

**Verification**: `node --check tests/apollo/wasm.cjs`; `scripts/test.sh integration --suite wasm`; then `scripts/test.sh integration` with all three suites. Run two integration commands concurrently in independent run directories and inspect both summaries/service logs. Code-review fetch/global setup separation and substantiate real networking with run-ID values and server evidence.

## T08 — Migrate native compatibility mocks and retain precise regressions

- [x] **T08 complete**

**Purpose**: Replace existing native claims of Apollo compatibility with passing real-server cases while preserving fault, parser, cache, and lifecycle coverage.

**Requirements/scenarios**: FR-001, FR-013; AC-007–012, AC-018; migration map in plan §5.

**Dependencies**: T05, T06.

**Files (4)**: `src/lib.rs`, `src/test_support.rs`, `src/namespace/json.rs`, `src/namespace/yaml.rs`.

**Work**: Inventory native tests before editing. Move/remove duplicated format/scalar/secret/gray compatibility tests only after linking each to its real replacement. Remove network helper dependencies from namespace conversion unit tests. Preserve `MockHttpsServer` and explicit response fixtures for HTTP errors, TLS, stalls, request counting, backoff, and concurrency; replace reliance on the generic Apollo response router with focused local responses. Preserve callback reentrancy/hang and lifecycle regressions. Restrict production code changes to none.

**Acceptance**:

- [x] Every removed native compatibility assertion maps to a passing real replacement; pure format/conversion and cache/fault tests retain their intended coverage.
- [x] Native test helpers no longer present a fake Apollo implementation as compatibility evidence; dedicated fault cases still exercise their failures deterministically.
- [x] Fast checks and both native real suites pass; no public API or cache/poller behavior changes were smuggled into test migration.

**Verification**: `scripts/test.sh fast`; `scripts/test.sh integration --suite native`; `scripts/test.sh integration --suite rustls`; review the diff against the migration table and compare before/after test inventory. A missing mapped scenario is unfinished work, not a reason to delete the row.

## T09 — Finish the WASM mock boundary and fast suite

- [x] **T09 complete**

**Purpose**: Preserve targeted WASM regression tests without allowing them to substitute for the real generated-package suite.

**Requirements/scenarios**: FR-008, FR-013; AC-014, AC-018.

**Dependencies**: T07, T08; exclusive ownership of `src/lib.rs`.

**Files (3)**: `src/lib.rs`, `scripts/test.sh`, `tests/apollo/wasm.cjs`.

**Work**: Inventory WASM tests. Migrate duplicated ordinary format/secret/gray claims to the real Node cases. Name/scope retained fetch setup for fault, listener, host storage/global, and timeout tests, without leaving misleading shared “real server” setup comments. Keep fault mocks within the unit-test process. Ensure fast mode retains native default/Rustls unit/fault runs, doctests, all Clippy modes, and WASM library tests; mark the real integration target explicitly excluded in output. Preserve generated export smoke behavior.

**Acceptance**:

- [x] Removed WASM compatibility cases have mapped real replacements; error/global/storage/lifecycle regressions remain and cannot leak fake fetch into Node integration.
- [x] Fast mode succeeds without Docker and labels its scope; full mode requires all three real runtime suites after the fast checks.
- [x] Runtime feature selection is mutually valid (Rustls native only), and zero/unselected integration tests cannot yield full-suite success.

**Verification**: `scripts/test.sh fast` with Docker unavailable; `scripts/test.sh integration --suite wasm`; `scripts/test.sh` on a working runtime. Inspect the selected-test summary and generated API export smoke evidence. Use meaningful behavior assertions, not tests that merely grep implementation strings.

### Checkpoint C — Test boundaries are complete

- [x] All default-TLS/Rustls/Node compatibility cases pass against the real server.
- [x] Every migrated assertion has a destination; fault tests remain deterministic and Docker-independent.
- [x] Full, focused, and fast commands report their actual coverage without silent skipping.

## T10 — Integrate the same lifecycle with GitHub Actions

- [x] **T10 complete**

**Purpose**: Make ordinary CI use the local test lifecycle and retain useful diagnostics for setup and assertion failures.

**Requirements/scenarios**: FR-009–012, FR-014; AC-013–017; SC-003–004.

**Dependencies**: T04–T09.

**Files (2)**: `.github/workflows/rust.yml`, `scripts/apollo-test.sh`.

**Work**: Retain existing jobs and event scope. Add explicit Node 24 where Node is used, tool/runtime preflight, test job timeout, and a known diagnostic location. Keep `scripts/test.sh` as the test job command. Upload diagnostics with always-run handling and add ownership-aware fallback teardown. Use public images and ordinary PR permissions; no secrets, private endpoints, shared DB/cache volume, CI-only seed scripts, or continue-on-error. Confirm current official action versions during implementation.

**Acceptance**:

- [x] PR/main test runs execute identical local orchestration and all three real runtimes; existing quality/build/export checks remain.
- [x] Setup/assertion failures preserve logs/metadata as downloadable artifacts and leave the job failed; fallback cleanup is safe when setup never started or already cleaned up.
- [x] The workflow requires no credentials unavailable to fork PRs and never uses privileged PR triggers to bypass that constraint.

**Verification**: Validate workflow syntax with available repository-compatible tooling; execute on an authorized PR branch and inspect job logs/artifacts. Exercise one intentional controlled failure in a disposable verification branch if authorized, or record that CI failure-path verification remains pending. Record actual PR/main run links as they become available; do not merge/push/create external activity solely to manufacture evidence without session authorization.

## T11a — Document the contributor workflow and provenance

- [x] **T11a complete**

**Purpose**: Give contributors one reproducible guide and remove obsolete claims about test prerequisites once the implementation exists.

**Requirements/scenarios**: FR-002, FR-012–014; AC-001–003, AC-015–018; SC-006.

**Dependencies**: T10.

**Files (4)**: `tests/apollo/README.md`, `README.md`, `README_zh.md`, `spec/supporting/design.md`.

**Work**: Document installed tools, Docker resource expectations, Node version, full/fast/focused commands, initial downloads, fixture editing/publication, image/schema upgrade procedure, dynamic ports, log locations, signal limitations, and exact scoped cleanup. Link both READMEs to the guide. Update the current technical design only after implementation so its “no Docker required” and mock test descriptions accurately distinguish modes. Do not rewrite unrelated multilingual wiki pages.

**Acceptance**:

- [x] A contributor can run the full suite from the documented prerequisites and identify the explicit Docker-free alternative without manual Apollo setup.
- [x] Pinned images/schema/fixture version, provenance, and deliberate upgrade verification steps are recorded together.
- [x] Documentation distinguishes tested platforms from proposed support, and real-server evidence from remaining fault mocks or untested browser behavior.

**Verification**: Follow the guide from a fresh checkout/test volume using `scripts/test.sh`; check local Markdown links and `git diff --check`; review script help against every documented command. Treat missing tool/network/platform access as an explicit verification gap, not a passing run.

## T11b — Close traceability and verify the complete handoff

- [ ] **T11b complete**

**Purpose**: Confirm the feature against its acceptance scenarios and leave evidence another agent can audit.

**Requirements/scenarios**: FR-001–014; AC-001–018; SC-001–006.

**Dependencies**: T11a.

**Files (3)**: `spec/supporting/traceability.md`, `spec/005-real-apollo-testing/plan.md`, `spec/005-real-apollo-testing/tasks.md`.

**Work**: Update test mappings/evidence after migration, preserving stable existing feature IDs and unresolved product decisions. Record full commands/revisions, image IDs, fixture hashes, platforms, measured timings/resources, negative/control results, and CI links. Run two clean full local cycles, one concurrent-isolation exercise, fast mode without Docker, and deliberate missing-Docker/seed/assertion/interrupt checks. Use Linux amd64 and macOS ARM64 evidence; record any missing environment honestly.

**Acceptance**:

- [ ] Every mandatory scenario is linked to tests or an explicit environment-level verification result; no missing platform/CI run is labeled passed.
- [ ] Two clean runs, concurrent isolation, failure diagnostics, bounded waits, and ownership-scoped cleanup are demonstrated alongside all real runtime suites and retained fast checks.
- [ ] Final diff contains only scoped test infrastructure/test migration/CI/docs changes; discovered production defects are separately specified instead of weakening this feature's requirements.

**Verification**: `scripts/test.sh` twice with fresh run state; `scripts/test.sh fast` without Docker; simultaneous `scripts/test.sh integration` invocations; targeted failure/interrupt exercises; review PR/main CI logs/artifacts and documented architecture evidence. Run `git diff --check` and local-document link checks. Review only relevant checks again if new changes/failures justify it.

### Checkpoint D — Implementation ready for maintainer review

- [ ] Required local and CI acceptance evidence is linked, with any environmental gap explicitly reported.
- [ ] All scoped checks pass; no compatibility test is silently skipped or served by an Apollo emulator.
- [ ] Documentation and migration traceability match the final behavior.
- [ ] No production server, external configuration, unrelated Docker resources, public client API, or unresolved product policy was changed.

## Planning-Session Evidence

The initial planning session read repository rules, current feature specifications, source/test/workflow files and revision history; checked upstream Apollo 2.5.2 controllers/schema/profiles, official Docker/Node/GitHub documentation, and local installed tool versions. It created this checklist, the specification, and the technical plan. All implementation tasks remain unchecked. No real-server runtime or CI result is asserted here.
