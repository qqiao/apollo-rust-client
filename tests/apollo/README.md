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
   - Mandatory fields: `namespaceId`, `key`, `value`, `dataChangeCreatedBy`, `dataChangeLastModifiedBy`.
   - Missing `dataChangeLastModifiedBy` causes a database `DataIntegrityViolationException` when creating the change audit commit.
3. **Release publication** (`POST /apps/{appId}/clusters/{clusterName}/namespaces/{namespaceName}/releases?name={name}&operator={operator}&isEmergencyPublish=false`):
   - Publishes pending changes into an immutable release.
   - Saved items are invisible to ConfigService before publication; only published releases are served to clients.

## Startup Performance Measurements

- Cold start (initial image pull + initialization): ~134s.
- Warm start (images cached locally): ~21-22s until all services healthy.
- Peak memory for stack: ~2-3 GiB.
