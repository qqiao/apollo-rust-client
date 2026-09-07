#!/bin/bash
# Proof script for T01: demonstrates startup, schema readiness, AdminService publication,
# negative control (saved item invisible prior to release), positive control (published value returned),
# and project-scoped cleanup.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${SCRIPT_DIR}/compose.yaml"
PROJECT="${APOLLO_TEST_PROJECT:-apollo-test-t01-proof-$$}"

cleanup() {
    echo "Tearing down proof stack for project ${PROJECT}..."
    docker compose -f "${COMPOSE_FILE}" -p "${PROJECT}" down --volumes --remove-orphans || true
}
trap cleanup EXIT

echo "Starting stack with project ${PROJECT}..."
START_TIME=$(date +%s)
docker compose -f "${COMPOSE_FILE}" -p "${PROJECT}" up -d --wait --wait-timeout 180

UP_TIME=$(date +%s)
echo "Stack is healthy! Elapsed: $((UP_TIME - START_TIME))s"

CONFIG_PORT=$(docker compose -f "${COMPOSE_FILE}" -p "${PROJECT}" port configservice 8080 | awk -F: '{print $NF}')
ADMIN_PORT=$(docker compose -f "${COMPOSE_FILE}" -p "${PROJECT}" port adminservice 8090 | awk -F: '{print $NF}')

echo "Discovered ConfigService port: ${CONFIG_PORT}"
echo "Discovered AdminService port: ${ADMIN_PORT}"

# Check health endpoints
echo "Probing ConfigService health..."
curl -sf "http://127.0.0.1:${CONFIG_PORT}/health" > /dev/null
echo "ConfigService /health OK"

echo "Probing AdminService health..."
curl -sf "http://127.0.0.1:${ADMIN_PORT}/health" > /dev/null
echo "AdminService /health OK"

# 1. Create App
echo "1. Creating App 101010101..."
CREATE_APP_RESP=$(curl -s -w "\nHTTP_STATUS:%{http_code}\n" -X POST "http://127.0.0.1:${ADMIN_PORT}/apps" \
    -H "Content-Type: application/json" \
    -d '{"appId":"101010101","name":"test-app","orgId":"TEST","orgName":"Test Org","ownerName":"apollo-test","ownerEmail":"apollo-test@example.com","dataChangeCreatedBy":"apollo-test"}')
echo "${CREATE_APP_RESP}"
if ! echo "${CREATE_APP_RESP}" | grep -q "HTTP_STATUS:200"; then
    echo "ERROR: App creation failed"
    exit 1
fi

# 2. Get default namespace ID
echo "2. Getting default namespace..."
NS_RESP=$(curl -s "http://127.0.0.1:${ADMIN_PORT}/apps/101010101/clusters/default/namespaces/application")
echo "${NS_RESP}"
NS_ID=$(echo "${NS_RESP}" | node -e 'let d = ""; process.stdin.on("data", c => d += c); process.stdin.on("end", () => console.log(JSON.parse(d).id))')
echo "Namespace ID: ${NS_ID}"

# 3. Create Item
echo "3. Creating item proof.key=proof.value..."
CREATE_ITEM_RESP=$(curl -s -w "\nHTTP_STATUS:%{http_code}\n" -X POST "http://127.0.0.1:${ADMIN_PORT}/apps/101010101/clusters/default/namespaces/application/items" \
    -H "Content-Type: application/json" \
    -d "{\"namespaceId\":${NS_ID},\"key\":\"proof.key\",\"value\":\"proof.value\",\"dataChangeCreatedBy\":\"apollo-test\",\"dataChangeLastModifiedBy\":\"apollo-test\"}")
echo "${CREATE_ITEM_RESP}"
if ! echo "${CREATE_ITEM_RESP}" | grep -q "HTTP_STATUS:200"; then
    echo "ERROR: Item creation failed"
    exit 1
fi

# 4. Read ConfigService BEFORE publish (Negative Control: unpublished item must not be served)
echo "4. Reading ConfigService before publish (must NOT contain proof.key)..."
PRE_PUBLISH_RESP=$(curl -s "http://127.0.0.1:${CONFIG_PORT}/configfiles/json/101010101/default/application")
echo "Pre-publish response: ${PRE_PUBLISH_RESP}"
if echo "${PRE_PUBLISH_RESP}" | grep -q "proof.key"; then
    echo "ERROR: proof.key found before publish!"
    exit 1
fi
echo "Confirmed: proof.key is absent before publish."

# 5. Publish release
echo "5. Publishing release..."
PUBLISH_RESP=$(curl -s -w "\nHTTP_STATUS:%{http_code}\n" -X POST "http://127.0.0.1:${ADMIN_PORT}/apps/101010101/clusters/default/namespaces/application/releases?name=proof-release&operator=apollo-test&isEmergencyPublish=false")
echo "${PUBLISH_RESP}"
if ! echo "${PUBLISH_RESP}" | grep -q "HTTP_STATUS:200"; then
    echo "ERROR: Release publication failed"
    exit 1
fi

# 6. Read ConfigService AFTER publish (Positive Control: published item must be served)
echo "6. Reading ConfigService after publish (must contain proof.key)..."
FOUND=0
for i in {1..20}; do
    POST_PUBLISH_RESP=$(curl -s "http://127.0.0.1:${CONFIG_PORT}/configfiles/json/101010101/default/application")
    echo "Attempt ${i}: ${POST_PUBLISH_RESP}"
    if echo "${POST_PUBLISH_RESP}" | grep -q "proof.value"; then
        FOUND=1
        break
    fi
    sleep 0.5
done

if [ "${FOUND}" -ne 1 ]; then
    echo "ERROR: proof.value not observed after publish!"
    exit 1
fi
echo "SUCCESS: proof.value successfully verified via ConfigService on port ${CONFIG_PORT}!"
