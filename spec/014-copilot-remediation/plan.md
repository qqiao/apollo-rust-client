# Technical Plan: GitHub Copilot Review Remediation

**Feature ID:** `014-copilot-remediation`  
**Status:** Design plan awaiting HumanReviewer approval.  
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
