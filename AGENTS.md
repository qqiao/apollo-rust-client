# Apollo Rust client agent guide

Follow ALL rules in `.agent/**/*.md`, especially the latest-stable Rust and mandatory
spec-driven development rules. Read `spec/README.md` and applicable feature specs
before changing behavior; inspect relevant `.agents/skills/*/SKILL.md` as needed.

## Start here

- Native Rust/Node-WASM Apollo client, edition 2024; library sources in `src/`.
- `src/lib.rs` exposes the client; `src/cache.rs` handles persistence and refresh;
  `src/namespace/` contains typed formats. Tests also live alongside Rust modules.
- `tests/tooling/` contains Node regression tests; `tests/apollo/` owns real-server
  fixtures, pinned Compose services, and generated-WASM integration checks.
- `scripts/build.sh` and `scripts/test.sh` are the public verification entry points.
- Read [development setup](docs/development.md). On prepared Linux x86_64 hosts run
  `scripts/setup-dev.sh`, then `source .dev/env.sh` in each Bash shell. After workspace
  resets, rerun setup rather than assuming cached tools or service access still exist.

## Verification and boundaries

- Use `scripts/test.sh fast` for Docker-independent checks; `scripts/test.sh` includes
  the real Apollo suite. Report which ran, exact failures, and missing prerequisites.
- Use Clippy rather than `cargo check`; do not replace the test entry point with an
  ad hoc selection of `cargo test` commands. `scripts/build.sh` validates both outputs.
- Preserve native-tls, Rustls, and WASM coverage. Rustls and default native-tls are
  mutually exclusive; WASM uses the default feature configuration.
- Follow specification → plan → tasks → implementation → verification. Reference
  stable feature/requirement IDs and keep documentation aligned with changes.
- Do not change dependency locks, container pins, authentication, or service lifecycle
  as an incidental environment fix. Keep generated `.dev/`, `target/`, and `pkg/`
  artifacts out of commits. Never claim full integration success from fast tests.
