# Real Apollo Integration Testing Infrastructure

This directory contains the integration testing environment for `apollo-rust-client` running against genuine Apollo services in Docker Compose.

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

## Configuration and Startup

- **Profiles**: `SPRING_PROFILES_ACTIVE=github,database-discovery` and `APOLLO_PROFILE=github,database-discovery`. The `github` profile configures the standard MySQL datasource properties; `database-discovery` enables internal service registration and discovery via database without Eureka.
- **Datasource**: `jdbc:mysql://mysql:3306/ApolloConfigDB?characterEncoding=utf8&useSSL=false&allowPublicKeyRetrieval=true&serverTimezone=UTC`
- **Ports**: Dynamic loopback ports published to `127.0.0.1`.
  - ConfigService listens on internal port 8080.
  - AdminService listens on internal port 8090.
  - MySQL listens on internal port 3306 within the private Compose network (not published to host).
- **Readiness**:
  - MySQL is probed with an authenticated TCP SQL query over loopback: `mysql --protocol=TCP -h127.0.0.1 -uapollo -papollo -e 'SELECT COUNT(1) FROM ApolloConfigDB.ServerConfig;'`. This ensures MySQL TCP networking is active and the full schema, including the final `ServerConfig` table and seed data, is completely loaded.
  - ConfigService and AdminService depend on MySQL being healthy, and provide `/health` endpoints probed via IPv4 loopback (`http://127.0.0.1:...`).
- **Project Isolation**:
  - Compose commands must pass `-p <unique-project-name>` (e.g. `apollo-test-<run-id>`) so concurrent or successive runs own independent networks, containers, and named volumes (`mysql-data`).
  - Cleanup must invoke `docker compose -f tests/apollo/compose.yaml -p <project-name> down --volumes --remove-orphans`.

## Executing the Infrastructure Proof

A standalone reproducible proof script is available at `tests/apollo/proof.sh`. It demonstrates:
1. Pinned stack startup with dynamic port discovery and health readiness.
2. App creation via AdminService.
3. Item creation in the default application namespace.
4. Negative control: verifies that saved-only items are absent from ConfigService responses before release.
5. Release publication via AdminService.
6. Positive control: verifies that the published item is immediately served by ConfigService on the discovered port.
7. Scoped teardown of the project and its volume.

To run:
```bash
./tests/apollo/proof.sh
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

## Startup and Lifecycle Performance

- Cold start (initial image pull + initialization): ~134s.
- Warm start (images cached locally): ~21-27s until all services healthy.
- Seed operation: ~1s for all apps, clusters, namespaces, releases, branches, and rules.
- Verify operation: ~1s for full suite across all formats, authentication controls, and grayscale routing.
- Memory footprint: ~2-3 GiB across MySQL and Apollo Java services.

## T03 Qualification Evidence

The end-to-end proof script (`scratch/t03_proof.sh`) was executed on a fresh disposable stack (`apollo-test-t03-proof-10202`) with the following verified outcomes:

1. **Clean Stack Startup**: Dynamic ports discovered (`ConfigService: 8080`, `AdminService: 8090`). Stack ready in ~27s.
2. **First Seed Pass**: All entities, access keys, baseline releases, child branches, branch releases, and grayscale rules initialized in 1s.
3. **Authentication Convergence Verification**:
   - `auth-unsigned-rejected`: HTTP 401 verified.
   - `auth-wrong-secret-rejected`: HTTP 401 verified.
   - `auth-valid-secret-accepted`: HTTP 200 verified with valid HMAC-SHA1 signature.
4. **Grayscale Routing Verification**:
   - Both `plain-app` (101010101) and `secret-app` (101010102) verified.
   - Matching IP (`?ip=1.2.3.4`) -> `grayScaleValue="true"` with baseline inheritance (`stringValue="string value"`).
   - Matching Label (`?label=GrayScale`) -> `grayScaleValue="true"` with baseline inheritance.
   - Matching Both (`?ip=1.2.3.4&label=GrayScale`) -> `grayScaleValue="true"`.
   - Combinatorial OR: `?ip=1.2.3.4&label=OtherLabel` -> `grayScaleValue="true"`.
   - Combinatorial OR: `?ip=1.2.3.5&label=GrayScale` -> `grayScaleValue="true"`.
   - Non-matching IP (`?ip=1.2.3.5`) -> `grayScaleValue="false"`.
   - Non-matching Label (`?label=OtherLabel`) -> `grayScaleValue="false"`.
   - Non-matching Both (`?ip=1.2.3.5&label=OtherLabel`) -> `grayScaleValue="false"`.
   - No targeting -> `grayScaleValue="false"`.
   - Protected app targeted security: unsigned IP (`?ip=1.2.3.4`), unsigned Label (`?label=GrayScale`), and wrong-secret queries all rejected with HTTP 401.
5. **Idempotency (Pass 2)**:
   - Second seed pass ran in <1s.
   - Diff between state from pass 1 and pass 2: 0 differences (all IDs, cluster names, and release keys stable).
   - Access key count on `101010102`: exactly 1 key (no key accumulation).
6. **Negative Fault Injection & Failure Detection**:
   - Disabled key on `101010102`: verification failed immediately (`Expected HTTP 401 for unsigned request to protected app 101010102, got HTTP 200`).
   - Altered grayscale rule on `101010101`: verification failed immediately (`Expected grayScaleValue="true", got "false"`).
   - Automatic reconciliation restored both controls to green status upon re-seed.
7. **Clean Teardown**: Project-scoped network and MySQL volume removed completely.

## Orchestration and Test Lifecycle (T04)

`scripts/test.sh` is the public repository test entry point, backed by `scripts/apollo-test.sh` for integration lifecycle management.

### Supported Test Modes

| Command | Purpose | Docker Required? |
|---|---|---|
| `scripts/test.sh` (or `scripts/test.sh all`) | Runs fast checks (Clippy x3, unit tests, doc tests, wasm tests), then executes all real Apollo integration suites. | Yes (for integration phase) |
| `scripts/test.sh fast` | Runs only fast checks (Clippy x3, unit tests, doc tests, wasm tests). Rejects `--suite` and `--filter`. | **No** (never queries or touches Docker) |
| `scripts/test.sh integration` | Provisions a disposable Apollo stack, runs all integration suites (`native`, `rustls`, `wasm`), captures diagnostics on failure, and tears down. | Yes |
| `scripts/test.sh integration --suite <name>` | Runs only the specified integration suite (`native`, `rustls`, or `wasm`). | Yes |
| `scripts/test.sh integration --filter <pattern>` | Passes test name filter to the integration runner. Fails visibly if 0 tests match. | Yes |
| `scripts/apollo-test.sh cleanup --run-dir <dir>` | Recovery tool: safely tears down an abandoned project using validated `ownership.json` metadata. | Yes |

### Exported Environment Variables

The orchestrator discovers loopback endpoints and configures child test runners with:

- `APOLLO_TEST_RUN_DIR`: Path to the unique run directory containing `ownership.json`, `state.json`, and diagnostic logs.
- `APOLLO_TEST_RUN_ID`: Unique execution and fixture identifier (`run-<timestamp>-<rand>`).
- `APOLLO_TEST_CONFIG_URL`: Loopback URL for ConfigService (e.g. `http://127.0.0.1:32800`).
- `APOLLO_TEST_ADMIN_URL`: Loopback URL for AdminService (e.g. `http://127.0.0.1:32801`).
- `APOLLO_TEST_SUITE`: Currently active suite (`native`, `rustls`, or `wasm`).
- `APOLLO_TEST_WASM_PACKAGE`: Run-scoped Node package output path for WASM bindings (`<run-dir>/wasm`).

### Timeout Budgets and Supervision

- **Image Pull**: 600s timeout.
- **Service Startup**: 300s timeout with container health inspection.
- **Fixture Seeding & Verification**: 300s timeout.
- **Suite Execution**: 300s timeout per suite with process group supervision.
- **Teardown**: 30s timeout.
- **Signal Handling**: Traps `SIGINT` (exit 130) and `SIGTERM` (exit 143), terminates active child processes, captures diagnostic logs, and cleans up Docker resources once.
- **Diagnostic Capture**: On any failure or interrupt, `docker compose ps -a` and `docker compose logs` are captured to `<run-dir>/logs/` before stack teardown.

## T04 Qualification Evidence

The end-to-end qualification suite (`scratch/t04_proof.sh`) executed on 2026-09-06 with the following verified outcomes:

1. **Static Syntax Checks**: `bash -n scripts/apollo-test.sh`, `sh -n scripts/test.sh`, and `node --check scripts/apollo-fixtures.mjs` passed cleanly.
2. **Docker-Free Fast Mode**: `scripts/test.sh fast` executed with Docker completely absent from `PATH` and passed 100% (46 native + 46 feature + 21 wasm tests + doctests + 3 Clippy targets).
3. **Missing Docker Handling**: `scripts/apollo-test.sh integration` in a Docker-free environment failed visibly with `ERROR: 'docker' is required for integration tests but not found in PATH.` and exit code 1.
4. **Project Isolation Under Active Workload**: A sentinel project (`sentinel-proof-project-12263`) was run concurrently during the integration lifecycle. The sentinel project remained running and untouched across all test executions, failures, and teardowns.
5. **Real Integration Lifecycle & Missing Suite Executor Detection**:
   - Disposable project `apollo-test-1788701134-eacc5f2f` provisioned on dynamic loopback ports (`ConfigService: 32800`, `AdminService: 32801`).
   - Declarative fixtures seeded, verified, and idempotency-checked.
   - Missing T05 suite executor (`tests/apollo_integration.rs`) detected and failed visibly with status 1 (per T04 contract, missing tests are never treated as a passing stub).
   - Diagnostics (`compose-ps.txt`, `compose-services.log`, `seed.log`, `verify.log`) captured and preserved in the run directory.
   - Project resources torn down completely without affecting the sentinel project.
6. **Failed Seed Fault Detection**: Broken fixture input properly rejected with structured diagnostic error.
7. **Signal Interruption Handling (SIGINT)**: Running integration process interrupted via `SIGINT`, terminated supervised child process, captured diagnostics, torn down stack, exited with code 130, and left sentinel project untouched.
8. **Recovery Cleanup Tool**: `scripts/apollo-test.sh cleanup --run-dir <dir>` validated ownership metadata and project prefix before safely tearing down target Compose resources.
