#!/usr/bin/env sh
set -eu

TASK="${1:-hk-check}"
PROFILE="${PREMATH_SQUEAK_SITE_PROFILE:-${PREMATH_EXECUTOR_PROFILE:-local}}"
RUNNER="${PREMATH_SQUEAK_SITE_RUNNER:-${PREMATH_EXECUTOR_RUNNER:-}}"
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"

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

Optional native gate witness handoff (used by ci.required):
  PREMATH_GATE_WITNESS_OUT=<path>
  PREMATH_GATE_CHECK_ID=<check-id>
  PREMATH_GATE_PROJECTION_DIGEST=<projection-digest>
  PREMATH_GATE_POLICY_DIGEST=<policy-digest>
  PREMATH_GATE_CTX_REF=<ctx-ref>
  PREMATH_GATE_DATA_HEAD_REF=<data-head-ref>
EOF
}

run_local() {
  mise run "$TASK"
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
  "$RUNNER" "$TASK"
}

emit_gate_witness_if_requested() {
  EXIT_CODE="$1"
  OUT_PATH="${PREMATH_GATE_WITNESS_OUT:-}"
  if [ -z "$OUT_PATH" ]; then
    return 0
  fi

  if [ -f "$OUT_PATH" ]; then
    # Runner/task already produced a native artifact.
    return 0
  fi

  CHECK_ID="${PREMATH_GATE_CHECK_ID:-$TASK}"
  PROJECTION_DIGEST="${PREMATH_GATE_PROJECTION_DIGEST:-proj1_unknown}"
  POLICY_DIGEST="${PREMATH_GATE_POLICY_DIGEST:-ci-topos-v0}"
  CTX_REF="${PREMATH_GATE_CTX_REF:-ctx:unknown}"
  DATA_HEAD_REF="${PREMATH_GATE_DATA_HEAD_REF:-HEAD}"

  python3 "$ROOT/tools/ci/emit_gate_witness.py" \
    --check-id "$CHECK_ID" \
    --exit-code "$EXIT_CODE" \
    --projection-digest "$PROJECTION_DIGEST" \
    --policy-digest "$POLICY_DIGEST" \
    --ctx-ref "$CTX_REF" \
    --data-head-ref "$DATA_HEAD_REF" \
    --out "$OUT_PATH" >/dev/null 2>&1 || true
}

if [ "$PROFILE" = "local" ]; then
  if run_local; then
    EXIT_CODE=0
  else
    EXIT_CODE=$?
  fi
elif [ "$PROFILE" = "external" ]; then
  if run_external; then
    EXIT_CODE=0
  else
    EXIT_CODE=$?
  fi
else
  echo "Unsupported PREMATH_SQUEAK_SITE_PROFILE: $PROFILE" >&2
  usage
  exit 2
fi

emit_gate_witness_if_requested "$EXIT_CODE"
exit "$EXIT_CODE"
