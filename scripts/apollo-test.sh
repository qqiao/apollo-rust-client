#!/usr/bin/env bash
# Apollo Integration Test Lifecycle Manager
# Provides portable orchestration for all/fast/integration modes,
# dynamic port discovery, declarative fixture seeding/verification,
# process supervision with finite timeouts, diagnostic capture,
# signal handling, and strictly scoped project cleanup.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/tests/apollo/compose.yaml"
FIXTURES_FILE="${REPO_ROOT}/tests/apollo/fixtures.json"
FIXTURES_SCRIPT="${REPO_ROOT}/scripts/apollo-fixtures.mjs"

# Global state tracking
CURRENT_CHILD_PID=""
PROJECT_NAME=""
RUN_DIR=""
CLEANED_UP=0
STAGE_FAILED=0
MODE="all"
SUITE=""
FILTER=""
RECOVERY_RUN_DIR=""

print_usage() {
  cat <<'EOF'
Usage: scripts/apollo-test.sh [mode] [options]

Modes:
  all (default)        Run fast checks followed by real integration test suites
  fast                 Run fast checks (Clippy, unit tests, doc tests, wasm tests; no Docker)
  integration          Run real Apollo integration tests using Docker Compose
  cleanup              Perform recovery cleanup of an abandoned or interrupted run directory

Integration Options:
  --suite <name>       Target integration suite: native, rustls, or wasm (default: all)
  --filter <pattern>   Filter tests by pattern
  -h, --help           Show this help message

Cleanup Options:
  --run-dir <path>     Path to the run directory containing ownership.json (required for cleanup)
EOF
}

run_with_timeout() {
  local timeout_sec="$1"
  shift

  set -m
  "$@" &
  local child_pid=$!
  set +m
  CURRENT_CHILD_PID=$child_pid

  local elapsed=0
  while kill -0 "$child_pid" 2>/dev/null; do
    if [ "$elapsed" -ge "$timeout_sec" ]; then
      echo "ERROR: Command timed out after ${timeout_sec}s: $*" >&2
      kill -TERM -- "-$child_pid" 2>/dev/null || kill -TERM "$child_pid" 2>/dev/null || true
      sleep 2
      kill -KILL -- "-$child_pid" 2>/dev/null || kill -KILL "$child_pid" 2>/dev/null || true
      wait "$child_pid" 2>/dev/null || true
      CURRENT_CHILD_PID=""
      return 124
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done

  wait "$child_pid"
  local status=$?
  CURRENT_CHILD_PID=""
  return $status
}

INTERRUPTED_STATUS=0

cleanup() {
  local exit_code=$?
  if [ "${INTERRUPTED_STATUS}" -ne 0 ]; then
    exit_code="${INTERRUPTED_STATUS}"
  fi
  if [ "${CLEANED_UP:-0}" -eq 1 ]; then
    return
  fi
  CLEANED_UP=1

  # 1. Terminate tracked child process and its process group if alive
  if [ -n "${CURRENT_CHILD_PID:-}" ] && kill -0 "$CURRENT_CHILD_PID" 2>/dev/null; then
    echo "[apollo-test] Terminating supervised child process group ${CURRENT_CHILD_PID}..." >&2
    kill -TERM -- "-$CURRENT_CHILD_PID" 2>/dev/null || kill -TERM "$CURRENT_CHILD_PID" 2>/dev/null || true
    sleep 1
    kill -KILL -- "-$CURRENT_CHILD_PID" 2>/dev/null || kill -KILL "$CURRENT_CHILD_PID" 2>/dev/null || true
    wait "$CURRENT_CHILD_PID" 2>/dev/null || true
    CURRENT_CHILD_PID=""
  fi

  # 2. Scoped Docker Compose cleanup if project was initialized
  if [ -n "${PROJECT_NAME:-}" ] && [ -n "${RUN_DIR:-}" ]; then
    if [ -f "${RUN_DIR}/ownership.json" ]; then
      # If run failed or was interrupted, capture diagnostic logs first (bounded)
      if [ "$exit_code" -ne 0 ]; then
        echo "[apollo-test] Capturing diagnostic logs before teardown..." >&2
        mkdir -p "${RUN_DIR}/logs"
        run_with_timeout 15 docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" ps -a > "${RUN_DIR}/logs/compose-ps.txt" 2>&1 || true
        run_with_timeout 30 docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" logs > "${RUN_DIR}/logs/compose-services.log" 2>&1 || true
      fi

      echo "[apollo-test] Tearing down Compose project ${PROJECT_NAME}..." >&2
      run_with_timeout 30 docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" down --volumes --remove-orphans >/dev/null 2>&1 || true
    fi
  fi

  if [ "$exit_code" -ne 0 ]; then
    echo "[apollo-test] Run aborted/failed with status ${exit_code}." >&2
    if [ -n "${RUN_DIR:-}" ] && [ -d "${RUN_DIR}" ]; then
      echo "[apollo-test] Diagnostics preserved at: ${RUN_DIR}" >&2
    fi
  else
    echo "[apollo-test] Run completed successfully."
    if [ -n "${RUN_DIR:-}" ] && [ -d "${RUN_DIR}" ]; then
      echo "[apollo-test] Artifacts preserved at: ${RUN_DIR}"
    fi
  fi

  return $exit_code
}

handle_signal() {
  local sig="$1"
  echo "[apollo-test] Received signal ${sig}, aborting..." >&2
  if [ "$sig" = "SIGINT" ]; then
    INTERRUPTED_STATUS=130
    cleanup
    exit 130
  else
    INTERRUPTED_STATUS=143
    cleanup
    exit 143
  fi
}

trap 'handle_signal SIGINT' INT
trap 'handle_signal SIGTERM' TERM
trap cleanup EXIT

run_recovery_cleanup() {
  local target_dir="$1"
  if [ ! -d "$target_dir" ]; then
    echo "ERROR: Run directory does not exist: ${target_dir}" >&2
    exit 1
  fi
  local ownership_file="${target_dir}/ownership.json"
  if [ ! -f "$ownership_file" ]; then
    echo "ERROR: Safety check failed: '${target_dir}' does not contain 'ownership.json'. Refusing cleanup." >&2
    exit 1
  fi

  local proj
  proj=$(node -e "const fs=require('fs'); try { const d=JSON.parse(fs.readFileSync(process.argv[1],'utf8')); console.log(d.project || ''); } catch { process.exit(1); }" "$ownership_file" 2>/dev/null || true)

  if [ -z "$proj" ]; then
    echo "ERROR: Safety check failed: Failed to extract 'project' from '${ownership_file}'." >&2
    exit 1
  fi

  case "$proj" in
    apollo-test-*)
      ;;
    *)
      echo "ERROR: Safety check failed: Project '${proj}' does not start with 'apollo-test-'. Refusing cleanup." >&2
      exit 1
      ;;
  esac

  echo "[apollo-test] Verified ownership for project '${proj}'. Tearing down Docker Compose resources..."
  docker compose -f "${COMPOSE_FILE}" -p "$proj" down --volumes --remove-orphans
  echo "[apollo-test] Recovery cleanup complete for project ${proj} (${target_dir})."
  exit 0
}

run_fast_checks() {
  echo "[apollo-test] Running fast checks (Clippy, unit tests, doc tests, wasm unit tests)..."
  echo "[apollo-test] NOTICE: Real Apollo integration tests (Docker) are explicitly excluded in fast mode."
  if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: 'cargo' is required but not found in PATH." >&2
    exit 1
  fi
  if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "ERROR: 'wasm-pack' is required but not found in PATH." >&2
    exit 1
  fi

  cargo clippy --all-targets -- -D warnings
  cargo clippy --no-default-features --features rustls --all-targets -- -D warnings
  cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings
  RUST_LOG=apollo_rust_client=trace cargo test --all-targets -- --nocapture
  cargo test --no-default-features --features rustls --lib -- --nocapture
  cargo test --doc
  RUST_BACKTRACE=1 wasm-pack test --node --lib -- --nocapture
  echo "[apollo-test] Fast checks completed successfully."
}

# Parse mode and options
if [ $# -gt 0 ] && [[ "$1" != -* ]]; then
  MODE="$1"
  shift
fi

case "$MODE" in
  all|fast|integration|cleanup)
    ;;
  -h|--help|help)
    print_usage
    exit 0
    ;;
  *)
    echo "ERROR: Unknown mode '$MODE'. Supported modes: all, fast, integration, cleanup" >&2
    print_usage
    exit 1
    ;;
esac

while [ $# -gt 0 ]; do
  case "$1" in
    --suite)
      if [ $# -lt 2 ]; then
        echo "ERROR: --suite requires an argument (<native|rustls|wasm>)" >&2
        exit 1
      fi
      SUITE="$2"
      shift 2
      case "$SUITE" in
        native|rustls|wasm)
          ;;
        *)
          echo "ERROR: Invalid suite '$SUITE'. Supported suites: native, rustls, wasm" >&2
          exit 1
          ;;
      esac
      ;;
    --filter)
      if [ $# -lt 2 ]; then
        echo "ERROR: --filter requires a pattern argument" >&2
        exit 1
      fi
      FILTER="$2"
      shift 2
      ;;
    --run-dir)
      if [ $# -lt 2 ]; then
        echo "ERROR: --run-dir requires a directory path" >&2
        exit 1
      fi
      RECOVERY_RUN_DIR="$2"
      shift 2
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      echo "ERROR: Unknown option '$1'." >&2
      print_usage
      exit 1
      ;;
  esac
done

if [ "$MODE" = "fast" ]; then
  if [ -n "$SUITE" ] || [ -n "$FILTER" ] || [ -n "$RECOVERY_RUN_DIR" ]; then
    echo "ERROR: 'fast' mode does not accept --suite, --filter, or --run-dir options." >&2
    exit 1
  fi
  run_fast_checks
  exit 0
fi

if [ "$MODE" = "cleanup" ]; then
  if [ -z "$RECOVERY_RUN_DIR" ]; then
    echo "ERROR: 'cleanup' mode requires --run-dir <directory>" >&2
    exit 1
  fi
  run_recovery_cleanup "$RECOVERY_RUN_DIR"
fi

if [ -n "$RECOVERY_RUN_DIR" ]; then
  echo "ERROR: --run-dir is only supported in 'cleanup' mode." >&2
  exit 1
fi

# Preflight checks for integration mode
echo "[apollo-test] Performing tool preflight checks..."
if ! command -v docker >/dev/null 2>&1; then
  echo "ERROR: 'docker' is required for integration tests but not found in PATH." >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "ERROR: Docker daemon is not running or not accessible." >&2
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "ERROR: 'docker compose' (v2 plugin) is required but not found." >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "ERROR: 'node' is required for integration fixtures but not found in PATH." >&2
  exit 1
fi

NODE_MAJOR=$(node -v 2>/dev/null | sed 's/^v//' | awk -F. '{print $1}')
if [ -n "$NODE_MAJOR" ] && [ "$NODE_MAJOR" -lt 18 ]; then
  echo "ERROR: Node.js version 18 or higher is required (found $(node -v))." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: 'cargo' is required but not found in PATH." >&2
  exit 1
fi

if [ -z "$SUITE" ] || [ "$SUITE" = "wasm" ]; then
  if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "ERROR: 'wasm-pack' is required for WASM integration suite but not found in PATH." >&2
    exit 1
  fi
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "ERROR: 'curl' is required for health checking but not found in PATH." >&2
  exit 1
fi

# If mode is 'all', execute fast checks first
if [ "$MODE" = "all" ]; then
  run_fast_checks
  echo "[apollo-test] Fast checks passed. Proceeding to integration test lifecycle..."
fi

# Pre-build integration test artifacts and validate filter before spinning up Docker
if [ -n "$FILTER" ] && [ "$SUITE" != "wasm" ]; then
  echo "[apollo-test] Validating test filter '${FILTER}' before starting services..."
  FILTER_MATCHES=$(cargo test --test apollo_integration -- --list --ignored "$FILTER" 2>/dev/null | grep ': test$' || true)
  if [ -z "$FILTER_MATCHES" ]; then
    echo "ERROR: Test filter '${FILTER}' matched zero tests in apollo_integration suite." >&2
    echo "Available integration tests:" >&2
    cargo test --test apollo_integration -- --list --ignored 2>/dev/null | grep ': test$' >&2 || true
    exit 1
  fi
  echo "[apollo-test] Filter '${FILTER}' matches:"
  echo "$FILTER_MATCHES" | sed 's/^/  - /'
fi

if [ -z "$SUITE" ] || [ "$SUITE" = "native" ]; then
  echo "[apollo-test] Pre-building native integration test binaries..."
  cargo test --test apollo_integration --no-run
fi

if [ -z "$SUITE" ] || [ "$SUITE" = "rustls" ]; then
  echo "[apollo-test] Pre-building rustls integration test binaries..."
  cargo test --no-default-features --features rustls --test apollo_integration --no-run
fi

# Initialize unique project and run directory
TIMESTAMP="$(date +%s)"
RAND_SUFFIX="$(od -vAn -N4 -tx1 /dev/urandom 2>/dev/null | tr -d ' \n' || head -c 8 /dev/urandom | od -tx1 -An | tr -d ' \n' | head -c 8)"
PROJECT_NAME="apollo-test-${TIMESTAMP}-${RAND_SUFFIX}"
RUN_ID="run-${TIMESTAMP}-${RAND_SUFFIX}"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/${PROJECT_NAME}.XXXXXX")"
mkdir -p "${RUN_DIR}/logs" "${RUN_DIR}/wasm" "${RUN_DIR}/cache"

cat > "${RUN_DIR}/ownership.json" <<EOF
{
  "schemaVersion": 1,
  "project": "${PROJECT_NAME}",
  "runId": "${RUN_ID}",
  "runDir": "${RUN_DIR}",
  "createdAt": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "composeFile": "${COMPOSE_FILE}"
}
EOF

echo "[apollo-test] Initialized run directory: ${RUN_DIR}"
echo "[apollo-test] Compose project: ${PROJECT_NAME}"

# Verify pinned container images (attempt pull, fallback to verifying local cache if registry has transient error)
echo "[apollo-test] Verifying pinned container images..."
if ! run_with_timeout 600 docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" pull --quiet 2>/dev/null; then
  echo "[apollo-test] Remote registry pull unavailable or failed; checking local cached images..."
  for img in \
    "mysql:8.4.11@sha256:b3b90af2a6552ae30c266fdb7d5dd55f3afb72404bb78d37fe8a23eb857fd3fb" \
    "apolloconfig/apollo-configservice:2.5.2@sha256:a5e4bb5755688fdfc77418e3e34c87095ecb9cf36c3618d05c838bec21f008e8" \
    "apolloconfig/apollo-adminservice:2.5.2@sha256:a7884c10d3fdef2a79c03f3d069fc10843837c9dec6e566a9d68a86d3db0dd95"; do
    if ! docker image inspect "$img" >/dev/null 2>&1; then
      echo "ERROR: Image $img is not available locally and pull failed." >&2
      exit 1
    fi
  done
  echo "[apollo-test] All pinned images verified in local cache."
fi

# Start Compose services (outer timeout 330s leaves room for inner --wait-timeout 300s to emit diagnostics)
echo "[apollo-test] Starting Apollo services in project ${PROJECT_NAME}..."
run_with_timeout 330 docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" up -d --wait --wait-timeout 300

# Discover dynamic loopback ports with safe fallback
CONFIG_PORT=$(docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" port configservice 8080 2>/dev/null | awk -F: '{print $NF}' || true)
ADMIN_PORT=$(docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" port adminservice 8090 2>/dev/null | awk -F: '{print $NF}' || true)

if [ -z "$CONFIG_PORT" ] || [ -z "$ADMIN_PORT" ]; then
  echo "ERROR: Failed to discover dynamically published ports for ConfigService or AdminService." >&2
  exit 1
fi

CONFIG_URL="http://127.0.0.1:${CONFIG_PORT}"
ADMIN_URL="http://127.0.0.1:${ADMIN_PORT}"
echo "[apollo-test] Discovered ConfigService URL: ${CONFIG_URL}"
echo "[apollo-test] Discovered AdminService URL: ${ADMIN_URL}"

# Seed declarative fixtures
echo "[apollo-test] Seeding declarative fixtures via AdminService..."
run_with_timeout 300 node "${FIXTURES_SCRIPT}" seed \
  --admin-url "${ADMIN_URL}" \
  --config-url "${CONFIG_URL}" \
  --run-id "${RUN_ID}" \
  --fixtures "${FIXTURES_FILE}" \
  --state-file "${RUN_DIR}/state.json" > "${RUN_DIR}/logs/seed.log" 2>&1

# Verify published fixtures
echo "[apollo-test] Verifying fixture publication & controls via ConfigService..."
run_with_timeout 300 node "${FIXTURES_SCRIPT}" verify \
  --admin-url "${ADMIN_URL}" \
  --config-url "${CONFIG_URL}" \
  --run-id "${RUN_ID}" \
  --fixtures "${FIXTURES_FILE}" > "${RUN_DIR}/logs/verify.log" 2>&1

# Check seed idempotency
echo "[apollo-test] Checking fixture seed idempotency..."
run_with_timeout 300 node "${FIXTURES_SCRIPT}" seed \
  --admin-url "${ADMIN_URL}" \
  --config-url "${CONFIG_URL}" \
  --run-id "${RUN_ID}" \
  --fixtures "${FIXTURES_FILE}" \
  --state-file "${RUN_DIR}/state-second.json" >> "${RUN_DIR}/logs/seed.log" 2>&1

# Verify state comparison between pass 1 and pass 2 (ignoring execution timestamp)
if ! node -e "
  const fs = require('fs');
  const s1 = JSON.parse(fs.readFileSync(process.argv[1], 'utf8'));
  const s2 = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  delete s1.timestamp;
  delete s2.timestamp;
  if (JSON.stringify(s1) !== JSON.stringify(s2)) {
    console.error('State differed between pass 1 and pass 2');
    process.exit(1);
  }
" "${RUN_DIR}/state.json" "${RUN_DIR}/state-second.json" > "${RUN_DIR}/logs/idempotency.err" 2>&1; then
  echo "ERROR: Fixture seed is not idempotent! State differed between pass 1 and pass 2." >&2
  diff -u "${RUN_DIR}/state.json" "${RUN_DIR}/state-second.json" >&2 || true
  STAGE_FAILED=1
else
  echo "[apollo-test] Fixture seed verified 100% idempotent."
fi

# Export environment variables for integration tests
export APOLLO_TEST_RUN_DIR="${RUN_DIR}"
export APOLLO_TEST_RUN_ID="${RUN_ID}"
export APOLLO_TEST_CONFIG_URL="${CONFIG_URL}"
export APOLLO_TEST_ADMIN_URL="${ADMIN_URL}"
export APOLLO_TEST_WASM_PACKAGE="${RUN_DIR}/wasm"

# Determine target suites
TARGET_SUITES=()
if [ -n "$SUITE" ]; then
  TARGET_SUITES=("$SUITE")
else
  TARGET_SUITES=("native" "rustls" "wasm")
fi

echo "[apollo-test] Executing selected integration test suites: ${TARGET_SUITES[*]}"

for s in "${TARGET_SUITES[@]}"; do
  echo "[apollo-test] --- Suite: $s ---"
  export APOLLO_TEST_SUITE="$s"

  if [ "$s" = "native" ]; then
    NATIVE_TEST_FILE="${REPO_ROOT}/tests/apollo_integration.rs"
    if [ ! -f "$NATIVE_TEST_FILE" ]; then
      echo "ERROR [suite: native]: Test executor file 'tests/apollo_integration.rs' does not exist yet. Missing required suite executor." >&2
      echo "missing_executor: tests/apollo_integration.rs" >> "${RUN_DIR}/logs/suite-native.log"
      STAGE_FAILED=1
      continue
    fi

    CMD=(cargo test --test apollo_integration -- --ignored --nocapture --test-threads=1)
    if [ -n "$FILTER" ]; then
      CMD+=("$FILTER")
    fi
    echo "[apollo-test] Running: ${CMD[*]}"
    TEST_OUTPUT_FILE="${RUN_DIR}/logs/suite-native.log"
    REQUIRED_NATIVE_CASES=(
      "real_apollo_formats_and_identity"
      "real_apollo_access_key"
      "real_apollo_grayscale"
      "real_apollo_release_refresh_and_listener"
      "real_apollo_polling"
      "real_apollo_preload_and_persistence"
    )
    if run_with_timeout 300 "${CMD[@]}" > "$TEST_OUTPUT_FILE" 2>&1; then
      cat "$TEST_OUTPUT_FILE"
      if grep -q "running 0 tests" "$TEST_OUTPUT_FILE"; then
        echo "ERROR [suite: native]: Zero tests were executed (filter='$FILTER'). Suite cannot pass with 0 tests." >&2
        STAGE_FAILED=1
      fi
      if [ -z "$FILTER" ]; then
        for rc in "${REQUIRED_NATIVE_CASES[@]}"; do
          if ! grep -q "test ${rc} \.\.\. ok" "$TEST_OUTPUT_FILE"; then
            echo "ERROR [suite: native]: Missing or failed required integration case '${rc}' in full run." >&2
            STAGE_FAILED=1
          fi
        done
      fi
    else
      cat "$TEST_OUTPUT_FILE"
      STAGE_FAILED=1
    fi

  elif [ "$s" = "rustls" ]; then
    RUSTLS_TEST_FILE="${REPO_ROOT}/tests/apollo_integration.rs"
    if [ ! -f "$RUSTLS_TEST_FILE" ]; then
      echo "ERROR [suite: rustls]: Test executor file 'tests/apollo_integration.rs' does not exist yet. Missing required suite executor." >&2
      echo "missing_executor: tests/apollo_integration.rs" >> "${RUN_DIR}/logs/suite-rustls.log"
      STAGE_FAILED=1
      continue
    fi

    CMD=(cargo test --no-default-features --features rustls --test apollo_integration -- --ignored --nocapture --test-threads=1)
    if [ -n "$FILTER" ]; then
      CMD+=("$FILTER")
    fi
    echo "[apollo-test] Running: ${CMD[*]}"
    TEST_OUTPUT_FILE="${RUN_DIR}/logs/suite-rustls.log"
    REQUIRED_RUSTLS_CASES=(
      "real_apollo_formats_and_identity"
      "real_apollo_access_key"
      "real_apollo_grayscale"
      "real_apollo_release_refresh_and_listener"
      "real_apollo_polling"
      "real_apollo_preload_and_persistence"
    )
    if run_with_timeout 300 "${CMD[@]}" > "$TEST_OUTPUT_FILE" 2>&1; then
      cat "$TEST_OUTPUT_FILE"
      if grep -q "running 0 tests" "$TEST_OUTPUT_FILE"; then
        echo "ERROR [suite: rustls]: Zero tests were executed (filter='$FILTER'). Suite cannot pass with 0 tests." >&2
        STAGE_FAILED=1
      fi
      if [ -z "$FILTER" ]; then
        for rc in "${REQUIRED_RUSTLS_CASES[@]}"; do
          if ! grep -q "test ${rc} \.\.\. ok" "$TEST_OUTPUT_FILE"; then
            echo "ERROR [suite: rustls]: Missing or failed required integration case '${rc}' in full run." >&2
            STAGE_FAILED=1
          fi
        done
      fi
    else
      cat "$TEST_OUTPUT_FILE"
      STAGE_FAILED=1
    fi

  elif [ "$s" = "wasm" ]; then
    WASM_TEST_FILE="${REPO_ROOT}/tests/apollo/wasm.cjs"
    if [ ! -f "$WASM_TEST_FILE" ]; then
      echo "ERROR [suite: wasm]: Test executor file 'tests/apollo/wasm.cjs' does not exist yet. Missing required suite executor." >&2
      echo "missing_executor: tests/apollo/wasm.cjs" >> "${RUN_DIR}/logs/suite-wasm.log"
      STAGE_FAILED=1
      continue
    fi

    echo "[apollo-test] Building WASM package for Node.js in ${RUN_DIR}/wasm..."
    export APOLLO_TEST_WASM_PACKAGE="${RUN_DIR}/wasm"
    if ! run_with_timeout 300 wasm-pack build --target nodejs --dev --out-dir "${RUN_DIR}/wasm" > "${RUN_DIR}/logs/wasm-build.log" 2>&1; then
      echo "ERROR [suite: wasm]: Failed to build WASM package for Node.js." >&2
      STAGE_FAILED=1
      continue
    fi

    WASM_CMD=(node --test --test-concurrency=1)
    if [ -n "$FILTER" ]; then
      WASM_CMD+=(--test-name-pattern="$FILTER")
    fi
    WASM_CMD+=("${WASM_TEST_FILE}")
    echo "[apollo-test] Running: ${WASM_CMD[*]}"
    TEST_OUTPUT_FILE="${RUN_DIR}/logs/suite-wasm.log"
    REQUIRED_WASM_CASES=(
      "real_apollo_wasm_export_smoke"
      "real_apollo_wasm_formats_and_identity"
      "real_apollo_wasm_access_key"
      "real_apollo_wasm_grayscale"
      "real_apollo_wasm_release_refresh_and_listener"
      "real_apollo_wasm_polling"
      "real_apollo_wasm_preload"
    )
    if run_with_timeout 300 "${WASM_CMD[@]}" > "$TEST_OUTPUT_FILE" 2>&1; then
      cat "$TEST_OUTPUT_FILE"
      if grep -q "pass 0" "$TEST_OUTPUT_FILE" || grep -q "# tests 0" "$TEST_OUTPUT_FILE"; then
        echo "ERROR [suite: wasm]: Zero tests were executed (filter='$FILTER'). Suite cannot pass with 0 tests." >&2
        STAGE_FAILED=1
      fi
      if [ -z "$FILTER" ]; then
        for rc in "${REQUIRED_WASM_CASES[@]}"; do
          if ! grep -q "${rc}" "$TEST_OUTPUT_FILE"; then
            echo "ERROR [suite: wasm]: Missing or failed required integration case '${rc}' in full run." >&2
            STAGE_FAILED=1
          fi
        done
      fi
    else
      cat "$TEST_OUTPUT_FILE"
      echo "ERROR [suite: wasm]: Node.js WASM integration tests failed." >&2
      STAGE_FAILED=1
    fi
  fi
done

if [ "$STAGE_FAILED" -ne 0 ]; then
  echo "ERROR: One or more integration suites failed or lacked required executors." >&2
  exit 1
fi

echo "[apollo-test] All requested integration suites passed successfully."
exit 0
