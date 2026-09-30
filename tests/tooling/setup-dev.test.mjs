import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo setup $safe '));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.mkdirSync(path.join(root, 'scripts'));
  for (const name of ['setup-dev.sh', 'dev-versions.sh']) {
    fs.copyFileSync(new URL(`../../scripts/${name}`, import.meta.url), path.join(root, 'scripts', name));
  }
  const bin = path.join(root, 'bin');
  fs.mkdirSync(bin);
  const executable = (name, body) => {
    fs.mkdirSync(path.dirname(name), { recursive: true });
    fs.writeFileSync(name, `#!/bin/bash\n${body}\n`, { mode: 0o755 });
  };
  executable(path.join(bin, 'uname'), 'if [[ $1 == -s ]]; then echo Linux; else echo x86_64; fi');
  executable(path.join(bin, 'pkg-config'), '[[ ${MISSING_SSL:-0} == 1 ]] && exit 1; echo 3.5.7');
  for (const name of ['cc', 'make', 'git']) executable(path.join(bin, name), 'exit 0');
  executable(path.join(bin, 'curl'), 'echo unexpected-download >&2; exit 97');
  const cargoBin = path.join(root, '.dev/cargo/bin');
  for (const name of ['cargo', 'rustup', 'rustc']) {
    executable(path.join(cargoBin, name), `echo '${name}' ":$*" >> "$SETUP_LOG"
if [[ $1 == fetch && \${FAIL_FETCH:-0} == 1 ]]; then exit 42; fi
echo '${name} test-version'`);
  }
  executable(path.join(cargoBin, 'wasm-pack'), 'echo "wasm-pack 0.15.0"');
  executable(path.join(root, '.dev/node/bin/node'), 'echo v24.19.0');
  const env = { ...process.env, PATH: `${bin}:${process.env.PATH}`, SETUP_LOG: path.join(root, 'calls'), APOLLO_RUST_TOOLCHAIN: 'stable' };
  const run = (args = [], overrides = {}) => spawnSync('/bin/bash', [path.join(root, 'scripts/setup-dev.sh'), ...args], { env: { ...env, ...overrides }, encoding: 'utf8' });
  return { root, env, run };
}

test('setup help and invalid arguments never install tools', (t) => {
  const f = fixture(t);
  assert.equal(f.run(['--help']).status, 0);
  assert.equal(f.run(['--unknown']).status, 2);
  assert.equal(fs.existsSync(f.env.SETUP_LOG), false);
});

test('setup fails clearly for unsupported hosts and missing OpenSSL', (t) => {
  const f = fixture(t);
  const missing = f.run([], { MISSING_SSL: '1' });
  assert.notEqual(missing.status, 0);
  assert.match(missing.stderr, /libssl-dev/);
  fs.writeFileSync(path.join(f.root, 'bin/uname'), '#!/bin/bash\necho Darwin\n', { mode: 0o755 });
  assert.match(f.run().stderr, /Linux x86_64 only/);
});

test('setup reuses tools, fetches locked dependencies, and safely activates paths', (t) => {
  const f = fixture(t);
  for (let i = 0; i < 2; i++) {
    const result = f.run();
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /Full tests additionally require/);
  }
  const calls = fs.readFileSync(f.env.SETUP_LOG, 'utf8');
  assert.equal((calls.match(/cargo :fetch --locked/g) || []).length, 2);
  assert.match(calls, /toolchain install stable.*wasm32-unknown-unknown.*--no-self-update/);
  assert.doesNotMatch(calls, /cargo :install/);
  const activation = spawnSync('/bin/bash', ['-c', 'source "$1/.dev/env.sh"; printf "%s" "$CARGO_HOME"', '_', f.root], { encoding: 'utf8' });
  assert.equal(activation.stdout, path.join(f.root, '.dev/cargo'));
  assert.equal(fs.existsSync(path.join(f.root, '.dev/setup.lock')), false);
});

test('setup propagates Cargo errors and does not publish a successful activation', (t) => {
  const f = fixture(t);
  assert.equal(f.run([], { FAIL_FETCH: '1' }).status, 42);
  assert.equal(fs.existsSync(path.join(f.root, '.dev/env.sh')), false);
  assert.equal(fs.existsSync(path.join(f.root, '.dev/setup.lock')), false);
});

test('setup rejects malformed toolchain overrides before installation', (t) => {
  const f = fixture(t);
  assert.match(f.run([], { APOLLO_RUST_TOOLCHAIN: '../../bad' }).stderr, /exact release/);
  assert.equal(fs.existsSync(f.env.SETUP_LOG), false);
});

test('setup rejects a corrupt downloaded archive before replacing an installed Node', (t) => {
  const f = fixture(t);
  const node = path.join(f.root, '.dev/node/bin/node');
  fs.writeFileSync(node, '#!/bin/bash\necho v0.0.0\n');
  fs.writeFileSync(path.join(f.root, 'bin/curl'), '#!/bin/bash\nprintf corrupt > "${@: -1}"\n');
  const result = f.run();
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Checksum mismatch/);
  assert.match(fs.readFileSync(node, 'utf8'), /v0.0.0/);
  assert.equal(fs.existsSync(path.join(f.root, '.dev/env.sh')), false);
});

test('setup refuses concurrent or stale setup locks', (t) => {
  const f = fixture(t);
  fs.mkdirSync(path.join(f.root, '.dev/setup.lock'));
  assert.match(f.run().stderr, /already running, or stale/);
  assert.equal(fs.existsSync(f.env.SETUP_LOG), false);
});

test('wasm-pack installation explicitly overrides an inherited global install root', (t) => {
  const f = fixture(t);
  fs.writeFileSync(path.join(f.root, '.dev/cargo/bin/wasm-pack'), '#!/bin/bash\necho "wasm-pack old"\n');
  const result = f.run([], { CARGO_INSTALL_ROOT: '/must-not-install-here' });
  assert.equal(result.status, 0, result.stderr);
  const calls = fs.readFileSync(f.env.SETUP_LOG, 'utf8');
  assert.ok(calls.includes(`cargo :install wasm-pack --version =0.15.0 --locked --root ${f.root}/.dev/cargo`));
  const activation = spawnSync('/bin/bash', ['-c', 'source "$1/.dev/env.sh"; printf "%s" "$WASM_PACK_CACHE"', '_', f.root], { encoding: 'utf8' });
  assert.equal(activation.stdout, path.join(f.root, '.dev/wasm-pack'));
});
