# Feature Specification: Test against a real Apollo server

**Feature ID**: `005-real-apollo-testing`
**Created**: 2026-09-06
**Status**: Implemented and verified — ready for maintainer review
**Input**: Restore real-server integration testing, preferably using Docker, initialize relevant data automatically, and support local development and GitHub Actions. Produce separate specification, plan, and tasks, then stop before coding.

## Purpose and Scope

Maintainers need evidence that this client interoperates with an actual Apollo configuration center. A test that synthesizes Apollo responses cannot establish that the server accepts the client's authentication or applies its targeting correctly. Contributors also need reproducible fixture setup without configuring a shared server or manually publishing data.

This feature provides a repeatable integration-test environment, automatic fixture preparation, real-server compatibility assertions, and a common local/CI workflow. It tests the existing client capabilities in features 001–004; it does not change their public APIs or settle their unrelated open decisions.

In scope: disposable real Apollo instances, published fixtures, native Rust and Node/WASM integration coverage, failure diagnostics, isolation, cleanup, and explicit separation from focused unit/fault tests. Technical choices, fixture payloads, file boundaries, commands, and source references are in [plan.md](plan.md); the implementation checklist is [tasks.md](tasks.md).

Out of scope: implementation during this planning session; a production Apollo deployment; Portal UI automation; arbitrary external-server administration; Kubernetes; an Apollo-version compatibility matrix; browser/CORS certification; replacing every fault mock; changing the client's polling protocol; resolving the existing product decision register.

## Intent and Proposed Defaults

The user explicitly requires real Apollo, automatic test-data initialization, local/GitHub Actions support, and a documentation-only handoff. The following are proposed defaults, not recorded user answers:

- Keep focused mocks for malformed responses, transport/TLS failures, deterministic timing, concurrency, and host-capability edge cases. Move claims about Apollo compatibility to real-server tests.
- Make the usual test entry point run the full suite; offer explicit fast and integration-only modes.
- Exercise both native TLS feature configurations and generated Node/WASM bindings. Use Linux CI and support local Linux/macOS, including Apple Silicon.
- Use one pinned Apollo version and disposable local instances without production credentials.

Clarification prompts about the mock boundary and default command were offered during planning. No answers are recorded at the time of this draft. These defaults remain easy to revise before coding; the draft does not claim specification or implementation approval.

## User Scenarios & Testing

### User Story 1 — Run meaningful compatibility tests locally (Priority: P1)

As a contributor, I want one command to prepare Apollo and test the client so that I can reproduce integration failures without maintaining a development server.

**Independent Test**: With documented tools installed and no existing test deployment, run the full test entry point and inspect its result and resource cleanup.

**Acceptance Scenarios**:

1. **AC-001** — **Given** a clean checkout, an available container runtime, and documented prerequisites, **When** the full test command runs, **Then** it starts real Apollo, initializes its data, executes the integration suite, and returns success only when every required stage succeeds.
2. **AC-002** — **Given** a missing/unavailable runtime, failed image retrieval, failed database initialization, or an unready Apollo service, **When** integration testing is requested, **Then** it fails within the documented deadline with a stage-specific diagnostic; it neither falls back to mocks nor reports success after skipping integration tests.
3. **AC-003** — **Given** the same checkout and fixture version, **When** two clean runs execute consecutively, **Then** both start from the specified initial data and produce the same functional assertions without manual setup or data repair.

### User Story 2 — Start with the data the tests actually need (Priority: P1)

As a test author, I want declarative, automatically published fixtures so that a passing test means Apollo served the intended release.

**Independent Test**: Initialize a fresh instance, query its client-facing API, and compare its responses with the fixture manifest. Repeat initialization before any test mutations.

**Acceptance Scenarios**:

1. **AC-004** — **Given** empty test storage, **When** initialization completes, **Then** applications, clusters, namespaces, values, access-key enforcement, and grayscale releases needed by AC-007–011 exist and have been verified through the client-facing service.
2. **AC-005** — **Given** an initialized instance that has not entered the test-mutation phase, **When** initialization is repeated, **Then** it succeeds without duplicate entities, accumulating access keys, or unnecessary new releases, and verifies the same baseline values.
3. **AC-006** — **Given** fixture creation or publishing fails, or a declared value/rule does not become visible, **When** initialization checks readiness, **Then** it fails with the affected identity and expected/observed result before running compatibility assertions. A healthy process or successful item edit alone is insufficient readiness.

### User Story 3 — Verify Apollo's selection and access behavior (Priority: P1)

As a maintainer, I want the real server to authorize requests and select configuration so that signatures and rollout targeting cannot pass merely because a mock ignored them.

**Independent Test**: Read initialized fixtures through the public client API with clean client caches and compare positive and negative cases.

**Acceptance Scenarios**:

1. **AC-007** — **Given** published Properties, JSON, YAML, and Text fixtures, two distinct application identities, a nondefault cluster, and an associated public Properties namespace, **When** the client reads each, **Then** its values, types, namespace identity, and cluster/application selection match the manifest. Missing Properties keys produce absence; a nonexistent private namespace produces the real server's retrieval error.
2. **AC-008** — **Given** an application with enforced access-key authentication and independently empty caches, **When** clients use the correct, missing, and incorrect secret, **Then** the correct secret succeeds and the other two reads fail as unauthorized. Signed reads with targeting parameters also succeed when authorized.
3. **AC-009** — **Given** an actual published grayscale branch with specific IP and label rules, **When** clients read using a matching IP, matching label, nonmatching IP, nonmatching label, or neither, **Then** the matching clients receive the gray value and the others receive the baseline value. The server performs this selection.

### User Story 4 — Observe real releases through the client (Priority: P1)

As a maintainer, I want to publish a changed value during a test and observe it through client refresh/listeners so that retrieval tests also exercise Apollo's release lifecycle.

**Independent Test**: Use a namespace reserved for one test, load its baseline, edit an item, publish it, and observe the value through explicit refresh and then periodic refresh.

**Acceptance Scenarios**:

1. **AC-010** — **Given** a loaded baseline and a registered listener, **When** a different value is saved but not released, **Then** a fresh server read and explicit refresh still yield the baseline; **When** it is published and becomes visible, **Then** explicit refresh returns the changed value and the listener observes that value. Refreshing identical content produces no additional change notification.
2. **AC-011** — **Given** a loaded namespace with polling enabled, **When** another changed value is published, **Then** the changed value is observed without a manual client refresh, within a bounded test wait, and the test stops/releases the client afterward.
3. **AC-012** — **Given** an initialized namespace and a new client, **When** the client preloads duplicate namespace names and later reads them, **Then** the expected values remain usable. A later native client using the same completed test-local persistent cache can read the retained value. Exact request counts and outage scheduling remain covered by focused tests.

### User Story 5 — Run the same checks in GitHub Actions (Priority: P1)

As a maintainer, I want integration checks on pull requests and main-branch pushes so that CI validates the same behavior contributors can reproduce locally.

**Independent Test**: Execute the repository workflow on a hosted Linux runner using only public artifacts and disposable fixture credentials.

**Acceptance Scenarios**:

1. **AC-013** — **Given** a pull request, including one from a fork, or a push to main, **When** the test job runs, **Then** it invokes the same orchestration and fixtures as local testing without a shared Apollo endpoint, repository secrets, or a manual setup step.
2. **AC-014** — **Given** the required runtime matrix, **When** tests execute, **Then** native default-TLS, native Rustls, and generated Node/WASM clients make real requests to Apollo. Unit-test `fetch` replacements cannot affect the Node integration process.
3. **AC-015** — **Given** an initialization or test failure, **When** the job finishes its diagnostic handling, **Then** service logs, stage/result information, fixture identity, and resolved image metadata are available as CI artifacts; the job remains failed.

### User Story 6 — Keep runs isolated and convenient (Priority: P2)

As a contributor, I want bounded cleanup and an explicit fast mode so that integration testing does not interfere with other work and routine unit checks remain usable without Docker.

**Independent Test**: Run two instances concurrently, interrupt one, and inspect the other instance and unrelated local resources. Separately run fast mode with Docker unavailable.

**Acceptance Scenarios**:

1. **AC-016** — **Given** two concurrent runs, **When** both initialize and one mutates or cleans up its fixtures, **Then** their service ports, database state, generated test artifacts, and client caches do not conflict, and the other run remains usable.
2. **AC-017** — **Given** success, assertion failure, partial startup failure, or a catchable interrupt/termination signal, **When** orchestration exits, **Then** it captures applicable diagnostics and attempts bounded cleanup of only its own containers, network, volume, and transient client data. Cleanup errors remain visible and cannot turn a failed test into success.
3. **AC-018** — **Given** Docker is unavailable, **When** explicit fast mode runs, **Then** existing unit/fault, lint, and documentation checks remain available without contacting Apollo; output clearly identifies that real-server integration checks were not requested.

### Edge Cases

- Access-key and namespace metadata may propagate after administrative writes. Readiness observes their actual effects using bounded polling.
- Authentication tests must not reuse previously cached successful data: cache identity currently omits credentials (`supporting/research.md`, D-007).
- A branch name is generated by Apollo and must not be assumed to equal its parent cluster. Grayscale negative controls must prevent an accidental wildcard rule from passing.
- MySQL initialization scripts run against fresh storage; restarting containers with an existing volume is not equivalent to a clean test run.
- An OS kill that cannot be trapped, daemon loss, or runner destruction cannot guarantee teardown. Run ownership and a documented scoped cleanup command provide recovery without promising impossible signal handling.
- A valid server may reject pathological identifiers or never generate malformed wire responses. Those cases retain focused protocol/unit coverage; they do not justify weakening real compatibility assertions.

## Requirements

### Functional Requirements

- **FR-001**: The integration suite MUST connect to a genuine Apollo server; synthetic HTTP responses MUST NOT stand in for Apollo compatibility evidence.
- **FR-002**: A documented command MUST provision and operate the full test lifecycle from a clean checkout with documented prerequisites.
- **FR-003**: Fixture initialization MUST create and publish all required test data automatically and validate client-visible results before integration assertions start.
- **FR-004**: Repeated clean runs MUST be deterministic, and repeated initialization before mutation MUST be idempotent as specified in AC-005.
- **FR-005**: Real-server coverage MUST include formats, application/cluster selection, public namespace association, enforced access-key positive/negative cases, and grayscale positive/negative cases in AC-007–009.
- **FR-006**: Real-server coverage MUST include unpublished-versus-published behavior, explicit refresh, listener delivery, and periodic refresh in AC-010–011.
- **FR-007**: Real-server coverage MUST include the preload and native persistence smoke outcomes in AC-012 without claiming complete cache-state-machine verification.
- **FR-008**: Default native TLS, Rustls, and generated Node/WASM integration runs MUST execute against Apollo in isolated runtime processes.
- **FR-009**: Local testing and GitHub Actions MUST use the same lifecycle, data, and assertions; ordinary PRs and main pushes MUST execute integration checks without private credentials.
- **FR-010**: Initialization, readiness, runtime waits, and cleanup MUST have documented finite bounds and meaningful failure results; no silent skipping, mock fallback, or continue-on-error is permitted for required stages.
- **FR-011**: Concurrent runs MUST isolate ports, containers, storage, test mutations, and generated artifacts; automated deletion MUST be scoped to the current run.
- **FR-012**: Failures MUST preserve enough stage and service evidence for local/CI diagnosis before teardown; original failures MUST remain failures.
- **FR-013**: Explicit fast mode MUST remain Docker-independent and preserve focused regression coverage; migrated compatibility tests MUST have recorded real-server replacements before their mock versions are removed.
- **FR-014**: Tool/image/schema/fixture versions and contributor instructions MUST be recorded together so future agents can reproduce and deliberately update the environment.

### Key Entities

- **Test run**: One owned lifecycle with unique identity, endpoints, transient state, and final stage outcomes.
- **Fixture manifest**: Versioned intended applications, namespaces, values, authorization settings, targeting rules, and mutation reservations.
- **Published fixture**: Configuration released by Apollo and independently observed at its client-facing boundary.
- **Compatibility assertion**: A client outcome supported by a request to the real server rather than an emulated response.
- **Diagnostic bundle**: Logs and metadata retained after a run without retaining its database as a reusable test baseline.

## Success Criteria

- **SC-001**: AC-001–006 pass on two consecutive clean local runs, including a repeated pre-mutation initialization; no manual fixture preparation is required.
- **SC-002**: Every AC-007–011 compatibility case passes for default native TLS, Rustls, and generated Node/WASM; AC-012 passes for preload on all three and persistence on native clients.
- **SC-003**: A GitHub Actions PR run and main-push run execute the full suite using public dependencies and disposable credentials. An authorized fork PR run, or explicitly recorded fork-event verification, establishes AC-013 without claiming evidence that was not obtained.
- **SC-004**: Deliberate missing-Docker, broken-seed, test-failure, and catchable-interrupt exercises demonstrate nonzero failure results and applicable diagnostics/cleanup; no required integration stage silently passes with zero selected tests.
- **SC-005**: Two concurrent local runs demonstrate isolation, and teardown removes only their own resources. Explicit fast mode works with Docker unavailable.
- **SC-006**: The handoff records a requirement-to-test migration mapping, pinned artifacts with provenance, measured execution evidence on Linux and macOS ARM64, and all remaining validation gaps. None of these runtime results is claimed by this documentation-only draft.

## Assumptions and Dependencies

Depends on [001](../001-read-configuration/spec.md), [002](../002-retain-configuration/spec.md), [003](../003-observe-updates/spec.md), and [004](../004-javascript-client/spec.md) for the client outcomes being tested. Those retrospective specifications remain drafts; this feature does not promote them to approved product policy. Their [wire contracts](../supporting/contracts.md) and [open decisions](../supporting/research.md) constrain test interpretation.

Contributors provide a working container runtime with adequate resources, stable Rust, the WASM toolchain, Node, and initial network access for public dependencies. Subsequent fixture preparation uses repository-owned data. Exact versions, deadline defaults, resource estimates, and administrative API dependencies are implementation decisions in the plan, not newly inferred client performance or support guarantees.
