#!/usr/bin/env bash
set -euo pipefail

upstream_root="${PREMATH_TUSK_ROOT:-${TUSK_SHARED_ROOT:-}}"

if [ -z "${upstream_root}" ]; then
  echo "tusk-paths.sh: PREMATH_TUSK_ROOT is required" >&2
  exit 1
fi

source "${upstream_root}/scripts/tusk-paths.sh"
