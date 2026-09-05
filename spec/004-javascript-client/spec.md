# Feature Specification: Consume Apollo configuration from JavaScript

**Feature ID**: `004-javascript-client`
**Created**: 2026-09-06
**Status**: Draft — retrospectively reconstructed, pending maintainer acceptance
**Input**: Reconstruct the requirements for JavaScript developers using the client's WebAssembly package in browser and Node-style environments.

## Purpose and Scope

JavaScript applications need the same configuration retrieval, retention, and update capabilities without implementing a separate Apollo client. The package must present usable JavaScript values and adapt to host capabilities such as environment variables, storage, timers, and certificate verification.

In scope: exported client/configuration classes, promises and synchronous value access, JavaScript callbacks, Node environment loading, browser storage fallback, and explicit release of owned wrappers. New browser/Node version commitments, CORS bypass, and native transport injection from JavaScript are outside scope.

## User Scenarios & Testing

### User Story 1 — Read configuration with JavaScript values (Priority: P1)

As a JavaScript developer, I want to construct a client and await namespaces so that configuration integrates naturally with my application.

**Why this priority**: The package must deliver readable values before retention or subscriptions add value.
**Independent Test**: Load generated package exports, construct a client against controlled fetch responses, and read all supported formats.

**Acceptance Scenarios**:

1. **AC-001** — **Given** a loaded package and valid connection configuration, **When** a Client is constructed and a namespace is awaited, **Then** the package exposes usable configuration/client classes and resolves the requested value without requiring polling.
2. **AC-002** — **Given** Properties, JSON, YAML, and Text responses, **When** they are read, **Then** Properties exposes synchronous scalar getters, JSON/YAML yields structured JavaScript-compatible values, and Text yields a string; signed integer Properties values use bigint without narrowing to a JavaScript number.
3. **AC-003** — **Given** malformed connection configuration or failed retrieval with no retained response, **When** construction or a read is attempted, **Then** the caller can catch the construction error or rejected promise with diagnostic information.
4. **AC-004** — **Given** a preload array containing a non-string entry, **When** preload is called, **Then** it rejects with the entry's index before starting namespace retrieval; valid string lists and explicit refresh remain available.

### User Story 2 — React to updates from JavaScript (Priority: P1)

As a JavaScript developer, I want update callbacks and polling control so that my application can react to configuration changes.

**Why this priority**: JavaScript consumers need the same update capability as native applications.
**Independent Test**: Register a JS callback, drive changed/failed responses, and start/stop polling through exported methods.

**Acceptance Scenarios**:

1. **AC-005** — **Given** a listener and a changed Properties response, **When** the notification is delivered, **Then** the callback receives a plain data object and an undefined error; a refresh failure supplies undefined data and error information instead.
2. **AC-006** — **Given** a listener that throws a JavaScript exception, **When** it is invoked, **Then** the exception is diagnosed and later listeners for that notification can still run.
3. **AC-007** — **Given** the exported client, **When** start and stop are called, **Then** both are synchronous operations with feature 003's lifecycle outcomes; namespace, preload, refresh, and listener registration are awaitable operations.

### User Story 3 — Run with available host configuration and storage (Priority: P1)

As a JavaScript developer, I want the client to adapt to my runtime so that browser and Node-style applications work without assuming capabilities the host does not provide.

**Why this priority**: Missing browser storage or process variables must have predictable outcomes.
**Independent Test**: Execute with controlled globals representing available/absent storage and Node/browser environments.

**Acceptance Scenarios**:

1. **AC-008** — **Given** a Node-style process environment with valid required variables, **When** environment configuration is loaded, **Then** those values and the standard defaults are used; without that environment, missing required variables are reported.
2. **AC-009** — **Given** working localStorage, **When** configuration is retrieved and a later matching client reads it, **Then** local retention is available; absent, inaccessible, or failing storage still permits remote retrieval and memory reuse.
3. **AC-010** — **Given** an insecure-HTTPS override in JavaScript, **When** a client is constructed, **Then** the host continues controlling certificate verification and the ineffective override is diagnosed.
4. **AC-011** — **Given** no retained response and a fetch or body that never completes, **When** a namespace is requested with a finite timeout, **Then** the promise terminates through the request deadline without relying on the fetch completing.

### User Story 4 — Release resources the application owns (Priority: P2)

As a JavaScript developer, I want explicit ownership and release rules so that creating and disposing clients does not unnecessarily retain WASM-owned wrappers.

**Why this priority**: Long-lived applications must manage resource lifetime, while ordinary one-off reads already provide value.
**Independent Test**: Construct a configuration, transfer it to a client, read a Properties wrapper, release live wrappers, and observe the owned poller's cancellation.

**Acceptance Scenarios**:

1. **AC-012** — **Given** a configuration transferred to a Client and a Properties wrapper returned by a read, **When** their use ends, **Then** the application can release the live Properties and Client wrappers; it does not reuse the consumed configuration. Plain JSON/YAML/Text and listener data require no WASM-wrapper release.

### Edge Cases

- Configuration timing values use the generated unsigned-64-bit/bigint interface; callers must respect generated optional-value types.
- Configuration construction and client validation are separate steps. Mutating configuration fields does not itself prove validity.
- Direct Properties reads and Properties listener payloads intentionally have different representations.
- Current conversion helpers can log and return null for serialization failures or malformed YAML. Whether these should instead reject is unresolved; do not infer a universal reject-on-invalid-content promise.
- Releasing a client cancels its owned poller; a guarantee covering detached revalidation and all callbacks is unresolved.
- Hosts without process variables should use direct configuration. Host TLS/CORS restrictions cannot be overridden by this library.

## Requirements

### Functional Requirements

- **FR-001**: The JavaScript package MUST expose configuration/client construction and namespace, preload, refresh, listener registration, start, and stop operations.
- **FR-002**: Async operations MUST be awaitable, while Properties getters and polling start/stop MUST be synchronous.
- **FR-003**: Direct namespace reads MUST provide the format-specific JavaScript representations in AC-002, preserving signed 64-bit Properties integers through bigint.
- **FR-004**: Client construction/retrieval failures MUST be catchable with diagnostics; invalid preload entries MUST identify their index before retrieval begins.
- **FR-005**: Listeners MUST receive `(data, error)` with only the applicable argument defined, using plain Properties data for successful notifications and isolating thrown JavaScript exceptions.
- **FR-006**: Environment loading MUST support Node-style process variables and report missing or invalid values rather than inventing a browser process environment.
- **FR-007**: Available localStorage MUST support matching-identity persistence; unavailable/failing storage MUST permit memory-only operation, and native cache-directory settings MUST have no storage effect in WASM.
- **FR-008**: HTTPS verification MUST remain under host control, with an ineffective insecure override diagnosed.
- **FR-009**: The configured request deadline MUST bound stalled fetch/body waiting in the JavaScript runtime.
- **FR-010**: Live WASM-owned wrappers MUST support explicit release, and the ownership contract MUST distinguish consumed configuration, owned Client/Properties wrappers, and ordinary JavaScript values.

### Key Entities

- **JavaScript client**: The consumer's owner of configuration operations and periodic-refresh lifetime.
- **Namespace representation**: A Properties wrapper, structured JavaScript-compatible value, or Text string returned to application code.
- **Callback result**: A data/error pair delivered to JavaScript following namespace change or refresh failure.
- **Host capabilities**: Runtime-provided environment access, fetch, storage, timers, and TLS policy.

## Success Criteria

### Measurable Outcomes

- **SC-001**: Generated-package checks expose every operation in FR-001, and all format/type fixtures in AC-001–004 produce the specified values or errors.
- **SC-002**: Changed/failed/throwing-callback fixtures in AC-005–007 deliver the specified argument shapes and lifecycle behavior.
- **SC-003**: Available/absent/failing-host-capability fixtures in AC-008–011 produce their specified environment, storage, TLS, and timeout outcomes.
- **SC-004**: The ownership scenario in AC-012 releases each still-owned wrapper once and does not require releasing plain values or reusing a consumed configuration.

## Assumptions and Dependencies

Depends on features 001–003 for shared behavior. The host supplies a compatible fetch implementation, timers, and permission to access Apollo. This specification establishes no new minimum browser/Node version or measured leak-free guarantee; exact binding signatures and ownership evidence are in [contracts](../supporting/contracts.md) and [traceability](../supporting/traceability.md).
