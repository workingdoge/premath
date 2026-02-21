#!/usr/bin/env sh
set -eu

TASK="${1:-ci-required-attested}"
ROOT="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"

CHEESE_RUNNER="$(sh "$ROOT/tools/infra/terraform/up.sh")"

PREMATH_SQUEAK_SITE_PROFILE=external \
PREMATH_SQUEAK_SITE_RUNNER="$CHEESE_RUNNER" \
  exec sh "$ROOT/tools/ci/run_gate.sh" "$TASK"
