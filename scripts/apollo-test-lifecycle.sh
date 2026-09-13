#!/usr/bin/env bash
# Apollo Integration Test Lifecycle Supervision & Teardown Helper
# Provides process-group supervision, bounded teardown, diagnostic capture,
# and deterministic exit status precedence.
# Compatible with Bash 3.2 and `set -e`.

CURRENT_CHILD_PID=""
CLEANED_UP=0
CACHED_FINAL_STATUS=""
INTERRUPTED_STATUS=0
DIAGNOSTIC_FAILURE=0
LIFECYCLE_PHASE="work"
CLEANUP_IN_PROGRESS=0

run_with_timeout() {
  local timeout_sec="$1"
  shift

  local child_pid=""
  local spawn_interrupted=0
  if [ "${LIFECYCLE_PHASE:-work}" = "work" ] || [ "${LIFECYCLE_PHASE:-}" = "spawning" ]; then
    LIFECYCLE_PHASE="spawning"
    set -m
    "$@" &
    child_pid=$!
    if [ -n "${LIFECYCLE_SPAWN_HOOK:-}" ]; then
      eval "$LIFECYCLE_SPAWN_HOOK"
    fi
    set +m
    CURRENT_CHILD_PID="$child_pid"
    if [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then
      spawn_interrupted=1
    fi
    LIFECYCLE_PHASE="work"
    if [ "$spawn_interrupted" -eq 1 ]; then
      echo "[apollo-test] Pending interrupt (${INTERRUPTED_STATUS}) handled after process registration." >&2
      exit "${INTERRUPTED_STATUS}"
    fi
  else
    set -m
    "$@" &
    child_pid=$!
    set +m
    CURRENT_CHILD_PID="$child_pid"
  fi

  local elapsed=0
  while kill -0 "$child_pid" 2>/dev/null || kill -0 -- "-$child_pid" 2>/dev/null; do
    if [ "$elapsed" -ge "$timeout_sec" ]; then
      echo "ERROR: Command timed out after ${timeout_sec}s: $*" >&2
      kill -TERM -- "-$child_pid" 2>/dev/null || kill -TERM "$child_pid" 2>/dev/null || true
      sleep 2 || true
      kill -KILL -- "-$child_pid" 2>/dev/null || kill -KILL "$child_pid" 2>/dev/null || true
      wait "$child_pid" 2>/dev/null || true
      CURRENT_CHILD_PID=""
      return 124
    fi
    # If the leader exited but descendants in the group are still alive, terminate them
    if ! kill -0 "$child_pid" 2>/dev/null && kill -0 -- "-$child_pid" 2>/dev/null; then
      kill -TERM -- "-$child_pid" 2>/dev/null || true
      sleep 1 || true
      kill -KILL -- "-$child_pid" 2>/dev/null || true
      break
    fi
    sleep 1 || true
    elapsed=$((elapsed + 1))
  done

  local status=0
  if wait "$child_pid" 2>/dev/null; then
    status=0
  else
    status=$?
  fi
  # Extra safety: ensure entire process group is reaped
  if kill -0 -- "-$child_pid" 2>/dev/null; then
    kill -KILL -- "-$child_pid" 2>/dev/null || true
  fi
  CURRENT_CHILD_PID=""
  return "$status"
}

teardown_project() {
  local project="$1"
  local run_dir="$2"
  local timeout_seconds="${3:-${TEARDOWN_TIMEOUT:-30}}"
  local compose_file="${COMPOSE_FILE:-}"

  local logs_dir="${run_dir}/logs"
  local log_file="${logs_dir}/teardown.log"
  local log_failed=0

  if ! mkdir -p "$logs_dir" 2>/dev/null || ! : >> "$log_file" 2>/dev/null; then
    echo "[apollo-test] WARNING: Failed to create or write teardown log '${log_file}'; falling back to direct output." >&2
    log_failed=1
    DIAGNOSTIC_FAILURE=1
  fi

  local td_status=0
  if [ "$log_failed" -eq 0 ]; then
    if run_with_timeout "$timeout_seconds" docker compose -f "$compose_file" -p "$project" down --volumes --remove-orphans >> "$log_file" 2>&1; then
      td_status=0
    else
      td_status=$?
    fi
  else
    if run_with_timeout "$timeout_seconds" docker compose -f "$compose_file" -p "$project" down --volumes --remove-orphans; then
      td_status=0
    else
      td_status=$?
    fi
  fi

  if [ "$td_status" -ne 0 ]; then
    echo "[apollo-test] ERROR: Teardown failed for Compose project '${project}' with status ${td_status}." >&2
    echo "[apollo-test] Run directory: ${run_dir}" >&2
    if [ "$log_failed" -eq 0 ] && [ -f "$log_file" ]; then
      echo "[apollo-test] Teardown log: ${log_file}" >&2
    fi
    local escaped_run_dir
    escaped_run_dir=$(printf '%q' "$run_dir")
    echo "[apollo-test] To retry cleanup, run: scripts/apollo-test.sh cleanup --run-dir ${escaped_run_dir}" >&2
    return "$td_status"
  fi

  if [ "$log_failed" -ne 0 ]; then
    return 1
  fi

  return 0
}

cleanup() {
  local original_status="${1:-$?}"

  if [ "${LIFECYCLE_PHASE:-}" = "finished" ] || [ "${CLEANED_UP:-0}" -eq 1 ]; then
    return "${CACHED_FINAL_STATUS:-0}"
  fi
  if [ "${CLEANUP_IN_PROGRESS:-0}" -eq 1 ]; then
    return 0
  fi
  CLEANUP_IN_PROGRESS=1
  LIFECYCLE_PHASE="cleanup"

  # 1. Terminate tracked child process group if alive
  if [ -n "${CURRENT_CHILD_PID:-}" ] && ( kill -0 "$CURRENT_CHILD_PID" 2>/dev/null || kill -0 -- "-$CURRENT_CHILD_PID" 2>/dev/null ); then
    echo "[apollo-test] Terminating supervised child process group ${CURRENT_CHILD_PID}..." >&2
    kill -TERM -- "-$CURRENT_CHILD_PID" 2>/dev/null || kill -TERM "$CURRENT_CHILD_PID" 2>/dev/null || true
    sleep 1 || true
    kill -KILL -- "-$CURRENT_CHILD_PID" 2>/dev/null || kill -KILL "$CURRENT_CHILD_PID" 2>/dev/null || true
    wait "$CURRENT_CHILD_PID" 2>/dev/null || true
    CURRENT_CHILD_PID=""
  fi

  local teardown_status=0
  # 2. Scoped Docker Compose cleanup if project was initialized and ownership exists
  if [ -n "${PROJECT_NAME:-}" ] && [ -n "${RUN_DIR:-}" ]; then
    if [ -f "${RUN_DIR}/ownership.json" ]; then
      # If run failed or was interrupted, capture diagnostic logs first (bounded)
      if [ "$original_status" -ne 0 ] || [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then
        echo "[apollo-test] Capturing diagnostic logs before teardown..." >&2
        mkdir -p "${RUN_DIR}/logs" 2>/dev/null || true
        run_with_timeout 15 docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" ps -a > "${RUN_DIR}/logs/compose-ps.txt" 2>&1 || true
        run_with_timeout 30 docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" logs > "${RUN_DIR}/logs/compose-services.log" 2>&1 || true
      fi

      echo "[apollo-test] Tearing down Compose project ${PROJECT_NAME}..." >&2
      local td_timeout="${TEARDOWN_TIMEOUT:-30}"
      if teardown_project "${PROJECT_NAME}" "${RUN_DIR}" "$td_timeout"; then
        teardown_status=0
      else
        teardown_status=$?
      fi
    fi
  fi

  # Exit status precedence (FR-002 / Status Table):
  # 1. Catchable signal status (130 / 143)
  # 2. Original nonzero stage status
  # 3. Nonzero teardown status
  # 4. Diagnostic capture failure status (1)
  # 5. 0 (success)
  local final_status=0
  if [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then
    final_status="${INTERRUPTED_STATUS}"
  elif [ "$original_status" -ne 0 ]; then
    final_status="$original_status"
  elif [ "$teardown_status" -ne 0 ]; then
    final_status="$teardown_status"
  elif [ "${DIAGNOSTIC_FAILURE:-0}" -ne 0 ]; then
    final_status=1
  else
    final_status=0
  fi

  CLEANED_UP=1
  LIFECYCLE_PHASE="finished"
  CLEANUP_IN_PROGRESS=0
  if [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then
    final_status="${INTERRUPTED_STATUS}"
  fi
  CACHED_FINAL_STATUS="$final_status"

  if [ "$final_status" -ne 0 ]; then
    echo "[apollo-test] Run aborted/failed with status ${final_status}." >&2
    if [ -n "${RUN_DIR:-}" ] && [ -d "${RUN_DIR}" ]; then
      echo "[apollo-test] Diagnostics preserved at: ${RUN_DIR}" >&2
    fi
  else
    echo "[apollo-test] Run completed successfully."
    if [ -n "${RUN_DIR:-}" ] && [ -d "${RUN_DIR}" ]; then
      echo "[apollo-test] Artifacts preserved at: ${RUN_DIR}"
    fi
  fi

  return "$final_status"
}

handle_signal() {
  local sig="$1"
  local sig_status=143
  if [ "$sig" = "SIGINT" ]; then
    sig_status=130
  fi

  # First signal wins: only record if INTERRUPTED_STATUS is not set yet
  if [ "${INTERRUPTED_STATUS:-0}" -eq 0 ]; then
    INTERRUPTED_STATUS="$sig_status"
  fi

  if [ "${LIFECYCLE_PHASE:-work}" = "work" ]; then
    echo "[apollo-test] Received signal ${sig}, aborting..." >&2
    LIFECYCLE_PHASE="cleanup"
    exit "$sig_status"
  elif [ "${LIFECYCLE_PHASE:-}" = "finished" ]; then
    echo "[apollo-test] Signal ${sig} received during terminal phase, exiting with status ${INTERRUPTED_STATUS}." >&2
    exit "${INTERRUPTED_STATUS}"
  else
    echo "[apollo-test] Signal ${sig} deferred: bounded cleanup already in progress (phase: ${LIFECYCLE_PHASE})." >&2
    return 0
  fi
}

lifecycle_exit_handler() {
  local exit_code=$?
  LIFECYCLE_PHASE="cleanup"
  cleanup "$exit_code"
  local final_status=$?
  LIFECYCLE_PHASE="finished"
  trap - EXIT
  exit "$final_status"
}

install_lifecycle_traps() {
  trap 'handle_signal SIGINT' INT
  trap 'handle_signal SIGTERM' TERM
  trap lifecycle_exit_handler EXIT
}

run_recovery_cleanup() {
  local target_dir="$1"
  local timeout_sec="${2:-${TEARDOWN_TIMEOUT:-30}}"

  LIFECYCLE_PHASE="cleanup"

  if [ ! -d "$target_dir" ]; then
    echo "[apollo-test] Run directory does not exist or was already removed: ${target_dir}. Fallback cleanup is safe."
    LIFECYCLE_PHASE="finished"
    return 0
  fi

  local ownership_file="${target_dir}/ownership.json"
  if [ ! -f "$ownership_file" ]; then
    echo "ERROR: Safety check failed: '${target_dir}' does not contain 'ownership.json'. Refusing cleanup." >&2
    LIFECYCLE_PHASE="finished"
    return 1
  fi

  local proj
  proj=$(node -e "const fs=require('fs'); try { const d=JSON.parse(fs.readFileSync(process.argv[1],'utf8')); if (typeof d === 'object' && d !== null && typeof d.project === 'string') { console.log(d.project); } else { process.exit(1); } } catch { process.exit(1); }" "$ownership_file" 2>/dev/null || true)

  if [ -z "$proj" ]; then
    echo "ERROR: Safety check failed: Failed to extract 'project' from '${ownership_file}'." >&2
    LIFECYCLE_PHASE="finished"
    return 1
  fi

  case "$proj" in
    apollo-test-*)
      ;;
    *)
      echo "ERROR: Safety check failed: Project '${proj}' does not start with 'apollo-test-'. Refusing cleanup." >&2
      LIFECYCLE_PHASE="finished"
      return 1
      ;;
  esac

  echo "[apollo-test] Verified ownership for project '${proj}'. Tearing down Docker Compose resources..."
  local td_status=0
  if teardown_project "$proj" "$target_dir" "$timeout_sec"; then
    td_status=0
  else
    td_status=$?
  fi

  LIFECYCLE_PHASE="finished"
  local final_status=0
  if [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then
    final_status="${INTERRUPTED_STATUS}"
  elif [ "$td_status" -ne 0 ]; then
    final_status="$td_status"
  elif [ "${DIAGNOSTIC_FAILURE:-0}" -ne 0 ]; then
    final_status=1
  else
    final_status=0
  fi

  if [ "$final_status" -eq 0 ]; then
    echo "[apollo-test] Recovery cleanup complete for project ${proj} (${target_dir})."
  else
    echo "[apollo-test] Recovery cleanup failed for project ${proj} (${target_dir}) with status ${final_status}." >&2
  fi

  # Preserve any signal arriving during or after final logging: first recorded signal wins
  if [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then
    final_status="${INTERRUPTED_STATUS}"
  fi
  return "$final_status"
}
