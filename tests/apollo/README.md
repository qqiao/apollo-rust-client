# Real Apollo Integration Testing Infrastructure

This directory contains the integration testing environment for `apollo-rust-client` running against genuine Apollo services in Docker Compose.

## Prerequisites and System Requirements

### Required Tools
- **Rust toolchain**: Stable Rust with `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).
- **wasm-pack**: For building WebAssembly Node/browser packages (`cargo install wasm-pack`).
- **Node.js**: Node.js 24 LTS or later.
- **Docker & Docker Compose**: Docker daemon with Compose plugin v2.20+ (`docker compose`).
  - Required for full testing (`scripts/test.sh`) and integration testing (`scripts/test.sh integration`).
  - **Not required** for fast testing (`scripts/test.sh fast`).

### Resource Expectations
- **RAM**: ~2–3 GiB free memory for MySQL and Apollo services.
- **CPU**: 2 CPU cores recommended.
- **Disk Space**: ~1.5 GiB for pinned container images and build caches.
- **Initial Network Access**: Required on initial run to download container images and dependencies; subsequent runs use local Docker cache.

## Test Entry Modes and Commands

`scripts/test.sh` is the public repository entry point. It dispatches to `scripts/apollo-test.sh` for integration lifecycle management.

| Command | Purpose | Docker Required? |
|---|---|---|
| `scripts/test.sh` (or `scripts/test.sh all`) | Runs fast checks (Clippy x3, unit tests, doc tests, wasm tests), then executes all real Apollo integration suites. | Yes (for integration phase) |
| `scripts/test.sh fast` | Runs fast checks only (Clippy, unit/fault tests, doc tests, WASM unit tests). Rejects `--suite` and `--filter`. | **No** (never queries or touches Docker) |
| `scripts/test.sh integration` | Provisions a disposable Apollo stack, runs all integration suites (`native`, `rustls`, `wasm`), captures diagnostics on failure, and tears down. | Yes |
| `scripts/test.sh integration --suite <name>` | Runs only the specified integration suite (`native`, `rustls`, or `wasm`). | Yes |
| `scripts/test.sh integration --filter <pattern>` | Passes test name filter to the integration runner. Fails visibly if 0 tests match. | Yes |
| `scripts/apollo-test.sh cleanup` | Safely tears down the most recent or specified run using validated `ownership.json` metadata. | Yes |
| `scripts/apollo-test.sh cleanup --run-dir <dir>` | Safely tears down a specific run directory using validated `ownership.json` metadata. | Yes |

## Tested Platforms vs. Boundary Scope

- **Tested Platforms**:
  - `linux/amd64` (GitHub Actions CI hosted runners).
  - `linux/arm64` / `darwin/arm64` (macOS Apple Silicon local development).
  - Node.js 24 LTS for generated WebAssembly package integration.
- **Real-Server Evidence**:
  - Formats: Properties, JSON, YAML, Text.
  - Selection: Applications, clusters, public namespace inheritance, missing keys.
  - Security: Enforced access-key authentication (valid signatures accepted, missing/invalid signatures rejected with HTTP 401).
  - Targeting: Real Apollo grayscale branch routing with client IP and label rules.
  - Release Lifecycle: Saved-only invisibility, release publication, explicit client refresh, event listeners, and periodic polling.
- **Mock / Fault Boundary (Docker-independent)**:
  - Deterministic network faults (hung requests, TCP resets, connection timeouts, HTTP 429/500 synthetic classifications, bad outer JSON) remain in unit/fault tests using in-process `MockHttpsServer` and scoped WASM test stubs.
- **Browser Execution Note**:
  - The WASM test suite runs under Node.js 24 using a standard in-memory storage fallback. Real browser runtime execution (Chrome/Firefox/Safari DOM, live browser localStorage) is not exercised by this suite.

## Pinned Container Images and Provenance

All images are pinned by multi-platform OCI index digest supporting both `linux/amd64` and `linux/arm64` (including Apple Silicon).

| Service | Pinned Reference | Digest (Multi-arch index) | Architectures |
|---|---|---|---|
| `mysql` | `mysql:8.4.11` | `sha256:b3b90af2a6552ae30c266fdb7d5dd55f3afb72404bb78d37fe8a23eb857fd3fb` | `linux/amd64`, `linux/arm64/v8` |
| `configservice` | `apolloconfig/apollo-configservice:2.5.2` | `sha256:a5e4bb5755688fdfc77418e3e34c87095ecb9cf36c3618d05c838bec21f008e8` | `linux/amd64`, `linux/arm64` |
| `adminservice` | `apolloconfig/apollo-adminservice:2.5.2` | `sha256:a7884c10d3fdef2a79c03f3d069fc10843837c9dec6e566a9d68a86d3db0dd95` | `linux/amd64`, `linux/arm64` |

### Database Schema Provenance

- **File**: `tests/apollo/sql/apolloconfigdb.sql`
- **Upstream commit**: `ed9785c87b390c22efece9ea7ec7eeab2d20a042` (Apollo v2.5.2)
- **Source**: `https://raw.githubusercontent.com/apolloconfig/apollo/ed9785c87b390c22efece9ea7ec7eeab2d20a042/scripts/sql/profiles/mysql-default/apolloconfigdb.sql`
- **License**: Apache License 2.0 (preserved in header)
- **SHA-256**: `7b725d81410d502c7a6ead3a16b6b4daf3b4434b3fa9c57829e67a87ff47ab26`

## Image and Schema Upgrade Procedure

To deliberately upgrade Apollo container images or database schema:
1. Identify the target Apollo release (e.g. `v2.x.y`).
2. Verify multi-arch support (`linux/amd64` and `linux/arm64`) for new container images:
   ```bash
   docker buildx imagetools inspect apolloconfig/apollo-configservice:<tag>
   docker buildx imagetools inspect apolloconfig/apollo-adminservice:<tag>
   ```
3. Record the exact index digest (`sha256:...`) and update:
   - `tests/apollo/compose.yaml` (image tags and digests)
   - `scripts/apollo-test.sh` (pinned verification loop)
   - `tests/apollo/README.md` (provenance tables)
4. Download the corresponding upstream schema file:
   ```bash
   curl -sSL "https://raw.githubusercontent.com/apolloconfig/apollo/<commit>/scripts/sql/profiles/mysql-default/apolloconfigdb.sql" \
     -o tests/apollo/sql/apolloconfigdb.sql
   ```
5. Record the new upstream commit and SHA-256 in `tests/apollo/README.md`.
6. Run full verification with `./scripts/test.sh` to ensure initialization, idempotency, and all three test runtimes pass cleanly.

## Configuration and Startup

- **Profiles**: `SPRING_PROFILES_ACTIVE=github,database-discovery` and `APOLLO_PROFILE=github,database-discovery`. The `github` profile configures standard MySQL datasource properties; `database-discovery` enables internal service registration via database without Eureka.
- **Datasource**: `jdbc:mysql://mysql:3306/ApolloConfigDB?characterEncoding=utf8&useSSL=false&allowPublicKeyRetrieval=true&serverTimezone=UTC`
- **Ports**: Dynamic loopback ports published to `127.0.0.1`.
  - ConfigService listens on internal port 8080.
  - AdminService listens on internal port 8090.
  - MySQL listens on internal port 3306 within the private Compose network (not published to host).
- **Readiness**:
  - MySQL is probed with an authenticated TCP SQL query over loopback: `mysql --protocol=TCP -h127.0.0.1 -uapollo -papollo -e 'SELECT COUNT(1) FROM ApolloConfigDB.ServerConfig;'`. This ensures MySQL TCP networking is active and the full schema, including seed data, is completely loaded.
  - ConfigService and AdminService depend on MySQL being healthy, and provide `/health` endpoints probed via IPv4 loopback (`http://127.0.0.1:...`).
- **Project Isolation**:
  - Compose commands pass `-p <unique-project-name>` (e.g. `apollo-test-<run-id>`) so concurrent or successive runs own independent networks, containers, and named volumes (`mysql-data`).
  - Cleanup invokes `docker compose -f tests/apollo/compose.yaml -p <project-name> down --volumes --remove-orphans`.

## Fixture Management (`scripts/apollo-fixtures.mjs`)

The declarative fixture manifest is stored in `tests/apollo/fixtures.json`. Fixtures are managed through `scripts/apollo-fixtures.mjs` using only Node built-ins.

### Available commands:

```bash
# Seed all fixtures idempotently and write state
node scripts/apollo-fixtures.mjs seed \
    --admin-url http://127.0.0.1:8090 \
    --config-url http://127.0.0.1:8080 \
    --run-id <run-id> \
    --fixtures tests/apollo/fixtures.json \
    --state-file /path/to/state.json

# Verify published data through ConfigService
node scripts/apollo-fixtures.mjs verify \
    --config-url http://127.0.0.1:8080 \
    --run-id <run-id> \
    --fixtures tests/apollo/fixtures.json

# Save item in a declared mutable namespace without publishing
node scripts/apollo-fixtures.mjs set-item \
    --admin-url http://127.0.0.1:8090 \
    --app 101010101 \
    --cluster default \
    --namespace updates-native \
    --key myKey --value myValue

# Publish release in a mutable namespace
node scripts/apollo-fixtures.mjs publish \
    --admin-url http://127.0.0.1:8090 \
    --app 101010101 \
    --cluster default \
    --namespace updates-native \
    --name my-release

# Set item and publish in a single operation
node scripts/apollo-fixtures.mjs set-and-publish \
    --admin-url http://127.0.0.1:8090 \
    --app 101010101 \
    --cluster default \
    --namespace updates-native \
    --key myKey --value myValue \
    --name my-release
```

## Administrative API Insights (Apollo 2.5.2)

1. **App creation** (`POST /apps`):
   - Mandatory fields: `appId`, `name`, `orgId`, `orgName`, `ownerName`, `ownerEmail`, `dataChangeCreatedBy`.
   - Missing `ownerEmail` results in a `400 Bad Request` Bean Validation failure.
2. **Item creation** (`POST /apps/{appId}/clusters/{clusterName}/namespaces/{namespaceName}/items`):
   - Mandatory fields: `namespaceId`, `key`, `value`, `comment` (`''`), `dataChangeCreatedBy`, `dataChangeLastModifiedBy`.
   - Missing `dataChangeLastModifiedBy` causes a database `DataIntegrityViolationException` when creating the change audit commit.
3. **AppNamespace creation** (`POST /apps/{appId}/appnamespaces`):
   - Mandatory fields: `appId`, `name`, `format`, `isPublic`, `comment` (`''`), `dataChangeCreatedBy`, `dataChangeLastModifiedBy`.
   - Missing `comment` causes a database `DataIntegrityViolationException` because column `Comment` is `NOT NULL`.
4. **Release publication** (`POST /apps/{appId}/clusters/{clusterName}/namespaces/{namespaceName}/releases?name={name}&operator={operator}&isEmergencyPublish=false`):
   - Publishes pending changes into an immutable release.
   - Saved items are invisible to ConfigService before publication; only published releases are served to clients.
5. **Properties namespace suffix handling**:
   - Apollo ConfigService strips `.properties` at its wire boundary via `NamespaceUtil.filterNamespaceName`.
   - For example, querying `/configfiles/json/101010101/default/config.properties` resolves to the namespace named `config`.
   - The fixture namespace is named `config` in Apollo, allowing client queries with or without `.properties` to succeed identically.

## Authentication Enforcement (Apollo 2.5.2)

- **Access Key Management**:
  - Keys are provisioned via `POST /apps/{appId}/accesskeys` with `mode: 0` (`FILTER`), `enabled: true`, and test secret (e.g. `apollo-integration-only-secret-v1`).
  - Mode `0` enforces authentication at ConfigService via `ClientAuthenticationFilter`.
  - Re-seeding checks existing keys via `GET /apps/{appId}/accesskeys` and updates them in-place if needed via `PUT /apps/{appId}/accesskeys/{id}/enable?mode=0`, preventing key accumulation.
- **Wire Authorization Protocol**:
  - Request headers:
    - `Timestamp`: Milliseconds since Unix epoch.
    - `Authorization`: `Apollo <appId>:<signature>`.
  - Signature calculation:
    `signature = Base64(HMAC-SHA1(secret, timestamp + "\n" + pathAndQuery))`
  - Negative controls:
    - Unsigned requests to protected apps fail with HTTP `401 Unauthorized`.
    - Requests signed with an invalid secret fail with HTTP `401 Unauthorized`.

## Grayscale Routing and Branching

- **Branch Creation and Publication**:
  - A child branch is created via `POST /apps/{appId}/clusters/{cluster}/namespaces/{namespace}/branches?operator=apollo-test`.
  - Overridden configurations (e.g. `grayScaleValue="true"`) are created via `POST .../clusters/{branchCluster}/namespaces/{namespace}/items`.
  - The branch release is published via `POST .../clusters/{branchCluster}/namespaces/{namespace}/releases`.
- **Rule Installation and Convergence**:
  - Rules are installed via `PUT .../clusters/{cluster}/namespaces/{namespace}/branches/{branchCluster}/rules`.
  - Target rules specify `clientIpList=["1.2.3.4"]` and `clientLabelList=["GrayScale"]` using `GrayReleaseRuleItemDTO`.
  - Propagation occurs via database message notifications (`ReleaseMessageScanner`) within 1–2 seconds.
- **ConfigService Resolution**:
  - Matching queries (`?ip=1.2.3.4`, `?label=GrayScale`, or both) route to the branch release (`grayScaleValue="true"`).
  - Non-matching queries (`?ip=1.2.3.5`, `?label=OtherLabel`, or no query parameters) route to the parent baseline release (`grayScaleValue="false"`).
  - Targeted queries on protected apps also require valid signatures; unsigned targeting fails with HTTP 401.

## Supervision, Diagnostic Logs, and Cleanup

- **Diagnostic Location**:
  - Active run directory: `${RUNNER_TEMP:-/tmp}/apollo-test-...`
  - Symlinked pointer: `target/apollo-test-latest`
  - Log files: `target/apollo-test-latest/logs/compose-services.log`, `compose-ps.txt`, `seed.log`, `verify.log`, `suite-native.log`, `suite-rustls.log`, `suite-wasm.log`
  - State files: `target/apollo-test-latest/ownership.json`, `state.json`, `state-second.json`
- **Signal Handling & Limitations**:
  - Traps `SIGINT` (exit 130) and `SIGTERM` (exit 143), terminates active child processes, captures diagnostic logs, and cleans up Docker resources once.
  - OS kills that cannot be caught (e.g. `SIGKILL`, system crashes, Docker daemon restart) cannot run traps.
- **Scoped Recovery Cleanup**:
  - If a previous run was aborted or killed, run:
    ```bash
    scripts/apollo-test.sh cleanup
    # Or for a specific run directory:
    scripts/apollo-test.sh cleanup --run-dir /path/to/run/dir
    ```
  - Safety check: `cleanup` validates `ownership.json` and verifies that `project` begins with `apollo-test-` before issuing `docker compose down`. It never performs global Docker pruning.
