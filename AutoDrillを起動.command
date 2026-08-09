#!/bin/zsh
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "$0")" && pwd)"
NEXT_BIN="$ROOT/apps/web/node_modules/.bin/next"
CONTRACT_CHECK="$ROOT/scripts/sync-web-contract.mjs"

# Finder からの .command 起動では asdf の初期化状態が Terminal と異なることがある。
# shims より先に、インストール済み Node.js の bin を直接 PATH に入れる。
ASDF_NODE_BIN=""
if [[ -d "$HOME/.asdf/installs/nodejs" ]]; then
  ASDF_NODE_BIN="$(/bin/ls -td "$HOME"/.asdf/installs/nodejs/*/bin 2>/dev/null | /usr/bin/head -n 1 || true)"
fi
export PATH="${ASDF_NODE_BIN:+$ASDF_NODE_BIN:}$HOME/.asdf/shims:$HOME/.asdf/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

pause_on_error() {
  local code=$?
  if (( code != 0 && code != 130 )); then
    echo
    echo "AutoDrill の起動に失敗しました（終了コード: $code）。"
    echo "この画面のエラー内容をそのまま共有してください。"
    echo
    read -r "?Enterキーで閉じます..."
  fi
}
trap pause_on_error EXIT

pick_port() {
  local port=3000
  while (( port <= 3010 )); do
    if ! /usr/sbin/lsof -nP -iTCP:"$port" -sTCP:LISTEN -t >/dev/null 2>&1; then
      echo "$port"
      return 0
    fi
    (( port++ ))
  done
  return 1
}

start_web() {
  if [[ ! -x "$NEXT_BIN" ]]; then
    return 1
  fi

  echo "Rust/Web 契約を確認します。"
  node "$CONTRACT_CHECK"

  local port
  port="$(pick_port)" || {
    echo "3000〜3010番ポートがすべて使用中です。"
    return 1
  }
  local url="http://localhost:$port"

  echo "AutoDrill を起動します。"
  echo "URL: $url"
  echo "終了するときは、このウィンドウで Ctrl+C を押してください。"
  echo

  # サーバーが応答してから既定ブラウザを開く。
  # AUTODRILL_NO_BROWSER=1 は自動検証用で、通常利用では設定不要。
  if [[ "${AUTODRILL_NO_BROWSER:-0}" != "1" ]]; then
    (
      for _ in {1..60}; do
        if /usr/bin/curl -fsS "http://127.0.0.1:$port/" >/dev/null 2>&1; then
          /usr/bin/open "$url"
          exit 0
        fi
        /bin/sleep 0.5
      done
    ) &
  fi

  cd "$ROOT/apps/web"
  "$NEXT_BIN" dev -p "$port"
}

# 通常はこちら。既にインストール済みの依存だけで起動するため pnpm は不要。
if [[ -x "$NEXT_BIN" ]]; then
  start_web
  exit $?
fi

# node_modules が無い場合だけ依存を復元する。
cd "$ROOT"
if command -v pnpm >/dev/null 2>&1; then
  echo "依存関係が見つからないため pnpm で復元します。"
  pnpm install --frozen-lockfile
elif command -v npm >/dev/null 2>&1; then
  echo "依存関係が見つからないため npm 経由で pnpm 11.9.0 を一時実行して復元します。"
  echo "Corepack は使用しません。"
  npm exec --yes --package=pnpm@11.9.0 -- pnpm install --frozen-lockfile
else
  echo "Node.js/npm が見つかりません。"
  echo "asdf の Node.js 環境を確認してください。"
  exit 1
fi

if [[ ! -x "$NEXT_BIN" ]]; then
  echo "依存関係の復元後も Next.js 実行ファイルが見つかりません。"
  exit 1
fi

start_web
