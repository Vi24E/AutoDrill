#!/usr/bin/env bash

# Build the Rust/WASM package without changing the caller's Rust toolchain.
# The target and wasm-bindgen tooling must already be installed; this script
# deliberately does not run `rustup target add` or allow wasm-pack to install
# global components.

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TARGET="wasm32-unknown-unknown"
OUTPUT_DIR="$REPO_ROOT/apps/web/public/wasm/pkg"
OUTPUT_RELATIVE="../../apps/web/public/wasm/pkg"

if ! command -v rustup >/dev/null 2>&1; then
  echo "error: rustup is required to verify the $TARGET target" >&2
  exit 1
fi

if ! rustup target list --installed | grep -Fxq "$TARGET"; then
  cat >&2 <<EOF
error: Rust target $TARGET is not installed.
Install it in your toolchain outside this script, then retry:
  rustup target add $TARGET
EOF
  exit 2
fi

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack is required (install it before running this script)" >&2
  exit 1
fi

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  cat >&2 <<'EOF'
error: wasm-bindgen CLI is required in PATH.
Install a version matching the Cargo wasm-bindgen dependency before retrying;
this script never installs global tooling.
EOF
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

# `--mode no-install` is a second guard against wasm-pack changing the Rust
# toolchain. The generated package is intentionally ignored by Git and loaded
# by the browser seam in src/wasm/load-generated.ts.
exec wasm-pack build "$REPO_ROOT/crates/drill-wasm" \
  --target web \
  --release \
  --mode no-install \
  --no-typescript \
  --out-dir "$OUTPUT_RELATIVE"
