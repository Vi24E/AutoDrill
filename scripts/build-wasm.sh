#!/usr/bin/env bash

# Build the Rust/WASM package without changing the caller's Rust toolchain.
# The target and wasm-bindgen tooling must already be installed; this script
# deliberately does not run `rustup target add` or allow wasm-pack to install
# global components.

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
# shellcheck source=wasm-toolchain.sh
source "$SCRIPT_DIR/wasm-toolchain.sh"
TARGET="wasm32-unknown-unknown"
BUILD_MODE="${1:-web}"
case "$BUILD_MODE" in
  web)
    OUTPUT_DIR="$REPO_ROOT/apps/web/public/wasm/pkg"
    OUTPUT_RELATIVE="../../apps/web/public/wasm/pkg"
    ;;
  qa)
    OUTPUT_DIR="$REPO_ROOT/apps/qa/wasm"
    OUTPUT_RELATIVE="../../apps/qa/wasm"
    ;;
  *)
    echo "error: build mode must be 'web' or 'qa'" >&2
    exit 64
    ;;
esac

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

if ! verify_wasm_tools "$REPO_ROOT"; then
  echo "Install the repository-pinned tools with scripts/install-wasm-tools.sh, then retry." >&2
  exit 1
fi

rm -rf -- "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# The browser consumes the generated files directly; this is not an npm package.
# `--no-pack` avoids package metadata/license side effects, while cleaning the
# output directory first prevents stale glue/types from surviving upgrades.
# `--mode no-install` is a second guard against wasm-pack changing the Rust
# toolchain. The generated package is intentionally ignored by Git and loaded
# by the browser seam in src/wasm/load-generated.ts.
if [ "$BUILD_MODE" = "qa" ]; then
  exec wasm-pack build "$REPO_ROOT/crates/drill-wasm" \
    --target web \
    --release \
    --mode no-install \
    --no-typescript \
    --no-pack \
    --out-dir "$OUTPUT_RELATIVE" \
    -- --features qa-diagnostics
fi

exec wasm-pack build "$REPO_ROOT/crates/drill-wasm" \
  --target web \
  --release \
  --mode no-install \
  --no-typescript \
  --no-pack \
  --out-dir "$OUTPUT_RELATIVE"
