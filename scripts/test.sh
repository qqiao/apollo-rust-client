#!/bin/sh
# Main test entry point for apollo-rust-client.
# Supports 'all' (default), 'fast' (Docker-free), and 'integration' (real Apollo).
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

print_usage() {
  cat <<'EOF'
Usage: scripts/test.sh [mode] [options]

Modes:
  all (default)        Run fast checks (Clippy, unit tests, doc tests, wasm unit tests)
                       followed by all integration test suites
  fast                 Run Clippy, unit tests, doc tests, and WASM unit tests (no Docker needed)
  integration          Run real Apollo integration test suites using Docker Compose

Integration Options:
  --suite <name>       Target suite: native, rustls, or wasm (default: all)
  --filter <pattern>   Filter tests by pattern
  -h, --help           Show this help message
EOF
}

run_fast_checks() {
  echo "[test.sh] Running fast checks (Clippy, unit tests, doc tests, wasm unit tests)..."
  cargo clippy --all-targets -- -D warnings && \
  cargo clippy --no-default-features --features rustls --all-targets -- -D warnings && \
  cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings && \
  RUST_LOG=apollo_rust_client=trace cargo test --all-targets -- --nocapture && \
  cargo test --doc && \
  RUST_BACKTRACE=1 wasm-pack test --node --lib -- --nocapture
}

MODE="all"
if [ $# -gt 0 ]; then
  case "$1" in
    all|fast|integration)
      MODE="$1"
      shift
      ;;
    -h|--help|help)
      print_usage
      exit 0
      ;;
    -*)
      # Options passed without explicit mode; default to all
      MODE="all"
      ;;
    *)
      echo "ERROR: Unknown mode '$1'. Supported modes: all, fast, integration" >&2
      print_usage
      exit 1
      ;;
  esac
fi

if [ "$MODE" = "fast" ]; then
  for arg in "$@"; do
    case "$arg" in
      -h|--help|help)
        print_usage
        exit 0
        ;;
    esac
  done
  if [ $# -gt 0 ]; then
    echo "ERROR: 'fast' mode does not accept additional options ($*)." >&2
    exit 1
  fi
  run_fast_checks
  exit 0
fi

validate_options() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --suite)
        if [ "$#" -lt 2 ]; then
          echo "ERROR: --suite requires an argument (<native|rustls|wasm>)" >&2
          exit 1
        fi
        case "$2" in
          native|rustls|wasm)
            ;;
          *)
            echo "ERROR: Invalid suite '$2'. Supported suites: native, rustls, wasm" >&2
            exit 1
            ;;
        esac
        shift 2
        ;;
      --filter)
        if [ "$#" -lt 2 ]; then
          echo "ERROR: --filter requires a pattern argument" >&2
          exit 1
        fi
        shift 2
        ;;
      -h|--help|help)
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
}

validate_options "$@"

if [ "$MODE" = "integration" ]; then
  exec "${SCRIPT_DIR}/apollo-test.sh" integration "$@"
elif [ "$MODE" = "all" ]; then
  run_fast_checks
  echo "[test.sh] Fast checks passed. Dispatching to integration tests..."
  exec "${SCRIPT_DIR}/apollo-test.sh" integration "$@"
fi
