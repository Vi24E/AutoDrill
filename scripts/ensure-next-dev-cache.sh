#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
WEB_ROOT="$REPO_ROOT/apps/web"
CACHE_DIR="$WEB_ROOT/.next-dev"
SIGNATURE_FILE="$CACHE_DIR/.autodrill-source-signature"

# Turbopack normally invalidates its own cache, but module renames/removals have
# previously left a deleted PDF implementation reachable from a stale dev
# chunk. Compute a cheap source signature at process startup and discard the
# dev cache only when the source/config surface changed since the last start.
source_signature=$(
  {
    find "$WEB_ROOT/src" -type f -print0
    printf '%s\0' \
      "$WEB_ROOT/package.json" \
      "$WEB_ROOT/next.config.mjs" \
      "$REPO_ROOT/pnpm-lock.yaml"
  } \
    | sort -z \
    | xargs -0 shasum -a 256 \
    | shasum -a 256 \
    | awk '{print $1}'
)

cached_signature=''
if [[ -f "$SIGNATURE_FILE" ]]; then
  cached_signature=$(cat "$SIGNATURE_FILE")
fi

if [[ "$cached_signature" == "$source_signature" ]]; then
  exit 0
fi

if [[ -d "$CACHE_DIR" ]]; then
  echo "Web source/config changed since the previous dev start; clearing stale Turbopack cache."
  rm -rf "$CACHE_DIR"
fi
mkdir -p "$CACHE_DIR"
printf '%s\n' "$source_signature" > "$SIGNATURE_FILE"
