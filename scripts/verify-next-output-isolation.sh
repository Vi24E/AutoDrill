#!/usr/bin/env bash

# End-to-end regression for the phase-specific Next.js output directories.
# It intentionally exercises next dev, an overlapping next build, and next
# start instead of treating a config unit test as proof of process isolation.

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
WEB_DIR="$REPO_ROOT/apps/web"
DEV_PORT=${AUTODRILL_VERIFY_DEV_PORT:-3100}
PROD_PORT=${AUTODRILL_VERIFY_PROD_PORT:-3101}
DEV_ORIGIN="http://127.0.0.1:$DEV_PORT"
PROD_ORIGIN="http://127.0.0.1:$PROD_PORT"
TEMP_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/autodrill-next-verify.XXXXXX")
DEV_PID=""
BUILD_PID=""
PROD_PID=""
OUTPUTS_RESTORED=false
OUTPUTS_STASHED=false
HAD_NEXT_OUTPUT=false
HAD_NEXT_DEV_OUTPUT=false

stop_process() {
  local pid=$1
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}

cleanup() {
  local exit_status=$?
  trap - EXIT INT TERM
  stop_process "$BUILD_PID"
  stop_process "$DEV_PID"
  stop_process "$PROD_PID"

  if [[ "$OUTPUTS_STASHED" != true ]]; then
    OUTPUTS_RESTORED=true
  elif [[ "$OUTPUTS_RESTORED" != true ]]; then
    local restore_failed=false
    local output_name
    local generated_path
    local preexisting_path
    local had_preexisting
    for output_name in .next .next-dev; do
      generated_path="$TEMP_ROOT/generated-${output_name#.}"
      preexisting_path="$TEMP_ROOT/preexisting-${output_name#.}"
      case "$output_name" in
        .next) had_preexisting=$HAD_NEXT_OUTPUT ;;
        .next-dev) had_preexisting=$HAD_NEXT_DEV_OUTPUT ;;
      esac

      if [[ "$had_preexisting" == true ]] && [[ ! -e "$preexisting_path" ]]; then
        echo "warning: pre-existing output was not stashed; leaving it untouched: $WEB_DIR/$output_name" >&2
        restore_failed=true
        continue
      fi

      if [[ -e "$WEB_DIR/$output_name" ]]; then
        if [[ -e "$generated_path" ]] || ! mv "$WEB_DIR/$output_name" "$generated_path"; then
          echo "warning: could not move generated output before restoration: $WEB_DIR/$output_name" >&2
          restore_failed=true
          continue
        fi
      fi
      if [[ -e "$preexisting_path" ]]; then
        if [[ -e "$WEB_DIR/$output_name" ]] || ! mv "$preexisting_path" "$WEB_DIR/$output_name"; then
          echo "warning: could not restore pre-existing output: $WEB_DIR/$output_name" >&2
          restore_failed=true
        fi
      fi
    done
    if [[ "$restore_failed" == false ]]; then
      OUTPUTS_RESTORED=true
    fi
  fi

  if [[ "$OUTPUTS_RESTORED" == true ]]; then
    case "$TEMP_ROOT" in
      "${TMPDIR:-/tmp}"/autodrill-next-verify.*) rm -rf -- "$TEMP_ROOT" ;;
      *) echo "warning: refusing to remove unexpected temp path: $TEMP_ROOT" >&2 ;;
    esac
  else
    echo "warning: preserving recovery directory after incomplete restore: $TEMP_ROOT" >&2
    exit_status=1
  fi
  exit "$exit_status"
}
trap cleanup EXIT INT TERM

for command_name in curl grep lsof node ps sed sort pnpm; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "error: $command_name is required" >&2
    exit 1
  fi
done

assert_port_available() {
  local port=$1
  if ! node -e '
    const net = require("node:net");
    const server = net.createServer();
    server.once("error", () => process.exit(1));
    server.once("listening", () => server.close(() => process.exit(0)));
    server.listen(Number(process.argv[1]), "127.0.0.1");
  ' "$port"; then
    echo "error: verification port is already in use: $port" >&2
    exit 1
  fi
}

assert_output_not_in_use() {
  local output_path=$1
  local scan_name=${output_path##*/}
  local scan_output="$TEMP_ROOT/lsof-${scan_name#.}.out"
  local scan_error="$TEMP_ROOT/lsof-${scan_name#.}.err"
  local scan_status
  if [[ ! -d "$output_path" ]]; then
    return 0
  fi
  if lsof -t +D "$output_path" > "$scan_output" 2> "$scan_error"; then
    scan_status=0
  else
    scan_status=$?
  fi
  if [[ -s "$scan_error" ]]; then
    echo "error: could not prove that Next output is unused: $output_path" >&2
    sed 's/^/  lsof: /' "$scan_error" >&2
    exit 1
  fi
  if [[ -s "$scan_output" ]]; then
    echo "error: Next output is in use; stop the related dev/start server first: $output_path" >&2
    exit 1
  fi
  # lsof returns 1 with no diagnostic when a valid scan finds no open files.
  # Every other empty-result status is ambiguous and therefore fails closed.
  if [[ "$scan_status" -ne 1 ]]; then
    echo "error: lsof returned an unexpected empty scan for: $output_path (status $scan_status)" >&2
    exit 1
  fi
}

assert_no_next_server_for_web() {
  local pid
  local process_command
  local process_cwd
  while IFS= read -r pid; do
    process_command=$(ps -p "$pid" -o command= 2>/dev/null || true)
    case "$process_command" in
      *"/next/dist/bin/next dev"*|*"/next/dist/bin/next start"*|"next-server "*) ;;
      *) continue ;;
    esac
    process_cwd=$(lsof -a -p "$pid" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' || true)
    if [[ "$process_cwd" == "$REPO_ROOT" || "$process_cwd" == "$REPO_ROOT/"* ]]; then
      echo "error: Next.js server is running inside the AutoDrill repository (pid $pid); stop it before verification" >&2
      exit 1
    fi
  done < <(ps -axo pid=)
}

assert_port_available "$DEV_PORT"
assert_port_available "$PROD_PORT"
assert_no_next_server_for_web
assert_output_not_in_use "$WEB_DIR/.next"
assert_output_not_in_use "$WEB_DIR/.next-dev"

if [[ -e "$WEB_DIR/.next" ]]; then
  HAD_NEXT_OUTPUT=true
fi
if [[ -e "$WEB_DIR/.next-dev" ]]; then
  HAD_NEXT_DEV_OUTPUT=true
fi
OUTPUTS_STASHED=true
for output_name in .next .next-dev; do
  if [[ -e "$WEB_DIR/$output_name" ]]; then
    mv "$WEB_DIR/$output_name" "$TEMP_ROOT/preexisting-${output_name#.}"
  fi
done

wait_for_page() {
  local url=$1
  local attempts=${2:-150}
  local index
  for ((index = 0; index < attempts; index += 1)); do
    if curl -fs -o /dev/null "$url"; then
      return 0
    fi
    sleep 0.1
  done
  echo "error: page did not become ready: $url" >&2
  return 1
}

collect_assets() {
  local url=$1
  local html_file=$2
  local assets_file=$3
  local expected_status=${4:-2xx}
  local status

  status=$(curl -sS -o "$html_file" -w '%{http_code}' "$url")
  case "$expected_status" in
    2xx)
      if [[ "$status" != 2* ]]; then
        echo "error: expected 2xx HTML from $url, got $status" >&2
        return 1
      fi
      ;;
    *)
      if [[ "$status" != "$expected_status" ]]; then
        echo "error: expected HTTP $expected_status from $url, got $status" >&2
        return 1
      fi
      ;;
  esac

  grep -oE '(href|src)="[^"]*/_next/[^"]*"' "$html_file" \
    | sed -E 's/^(href|src)="([^"]+)"$/\2/' \
    | sort -u > "$assets_file"
  if [[ ! -s "$assets_file" ]]; then
    echo "error: no Next.js assets found in $url" >&2
    return 1
  fi
}

check_assets() {
  local origin=$1
  local assets_file=$2
  local asset
  while IFS= read -r asset; do
    if ! curl -fsS -o /dev/null "$origin$asset"; then
      echo "error: asset was not available: $origin$asset" >&2
      return 1
    fi
  done < "$assets_file"
}

echo "[verify-next-output] starting clean development server on $DEV_ORIGIN"
(
  cd "$REPO_ROOT"
  exec pnpm --filter @autodrill/web exec next dev -H 127.0.0.1 -p "$DEV_PORT"
) > "$TEMP_ROOT/dev.log" 2>&1 &
DEV_PID=$!
wait_for_page "$DEV_ORIGIN/"
collect_assets "$DEV_ORIGIN/" "$TEMP_ROOT/dev-before.html" "$TEMP_ROOT/dev-assets.txt"
check_assets "$DEV_ORIGIN" "$TEMP_ROOT/dev-assets.txt"

echo "[verify-next-output] polling development assets during production build"
(
  cd "$REPO_ROOT"
  exec pnpm build
) > "$TEMP_ROOT/build.log" 2>&1 &
BUILD_PID=$!
POLL_COUNT=0
while kill -0 "$BUILD_PID" 2>/dev/null; do
  check_assets "$DEV_ORIGIN" "$TEMP_ROOT/dev-assets.txt"
  POLL_COUNT=$((POLL_COUNT + 1))
  sleep 0.1
done
if ! wait "$BUILD_PID"; then
  cat "$TEMP_ROOT/build.log" >&2
  exit 1
fi
BUILD_PID=""
if ((POLL_COUNT == 0)); then
  echo "error: build finished before any overlap poll ran" >&2
  exit 1
fi

collect_assets "$DEV_ORIGIN/" "$TEMP_ROOT/dev-after.html" "$TEMP_ROOT/dev-after-assets.txt"
check_assets "$DEV_ORIGIN" "$TEMP_ROOT/dev-after-assets.txt"
collect_assets "$DEV_ORIGIN/adviser-probe" "$TEMP_ROOT/dev-unvisited.html" "$TEMP_ROOT/dev-unvisited-assets.txt" 404
check_assets "$DEV_ORIGIN" "$TEMP_ROOT/dev-unvisited-assets.txt"

stop_process "$DEV_PID"
DEV_PID=""

echo "[verify-next-output] starting production server on $PROD_ORIGIN"
(
  cd "$REPO_ROOT"
  exec pnpm --filter @autodrill/web exec next start -H 127.0.0.1 -p "$PROD_PORT"
) > "$TEMP_ROOT/start.log" 2>&1 &
PROD_PID=$!
wait_for_page "$PROD_ORIGIN/"
collect_assets "$PROD_ORIGIN/" "$TEMP_ROOT/prod.html" "$TEMP_ROOT/prod-assets.txt"
check_assets "$PROD_ORIGIN" "$TEMP_ROOT/prod-assets.txt"

echo "[verify-next-output] passed ($POLL_COUNT build-overlap polls)"
