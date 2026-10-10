# Development environment

## Fresh or reset Linux x86_64 workspace

Run from a checkout with Bash. On Debian 13 / Ubuntu, ask the host administrator to
install the following packages (or run this yourself when authorized):

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev curl ca-certificates xz-utils git
scripts/setup-dev.sh
source .dev/env.sh
scripts/test.sh fast
```

The default `native-tls` feature needs OpenSSL headers and pkg-config on Linux.
The compiler/build tools also support native dependencies used by the Rustls test
configuration. Setup checks these prerequisites before downloading tools. No Java,
MySQL, or Apollo installation on the host is needed.

Setup installs tools and wasm-pack’s auxiliary binary cache only under ignored `.dev/`, leaves shell profiles and global
Rust defaults alone, and fetches the existing Cargo.lock with `--locked`. Run it
again after a reset or tool-version update. Source `.dev/env.sh` in every new Bash
shell; it contains absolute paths, so rerun setup after moving the checkout. Do not
commit `.dev/`, generated `pkg/`, or build/test artifacts. A surviving cache is an
optimization, never a prerequisite. Only one setup process may run per checkout;
a stale `.dev/setup.lock` may be removed after confirming no setup is still running.

## Version policy and network

- Rust: latest **stable**, with Clippy, rustfmt, and `wasm32-unknown-unknown`, as
  required by `.agent/rules/rust.md`. Setup updates this channel on each invocation.
- Node: **24.19.0**, matching the existing CI Node 24 major.
- wasm-pack: **0.15.0**, installed with its published lockfile (`cargo install --locked`).
- rustup bootstrap: **1.29.1**. The bootstrap binary and Node archive have checked-in
  SHA-256 hashes in `scripts/dev-versions.sh`; Rust component downloads are verified
  by rustup, and crate downloads by Cargo.

Resolved versions are recorded in `.dev/versions.txt`. Stable Rust and host packages
intentionally float: this is repeatable recovery, not a hermetic OS image. For
**diagnosing a past run only**, use the exact recorded compiler version:

```bash
APOLLO_RUST_TOOLCHAIN=1.98.1 scripts/setup-dev.sh
source .dev/env.sh
```

Rerun without the override before validating a change under the latest-stable rule.
Update auxiliary pins deliberately, verify the Node/rustup hashes from their official
release locations, rerun setup twice and all relevant checks, and record versions.
Do not update Cargo.lock or container image digests just to bootstrap a workspace.

Initial setup requires HTTPS access to `static.rust-lang.org`, `nodejs.org`,
`index.crates.io` and `static.crates.io`. wasm-pack may fetch matching wasm-bindgen
and wasm-opt binaries from GitHub during builds/tests. Integration tests additionally
pull the pinned images from Docker Hub. Proxy/firewall failures are blockers, not
permission to substitute unverified binaries or skip required tests.

Official auxiliary artifacts:
- [Node 24.19.0 checksums](https://nodejs.org/dist/v24.19.0/SHASUMS256.txt)
- [rustup 1.29.1 checksum](https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init.sha256)
- [wasm-pack crate](https://crates.io/crates/wasm-pack/0.15.0)

## Commands and runtime boundary

```bash
scripts/build.sh                    # native build + generated Node/WASM package smoke
scripts/test.sh fast                # Clippy x3, native/rustls/doc/WASM/tooling/docs checks
scripts/test.sh                     # fast checks + all real Apollo integration suites
scripts/test.sh integration         # real Apollo only
scripts/test.sh integration --suite native  # or rustls / wasm
```

Fast tests need Node and wasm-pack but never Docker. Full/integration tests require
an already configured, accessible Docker daemon plus Compose v2.20+, about 2–3 GiB
free RAM and capacity for container/build caches. Setup does not install Docker,
modify group memberships/socket permissions, or start services. Use the existing
[real Apollo testing guide](../tests/apollo/README.md) for digest-pinned MySQL 8.4.11,
Apollo 2.5.2, isolated ports/fixtures, diagnostics, and scoped cleanup. No personal
Apollo credentials are needed. Always distinguish fast-only evidence from full tests.

Automatic bootstrap is limited to Linux x86_64. On other existing supported test
platforms, install stable Rust and the WASM target, Node 24 and wasm-pack manually
using the same versions, then use the existing build/test entry points. This change
does not claim new macOS, ARM, or Windows bootstrap validation.
