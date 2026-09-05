# Feature Specification: Read typed Apollo configuration

**Feature ID**: `001-read-configuration`
**Created**: 2026-09-06
**Status**: Draft — retrospectively reconstructed, pending maintainer acceptance
**Input**: Reconstruct the existing client's requirements for application developers who need to retrieve and use Apollo configuration.

## Purpose and Scope

An application developer needs configuration for the correct application, deployment cluster, and rollout audience, in a form the application can use safely. This feature provides connection setup, authenticated retrieval, namespace format selection, typed extraction, and actionable failures.

In scope: native Rust consumption, direct/environment configuration, optional access-key authentication, IP/label targeting, Properties/JSON/YAML/Text, and request deadlines. Retained configuration, update subscriptions, and JavaScript consumption have separate feature specifications. Apollo administration, configuration writes, XML parsing, and server discovery are outside this feature.

## User Scenarios & Testing

### User Story 1 — Read an application's configuration (Priority: P1)

As an application developer, I want to identify my application and Apollo server and read a namespace so that my application uses centrally managed settings.

**Why this priority**: Every other client capability depends on successful configuration retrieval.
**Independent Test**: Publish a Properties namespace on a controlled Apollo endpoint, create a client with only required settings, and read its values without enabling update polling.

**Acceptance Scenarios**:

1. **AC-001** — **Given** an application with a published namespace and valid connection settings, **When** the application requests that namespace, **Then** it receives that application's configuration from the selected cluster without first starting polling.
2. **AC-002** — **Given** omitted optional settings, **When** configuration is constructed, **Then** the cluster is `default`, freshness lifetime is 600 seconds, refresh interval is 30 seconds, and request timeout is 10 seconds.
3. **AC-003** — **Given** two namespaces with different names, including a dotted public Properties name, **When** each is requested, **Then** each returns its own values and the dotted name remains a Properties namespace unless its final suffix names a supported content format.

### User Story 2 — Configure deployment and detect mistakes (Priority: P1)

As a service operator, I want to supply deployment configuration through environment variables and receive specific errors so that configuration mistakes can be corrected before serving requests.

**Why this priority**: Incorrect deployment identity prevents all useful reads or selects the wrong configuration.
**Independent Test**: Construct configurations from controlled environment inputs; no Apollo connection is needed to test validation.

**Acceptance Scenarios**:

1. **AC-004** — **Given** required environment settings and valid optional overrides, **When** environment configuration is loaded, **Then** supplied values are used and omitted values receive the same defaults as direct configuration.
2. **AC-005** — **Given** a missing required environment variable, malformed optional number/boolean, blank application or cluster, non-HTTP(S) server URL, or base URL with a query/fragment, **When** configuration is validated, **Then** construction fails with the offending variable or field identified.
3. **AC-006** — **Given** a zero freshness lifetime, **When** configuration is validated, **Then** it is accepted; **Given** a zero refresh interval or request timeout, **When** configuration is validated, **Then** it is rejected.

### User Story 3 — Access the correct rollout configuration (Priority: P1)

As an application developer, I want authentication and rollout targeting to accompany requests so that Apollo can authorize access and choose configuration for my instance.

**Why this priority**: A successful request is insufficient if it addresses the wrong audience or cannot access a protected namespace.
**Independent Test**: Use an Apollo-compatible fixture that checks signatures and returns distinct responses for IP and label targeting.

**Acceptance Scenarios**:

1. **AC-007** — **Given** no retained response and an Apollo fixture that requires an access secret, **When** a read is made with that secret, **Then** the request carries a timestamp and valid Apollo signature; when the fixture rejects an unsigned request, the client reports that HTTP failure.
2. **AC-008** — **Given** configured IP and/or label targeting, **When** a namespace is requested, **Then** the supplied targeting values reach Apollo without losing reserved characters and the corresponding rollout response is returned.
3. **AC-009** — **Given** a server URL with a base path and identifiers containing reserved characters, **When** a request is issued, **Then** that base path is retained and identifiers cannot introduce additional URL path or query components.

### User Story 4 — Consume configuration in its published format (Priority: P1)

As an application developer, I want to extract scalars and structured settings without implementing Apollo format handling myself.

**Why this priority**: Retrieved data only provides value when the application can use it correctly.
**Independent Test**: Supply Properties, JSON, YAML, and Text fixtures and compare the extracted values with known results.

**Acceptance Scenarios**:

1. **AC-010** — **Given** Properties containing `"port":"8080"`, `"retries":3`, and `"enabled":true`, **When** the matching scalar types are requested, **Then** the values are 8080, 3, and true; missing keys, incompatible types, and collection values return no scalar value.
2. **AC-011** — **Given** valid JSON or YAML content and a matching application data type, **When** it is converted, **Then** the resulting fields match the content; YAML 1.1 values `yes` and `off` convert to true and false where booleans are requested.
3. **AC-012** — **Given** Text content containing line breaks, **When** read, **Then** its content is preserved; **Given** an XML namespace, **When** read, **Then** an explicit unsupported-format error is returned.
4. **AC-013** — **Given** missing/non-string content for JSON, YAML, or Text, **When** a typed namespace is requested, **Then** a content error is returned; malformed JSON fails on namespace conversion, while malformed YAML or a structured type mismatch fails on native typed conversion.

### User Story 5 — Diagnose failed or stalled requests (Priority: P1)

As an application developer, I want failed requests to have useful diagnostics and a finite wait so that configuration retrieval cannot indefinitely stall startup.

**Why this priority**: The application needs a definite result even when Apollo or its network fails.
**Independent Test**: Request uncached namespaces from fixtures returning non-success statuses, malformed responses, and stalled bodies.

**Acceptance Scenarios**:

1. **AC-014** — **Given** no retained response and an HTTP error, invalid response JSON, or transport failure, **When** a namespace is requested, **Then** the read fails with the corresponding error category; HTTP errors include status and response detail.
2. **AC-015** — **Given** a request timeout of T seconds and a response that never completes, **When** an uncached request is made, **Then** the network wait ends on timeout, including when only the response body stalls or a custom native transport is used.

### User Story 6 — Use the deployment's HTTPS transport (Priority: P2)

As an operator, I want secure defaults and control over native transport configuration so that the client works with my deployment's trust and connectivity requirements.

**Why this priority**: Standard deployments work with defaults; specialized deployments need an explicit integration path.
**Independent Test**: Use a self-signed HTTPS fixture and configurations with default validation, an explicit development override, and a supplied native transport.

**Acceptance Scenarios**:

1. **AC-016** — **Given** an untrusted certificate, **When** the default native transport connects, **Then** verification fails; an explicitly enabled insecure override permits that development connection.
2. **AC-017** — **Given** a caller-supplied native transport, **When** configuration is retrieved, **Then** its trust/connectivity settings take precedence and the client's outer request deadline still applies; an ignored insecure override is diagnosed.

### Edge Cases

- Final suffix matching is case-insensitive. `.json`, `.yaml`/`.yml`, and `.txt` select content formats; no recognized suffix selects Properties. `.xml` reports unsupported format.
- Properties keys are exact keys, including dots. Scalar conversion does not interpret dotted keys as paths or apply YAML's permissive boolean rules.
- Native YAML namespace retrieval does not itself promise syntax validation; typed conversion does. JavaScript conversion has separate failure behavior in feature 004.
- Empty namespaces, standalone dot segments, very large timing values, and clock-skew policy remain outside the agreed input guarantees; see the decision register.
- The request timeout governs network waiting through body completion, not an entire cache-read operation including queueing, callbacks, and storage.

## Requirements

### Functional Requirements

- **FR-001**: The client MUST let an application select an Apollo server, application, cluster, and namespace and read without enabling polling.
- **FR-002**: The client MUST support direct configuration and environment loading with the defaults in AC-002 and explicit optional overrides.
- **FR-003**: Configuration validation MUST reject the invalid inputs in AC-005–006 and identify the relevant field or variable.
- **FR-004**: The client MUST support optional Apollo access-key authentication and IP/label rollout targeting without changing their supplied meaning.
- **FR-005**: Request addressing MUST preserve the server base path and encode reserved characters in identifiers and targeting values.
- **FR-006**: The client MUST select Properties, JSON, YAML, or Text by the namespace's final suffix and explicitly reject XML as unsupported.
- **FR-007**: Properties MUST support synchronous string, signed integer, floating-point, and boolean extraction, returning absence on unsupported values or failed conversion.
- **FR-008**: Native consumers MUST be able to synchronously convert JSON/YAML into application data types, preserving YAML 1.1 scalar compatibility and reporting conversion errors.
- **FR-009**: Text retrieval MUST preserve the supplied string; content-based formats MUST report a missing/non-string content field.
- **FR-010**: Uncached retrieval failures MUST distinguish configuration, transport, HTTP status, response parsing, namespace conversion, and timeout failures sufficiently for diagnosis.
- **FR-011**: A finite configurable request deadline MUST cover network waiting through response-body completion, including caller-supplied native transports.
- **FR-012**: Native HTTPS MUST verify certificates by default, allow an explicit insecure override, and respect supplied transport settings with a diagnostic when the override cannot apply.

### Key Entities

- **Connection settings**: Application identity, server address, cluster, optional authentication and rollout targeting, timing controls, and native transport/storage preferences.
- **Namespace**: A named configuration collection scoped to an application and cluster, with a format determined by its name.
- **Configuration value**: Scalar Properties values, structured JSON/YAML data, or Text that the application consumes.
- **Retrieval failure**: A categorized unsuccessful operation with enough contextual information to diagnose its cause.

## Success Criteria

### Measurable Outcomes

- **SC-001**: All fixtures in AC-001–009 return the selected identity's expected values or specified validation/access failure; no fixture returns another identity's configuration.
- **SC-002**: Every format/value fixture in AC-010–013 yields the expected value, absence, or error, including case-insensitive names and YAML legacy booleans.
- **SC-003**: Every uncached error fixture in AC-014 produces the expected category, and every stalled-request fixture in AC-015 terminates through the configured deadline rather than waiting for server completion.
- **SC-004**: The three transport configurations in AC-016–017 exhibit their specified trust behavior without bypassing the outer deadline.

## Assumptions and Dependencies

Apollo already hosts and authorizes the requested configuration. Native consumers supply an asynchronous runtime. Environment names and external API contracts are recorded separately in [contracts](../supporting/contracts.md). Priorities express dependency/value, not a newly approved release plan. Test clock/scheduler tolerances belong in the verification method; no production latency SLO is inferred.
