#!/bin/zsh
set -uo pipefail

RESOURCE_ROOT="$(cd -- "$(dirname -- "$0")/../Resources" && pwd)"
LOG_DIR="${HOME}/Library/Logs/AutoDrill"
LOG_FILE="${LOG_DIR}/problem-qa-launcher.log"

/bin/mkdir -p "$LOG_DIR"
exec >>"$LOG_FILE" 2>&1

show_error() {
  /usr/bin/osascript -e 'display dialog "AutoDrill Problem QAを起動できませんでした。\n\n詳しい原因:\n~/Library/Logs/AutoDrill/problem-qa-launcher.log" with title "AutoDrill Problem QA" buttons {"OK"} default button "OK" with icon stop' >/dev/null
}

REQUIRED_NODE_VERSION="$(/usr/bin/tr -d '[:space:]' < "$RESOURCE_ROOT/node-version")"
typeset -a NODE_CANDIDATES
NODE_CANDIDATES=(
  "${AUTODRILL_QA_NODE_PATH:-}"
  "${HOME}/.asdf/installs/nodejs/${REQUIRED_NODE_VERSION}/bin/node"
  "${HOME}/.nvm/versions/node/v${REQUIRED_NODE_VERSION}/bin/node"
  "/opt/homebrew/bin/node"
  "/usr/local/bin/node"
)

NODE_BINARY=""
for candidate in "${NODE_CANDIDATES[@]}"; do
  [[ -n "$candidate" && -x "$candidate" ]] || continue
  actual_version="$($candidate -p 'process.versions.node' 2>/dev/null || true)"
  if [[ "$actual_version" == "$REQUIRED_NODE_VERSION" ]]; then
    NODE_BINARY="$candidate"
    break
  fi
done

if [[ -z "$NODE_BINARY" ]]; then
  print -- "[$(/bin/date -u '+%Y-%m-%dT%H:%M:%SZ')] Node.js ${REQUIRED_NODE_VERSION} not found"
  show_error
  exit 1
fi

export AUTODRILL_QA_GIT_SHA="$(/usr/bin/tr -d '[:space:]' < "$RESOURCE_ROOT/git-sha")"
export AUTODRILL_QA_GIT_STATE_PATH="$RESOURCE_ROOT/git-state.json"
export AUTODRILL_QA_WASM_DIR="$RESOURCE_ROOT/qa/wasm"
export PATH="$(dirname -- "$NODE_BINARY"):/usr/bin:/bin:${PATH:-}"
print -- "[$(/bin/date -u '+%Y-%m-%dT%H:%M:%SZ')] launch node=$NODE_BINARY git=$AUTODRILL_QA_GIT_SHA"

"$NODE_BINARY" "$RESOURCE_ROOT/qa/src/desktop.mjs"
code=$?
print -- "[$(/bin/date -u '+%Y-%m-%dT%H:%M:%SZ')] exit code=$code"
if (( code != 0 && code != 130 )); then
  show_error
fi
exit "$code"
