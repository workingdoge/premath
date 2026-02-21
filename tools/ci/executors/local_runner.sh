#!/usr/bin/env sh
set -eu

TASK="${1:-hk-check}"
exec mise run "$TASK"
