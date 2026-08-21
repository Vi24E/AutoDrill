#!/bin/zsh
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "$0")" && pwd)"
NEXT_BIN="$ROOT/apps/web/node_modules/.bin/next"
CONTRACT_CHECK="$ROOT/scripts/sync-web-contract.mjs"

# Finder からの .command 起動でも repository が固定した Node.js を使う。
# 「インストール済みの最新版」を選ぶと、別プロジェクト用 Node の追加だけで
# AutoDrill の実行環境が変わるため、.nvmrc を唯一のバージョン源にする。
NODE_VERSION="$(/usr/bin/tr -d '[:space:]' < "$ROOT/.nvmrc")"
ASDF_NODE_BIN="$HOME/.asdf/installs/nodejs/$NODE_VERSION/bin"
if [[ ! -x "$ASDF_NODE_BIN/node" ]]; then
  echo "AutoDrill が必要とする Node.js $NODE_VERSION が見つかりません。"
  echo "asdf install nodejs $NODE_VERSION を実行してください。"
  exit 1
fi
export PATH="$ASDF_NODE_BIN:$HOME/.asdf/shims:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

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

  echo "Web開発キャッシュを確認します。"
  "$ROOT/scripts/ensure-next-dev-cache.sh"

  echo "Rust/WASM を確認します。"
  "$ROOT/scripts/ensure-wasm.sh"

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
  PNPM_SPEC="$(node -p "require('$ROOT/package.json').packageManager")"
  echo "依存関係が見つからないため npm 経由で $PNPM_SPEC を一時実行して復元します。"
  npm exec --yes --package="$PNPM_SPEC" -- pnpm install --frozen-lockfile
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
