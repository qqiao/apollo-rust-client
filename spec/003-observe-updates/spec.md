# Feature Specification: Observe configuration updates

**Feature ID**: `003-observe-updates`
**Created**: 2026-09-06
**Status**: Draft — retrospectively reconstructed, pending maintainer acceptance
**Input**: Reconstruct the requirements for applications that need automatic refresh and notifications of configuration changes.

## Purpose and Scope

An application needs to learn when configuration changes without repeatedly implementing its own refresh loop. Operators need healthy namespaces to continue refreshing when another fails, and applications need to start and stop that activity as part of their lifecycle.

In scope: opt-in periodic refresh, namespace registration, change/error listeners, retry backoff, explicit start/stop/restart, and cleanup of the owned polling activity. Push notifications, Apollo long polling, listener removal, callback replay on registration, and globally ordered events are outside scope.

## User Scenarios & Testing

### User Story 1 — React to changed settings (Priority: P1)

As an application developer, I want a callback when a namespace changes so that my application can update its behavior without restarting.

**Why this priority**: Delivering changes is the core value of subscriptions.
**Independent Test**: Register listeners, fetch a first response, then explicitly refresh with unchanged and changed responses; no periodic scheduler is needed for this story.

**Acceptance Scenarios**:

1. **AC-001** — **Given** listeners registered before the namespace is populated, **When** its first response is loaded, **Then** listeners receive that namespace; when a changed response replaces it, listeners receive the new value.
2. **AC-002** — **Given** an already loaded namespace, **When** the same response is refreshed or a new listener is merely registered, **Then** neither event alone produces a value callback.
3. **AC-003** — **Given** several listeners registered for one namespace, **When** one change is delivered, **Then** they are invoked in registration order; a caught callback failure does not prevent later listeners receiving that notification.
4. **AC-004** — **Given** a loaded namespace and an explicit or automatic refresh that starts a retrieval and fails, **When** failure is reported, **Then** listeners receive refresh-error information and retained data is preserved; an initial read-fetch failure alone is returned to its caller without a duplicate error callback.
5. **AC-005** — **Given** a changed response with malformed JSON content, missing/non-string content in JSON/YAML/Text, or an XML namespace, **When** listeners are notified, **Then** they receive the corresponding content/parsing/unsupported-format failure instead of a successful namespace value.

### User Story 2 — Keep registered namespaces up to date (Priority: P1)

As an application developer, I want to enable periodic refresh for namespaces my application uses so that subsequent reads and subscriptions see updated configuration automatically.

**Why this priority**: Manual refresh cannot provide automatic updates.
**Independent Test**: Register a namespace, start polling, change the fixture's response, and observe the new value without making a manual refresh call.

**Acceptance Scenarios**:

1. **AC-006** — **Given** registered namespaces and a stopped client, **When** polling starts, **Then** refresh attempts occur without the application issuing reads or manual refreshes, and later changed values become readable and observable.
2. **AC-007** — **Given** a client with no registered namespaces, **When** polling starts, **Then** no namespace is invented or fetched; namespaces subsequently used by reads, preload, refresh, or listener registration become eligible for later polling.
3. **AC-008** — **Given** a completed refresh round and no retry delay, **When** polling continues, **Then** the next round is scheduled after the configured refresh interval, independently of retained-data freshness.

### User Story 3 — Continue updating healthy namespaces during failures (Priority: P1)

As a service operator, I want failed namespaces retried less aggressively while healthy ones continue updating so that one outage does not suppress all configuration updates.

**Why this priority**: A single failing namespace must not disable a client's other subscriptions.
**Independent Test**: Register one healthy and one failing namespace, advance polling through multiple rounds, and inspect each namespace's requests and events.

**Acceptance Scenarios**:

1. **AC-009** — **Given** one namespace repeatedly fails while another succeeds, **When** polling continues, **Then** healthy refreshes continue and repeated failures grow the retry delay toward a bounded range; later delays remain bounded rather than growing indefinitely. Test the growth, capped range, and the recovery in AC-010.
2. **AC-010** — **Given** a namespace previously in retry backoff, **When** its retrieval succeeds, **Then** normal periodic eligibility resumes without retaining the previous failure delay.

### User Story 4 — Control background activity with application lifetime (Priority: P1)

As an application developer, I want deterministic control of periodic refresh so that duplicate starts and shutdown do not leave unintended polling activity.

**Why this priority**: Applications must be able to own the lifetime of background work.
**Independent Test**: Start a client with a long interval, start it again, stop twice, restart it, and release it while observing periodic requests.

**Acceptance Scenarios**:

1. **AC-011** — **Given** running polling, **When** start is called again, **Then** an already-running error is returned and no additional polling activity is created.
2. **AC-012** — **Given** polling waiting for a long interval, **When** stop is called, **Then** stop does not wait for the interval and future periodic rounds cease; repeating stop is harmless.
3. **AC-013** — **Given** a stopped client, **When** polling is restarted, **Then** its registered namespaces remain eligible; **When** the client is released, **Then** its owned periodic activity is cancelled.

### Edge Cases

- First-load notifications may originate from local persistence and do not necessarily describe newly published server data.
- Changes are determined by the raw configuration response. Equivalent parsed content expressed with different raw content can generate a notification; a timestamp-only refresh cannot.
- Listeners are synchronous. Slow callbacks can delay the operation delivering them; no synchronous reentrant-refresh guarantee is made.
- Callback isolation covers caught/unwinding failures, not process aborts. JavaScript exceptions and conversion limitations are described in feature 004.
- Stop/release controls periodic activity. Already scheduled read-triggered revalidation is a separate unresolved lifetime issue.
- Retry delays govern polling; this feature does not guarantee an overall request-rate ceiling when applications also make manual refreshes or stale reads.

## Requirements

### Functional Requirements

- **FR-001**: Applications MUST be able to register namespace-specific listeners without fetching data or starting polling as a registration side effect.
- **FR-002**: Listeners MUST receive first population and subsequent changed-response notifications, with no replay merely on registration and no notification for unchanged responses.
- **FR-003**: Each notification MUST invoke its listener snapshot in registration order and isolate caught callback failures from subsequent listeners.
- **FR-004**: Failures of retrievals initiated by explicit refresh, polling, or stale revalidation MUST be observable by listeners; an initial read-fetch failure alone MUST be returned without a duplicate listener error event. Error notification when explicit refresh joins a failing initial read is pending decision D-011 in the research register.
- **FR-005**: Malformed JSON content, missing/non-string JSON/YAML/Text content, and unsupported XML MUST reach listeners as conversion errors rather than successful namespace values.
- **FR-006**: Consumers MUST be able to opt into periodic refreshing of namespaces registered through ordinary client operations.
- **FR-007**: Polling MUST use the configurable refresh interval separately from cache freshness and MUST NOT invent unused namespace names.
- **FR-008**: A failing namespace MUST NOT stop healthy namespaces from being refreshed in later rounds.
- **FR-009**: Repeated failures MUST reduce periodic retry frequency by growing the retry delay toward a bounded range; successful retrieval MUST reset failure backoff. The precise growth/jitter algorithm is an implementation choice described in technical design.
- **FR-010**: A client MUST reject duplicate starts, support repeated stop and restart, and cancel its owned periodic activity on release.
- **FR-011**: Stopping polling MUST NOT wait for the configured sleep interval to elapse.

### Key Entities

- **Subscription**: An association between a namespace and a callback, ordered by registration within that namespace.
- **Notification**: A namespace value or an operation/conversion failure delivered to a listener.
- **Polling lifecycle**: Stopped/running state controlling automatic refresh of registered namespaces.
- **Retry state**: Namespace-specific failure history and next periodic retry eligibility.

## Success Criteria

### Measurable Outcomes

- **SC-001**: First/unchanged/changed/failed-response fixtures produce exactly the notification categories and counts in AC-001–005, in per-notification registration order.
- **SC-002**: Every registered healthy namespace in AC-006–010 receives later refresh attempts while failure fixtures demonstrate backoff and recovery; no unused namespace is fetched.
- **SC-003**: The start/start/stop/stop/restart/release sequence produces the specified single polling lifecycle, and stopping a sleeping poller completes before its held interval is released.

## Assumptions and Dependencies

Depends on features 001 and 002 for retrieval, typed results, retention, and shared refresh. Callbacks should complete promptly. The interval is a delay after a completed round, not a guaranteed publication-to-delivery SLO. Exact scheduling/concurrency mechanics belong in the separate technical design.
