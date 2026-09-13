# Executor rules and common verification

## Contract hierarchy and status

The original specifications 001–004 remain retrospective drafts. This remediation set specifies the narrow outcomes requested by the review follow-up; it does not approve every earlier inferred policy. Package specifications describe intended outcomes, plans choose implementations, and task checkboxes track actual work. A request to implement a package authorizes its ordinary source/test/documentation steps; this handoff itself does not implement anything.

Read source symbols and existing tests before editing. Existing tests encode useful behavior, but the new acceptance scenarios cover gaps they missed. Preserve test counts/coverage qualitatively: added tests should increase coverage; do not freeze baseline counts or delete a failure to get green.

## Scope and engineering constraints

- Use the installed latest stable Rust, Clippy instead of cargo check, and the repository's Rust test entry point. Do not update dependencies or toolchains merely because this is a remediation task.
- No new dependency is expected. Rust tests can use existing Tokio/futures/serde/test fixtures; Node tests can use `node:test`, `assert`, `http`, `child_process`, `fs`, and `os`; shell must remain compatible with Bash 3.2 on macOS.
- Preserve native-TLS and Rustls as the existing separate native configurations, and WASM as its existing target. Do not use `--all-features`: native TLS flags are mutually exclusive and Rustls is rejected on WASM.
- Rust public documentation must compile. Use builder configuration examples. Use `Result`/`?` and explicit typed assertions rather than broad error-string matching where the API supports types.
- Keep temporary test artifacts isolated, clean up local servers/child processes even when assertions fail, and put a real deadline around fault tests. No sleeping and hoping a race happened.
- Prefer 1–5 files per task. If discovery widens a task, split it into named follow-ups within the same package before editing broadly.
- Do not make all `cache` internals public, change wire protocol, introduce retries to hide failed checks, rewrite schedulers, or normalize error shapes outside the selected spec.
- Do not read or edit ignored build outputs as source. Generated WASM can be built into a run-owned temporary directory for behavioral verification. Do not hand-edit generated bindings.
- Do not publish, push, merge, or administer external Apollo. Local integration uses the repository's disposable loopback Docker stack.

## Commands that exist at the review baseline

Run from the repository root:

```sh
scripts/test.sh fast
scripts/test.sh integration --suite native
scripts/test.sh integration --suite rustls
scripts/test.sh integration --suite wasm
scripts/test.sh
scripts/build.sh
cargo fmt --all -- --check
git diff --check
sh -n scripts/test.sh scripts/build.sh
bash -n scripts/apollo-test.sh tests/apollo/proof.sh
node --check scripts/apollo-fixtures.mjs
node --check scripts/wasm_api_smoke.js
node --check tests/apollo/wasm.cjs
```

`fast` takes no filters/options. `integration --filter <name>` exists for real-server cases; it is **not** a filter for the new native unit regressions. Use `scripts/test.sh fast` for those. Do not invent a new fast-filter interface just for this work.

The build script removes/regenerates `pkg`; inspect status before running and preserve unrelated user artifacts. Use `mktemp -d` plus `wasm-pack build --target nodejs --dev --out-dir "$task_package_dir"` when verifying bindings without that side effect; the package-008 plan defines the smoke command's optional package path. That optional argument does not exist until 008-T01 is implemented.

## Planned check commands — not available until their owning tasks land

- `node --test tests/tooling/*.test.mjs`: Node fault tests introduced by 009 and 010. Their scripts must be invoked by **both** existing fast-check paths, and failure must propagate. Until 009 lands, this glob may have no matches; do not call that a pass.
- `node scripts/check-doc-examples.mjs`: selected current Rust Markdown snippets compile through rustdoc; introduced by 011. No placeholder implementation or copying snippets manually into tests.
- `node scripts/check-doc-links.mjs`: local link checker introduced by 011. Scope, fragment handling, and exclusions are defined there.

These checks are Docker-independent, but Rust/WASM tools and dependencies remain prerequisites. A network/runtime failure is a blocked check with its actual diagnostic, not proof of a source regression or a passing suite.

## Evidence to record per package

Append a section to its tasks.md containing: revision/diff scope; platform and relevant tool versions; test names; exact commands; observed pre-fix failure and post-fix pass for behavior changes; expected failure/success statuses for fault probes; ordinary regression results; documentation/link checks; environmental gaps. Summarize outputs rather than paste sensitive payloads or huge logs. Relative repository test paths and scenario IDs should be sufficient to reproduce results on another machine.

For documentation-only corrections, use compiler/link/generated-package checks as appropriate; no need to provision Docker unless changed runtime tooling or behavior requires it. Do not repeatedly rerun identical broad suites after only prose changes.

## Final integration checkpoint

After all selected implementation packages have landed:

1. Run `scripts/test.sh` once for the complete fast + real-Apollo matrix, including the new Docker-independent tooling/doc checks. Record the actual totals and confirm required suites did not select zero tests.
2. Run `scripts/build.sh` for generated export/ownership smoke and native build, preserving unrelated files as described above.
3. Run the two document checkers, syntax checks for changed scripts, and `git diff --check`; verify they are already part of the appropriate fast path.
4. Run formatting check. Format changed Rust files/sections as appropriate; if unrelated pre-existing formatting still fails, enumerate it explicitly. Do not claim full formatting success unless the command passes, or expand scope silently to erase the baseline.
5. Verify every package's FR/AC/SC mapping, no unresolved checkbox is called complete, and current docs describe the implemented APIs. Remove planned labels for completed portions only; retain historical evidence and deferred decisions.
6. Inspect final diff: no unplanned dependency/lockfile changes, no generated package binaries committed, no accidental global Docker cleanup, no hidden fail-open checks, no test-only hooks in production builds.

CI runs are separate evidence. Do not claim Linux CI, fork-PR, browser, or Windows success from a local macOS run.
