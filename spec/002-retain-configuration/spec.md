# Feature Specification: Retain configuration through delays and outages

**Feature ID**: `002-retain-configuration`
**Created**: 2026-09-06
**Status**: Draft — retrospectively reconstructed, pending maintainer acceptance
**Input**: Reconstruct the client's availability and startup requirements for applications that must reuse configuration when Apollo is slow or unavailable.

## Purpose and Scope

Applications need previously retrieved configuration to remain available during remote delays and outages, and across restarts when local storage is available. They also need to avoid unnecessary duplicate requests and warm known namespaces before use.

In scope: memory reuse, optional persistence, freshness, stale reads with revalidation, concurrent reads/refreshes, warm-up, and explicit refresh. This feature does not promise a maximum stale age, a transaction across namespaces, encrypted storage, or coordination between separate client instances/processes. Background scheduling and subscriptions belong to feature 003.

## User Scenarios & Testing

### User Story 1 — Continue using configuration during an outage (Priority: P1)

As an application developer, I want a previously retrieved configuration to remain readable while Apollo is slow or unavailable so that a remote failure does not unnecessarily interrupt local operation.

**Why this priority**: Availability during remote failure is the primary value of retention.
**Independent Test**: Read a known value, expire its freshness, then hold or fail the next remote response while reading the namespace again.

**Acceptance Scenarios**:

1. **AC-001** — **Given** a retained fresh value and no polling/manual refresh, **When** the namespace is read, **Then** the value is returned without another remote request.
2. **AC-002** — **Given** a retained stale value and a held-open remote refresh, **When** the namespace is read, **Then** the old value is returned before the remote request completes and revalidation is attempted.
3. **AC-003** — **Given** retained data and a revalidation that fails in transport, HTTP status, or outer-response JSON parsing, **When** it completes, **Then** the previous data remains retained and readable; with no retained response, the same failure is returned to the initial caller.
4. **AC-004** — **Given** stale retained data and a successful revalidation returning a changed value, **When** revalidation completes, **Then** subsequent reads receive the new value.
5. **AC-016** — **Given** a stale response and a successful refresh with identical content, **When** it is read within the renewed freshness lifetime with polling disabled, **Then** no new read-triggered request occurs; after that renewed lifetime expires, a read again triggers revalidation.

### User Story 2 — Restart with locally retained configuration (Priority: P1)

As a service operator, I want successful configuration reads retained locally so that a later client can recover configuration even when Apollo is unavailable at startup.

**Why this priority**: Memory-only retention cannot help a restarted process.
**Independent Test**: Retrieve and persist a namespace, create a second client for the same identity/storage, then read while Apollo is unavailable.

**Acceptance Scenarios**:

1. **AC-005** — **Given** a complete persisted response for the same identity, **When** a new client reads the namespace, **Then** it can use that response; if stale, it is returned while revalidation is attempted.
2. **AC-006** — **Given** missing, unreadable, or corrupt persisted data and a working server, **When** the namespace is first read, **Then** remote retrieval recovers the value.
3. **AC-007** — **Given** successful retrieval but unavailable or unwritable persistence, **When** the read completes, **Then** the fetched value is usable in the current client and the storage failure does not turn retrieval into a failure.
4. **AC-008** — **Given** clients differing in server, application, cluster, namespace, IP, or label, **When** they use the same storage base, **Then** stored responses from those distinct identities are not substituted for one another.

### User Story 3 — Control freshness and warm application settings (Priority: P2)

As an application developer, I want to warm known namespaces and explicitly request updated values so that startup and operator-driven reloads can use the same client.

**Why this priority**: Basic reads remain useful alone; these operations improve control over when retrieval occurs.
**Independent Test**: Warm a namespace list containing duplicates, then explicitly refresh a previously fresh namespace after changing its remote data.

**Acceptance Scenarios**:

1. **AC-009** — **Given** a list with duplicate namespace names, **When** it is preloaded, **Then** all successful loads are reusable and duplicates do not cause redundant simultaneous initial retrievals; an empty list succeeds without fetching.
2. **AC-010** — **Given** one preload item cannot be retrieved, **When** preload completes, **Then** it reports a retrieval failure and does not promise rollback of other successful loads.
3. **AC-011** — **Given** fresh retained data, **When** an explicit refresh is requested, **Then** a remote retrieval is attempted and its success or failure is reported rather than treating freshness as a reason to skip it.
4. **AC-012** — **Given** freshness lifetime zero and a retained value, **When** a read occurs, **Then** that value is returned and revalidation is attempted; zero does not disable retention or make the read wait for fresh data.

### User Story 4 — Share overlapping work without blocking retained reads (Priority: P1)

As an application developer, I want concurrent callers to reuse in-progress retrieval so that normal concurrency does not multiply load or prevent access to retained data.

**Why this priority**: Applications commonly request configuration concurrently during startup and reloads.
**Independent Test**: Hold a remote response open, issue overlapping requests for one namespace on one client, and count requests/completions.

**Acceptance Scenarios**:

1. **AC-013** — **Given** one namespace with no retained data, **When** multiple reads overlap its initial retrieval, **Then** they share a successful result without issuing a separate remote request for every caller.
2. **AC-014** — **Given** a remote refresh already in progress for one namespace, **When** additional refresh requests overlap it, **Then** they share that attempt's completion; retained reads can complete before the held remote response is released.
3. **AC-015** — **Given** an initial retrieval whose caller is cancelled, **When** another caller requests the same namespace, **Then** it can complete a later retrieval rather than remaining blocked by the abandoned caller.

### Edge Cases

- Freshness at the TTL boundary includes equality; lifetime zero is always stale. Future timestamps and extreme durations have unresolved policy.
- Retention concerns the received configuration response. It does not guarantee that later typed conversion succeeds; preload and refresh do not certify every content format's validity.
- Shared refresh callers receive the same success/failure outcome but need not receive identical typed error objects.
- Request sharing is within one client's namespace, not a global or cross-process guarantee. Nonoverlapping calls may produce separate requests.
- Explicit refresh can bypass periodic retry backoff. Whether read-triggered revalidation should do so remains a decision candidate.

## Requirements

### Functional Requirements

- **FR-001**: The client MUST reuse fresh retained configuration without a new request caused solely by the read.
- **FR-002**: The client MUST return stale retained configuration without waiting for remote revalidation and attempt revalidation when no such work is already active for that namespace.
- **FR-003**: Transport, HTTP-status, and outer-response JSON failures MUST leave previously retained responses intact; initial retrieval without retained data MUST report failure.
- **FR-004**: A successful remote refresh MUST replace the retained response and renew its freshness, including when values are unchanged.
- **FR-005**: The client MUST attempt local persistence where available and reuse complete stored responses for the matching identity on a later client load.
- **FR-006**: Persistence read/write failures and corrupt entries MUST NOT prevent a successful remote response from being used in memory.
- **FR-007**: Stored configuration MUST be isolated by server, application, cluster, namespace, IP, and label identity.
- **FR-008**: Consumers MUST be able to preload namespace lists with duplicates or no entries, with retrieval failures reported and no all-or-nothing guarantee.
- **FR-009**: Consumers MUST be able to explicitly refresh a namespace irrespective of retained freshness.
- **FR-010**: Zero freshness lifetime MUST retain existing data while making reads eligible for revalidation.
- **FR-011**: Overlapping reads/refreshes for one client's namespace MUST share active retrieval work, without making retained reads wait for remote completion.
- **FR-012**: Cancelling a waiting/initial retrieval caller MUST NOT permanently prevent subsequent retrieval of the namespace.

### Key Entities

- **Retained response**: A previously retrieved namespace response with a retrieval time and an associated configuration identity.
- **Freshness policy**: The lifetime used to decide whether retained data should trigger revalidation; distinct from periodic refresh scheduling.
- **Persistent copy**: A recoverable local representation of a retained response; optional when the runtime/storage cannot support it.
- **Preload request**: A set/list of namespace reads used to make configuration available before application use.

## Success Criteria

### Measurable Outcomes

- **SC-001**: In AC-002 and AC-014, every retained read completes while the controlled remote response is still held open.
- **SC-002**: All outage/recovery fixtures preserve the specified old value on failed retrieval and expose the new value after successful revalidation; identical-content refresh renews freshness as specified in AC-016.
- **SC-003**: Each identity-isolation fixture returns only its own persisted response; corrupt/unwritable-storage fixtures recover through remote retrieval.
- **SC-004**: Overlapping callers in AC-013–014 produce one shared active attempt per namespace; cancellation recovery in AC-015 completes after the fixture permits a new response.
- **SC-005**: Empty, duplicate, failed, explicit-refresh, and zero-lifetime fixtures produce all outcomes specified in AC-009–012.

## Assumptions and Dependencies

Depends on feature 001 for remote retrieval and typed consumption. Storage is optional and managed by the deployment/runtime. Restart scenarios assume persistence actually completed. The retention policy favors availability; no maximum stale age or cross-namespace consistency guarantee is newly introduced by this reconstruction.
