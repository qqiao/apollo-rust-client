# Technical Plan: GitHub Copilot Review Remediation

**Feature ID:** `014-copilot-remediation`  
**Status:** Implemented and verified (including Leader review remediation).
**Baseline documents:** [spec.md](spec.md), [tasks.md](tasks.md).

---

## 1. Failure Mechanisms & Component Architecture

### Component 1: Fixture Timeout Diagnostic URL Redaction (`scripts/apollo-fixtures.mjs`)

- **Root Cause (F1):** Line 95 of `scripts/apollo-fixtures.mjs` directly templates `${url}` into the `TimeoutError` message. If the test or caller passes a URL with HTTP basic credentials (`http://admin:pass@host/api`) or sensitive query parameters (`?access_token=xyz&secret=123`), these secrets are emitted in the diagnostic output, violating AC-005 and FR-006 of specification 010.
- **Technical Design:**
  1. Add a dedicated `redactUrl(rawUrl)` helper function:
     ```javascript
     export function redactUrl(rawUrl) {
       try {
         const parsed = new URL(rawUrl);
         // Strip userinfo
         if (parsed.username || parsed.password) {
           parsed.username = '***';
           parsed.password = '***';
         }
         // Redact sensitive query parameters
         const SENSITIVE_PARAM_PATTERN = /^(token|secret|key|password|auth|credential|access_token|client_secret)$/i;
         for (const [key, value] of parsed.searchParams.entries()) {
           if (SENSITIVE_PARAM_PATTERN.test(key)) {
             parsed.searchParams.set(key, 'REDACTED');
           }
         }
         return parsed.toString();
       } catch {
         // Fallback for relative or malformed URLs: basic regex redaction
         return rawUrl
           .replace(/\/\/[^:]+:[^@]+@/, '//***:***@')
           .replace(/([?&](?:token|secret|key|password|auth|credential)=)[^&]*/gi, '$1REDACTED');
       }
     }
     ```
  2. In `requestJson`, format timeout error with `redactUrl(url)`.

---

### Component 2: Markdown Link Checker Robustness (`scripts/check-doc-links.mjs`)

- **Root Cause (F2 — Fence Tracking):** `check-doc-links.mjs` lines 68–71 toggle a single boolean `inCodeBlock` whenever any line starts with 3 backticks or tildes. If an outer code fence uses 4 backticks (e.g. wrapping an example that contains 3 backticks), encountering the inner 3 backticks flips `inCodeBlock` to false prematurely.
  - *Fix:* Adopt CommonMark-compliant fence tracking matching `scripts/check-doc-examples.mjs`: parse opening fence with `parseOpeningFence(line)`, record `currentFenceChar` and `currentFenceLen`, and only close when matching opening character with length $\ge \text{currentFenceLen}$ and no trailing info string.
- **Root Cause (F3 — Angle-Bracket Destinations):** Lines 111–119 strip `<` and `>`, then split at the first space `raw.indexOf(' ')`. In CommonMark, `<destination>` allows spaces inside the destination itself (e.g. text `guide` with destination `<docs/Getting Started.md>`). Truncating at the first space turns `docs/Getting Started.md` into `docs/Getting`, resulting in false broken-link reports.
  - *Fix:* If `raw.startsWith('<')`, search for closing `>` with `raw.indexOf('>')`. Extract the destination exclusively from inside `<...>`, and ignore anything after `>` (optional title). Only apply space-splitting for title removal if the destination was unbracketed.
- **Root Cause (F4 — Protocol-Relative URLs):** Line 122 matches external schemes with `/^(https?:|mailto:|ftp:|data:)/i`. Protocol-relative URLs (e.g. text `docs` with target `//docs.example.com/guide`) start with `//` and have no explicit scheme. They are misidentified as repo-root relative paths (`.//docs.example.com/...`) and reported as broken local files.
  - *Fix:* Extend external regex to `/^(https?:|mailto:|ftp:|data:|\/\/)/i`.

---

### Component 3: Process Group Supervision & Signal Race Elimination (`scripts/apollo-test-lifecycle.sh` & `scripts/apollo-test.sh`)

- **Root Cause (F8 — Spawn Window Race):** In `run_with_timeout`:
  ```bash
  set -m
  "$@" &
  local child_pid=$!
  set +m
  CURRENT_CHILD_PID="$child_pid"
  ```
  If SIGINT or SIGTERM arrives between background spawn (`"$@" &`) and `CURRENT_CHILD_PID="$child_pid"`, `handle_signal` triggers cleanup while `CURRENT_CHILD_PID` is empty. The spawned background process is never registered and remains running unmanaged.
  - *Fix:* Introduce a transient `LIFECYCLE_PHASE="spawning"`. When `handle_signal` receives a signal while in `spawning` phase, it records `INTERRUPTED_STATUS` and defers immediate exit. Immediately after storing `$child_pid` into `CURRENT_CHILD_PID`, the runner transitions phase to `work` and inspects `INTERRUPTED_STATUS`; if non-zero, it directly invokes cleanup to terminate the newly registered process group.
- **Root Cause (F6 — Descendant Process Group Leaks):**
  Lines 26 and 113 probe only `kill -0 "$child_pid"`. If a supervised command spawns background subprocesses in its process group and the leader exits, `kill -0 "$child_pid"` becomes false while child descendants in group `-$child_pid` are still alive.
  - *Fix:* Probe both the leader and the process group: `kill -0 "$child_pid" 2>/dev/null || kill -0 -- "-$child_pid" 2>/dev/null`. During timeout or cleanup, send `kill -TERM -- "-$CURRENT_CHILD_PID"` followed by `kill -KILL -- "-$CURRENT_CHILD_PID"` to terminate all processes in the group.
- **Root Cause (F7 — Recovery Status Race):**
  In `run_recovery_cleanup`:
  `final_status` is computed before `LIFECYCLE_PHASE="finished"` is set. If a signal arrives during the log output or before the phase transition, `INTERRUPTED_STATUS` is updated but the precomputed `final_status` is returned.
  - *Fix:* Transition `LIFECYCLE_PHASE="finished"` and snapshot `final_status` at the return boundary, ensuring `INTERRUPTED_STATUS` outranks all other statuses.
- **Root Cause (F5 — CLI Recovery Exit Status):**
  In `scripts/apollo-test.sh:161-162`:
  ```bash
  run_recovery_cleanup "$RECOVERY_RUN_DIR"
  exit $?
  ```
  If a signal was handled and deferred during recovery, `run_recovery_cleanup` returns. The caller must verify `INTERRUPTED_STATUS` before exit:
  ```bash
  recovery_status=0
  if run_recovery_cleanup "$RECOVERY_RUN_DIR"; then
    recovery_status=0
  else
    recovery_status=$?
  fi
  if [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then
    exit "$INTERRUPTED_STATUS"
  fi
  exit "$recovery_status"
  ```

---

## 2. Deterministic Regression Testing Strategy

### 1. `tests/tooling/apollo-http.test.mjs`
- Test `redactUrl` directly with:
  - Userinfo credentials: `http://user:pass@127.0.0.1:8080/api` $\rightarrow$ `http://***:***@127.0.0.1:8080/api`
  - Query parameters: `http://127.0.0.1:8080/api?token=secret123&client_secret=topsecret&page=1` $\rightarrow$ query parameters redacted to `REDACTED`, non-sensitive params (`page=1`) preserved.
  - Timeout error message test: Verify error message contains redacted URL when request times out.

### 2. `tests/tooling/doc-links.test.mjs`
- Test nested fences: 4-backtick fence containing 3-backtick fence with dummy link syntax (text `nested` target `do-not-extract.md`) is skipped.
- Test angle brackets with spaces: text `guide` target `<docs/Getting Started.md>` resolves correctly to existing file without broken link warning.
- Test protocol-relative URLs: text `cdn` target `//cdn.example.com/lib.js` is recognized as external and excluded.

### 3. `tests/tooling/apollo-lifecycle.test.mjs`
- Test process group descendant termination: Supervised command that forks a background worker (`sh -c 'sleep 10 &'`) and exits leader immediately $\rightarrow$ cleanup terminates process group so background worker is reaped.
- Test spawn race condition: Subshell signal trigger during process launch.
- Test recovery signal preservation: Signal during recovery ensures non-zero exit status (130 or 143).

---

## 3. Verification Protocol

1. Run focused tooling tests:
   ```bash
   node --test tests/tooling/apollo-http.test.mjs
   node --test tests/tooling/doc-links.test.mjs
   node --test tests/tooling/apollo-lifecycle.test.mjs
   ```
2. Run full fast verification:
   ```bash
   scripts/test.sh fast
   ```
3. Run native integration suite (validating Docker lifecycle and teardown):
   ```bash
   scripts/test.sh integration --suite native
   ```
4. Check formatting and diff hygiene:
   ```bash
   git diff --check
   ```

---

## 4. Leader Review Remediation & Defect Resolution (Commit `797285c`)

During independent review of commit `797285c`, four defects and readiness timing sensitivities were identified and addressed:

1. **P1 — URL Objects Bypassing Redaction (`scripts/apollo-fixtures.mjs:54-57`):**
   - *Problem:* `redactUrl` returned non-string inputs unmodified, allowing `URL` instances to leak tokens and query parameters when stringified by `requestJson` timeout messages.
   - *Resolution:* Normalized `rawUrl` to string via `rawUrl instanceof URL ? rawUrl.toString() : String(rawUrl)` before parsing and sanitizing. Covered with unit test `redactUrl handles URL object input and requestJson redacts URL object timeouts (P1)`.

2. **P2 — Stale Recovery Status on Late Signal (`scripts/apollo-test-lifecycle.sh:300-318` & `scripts/apollo-test.sh:160-171`):**
   - *Problem:* `run_recovery_cleanup` snapshotted `final_status` before outputting its completion banner. An interrupt arriving during or after the banner recorded `INTERRUPTED_STATUS` but returned the stale snapshot (0).
   - *Resolution:* Re-evaluated `if [ "${INTERRUPTED_STATUS:-0}" -ne 0 ]; then final_status="${INTERRUPTED_STATUS}"; fi` immediately prior to return, and installed `recovery_exit_handler` trap on `EXIT` in `scripts/apollo-test.sh` cleanup mode. Verified with deterministic tests covering late SIGTERM, late SIGINT, and both signal arrival sequences.

3. **P2 — Spawn Test Process Liveness & Readiness Verification (`tests/tooling/apollo-lifecycle.test.mjs`):**
   - *Problem:* Harness generated literal `$$` inside single quotes, resulting in NaN and bypassing liveness assertion. Optional file checks swallowed missing readiness.
   - *Resolution:* Converted to standalone helper scripts (`worker.sh` and `leader.sh`) that record real integer PIDs, write readiness files, and trigger deterministic signals without string-escaping issues. Replaced optional `.catch(() => {})` with mandatory `waitForFile` and explicit assertions verifying valid positive integer PIDs and `isProcessAlive(pid) === false`.

4. **P2 — Basic Auth Percent-Encoding in Decoded Credentials (`scripts/apollo-fixtures.mjs:89-98`):**
   - *Problem:* `parsed.username` and `parsed.password` in WHATWG `URL` objects are percent-encoded (`p%40ss`), causing HTTP Basic auth to send literal percent-encoded characters instead of decoded characters.
   - *Resolution:* Decoded `decodeURIComponent(parsed.username)` and `decodeURIComponent(parsed.password)` before Base64 encoding. Verified that explicit caller-provided `Authorization` headers retain precedence.

---

## 5. Leader Re-Review Remediation & Hardening (Commit `0273182`)

During independent re-review of commit `0273182`, three required findings were investigated, resolved, and verified:

1. **P2 — Terminal-Phase Signals at Return/Exit Boundaries (`scripts/apollo-test-lifecycle.sh:handle_signal` & `scripts/apollo-test.sh`):**
   - *Problem:* Signals arriving in `finished` phase (e.g. at the `return "$final_status"` boundary in helper or before `exit "$rc"` in CLI) were previously treated as "cleanup in progress" and deferred by `handle_signal`, causing the command to exit with status 0 despite `INTERRUPTED_STATUS` being set to 143/130.
   - *Resolution:* In `handle_signal`, added an explicit check: if `LIFECYCLE_PHASE="finished"`, exit immediately with `exit "${INTERRUPTED_STATUS}"` without deferral. Added regression coverage for helper return boundary, CLI exit boundary, and first-signal precedence (`014/AC-009`, `014/AC-010`).

2. **P2 — Spawn Race Window & Deterministic Handshake Injection (`scripts/apollo-test-lifecycle.sh` & `tests/tooling/apollo-lifecycle.test.mjs`):**
   - *Problem:* The previous spawn test signaled the parent from the child worker script, which could execute after `CURRENT_CHILD_PID` was already assigned, passing even against pre-fix helpers lacking spawn-phase protection.
   - *Resolution:* Added `LIFECYCLE_SPAWN_HOOK` injection point in `run_with_timeout` immediately after background spawn (`$!`) and before `CURRENT_CHILD_PID="$child_pid"`. The test uses this hook to assert `PHASE=spawning` and `CURRENT_CHILD_PID=empty`, injects `kill -TERM "$$"`, and asserts the runner exits 143 while the child process is terminated and reaped (`014/AC-007`). Ordinary worker interrupt test preserved separately.

3. **P2 — Readiness Timeout Budgets & Startup Latency Analysis (`tests/tooling/apollo-lifecycle.test.mjs` & `scripts/apollo-test-lifecycle.sh`):**
   - *Problem:* Explicit call sites passed 5000ms for file readiness and 10000ms for exit watchdog at lines 711, 718, 845, and 847, which were unaffected by helper function default changes.
   - *Investigation & Startup Timings:*
     - Teardown pipeline executes multiple subshells sequentially (`docker compose ps --format json`, `docker compose logs --no-color`, `docker compose down --volumes --remove-orphans`) and process tree traversal.
     - Recovery cleanup previously invoked `node -e` to parse `ownership.json`, introducing ~300ms of Node startup latency per invocation. Replaced with fast grep/sed extraction (<5ms) with Node fallback.
     - On macOS / Darwin, sequential process spawning and polling under multi-process test load can aggregate several seconds of latency before Docker down is invoked.
     - Updated explicit test call sites to 10000ms for file readiness and 20000ms for exit watchdog. Production timeout contracts in `scripts/apollo-test.sh` and `scripts/apollo-test-lifecycle.sh` remain unchanged.
