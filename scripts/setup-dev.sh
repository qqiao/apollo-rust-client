#!/usr/bin/env bash
# Bootstrap disposable Linux x86_64 checkouts without changing the host toolchain.
set -euo pipefail

usage() {
  cat <<'HELP'
Usage: scripts/setup-dev.sh [--help]
Install checkout-local development tools and fetch locked Cargo dependencies.
Prerequisites (Debian/Ubuntu): build-essential pkg-config libssl-dev curl
  ca-certificates xz-utils git
Docker is not installed or contacted. See docs/development.md for full tests.
Optional: APOLLO_RUST_TOOLCHAIN=<exact-version> to reproduce a recorded run.
HELP
}
if [[ $# -gt 0 ]]; then
  if [[ $# == 1 && ( $1 == --help || $1 == -h ) ]]; then usage; exit 0; fi
  usage >&2; exit 2
fi
fail() { printf 'setup-dev: %s\n' "$*" >&2; exit 1; }
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=dev-versions.sh
source "$ROOT/scripts/dev-versions.sh"
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] ||
  fail 'Automatic setup supports Linux x86_64 only; see docs/development.md.'
for tool in cc make pkg-config curl tar xz sha256sum git; do
  command -v "$tool" >/dev/null || fail "Missing $tool. Install the prerequisites shown by --help."
done
pkg-config --exists openssl || fail 'Missing OpenSSL development files (Debian/Ubuntu: libssl-dev).'
TOOLCHAIN=${APOLLO_RUST_TOOLCHAIN:-stable}
[[ $TOOLCHAIN == stable || $TOOLCHAIN =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
  fail 'APOLLO_RUST_TOOLCHAIN must be stable or an exact release such as 1.98.1.'
DEV="$ROOT/.dev"
export CARGO_HOME="$DEV/cargo" RUSTUP_HOME="$DEV/rustup"
export RUSTUP_TOOLCHAIN="$TOOLCHAIN"
export WASM_PACK_CACHE="$DEV/wasm-pack"
export PATH="$DEV/node/bin:$CARGO_HOME/bin:$PATH"
mkdir -p "$DEV"
# A failed/concurrent installer must never expose a partially extracted Node directory.
mkdir "$DEV/setup.lock" 2>/dev/null || fail 'Setup already running, or stale .dev/setup.lock; inspect before removing it.'
TMP=$(mktemp -d "$DEV/setup.XXXXXX")
cleanup() { rm -rf -- "$TMP"; rmdir "$DEV/setup.lock"; }
trap cleanup EXIT
fetch() { curl --fail --location --silent --show-error --retry 3 --connect-timeout 30 --max-time 600 "$1" -o "$2"; }
verify() { printf '%s  %s\n' "$1" "$2" | sha256sum --check --status || fail "Checksum mismatch: $2"; }

if [[ ! -x $CARGO_HOME/bin/rustup ]]; then
  fetch "https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/x86_64-unknown-linux-gnu/rustup-init" "$TMP/rustup-init"
  verify "$RUSTUP_SHA256" "$TMP/rustup-init"
  chmod +x "$TMP/rustup-init"
  "$TMP/rustup-init" -y --no-modify-path --profile minimal --default-toolchain none
fi
"$CARGO_HOME/bin/rustup" toolchain install "$TOOLCHAIN" --profile minimal --component clippy --component rustfmt --target wasm32-unknown-unknown --no-self-update
if [[ ! -x $DEV/node/bin/node ]] || [[ $("$DEV/node/bin/node" --version) != "v$NODE_VERSION" ]]; then
  fetch "https://nodejs.org/dist/v$NODE_VERSION/node-v$NODE_VERSION-linux-x64.tar.xz" "$TMP/node.tar.xz"
  verify "$NODE_SHA256" "$TMP/node.tar.xz"
  mkdir "$TMP/node"
  tar -xJf "$TMP/node.tar.xz" -C "$TMP/node" --strip-components=1
  [[ $("$TMP/node/bin/node" --version) == "v$NODE_VERSION" ]] || fail 'Unexpected Node version.'
  rm -rf -- "$DEV/node"
  mv "$TMP/node" "$DEV/node"
fi
if [[ ! -x $CARGO_HOME/bin/wasm-pack ]] || [[ $("$CARGO_HOME/bin/wasm-pack" --version) != "wasm-pack $WASM_PACK_VERSION" ]]; then
  cargo install wasm-pack --version "=$WASM_PACK_VERSION" --locked --root "$CARGO_HOME"
fi
(cd "$ROOT" && cargo fetch --locked)
# %q keeps spaces and shell metacharacters in checkout paths safe; source with Bash.
{
  printf 'export CARGO_HOME=%q\n' "$CARGO_HOME"
  printf 'export RUSTUP_HOME=%q\n' "$RUSTUP_HOME"
  printf 'export RUSTUP_TOOLCHAIN=%q\n' "$TOOLCHAIN"
  printf 'export WASM_PACK_CACHE=%q\n' "$WASM_PACK_CACHE"
  printf 'export PATH=%q:%q:"$PATH"\n' "$DEV/node/bin" "$CARGO_HOME/bin"
} > "$DEV/env.sh"
{
  rustc --version
  cargo --version
  node --version
  wasm-pack --version
  printf 'OpenSSL %s\n' "$(pkg-config --modversion openssl)"
} > "$DEV/versions.txt"
cat "$DEV/versions.txt"
printf '\nSetup complete. Activate in each Bash shell:\n  source %q\n' "$DEV/env.sh"
printf 'Then: scripts/test.sh fast or scripts/build.sh\nFull tests additionally require a working Docker daemon and Compose v2.20+.\n'
