#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
# shellcheck source=wasm-toolchain.sh
source "$SCRIPT_DIR/wasm-toolchain.sh"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required" >&2
  exit 1
fi

BINDGEN_VERSION="$(wasm_bindgen_lock_version "$REPO_ROOT/Cargo.lock")"
if [[ -z "$BINDGEN_VERSION" ]]; then
  echo "error: could not resolve wasm-bindgen from Cargo.lock" >&2
  exit 1
fi

if [[ "$(wasm_tool_version wasm-pack || true)" != "$WASM_PACK_VERSION" ]]; then
  cargo install wasm-pack --version "$WASM_PACK_VERSION" --locked
fi

if [[ "$(wasm_tool_version wasm-bindgen || true)" != "$BINDGEN_VERSION" ]]; then
  cargo install wasm-bindgen-cli --version "$BINDGEN_VERSION" --locked
fi

verify_wasm_tools "$REPO_ROOT"
printf 'WASM tools verified: wasm-pack %s, wasm-bindgen %s.\n' "$WASM_PACK_VERSION" "$BINDGEN_VERSION"
