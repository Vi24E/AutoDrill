#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
WASM="$REPO_ROOT/apps/web/public/wasm/pkg/drill_wasm_bg.wasm"
GLUE="$REPO_ROOT/apps/web/public/wasm/pkg/drill_wasm.js"

needs_build=0
if [[ ! -f "$WASM" || ! -f "$GLUE" ]]; then
  needs_build=1
else
  newest=$(
    find       "$REPO_ROOT/crates/drill-core/src"       "$REPO_ROOT/crates/drill-wasm/src"       "$REPO_ROOT/crates/drill-core/Cargo.toml"       "$REPO_ROOT/crates/drill-wasm/Cargo.toml"       "$REPO_ROOT/Cargo.lock"       -type f -newer "$WASM" -print -quit
  )
  if [[ -n "$newest" ]]; then
    needs_build=1
  fi
fi

if (( needs_build == 0 )); then
  exit 0
fi

echo "Rust source is newer than the browser WASM package; rebuilding WASM."
exec "$SCRIPT_DIR/build-wasm.sh"
