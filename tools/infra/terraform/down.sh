#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)"
TF_DIR="${PREMATH_TF_DIR:-$ROOT/infra/terraform}"

detect_tf_bin() {
  if [ -n "${PREMATH_TF_BIN:-}" ]; then
    printf '%s\n' "$PREMATH_TF_BIN"
    return
  fi
  if command -v tofu >/dev/null 2>&1; then
    printf '%s\n' "tofu"
    return
  fi
  if command -v terraform >/dev/null 2>&1; then
    printf '%s\n' "terraform"
    return
  fi
  echo "Missing Terraform-compatible binary. Install 'tofu' or 'terraform', or set PREMATH_TF_BIN." >&2
  exit 2
}

if [ ! -d "$TF_DIR" ]; then
  echo "Terraform directory not found: $TF_DIR" >&2
  exit 2
fi

TF_BIN="$(detect_tf_bin)"

"$TF_BIN" -chdir="$TF_DIR" init -input=false >/dev/null
"$TF_BIN" -chdir="$TF_DIR" destroy -auto-approve -input=false >/dev/null
