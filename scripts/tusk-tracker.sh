#!/usr/bin/env bash
set -euo pipefail

script_path="${BASH_SOURCE[0]:-$0}"
script_dir="$(CDPATH= cd -- "$(dirname -- "${script_path}")" && pwd)"
upstream_root="${PREMATH_TUSK_ROOT:-${TUSK_SHARED_ROOT:-}}"

if [ -z "${upstream_root}" ]; then
  echo "tusk-tracker.sh: PREMATH_TUSK_ROOT is required" >&2
  exit 1
fi

export TUSK_PATHS_SH="${TUSK_PATHS_SH:-${script_dir}/tusk-paths.sh}"

exec bash "${upstream_root}/scripts/tusk-tracker.sh" "$@"
