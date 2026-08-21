#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

node "$SCRIPT_DIR/verify-js-toolchain.mjs"

EXPECTED_RUST=$(awk -F '"' '/^channel = / { print $2; exit }' "$REPO_ROOT/rust-toolchain.toml")
ACTUAL_RUST=$(rustc --version | awk '{ print $2 }')
if [[ -z "$EXPECTED_RUST" || "$ACTUAL_RUST" != "$EXPECTED_RUST" ]]; then
  echo "error: Rust toolchain mismatch: expected ${EXPECTED_RUST:-<missing>}, got ${ACTUAL_RUST:-<missing>}" >&2
  exit 1
fi

# shellcheck source=wasm-toolchain.sh
source "$SCRIPT_DIR/wasm-toolchain.sh"
verify_wasm_tools "$REPO_ROOT"

printf 'Toolchain verified: Rust %s, wasm-pack %s, wasm-bindgen %s.\n' \
  "$ACTUAL_RUST" \
  "$WASM_PACK_VERSION" \
  "$(wasm_bindgen_lock_version "$REPO_ROOT/Cargo.lock")"
