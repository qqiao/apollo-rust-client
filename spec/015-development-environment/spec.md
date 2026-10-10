# 015 — Recoverable development environment

Status: proposed implementation of the requested environment setup; review in the draft PR.

## Purpose and scope

Contributors and coding agents should recover the native and Node/WASM development
workflow after a disposable Linux workspace resets. This extends
[005/FR-014](../005-real-apollo-testing/spec.md) without changing client behavior,
container images, dependency versions, or the latest-stable Rust policy.

## Prioritized stories and independent acceptance tests

### P1: Bootstrap a fresh checkout
- **AC-001:** Given Linux x86_64 and the documented system packages, when a contributor
  runs setup, then the checkout contains Rust stable with Clippy/rustfmt/WASM target,
  the pinned Node and wasm-pack versions, locked Cargo dependencies, and activation instructions.
- **AC-002:** Given successful setup, when setup runs again, then it reuses compatible
  installed tools without changing shell profiles or the global Rust default.

### P1: Diagnose limits honestly
- **AC-003:** Given missing prerequisites, an unsupported host, or a failed download,
  when setup runs, then it fails clearly without claiming build/test success.
- **AC-004:** Given no Docker, when setup runs, then native/WASM preparation remains
  available, and full integration tests remain explicitly dependent on Docker Compose.

## Functional requirements
- **FR-001:** Bootstrap MUST isolate its writable tools in ignored checkout-local storage.
- **FR-002:** Downloaded standalone tool installers/archives MUST be checksum verified.
- **FR-003:** Setup MUST follow the latest-stable Rust rule and record exact resolved versions;
  an explicit version override MAY reproduce a previous run for diagnosis.
- **FR-004:** Setup MUST fetch Cargo dependencies with the checked-in lockfile and never
  install Docker, change permissions, start services, or run integration tests implicitly.
- **FR-005:** Contributor and agent instructions MUST distinguish setup, build, fast tests,
  and full tests, and record network, native-library, and container prerequisites.

## Key entities
- Tool selection: checked-in Node/wasm-pack/installer versions and selected Rust channel.
- Local environment: disposable tool cache, activation script, and resolved version record.

## Success criteria
- **SC-001:** Two successive setup runs succeed on the supported host with prerequisites.
- **SC-002:** Offline regression checks cover help/argument validation, missing prerequisites,
  cached-tool reuse, lockfile fetching, and failure propagation without real installations.
- **SC-003:** Documentation and available repository checks pass; unavailable network/runtime
  verification is reported rather than represented as a pass.

## Assumptions
Initial network access is needed. System libraries and Docker are host-managed. Stable
Rust and distribution packages float intentionally, so this is repeatable setup rather
than a byte-for-byte hermetic OS image. Other platforms retain the existing manual workflow.
