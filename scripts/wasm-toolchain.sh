#!/usr/bin/env bash

# Canonical versions and checks for cargo-installed WASM build tools.
# wasm-bindgen-cli must match the resolved wasm-bindgen crate exactly, so its
# version is derived from Cargo.lock instead of being duplicated here.

WASM_PACK_VERSION="0.15.0"

wasm_bindgen_lock_version() {
  local lockfile=$1
  awk '
    $0 == "name = \"wasm-bindgen\"" { in_package = 1; next }
    in_package && $1 == "version" && $2 == "=" {
      gsub(/\"/, "", $3)
      print $3
      exit
    }
    in_package && $0 == "[[package]]" { exit 1 }
  ' "$lockfile"
}

wasm_tool_version() {
  local command_name=$1
  "$command_name" --version 2>/dev/null | awk '{ print $2 }'
}

verify_wasm_tools() {
  local repo_root=$1
  local expected_bindgen
  expected_bindgen="$(wasm_bindgen_lock_version "$repo_root/Cargo.lock")"
  if [[ -z "$expected_bindgen" ]]; then
    echo "error: could not resolve wasm-bindgen from Cargo.lock" >&2
    return 1
  fi

  if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "error: wasm-pack $WASM_PACK_VERSION is required" >&2
    return 1
  fi
  local actual_wasm_pack
  actual_wasm_pack="$(wasm_tool_version wasm-pack)"
  if [[ "$actual_wasm_pack" != "$WASM_PACK_VERSION" ]]; then
    echo "error: wasm-pack version mismatch: expected $WASM_PACK_VERSION, got $actual_wasm_pack" >&2
    return 1
  fi

  if ! command -v wasm-bindgen >/dev/null 2>&1; then
    echo "error: wasm-bindgen CLI $expected_bindgen is required" >&2
    return 1
  fi
  local actual_bindgen
  actual_bindgen="$(wasm_tool_version wasm-bindgen)"
  if [[ "$actual_bindgen" != "$expected_bindgen" ]]; then
    echo "error: wasm-bindgen CLI mismatch: Cargo.lock requires $expected_bindgen, got $actual_bindgen" >&2
    return 1
  fi
}
