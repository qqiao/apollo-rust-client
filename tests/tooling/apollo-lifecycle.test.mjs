import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '../..');
const LIFECYCLE_SCRIPT = path.join(REPO_ROOT, 'scripts/apollo-test-lifecycle.sh');
const MAIN_SCRIPT = path.join(REPO_ROOT, 'scripts/apollo-test.sh');


function isProcessAlive(pid) {
  if (!pid) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function safeKillPid(pid) {
  if (!pid) return;
  try {
    process.kill(-pid, "SIGKILL");
  } catch {}
  try {
    process.kill(pid, "SIGKILL");
  } catch {}
}

async function waitForFile(filePath, timeoutMs = 10000) {
  const start = Date.now();
  while (!fs.existsSync(filePath)) {
    if (Date.now() - start > timeoutMs) {
      throw new Error(`Timed out waiting for file: ${filePath}`);
    }
    await new Promise((r) => setTimeout(r, 50));
  }
}

async function waitForExit(child, timeoutMs = 15000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      try {
        child.kill("SIGKILL");
      } catch {}
      reject(new Error(`Child process ${child.pid} timed out waiting for exit`));
    }, timeoutMs);

    child.on("close", (code, signal) => {
      clearTimeout(timer);
      resolve({ code, signal });
    });
  });
}

function makeTempDir(prefix = 'apollo-lifecycle-test-') {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function createStubDocker(tempDir) {
  const binDir = path.join(tempDir, 'bin');
  fs.mkdirSync(binDir, { recursive: true });
  const dockerPath = path.join(binDir, 'docker');
  const logPath = path.join(tempDir, 'docker-calls.log');

  const dockerContent = `#!/usr/bin/env bash
set -e
LOG_FILE="${logPath}"
printf "%s\\n" "$*" >> "$LOG_FILE"

if [ -n "\${STUB_DOCKER_READY_FILE:-}" ]; then
  trigger="\${STUB_TRIGGER_STAGE:-down}"
  if [[ "$*" == *"$trigger"* ]]; then
    pid_file="\${STUB_DOCKER_PID_FILE:-\${STUB_DOCKER_READY_FILE}.pid}"
    echo "$$" > "$pid_file"
    sleep 30 &
    desc_pid=$!
    echo "$desc_pid" >> "$pid_file"
    touch "$STUB_DOCKER_READY_FILE"

    if [ -n "\${STUB_RELEASE_FILE:-}" ]; then
      while [ ! -f "\${STUB_RELEASE_FILE}" ]; do
        sleep 0.05
      done
    elif [ "\${STUB_DOCKER_BLOCK:-0}" -eq 1 ]; then
      sleep 30
    fi
    kill -TERM "$desc_pid" 2>/dev/null || true
    wait "$desc_pid" 2>/dev/null || true
  fi
fi


BEHAVIOR="\${STUB_DOCKER_BEHAVIOR:-success}"
case "$BEHAVIOR" in
  "success")
    echo "Stub docker: simulated success for: $*"
    exit 0
    ;;
  "fail42")
    echo "Stub docker: simulated failure 42 for: $*" >&2
    exit 42
    ;;
  "hang")
    echo "Stub docker: hanging for: $*"
    # Sleep in background so signal propagation is clean
    sleep 60
    exit 0
    ;;
  *)
    echo "Stub docker: unknown behavior $BEHAVIOR" >&2
    exit 1
    ;;
esac
`;

  fs.writeFileSync(dockerPath, dockerContent, { mode: 0o755 });
  return { binDir, dockerPath, logPath };
}

function runBashHarness(harnessScript, env = {}, timeoutMs = 8000) {
  const result = spawnSync('bash', ['-c', harnessScript], {
    env: { ...process.env, ...env },
    encoding: 'utf8',
    timeout: timeoutMs,
  });

  return {
    status: result.status,
    signal: result.signal,
    stdout: result.stdout || '',
    stderr: result.stderr || '',
  };
}

// -----------------------------------------------------------------------------
// T01: Source safety and process supervision
// -----------------------------------------------------------------------------

test('lifecycle helper can be sourced without side effects (no docker execution, no traps installed, no exit)', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      echo "SOURCED_OK"
      trap -p EXIT
    `;
    const res = runBashHarness(harness, { PATH: `${binDir}:${process.env.PATH}` });
    assert.equal(res.status, 0, `Failed to source lifecycle helper: ${res.stderr}`);
    assert.match(res.stdout, /SOURCED_OK/);
    assert.ok(!fs.existsSync(logPath), 'Docker should NOT be called merely on sourcing');
    // Verify no EXIT trap was set just by sourcing
    assert.ok(!res.stdout.includes('trap -- \'cleanup'), 'Sourcing should not automatically install traps');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('run_with_timeout returns 0 on successful child command and clears CURRENT_CHILD_PID', () => {
  const harness = `
    set -euo pipefail
    source "${LIFECYCLE_SCRIPT}"
    run_with_timeout 5 echo "hello from child"
    status=$?
    echo "STATUS=$status"
    echo "CHILD_PID=$CURRENT_CHILD_PID"
  `;
  const res = runBashHarness(harness);
  assert.equal(res.status, 0);
  assert.match(res.stdout, /STATUS=0/);
  assert.match(res.stdout, /CHILD_PID=\s*$/m);
});

test('run_with_timeout returns child exit code on failure and clears CURRENT_CHILD_PID', () => {
  const harness = `
    set -euo pipefail
    source "${LIFECYCLE_SCRIPT}"
    run_with_timeout 5 bash -c "exit 42" || status=$?
    echo "STATUS=\${status:-0}"
    echo "CHILD_PID=$CURRENT_CHILD_PID"
  `;
  const res = runBashHarness(harness);
  assert.equal(res.status, 0);
  assert.match(res.stdout, /STATUS=42/);
  assert.match(res.stdout, /CHILD_PID=\s*$/m);
});

test('run_with_timeout terminates hung process within watchdog bound and returns 124', () => {
  const start = Date.now();
  const harness = `
    set -euo pipefail
    source "${LIFECYCLE_SCRIPT}"
    run_with_timeout 1 sleep 30 || status=$?
    echo "STATUS=\${status:-0}"
    echo "CHILD_PID=$CURRENT_CHILD_PID"
  `;
  const res = runBashHarness(harness, {}, 6000);
  const duration = Date.now() - start;
  assert.equal(res.status, 0);
  assert.match(res.stdout, /STATUS=124/);
  assert.match(res.stdout, /CHILD_PID=\s*$/m);
  // Should terminate around 1s + ~2s grace = ~3s, well under 6s watchdog
  assert.ok(duration < 5500, `Expected termination within 5.5s, took ${duration}ms`);
});

// -----------------------------------------------------------------------------
// T02: Automatic cleanup results, precedence, and diagnostics
// -----------------------------------------------------------------------------

test('status table: success + teardown success -> 0, prints success banner', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-p1' }));

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME="apollo-test-p1"
      RUN_DIR="${runDir}"
      install_lifecycle_traps
      exit 0
    `;
    const res = runBashHarness(harness, { PATH: `${binDir}:${process.env.PATH}` });
    assert.equal(res.status, 0);
    assert.match(res.stdout, /Run completed successfully/);
    assert.ok(fs.existsSync(path.join(runDir, 'logs/teardown.log')), 'teardown.log must be created');
    const tdLog = fs.readFileSync(path.join(runDir, 'logs/teardown.log'), 'utf8');
    assert.match(tdLog, /simulated success/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('status table: success + teardown failure 42 -> 42, prints failure details and recovery command, no success banner', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-p1' }));

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME="apollo-test-p1"
      RUN_DIR="${runDir}"
      install_lifecycle_traps
      exit 0
    `;
    const res = runBashHarness(harness, {
      PATH: `${binDir}:${process.env.PATH}`,
      STUB_DOCKER_BEHAVIOR: 'fail42',
    });
    assert.equal(res.status, 42);
    assert.doesNotMatch(res.stdout, /Run completed successfully/);
    assert.match(res.stderr, /Run aborted\/failed with status 42/);
    assert.match(res.stderr, /Teardown failed for Compose project 'apollo-test-p1'/);
    assert.match(res.stderr, /cleanup --run-dir/);
    assert.ok(fs.existsSync(path.join(runDir, 'logs/teardown.log')), 'teardown.log must be retained on failure');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('status table: stage failure 7 + teardown failure 42 -> 7, cleanup failure reported', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-p1' }));

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME="apollo-test-p1"
      RUN_DIR="${runDir}"
      install_lifecycle_traps
      exit 7
    `;
    const res = runBashHarness(harness, {
      PATH: `${binDir}:${process.env.PATH}`,
      STUB_DOCKER_BEHAVIOR: 'fail42',
    });
    assert.equal(res.status, 7, 'Original stage exit 7 must take precedence over teardown exit 42');
    assert.match(res.stderr, /Teardown failed for Compose project 'apollo-test-p1'/);
    assert.match(res.stderr, /cleanup --run-dir/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('status table: success + teardown timeout -> 124', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-p1' }));

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME="apollo-test-p1"
      RUN_DIR="${runDir}"
      TEARDOWN_TIMEOUT=1
      install_lifecycle_traps
      exit 0
    `;
    const res = runBashHarness(harness, {
      PATH: `${binDir}:${process.env.PATH}`,
      STUB_DOCKER_BEHAVIOR: 'hang',
    }, 7000);
    assert.equal(res.status, 124);
    assert.match(res.stderr, /Teardown failed for Compose project 'apollo-test-p1' with status 124/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('status table: success + unwritable teardown log -> 1, teardown still runs', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-p1' }));

    // Make logs a file so logs/teardown.log cannot be created
    fs.writeFileSync(path.join(runDir, 'logs'), 'blocking-file');

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME="apollo-test-p1"
      RUN_DIR="${runDir}"
      install_lifecycle_traps
      exit 0
    `;
    const res = runBashHarness(harness, { PATH: `${binDir}:${process.env.PATH}` });
    assert.equal(res.status, 1, 'Diagnostic failure status must be 1');
    assert.match(res.stderr, /WARNING: Failed to create or write teardown log/);
    assert.ok(fs.existsSync(logPath), 'Docker down must still be executed even if log path is unwritable');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('status table: SIGINT -> 130 and SIGTERM -> 143', async () => {
  for (const [sig, expectedStatus] of [['SIGINT', 130], ['SIGTERM', 143]]) {
    const tempDir = makeTempDir();
    try {
      const { binDir, logPath } = createStubDocker(tempDir);
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-p1' }));

      const readyFile = path.join(tempDir, 'ready');
      const harness = `
        set -euo pipefail
        PATH="${binDir}:$PATH"
        source "${LIFECYCLE_SCRIPT}"
        COMPOSE_FILE="${tempDir}/compose.yaml"
        touch "$COMPOSE_FILE"
        PROJECT_NAME="apollo-test-p1"
        RUN_DIR="${runDir}"
        install_lifecycle_traps
        touch "${readyFile}"
        while true; do
          sleep 0.1
        done
      `;

      const child = spawn('bash', ['-c', harness], {
        env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      });

      // Wait for child to touch readyFile
      let waited = 0;
      while (!fs.existsSync(readyFile) && waited < 40) {
        if (child.exitCode !== null) break;
        await new Promise((r) => setTimeout(r, 50));
        waited++;
      }

      child.kill(sig);

      const exitPromise = new Promise((resolve) => {
        child.on('close', (code, signal) => resolve({ code, signal }));
      });
      const res = await exitPromise;
      assert.equal(res.code, expectedStatus, `Signal ${sig} must produce status ${expectedStatus}`);
      assert.ok(fs.existsSync(logPath), `Teardown must be called on ${sig}`);
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('repeated cleanup calls execute teardown only once', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-p1' }));

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME="apollo-test-p1"
      RUN_DIR="${runDir}"
      cleanup 0
      cleanup 0
    `;
    const res = runBashHarness(harness, { PATH: `${binDir}:${process.env.PATH}` });
    assert.equal(res.status, 0);
    const dockerCalls = fs.readFileSync(logPath, 'utf8').trim().split('\n');
    assert.equal(dockerCalls.length, 1, 'Docker down must be called exactly once');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('unowned runs (missing PROJECT_NAME or ownership.json) do not call docker down', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    // No ownership.json written

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME=""
      RUN_DIR="${runDir}"
      cleanup 7 || status=$?
      echo "STATUS=\${status:-0}"
    `;
    const res = runBashHarness(harness, { PATH: `${binDir}:${process.env.PATH}` });
    assert.equal(res.status, 0);
    assert.match(res.stdout, /STATUS=7/);
    assert.ok(!fs.existsSync(logPath), 'Docker down must NOT be called for unowned run');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

// -----------------------------------------------------------------------------
// T03: Explicit CLI recovery with scoped teardown
// -----------------------------------------------------------------------------

test('CLI recovery: valid ownership and successful teardown exits 0 and passes -p project', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-rec1' }));

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', runDir], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 0, `Recovery failed: ${res.stderr}`);
    assert.match(res.stdout, /Recovery cleanup complete for project apollo-test-rec1/);
    const dockerCalls = fs.readFileSync(logPath, 'utf8');
    assert.match(dockerCalls, /-p apollo-test-rec1 down --volumes --remove-orphans/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('CLI recovery: valid ownership and failure 42 exits 42 with error details', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-rec2' }));

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', runDir], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}`, STUB_DOCKER_BEHAVIOR: 'fail42' },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 42);
    assert.match(res.stderr, /Teardown failed for Compose project 'apollo-test-rec2'/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('CLI recovery: invalid project prefix refuses cleanup and exits 1 without calling docker', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'unauthorized-project' }));

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', runDir], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 1);
    assert.match(res.stderr, /Safety check failed.*does not start with 'apollo-test-'/);
    assert.ok(!fs.existsSync(logPath), 'Docker down must NOT be called for untrusted project');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('CLI recovery: nested project before actual owner selects top-level owner (009/FR-003 / P1)', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(
      path.join(runDir, 'ownership.json'),
      JSON.stringify({ metadata: { project: 'apollo-test-unrelated' }, project: 'apollo-test-owner' })
    );

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', runDir], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 0, `Recovery failed: ${res.stderr}`);
    assert.match(res.stdout, /Recovery cleanup complete for project apollo-test-owner/);
    const dockerCalls = fs.readFileSync(logPath, 'utf8');
    assert.match(dockerCalls, /-p apollo-test-owner down --volumes --remove-orphans/);
    assert.doesNotMatch(dockerCalls, /apollo-test-unrelated/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('CLI recovery: malformed JSON ownership refuses cleanup and exits 1 without calling docker (009/AC-008 / P1)', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), 'not JSON "project":"apollo-test-unrelated"');

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', runDir], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 1);
    assert.match(res.stderr, /Safety check failed: Failed to extract 'project'/);
    assert.ok(!fs.existsSync(logPath), 'Docker down must NOT be called for malformed ownership');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('CLI recovery: nested-only project without top-level owner refuses cleanup and exits 1 without calling docker (009/AC-008 / P1)', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(
      path.join(runDir, 'ownership.json'),
      JSON.stringify({ metadata: { project: 'apollo-test-unrelated' } })
    );

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', runDir], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 1);
    assert.match(res.stderr, /Safety check failed: Failed to extract 'project'/);
    assert.ok(!fs.existsSync(logPath), 'Docker down must NOT be called when top-level project is absent');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('CLI recovery: nonexistent directory exits 0 safely without calling docker', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const nonExistent = path.join(tempDir, 'does-not-exist');

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', nonExistent], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 0);
    assert.match(res.stdout, /Run directory does not exist or was already removed.*Fallback cleanup is safe/);
    assert.ok(!fs.existsSync(logPath), 'Docker down must NOT be called for nonexistent dir');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('run directories with spaces and single quotes survive argv handling and print shell-safe command', () => {
  const tempDir = makeTempDir();
  try {
    const { binDir } = createStubDocker(tempDir);
    const specialRunDir = path.join(tempDir, "run with spaces and 'quotes'");
    fs.mkdirSync(specialRunDir, { recursive: true });
    fs.writeFileSync(path.join(specialRunDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-special' }));

    const res = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', specialRunDir], {
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}`, STUB_DOCKER_BEHAVIOR: 'fail42' },
      encoding: 'utf8',
      timeout: 5000,
    });
    assert.equal(res.status, 42);
    // Ensure the recovery command in stderr properly escaped the special path
    assert.match(res.stderr, /cleanup --run-dir/);
    assert.match(res.stderr, /run\\ with\\ spaces/);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

// -----------------------------------------------------------------------------
// T01 & T02 Review Follow-up Regressions (Finding V1)
// -----------------------------------------------------------------------------

test('status table: signal during automatic teardown (INT and TERM) finishes boundedly and reaps child process group', async () => {
  for (const [sig, expectedStatus] of [['SIGTERM', 143], ['SIGINT', 130]]) {
    const tempDir = makeTempDir();
    const readyFile = path.join(tempDir, 'down_ready');
    const pidFile = path.join(tempDir, 'down.pid');
    let trackedPids = [];
    let runnerChild = null;

    try {
      const { binDir, logPath } = createStubDocker(tempDir);
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-v1' }));

      const harness = `
        set -euo pipefail
        PATH="${binDir}:$PATH"
        source "${LIFECYCLE_SCRIPT}"
        COMPOSE_FILE="${tempDir}/compose.yaml"
        touch "$COMPOSE_FILE"
        PROJECT_NAME="apollo-test-v1"
        RUN_DIR="${runDir}"
        TEARDOWN_TIMEOUT=1
        install_lifecycle_traps
        exit 0
      `;

      const startTime = Date.now();
      runnerChild = spawn('bash', ['-c', harness], {
        env: {
          ...process.env,
          PATH: `${binDir}:${process.env.PATH}`,
          STUB_DOCKER_READY_FILE: readyFile,
          STUB_DOCKER_PID_FILE: pidFile,
          STUB_DOCKER_BLOCK: '1',
        },
      });
      const exitPromise = waitForExit(runnerChild, 10000);

      await waitForFile(readyFile, 5000);
      const pidLines = fs.readFileSync(pidFile, 'utf8').trim().split('\n');
      trackedPids = pidLines.map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);
      assert.ok(trackedPids.length >= 1, 'Stub docker should have recorded at least one PID');
      for (const p of trackedPids) {
        assert.ok(isProcessAlive(p), `Child PID ${p} should be alive initially`);
      }

      // Send signal while teardown is active
      runnerChild.kill(sig);

      const exitRes = await exitPromise;
      const elapsedSec = (Date.now() - startTime) / 1000;

      assert.equal(exitRes.code, expectedStatus, `Runner must exit with signal status ${expectedStatus}`);
      assert.ok(elapsedSec <= 6.0, `Elapsed time must be bounded by timeout+grace (got ${elapsedSec}s)`);

      // Verify NO surviving children
      for (const p of trackedPids) {
        assert.equal(isProcessAlive(p), false, `Supervised process ${p} must be terminated and reaped`);
      }

      const logCalls = fs.readFileSync(logPath, 'utf8').trim().split('\n');
      assert.equal(logCalls.length, 1, `Teardown down must be called exactly once, got: ${logCalls.length}`);
    } finally {
      if (fs.existsSync(pidFile)) {
        try {
          const pids = fs.readFileSync(pidFile, 'utf8').trim().split('\n').map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);
          for (const p of pids) if (!trackedPids.includes(p)) trackedPids.push(p);
        } catch {}
      }
      for (const p of trackedPids) safeKillPid(p);
      if (runnerChild && isProcessAlive(runnerChild.pid)) safeKillPid(runnerChild.pid);
      if (runnerChild) await waitForExit(runnerChild, 2000).catch(() => {});
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('status table: mixed repeated signals during cleanup (INT -> TERM and TERM -> INT) preserve first signal', async () => {
  for (const [firstSig, secondSig, expectedStatus] of [['SIGINT', 'SIGTERM', 130], ['SIGTERM', 'SIGINT', 143]]) {
    const tempDir = makeTempDir();
    const workReadyFile = path.join(tempDir, 'work_ready');
    const downReadyFile = path.join(tempDir, 'down_ready');
    const pidFile = path.join(tempDir, 'down.pid');
    let trackedPids = [];
    let runnerChild = null;

    try {
      const { binDir, logPath } = createStubDocker(tempDir);
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-mixed' }));

      const harness = `
        set -euo pipefail
        PATH="${binDir}:$PATH"
        source "${LIFECYCLE_SCRIPT}"
        COMPOSE_FILE="${tempDir}/compose.yaml"
        touch "$COMPOSE_FILE"
        PROJECT_NAME="apollo-test-mixed"
        RUN_DIR="${runDir}"
        TEARDOWN_TIMEOUT=1
        install_lifecycle_traps
        touch "${workReadyFile}"
        while true; do sleep 0.1; done
      `;

      runnerChild = spawn('bash', ['-c', harness], {
        env: {
          ...process.env,
          PATH: `${binDir}:${process.env.PATH}`,
          STUB_DOCKER_READY_FILE: downReadyFile,
          STUB_DOCKER_PID_FILE: pidFile,
          STUB_DOCKER_BLOCK: '1',
        },
      });
      const exitPromise = waitForExit(runnerChild, 20000);

      await waitForFile(workReadyFile, 10000);
      // First signal sends runner into cleanup
      runnerChild.kill(firstSig);

      // Wait until teardown down stage starts
      await waitForFile(downReadyFile, 10000);
      const pidLines = fs.readFileSync(pidFile, 'utf8').trim().split('\n');
      trackedPids = pidLines.map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);

      // Second signal arrives while teardown is underway
      runnerChild.kill(secondSig);

      const exitRes = await exitPromise;
      assert.equal(exitRes.code, expectedStatus, `First signal (${firstSig} -> ${expectedStatus}) must outrank second signal (${secondSig})`);

      for (const p of trackedPids) {
        assert.equal(isProcessAlive(p), false, `Tracked child ${p} must be reaped`);
      }
      const logCalls = fs.readFileSync(logPath, 'utf8').trim().split('\n');
      const downCalls = logCalls.filter((call) => call.includes('down'));
      assert.equal(downCalls.length, 1, `Teardown down must run only once despite repeated signals`);
    } finally {
      if (fs.existsSync(pidFile)) {
        try {
          const pids = fs.readFileSync(pidFile, 'utf8').trim().split('\n').map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);
          for (const p of pids) if (!trackedPids.includes(p)) trackedPids.push(p);
        } catch {}
      }
      for (const p of trackedPids) safeKillPid(p);
      if (runnerChild && isProcessAlive(runnerChild.pid)) safeKillPid(runnerChild.pid);
      if (runnerChild) await waitForExit(runnerChild, 2000).catch(() => {});
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('status table: repeated signal during diagnostic collection preserves first signal and finishes teardown', async () => {
  const tempDir = makeTempDir();
  const diagReadyFile = path.join(tempDir, 'diag_ready');
  const diagReleaseFile = path.join(tempDir, 'diag_release');
  const pidFile = path.join(tempDir, 'diag.pid');
  let trackedPids = [];
  let runnerChild = null;

  try {
    const { binDir, logPath } = createStubDocker(tempDir);
    const runDir = path.join(tempDir, 'run');
    fs.mkdirSync(runDir, { recursive: true });
    fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-diag' }));

    const harness = `
      set -euo pipefail
      PATH="${binDir}:$PATH"
      source "${LIFECYCLE_SCRIPT}"
      COMPOSE_FILE="${tempDir}/compose.yaml"
      touch "$COMPOSE_FILE"
      PROJECT_NAME="apollo-test-diag"
      RUN_DIR="${runDir}"
      TEARDOWN_TIMEOUT=1
      install_lifecycle_traps
      exit 7
    `;

    runnerChild = spawn('bash', ['-c', harness], {
      env: {
        ...process.env,
        PATH: `${binDir}:${process.env.PATH}`,
        STUB_TRIGGER_STAGE: 'logs',
        STUB_DOCKER_READY_FILE: diagReadyFile,
        STUB_DOCKER_PID_FILE: pidFile,
        STUB_RELEASE_FILE: diagReleaseFile,
      },
    });
    const exitPromise = waitForExit(runnerChild, 10000);

    await waitForFile(diagReadyFile, 5000);
    const pidLines = fs.readFileSync(pidFile, 'utf8').trim().split('\n');
    trackedPids = pidLines.map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);

    // Send SIGINT while diagnostic logs are being captured
    runnerChild.kill('SIGINT');

    // Release the diagnostic step so it can finish boundedly
    fs.writeFileSync(diagReleaseFile, 'release');

    const exitRes = await exitPromise;
    assert.equal(exitRes.code, 130, 'Signal (130) received during diagnostics must outrank stage failure (7)');

    for (const p of trackedPids) {
      assert.equal(isProcessAlive(p), false, `Diagnostic process ${p} must be reaped`);
    }
    const logCalls = fs.readFileSync(logPath, 'utf8').trim().split('\n');
    // Diagnostic called ps and logs, then down
    assert.ok(logCalls.some((c) => c.includes('down')), 'Teardown down must still execute');
  } finally {
    if (fs.existsSync(pidFile)) {
      try {
        const pids = fs.readFileSync(pidFile, 'utf8').trim().split('\n').map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);
        for (const p of pids) if (!trackedPids.includes(p)) trackedPids.push(p);
      } catch {}
    }
    for (const p of trackedPids) safeKillPid(p);
    if (runnerChild && isProcessAlive(runnerChild.pid)) safeKillPid(runnerChild.pid);
    if (runnerChild) await waitForExit(runnerChild, 2000).catch(() => {});
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('CLI recovery: interrupted by SIGINT / SIGTERM reaps child and exits 130 / 143 without second down', async () => {
  for (const [sig, expectedStatus] of [['SIGINT', 130], ['SIGTERM', 143]]) {
    const tempDir = makeTempDir();
    const readyFile = path.join(tempDir, 'rec_ready');
    const pidFile = path.join(tempDir, 'rec.pid');
    let trackedPids = [];
    let runnerChild = null;

    try {
      const { binDir, logPath } = createStubDocker(tempDir);
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-rec-sig' }));

      runnerChild = spawn('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', runDir], {
        env: {
          ...process.env,
          PATH: `${binDir}:${process.env.PATH}`,
          STUB_DOCKER_READY_FILE: readyFile,
          STUB_DOCKER_PID_FILE: pidFile,
          STUB_DOCKER_BLOCK: '1',
          TEARDOWN_TIMEOUT: '1',
        },
      });
      const exitPromise = waitForExit(runnerChild, 20000);

      await waitForFile(readyFile, 10000);
      const pidLines = fs.readFileSync(pidFile, 'utf8').trim().split('\n');
      trackedPids = pidLines.map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);

      runnerChild.kill(sig);

      const exitRes = await exitPromise;
      assert.equal(exitRes.code, expectedStatus, `CLI recovery interrupted by ${sig} must exit ${expectedStatus}`);

      for (const p of trackedPids) {
        assert.equal(isProcessAlive(p), false, `Supervised process ${p} must not remain alive`);
      }
      const logCalls = fs.readFileSync(logPath, 'utf8').trim().split('\n');
      assert.equal(logCalls.length, 1, `Recovery must run down exactly once`);
    } finally {
      if (fs.existsSync(pidFile)) {
        try {
          const pids = fs.readFileSync(pidFile, 'utf8').trim().split('\n').map((l) => parseInt(l.trim(), 10)).filter((p) => !isNaN(p) && p > 0);
          for (const p of pids) if (!trackedPids.includes(p)) trackedPids.push(p);
        } catch {}
      }
      for (const p of trackedPids) safeKillPid(p);
      if (runnerChild && isProcessAlive(runnerChild.pid)) safeKillPid(runnerChild.pid);
      if (runnerChild) await waitForExit(runnerChild, 2000).catch(() => {});
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('run_with_timeout detects and terminates surviving descendants in process group when leader exits (F6 / AC-008)', async () => {
  const tempDir = makeTempDir();
  const pidFile = path.join(tempDir, 'descendant.pid');
  const readyFile = path.join(tempDir, 'descendant.ready');
  const workerScript = path.join(tempDir, 'worker.sh');
  const leaderScript = path.join(tempDir, 'leader.sh');

  try {
    fs.writeFileSync(
      workerScript,
      `#!/usr/bin/env bash
echo "$$" > "$1"
touch "$2"
sleep 10
`,
      { mode: 0o755 }
    );

    fs.writeFileSync(
      leaderScript,
      `#!/usr/bin/env bash
"${workerScript}" "$1" "$2" &
exit 0
`,
      { mode: 0o755 }
    );

    const harness = `
      set -euo pipefail
      source "${LIFECYCLE_SCRIPT}"

      # Leader launches background worker in same group and exits immediately
      run_with_timeout 2 "${leaderScript}" "${pidFile}" "${readyFile}"
    `;

    const runner = spawn('bash', ['-c', harness]);
    await waitForExit(runner, 10000);

    await waitForFile(readyFile, 5000);
    assert.ok(fs.existsSync(pidFile), 'Descendant PID file must exist');
    const descendantPid = parseInt(fs.readFileSync(pidFile, 'utf8').trim(), 10);
    assert.ok(Number.isInteger(descendantPid) && descendantPid > 0, `Descendant PID must be a valid positive integer, got ${descendantPid}`);
    assert.equal(isProcessAlive(descendantPid), false, `Descendant PID ${descendantPid} must be terminated and not orphaned`);
  } finally {
    if (fs.existsSync(pidFile)) {
      try {
        const pid = parseInt(fs.readFileSync(pidFile, 'utf8').trim(), 10);
        safeKillPid(pid);
      } catch {}
    }
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('handle_signal defers during spawning phase when signal arrives before child registration (F8 / AC-007 / P2)', async () => {
  const tempDir = makeTempDir();
  const proofFile = path.join(tempDir, 'spawn_proof.txt');
  const pidFile = path.join(tempDir, 'worker.pid');
  const readyFile = path.join(tempDir, 'worker.ready');
  const workerScript = path.join(tempDir, 'worker.sh');
  let trackedPid = null;

  try {
    fs.writeFileSync(
      workerScript,
      `#!/usr/bin/env bash
echo "$$" > "$1"
touch "$2"
sleep 30
`,
      { mode: 0o755 }
    );

    const harness = `
      set -euo pipefail
      source "${LIFECYCLE_SCRIPT}"
      install_lifecycle_traps

      export LIFECYCLE_SPAWN_HOOK='
        printf "PHASE=%s\\nCURRENT_CHILD_PID=%s\\nCHILD_PID=%s\\n" "\${LIFECYCLE_PHASE:-none}" "\${CURRENT_CHILD_PID:-empty}" "\$child_pid" > "${proofFile}"
        kill -TERM "$$"
      '
      run_with_timeout 10 "${workerScript}" "${pidFile}" "${readyFile}"
    `;

    const runner = spawn('bash', ['-c', harness]);
    const exitRes = await waitForExit(runner, 10000);

    assert.equal(exitRes.code, 143, 'Should exit with SIGTERM status 143');

    await waitForFile(proofFile, 5000);
    const proofContent = fs.readFileSync(proofFile, 'utf8');
    assert.match(proofContent, /PHASE=spawning/, 'Phase must be spawning at injection point');
    assert.match(proofContent, /CURRENT_CHILD_PID=empty/, 'CURRENT_CHILD_PID must not yet be registered at injection point');
    const childPidMatch = proofContent.match(/CHILD_PID=(\d+)/);
    assert.ok(childPidMatch && childPidMatch[1], 'Child PID must be captured in spawn proof');
    const childPid = parseInt(childPidMatch[1], 10);
    assert.ok(Number.isInteger(childPid) && childPid > 0, `Child PID must be a valid positive integer, got ${childPid}`);
    trackedPid = childPid;

    assert.equal(isProcessAlive(childPid), false, 'Spawned child process must be terminated and reaped');
  } finally {
    if (trackedPid) {
      safeKillPid(trackedPid);
    }
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('run_with_timeout reaps supervised child process when interrupted by worker signal', async () => {
  const tempDir = makeTempDir();
  const pidFile = path.join(tempDir, 'spawned.pid');
  const readyFile = path.join(tempDir, 'spawned.ready');
  const workerScript = path.join(tempDir, 'worker.sh');

  try {
    fs.writeFileSync(
      workerScript,
      `#!/usr/bin/env bash
echo "$$" > "$1"
touch "$2"
if [ -n "\${3:-}" ]; then
  kill -TERM "$3"
fi
sleep 10
`,
      { mode: 0o755 }
    );

    const harness = `
      set -euo pipefail
      source "${LIFECYCLE_SCRIPT}"
      install_lifecycle_traps

      runner_pid="$$"
      run_with_timeout 5 "${workerScript}" "${pidFile}" "${readyFile}" "$runner_pid"
    `;

    const runner = spawn('bash', ['-c', harness]);
    const exitRes = await waitForExit(runner, 10000);

    assert.equal(exitRes.code, 143, 'Should exit with SIGTERM status 143');

    await waitForFile(readyFile, 5000);
    assert.ok(fs.existsSync(pidFile), 'Spawned PID file must exist');
    const childPid = parseInt(fs.readFileSync(pidFile, 'utf8').trim(), 10);
    assert.ok(Number.isInteger(childPid) && childPid > 0, `Spawned PID must be a valid positive integer, got ${childPid}`);
    assert.equal(isProcessAlive(childPid), false, 'Child process must be terminated and reaped');
  } finally {
    if (fs.existsSync(pidFile)) {
      try {
        const pid = parseInt(fs.readFileSync(pidFile, 'utf8').trim(), 10);
        safeKillPid(pid);
      } catch {}
    }
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('run_recovery_cleanup exits immediately with signal status when late signal injected during final logging (P2)', async () => {
  for (const [sig, expectedStatus] of [['SIGTERM', 143], ['SIGINT', 130]]) {
    const tempDir = makeTempDir();
    try {
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-late-sig' }));

      const harness = `
        set -euo pipefail
        source "${LIFECYCLE_SCRIPT}"
        teardown_project() { return 0; }
        echo() {
          case "$*" in
            *"Recovery cleanup complete"*) kill -${sig} "$$" ;;
          esac
          builtin echo "$@"
        }
        trap 'handle_signal SIGTERM' TERM
        trap 'handle_signal SIGINT' INT
        run_recovery_cleanup "${runDir}"
      `;

      const res = spawnSync('bash', ['-c', harness], { encoding: 'utf8', timeout: 5000 });
      assert.equal(res.status, expectedStatus, `Process must exit with status ${expectedStatus} when interrupted during terminal logging`);
      assert.match(res.stderr, new RegExp(`Signal ${sig} received during terminal phase, exiting with status ${expectedStatus}`));
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('run_recovery_cleanup preserves first signal order when late signals injected during final logging (P2)', async () => {
  for (const [firstSig, secondSig, expectedStatus] of [['INT', 'TERM', 130], ['TERM', 'INT', 143]]) {
    const tempDir = makeTempDir();
    try {
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-order-sig' }));

      const harness = `
        set -euo pipefail
        source "${LIFECYCLE_SCRIPT}"
        teardown_project() { return 0; }
        echo() {
          case "$*" in
            *"Recovery cleanup complete"*)
              kill -${firstSig} "$$"
              kill -${secondSig} "$$"
              ;;
          esac
          builtin echo "$@"
        }
        trap 'handle_signal SIGTERM' TERM
        trap 'handle_signal SIGINT' INT
        run_recovery_cleanup "${runDir}"
      `;

      const res = spawnSync('bash', ['-c', harness], { encoding: 'utf8', timeout: 5000 });
      assert.equal(res.status, expectedStatus, `Process must exit with first signal status ${expectedStatus}`);
      assert.match(res.stderr, new RegExp(`exiting with status ${expectedStatus}`));
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('terminal-phase signal at helper return boundary exits with signal status (014/AC-009 / P2)', async () => {
  for (const [sig, expectedStatus] of [['SIGTERM', 143], ['SIGINT', 130]]) {
    const tempDir = makeTempDir();
    try {
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-boundary-helper' }));

      const hook = `
        set -T
        review_injected=0
        LIFECYCLE_PHASE=""
        review_inject() {
          if [ "$review_injected" -eq 0 ] && [ "$LIFECYCLE_PHASE" = finished ]; then
            if [ "$1" = 'return "$final_status"' ]; then
              review_injected=1
              kill -${sig} "$$"
            fi
          fi
          return 0
        }
        trap 'review_inject "$BASH_COMMAND"' DEBUG
      `;
      const hookPath = path.join(tempDir, 'hook.sh');
      fs.writeFileSync(hookPath, hook);

      const helper = spawnSync('bash', ['-c', `
        source "${LIFECYCLE_SCRIPT}"
        source "${hookPath}"
        trap 'handle_signal SIGTERM' TERM
        trap 'handle_signal SIGINT' INT
        teardown_project() { return 0; }
        run_recovery_cleanup "${runDir}"
      `], {
        encoding: 'utf8',
        timeout: 5000,
      });

      assert.equal(helper.status, expectedStatus, `Helper must exit ${expectedStatus} when interrupted at terminal return boundary`);
      assert.match(helper.stderr, new RegExp(`Signal ${sig} received during terminal phase, exiting with status ${expectedStatus}`));
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('terminal-phase signal at CLI exit boundary exits with signal status (014/AC-009 / P2)', async () => {
  for (const [sig, expectedStatus] of [['SIGTERM', 143], ['SIGINT', 130]]) {
    const tempDir = makeTempDir();
    try {
      const hook = `
        set -T
        review_injected=0
        LIFECYCLE_PHASE=""
        review_inject() {
          if [ "$review_injected" -eq 0 ] && [ "$LIFECYCLE_PHASE" = finished ]; then
            if [ "$1" = 'exit "$rc"' ]; then
              review_injected=1
              kill -${sig} "$$"
            fi
          fi
          return 0
        }
        trap 'review_inject "$BASH_COMMAND"' DEBUG
      `;
      const hookPath = path.join(tempDir, 'hook.sh');
      fs.writeFileSync(hookPath, hook);

      const cli = spawnSync('bash', [MAIN_SCRIPT, 'cleanup', '--run-dir', path.join(tempDir, 'missing')], {
        encoding: 'utf8',
        timeout: 5000,
        env: { ...process.env, BASH_ENV: hookPath },
      });

      assert.equal(cli.status, expectedStatus, `CLI must exit ${expectedStatus} when interrupted at terminal exit boundary`);
      assert.match(cli.stderr, new RegExp(`Signal ${sig} received during terminal phase, exiting with status ${expectedStatus}`));
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});

test('terminal-phase signal preserves first-signal precedence at boundaries (014/AC-010 / P2)', async () => {
  for (const [firstSig, terminalSig, expectedStatus] of [['SIGINT', 'SIGTERM', 130], ['SIGTERM', 'SIGINT', 143]]) {
    const tempDir = makeTempDir();
    try {
      const runDir = path.join(tempDir, 'run');
      fs.mkdirSync(runDir, { recursive: true });
      fs.writeFileSync(path.join(runDir, 'ownership.json'), JSON.stringify({ project: 'apollo-test-boundary-precedence' }));

      const hook = `
        set -T
        review_injected=0
        LIFECYCLE_PHASE=""
        review_inject() {
          if [ "$review_injected" -eq 0 ] && [ "$LIFECYCLE_PHASE" = finished ]; then
            if [ "$1" = 'return "$final_status"' ]; then
              review_injected=1
              kill -${terminalSig} "$$"
            fi
          fi
          return 0
        }
        trap 'review_inject "$BASH_COMMAND"' DEBUG
      `;
      const hookPath = path.join(tempDir, 'hook.sh');
      fs.writeFileSync(hookPath, hook);

      const helper = spawnSync('bash', ['-c', `
        source "${LIFECYCLE_SCRIPT}"
        source "${hookPath}"
        trap 'handle_signal SIGTERM' TERM
        trap 'handle_signal SIGINT' INT
        teardown_project() { return 0; }
        handle_signal "${firstSig}"
        run_recovery_cleanup "${runDir}"
      `], {
        encoding: 'utf8',
        timeout: 5000,
      });

      assert.equal(helper.status, expectedStatus, `Terminal boundary signal must preserve first signal status ${expectedStatus}`);
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
});
