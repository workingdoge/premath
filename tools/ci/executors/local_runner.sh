#!/usr/bin/env sh
set -eu

TASK="${1:-hk-check}"
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)"
exec sh "$ROOT/tools/ci/run_task.sh" "$TASK"
