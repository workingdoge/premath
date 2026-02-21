#!/usr/bin/env sh
set -eu

TASK="${1:-hk-check}"
PROFILE="${PREMATH_SQUEAK_SITE_PROFILE:-${PREMATH_EXECUTOR_PROFILE:-local}}"
RUNNER="${PREMATH_SQUEAK_SITE_RUNNER:-${PREMATH_EXECUTOR_RUNNER:-}}"

usage() {
  cat >&2 <<'EOF'
Usage:
  sh tools/ci/run_gate.sh [task]

Environment:
  PREMATH_SQUEAK_SITE_PROFILE=local|external   (default: local)
  PREMATH_SQUEAK_SITE_RUNNER=<path>            (required for external)

Legacy compatibility:
  PREMATH_EXECUTOR_PROFILE and PREMATH_EXECUTOR_RUNNER are still accepted.

Protocol for PREMATH_SQUEAK_SITE_RUNNER:
  - executable path
  - invoked as: <runner> <task>
  - must return nonzero on failure
EOF
}

run_local() {
  exec mise run "$TASK"
}

run_external() {
  if [ -z "$RUNNER" ]; then
    echo "PREMATH_SQUEAK_SITE_RUNNER is required when PREMATH_SQUEAK_SITE_PROFILE=external" >&2
    usage
    exit 2
  fi
  if [ ! -x "$RUNNER" ]; then
    echo "PREMATH_SQUEAK_SITE_RUNNER must be executable: $RUNNER" >&2
    exit 2
  fi
  exec "$RUNNER" "$TASK"
}

case "$PROFILE" in
  local)
    run_local
    ;;
  external)
    run_external
    ;;
  *)
    echo "Unsupported PREMATH_SQUEAK_SITE_PROFILE: $PROFILE" >&2
    usage
    exit 2
    ;;
esac
