# Plan: Honest test lifecycle teardown

**Status:** Implemented design plan retained for reference. Current verification and remaining acceptance limits are recorded in the [status audit](../verification-2026-09-13.md); original imperative/future-tense steps below are historical implementation guidance.

## Source context

Read `run_with_timeout`, `cleanup`, `handle_signal`, EXIT traps, and `run_recovery_cleanup` in [scripts/apollo-test.sh](../../scripts/apollo-test.sh), both fast entry points, and the diagnostics/upload paths in [workflow](../../.github/workflows/rust.yml). Baseline `down` redirects both outputs to `/dev/null` and uses `|| true`; explicit recovery runs unbounded `docker compose down`.

## Small shared lifecycle helper

For executable fault tests without provisioning Apollo, move only process supervision/teardown/exit-status functions into new `scripts/apollo-test-lifecycle.sh`, sourced by the entry script using its existing SCRIPT_DIR. The helper must not parse CLI args, install traps, execute Docker, or exit merely when sourced. It may use the existing documented lifecycle globals; avoid building a general shell framework.

Keep CLI parsing, preflight, fixture orchestration, suite selection, and ownership validation in the main script. Provide a small explicit `install_lifecycle_traps` function so the main script and test harness both install the actual same handlers; defining this function on source is safe, but do not call it automatically from the helper. A thin test harness can source the helper, initialize the same globals, and install the same exit handler. Test the **same function bodies** used by production, not copied snippets or extracted source strings.

Functions/behavior to implement:

1. `run_with_timeout`: preserve process-group TERM/KILL behavior and `CURRENT_CHILD_PID`. Capture `wait` failure in an explicit `if wait ...; then ...; else status=$?; fi` so `set -e` cannot skip bookkeeping. Clear child state on every ordinary return. Do not use `if ! command; then status=$?` to capture the original status: that records the negated condition's status.
2. A narrow `teardown_project(project, run_dir, timeout_seconds)` helper: create/use `logs/teardown.log`, run the existing scoped Compose down command via supervision, capture its status explicitly, retain output, and print an actionable failure with a shell-safe command such as `printf '%q'` for the run-dir argument. Default caller uses 30 seconds; tests pass 1 second to exercise actual watchdog behavior without a 30-second test. This parameter is internal, not a new public env/CLI feature.
3. `cleanup(original_status)`: accept the original status explicitly rather than discovering `$?` after other commands; preserve signal precedence; terminate an already tracked child before teardown; keep existing failure-only service log capture; call teardown once; compute/cache final status. A repeated call returns the cached status without repeating Docker. Keep `CLEANED_UP` plus a cached final result, with clear initialization.
4. One EXIT handler captures `$?` immediately, disables its own EXIT trap to avoid recursion, invokes cleanup in an explicit conditional, then exits with the returned final status. SIGINT/SIGTERM handlers record 130/143 and exit with that status so the one EXIT path owns cleanup. Do not call cleanup once in the signal handler and again from EXIT.
5. `run_recovery_cleanup` retains existing path/project validation, then uses the same bounded teardown helper. Missing-run and invalid-ownership behavior must stay safe. A recovery success/failure must not be followed by a contradictory automatic success message. Keep the main trap path aware that recovery already handled teardown, or exit through a single explicit finalization path.

Do not silently promote logging errors to success. If creation/redirection of teardown.log fails, print that failure and run the bounded command with visible stderr/stdout; retain a nonzero diagnostic status if no earlier/teardown error takes precedence. Use a distinct documented generic status such as 1 for diagnostic failure. Preserve original stage/signal statuses in all cases.

## Docker-independent tests

Create `tests/tooling/apollo-lifecycle.test.mjs` using Node's built-in test/assert/child_process/fs/os modules. Each test uses a unique temporary directory and a stub `docker` executable placed first in a subprocess-only PATH. The stub records argv and emits deterministic output/status; a hang mode waits so supervision must actually kill it. No real Docker binary or daemon is contacted.

Test a minimal Bash harness sourcing the lifecycle helper with `set -euo pipefail`. Initialize globals as production does, install the actual exit/signal handlers, and let it exit with a chosen stage status. Assert:

- 0 + success -> 0; 0 + 42 -> 42; 7 + 42 -> 7; 0 + hang -> 124.
- Logs retain stub output; stderr includes cleanup failure, exact project/run directory, log location, and a recovery command; success banner is absent on failure.
- INT/TERM during a supervised child -> 130/143; child is terminated and down is called once. Use child readiness output/IPC rather than a timing guess before sending signals.
- Repeated cleanup invokes down once and keeps final status; no-owned-run does not call down.
- Run directories containing spaces and a single quote survive command/argv handling; never eval the displayed command.
- Unwritable log path has visible fallback and does not prevent scoped down. A deterministic parent component that is a file is preferable to permission bits when tests may run as root.

Add explicit CLI recovery cases by spawning the real `scripts/apollo-test.sh cleanup --run-dir <temp>` with the stub PATH, valid/invalid ownership JSON, and success/failure stub. Its normal recovery deadline is 30 seconds; test the timeout branch through the shared helper at 1 second. Preserve invalid-prefix refusal and missing-run no-op. Put an outer subprocess watchdog (e.g. 8 seconds for 1-second helper timeouts plus grace) on each harness. In finally terminate remaining test children and remove only owned temp dirs.

## Integration into checks and CI

Add `node --test tests/tooling/*.test.mjs` to both existing fast-check functions. Use the repository root as execution context consistently with current commands; do not accidentally count zero matched files as success. Keep `scripts/test.sh fast` Docker-independent. This glob will pick up package 010's later tests; do not add a second duplicate invocation then.

The existing CI uploads `logs/`, so `teardown.log` is covered. Leave workflow event/permission/job topology unchanged. The fallback cleanup step uses `|| true` to preserve the original CI result; ensure the script now emits its failure visibly rather than silencing it. Do not claim this fallback creates a successful cleanup when it did not.

Run syntax checks, Node lifecycle tests, `scripts/test.sh fast`, one real native integration, and explicit recovery on that run's exact directory to verify idempotent real cleanup. Never test failure by damaging a user's Docker daemon or deleting unrelated resources. Document the 30-second deadline, termination grace, status table, log path, and explicit recovery command in tests/apollo/README.md.
