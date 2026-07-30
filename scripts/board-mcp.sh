#!/bin/sh
set -eu

ENV_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
exec "$ENV_ROOT/scripts/board-mcp.sh" "$@"
