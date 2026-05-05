#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
TASK="${1:-}"

if [ -z "$TASK" ]; then
  echo "usage: sh tools/ci/run_task.sh <task> [args...]" >&2
  exit 2
fi
shift
if [ "${1:-}" = "--" ]; then
  shift
fi

cd "$ROOT"

run_baseline() {
  awk '
    /"tasks"[[:space:]]*:/ { in_tasks = 1; next }
    in_tasks && /\]/ { exit }
    in_tasks {
      gsub(/[",]/, "")
      gsub(/^[[:space:]]+|[[:space:]]+$/, "")
      if ($0 != "") print
    }
  ' "$ROOT/tools/ci/baseline_tasks.json" | while IFS= read -r task; do
    sh "$ROOT/tools/ci/run_task.sh" "$task"
  done
}

run_ci_pipeline_test() {
  python3 tools/ci/test_pipeline_required.py
  python3 tools/ci/test_run_required_checks.py
  python3 tools/ci/test_required_witness_lineage.py
  python3 tools/ci/test_required_delta_client.py
  python3 tools/ci/test_required_projection_client.py
  python3 tools/ci/test_required_witness_client.py
  python3 tools/ci/test_required_gate_ref_client.py
  python3 tools/ci/test_required_witness_verify_client.py
  python3 tools/ci/test_required_witness_decide_client.py
  python3 tools/ci/test_required_decision_verify_client.py
  python3 tools/ci/test_harness_retry_policy.py
  python3 tools/ci/test_harness_escalation.py
  python3 tools/ci/test_pipeline_instruction.py
  python3 tools/ci/test_kcir_mapping_gate.py
  python3 tools/ci/test_control_plane_contract.py
  python3 tools/ci/test_drift_budget.py
  python3 tools/ci/test_instruction_check_client.py
  python3 tools/ci/test_proposal_check_client.py
  python3 tools/ci/test_client_transport_parity.py
  python3 tools/ci/test_instruction_reject_witness.py
  python3 tools/conformance/test_run_fixture_suites.py
  python3 tools/conformance/test_doctrine_site_contract.py
  python3 tools/conformance/test_doctrine_mcp_parity.py
  python3 tools/conformance/test_runtime_orchestration.py
}

case "$TASK" in
  rust-setup)
    if command -v rustup >/dev/null 2>&1; then
      rustup component add rustfmt clippy
    else
      echo "rustup not found; assuming rustfmt/clippy are provided by the active toolchain"
    fi
    ;;
  fmt)
    cargo fmt --all -- --check
    ;;
  lint)
    cargo clippy --workspace --all-targets -- -D warnings
    ;;
  build)
    cargo build --workspace
    ;;
  test)
    cargo test --workspace
    ;;
  test-toy)
    cargo test -p premath-kernel --test toy_vectors
    ;;
  test-kcir-toy)
    python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures
    ;;
  conformance-check)
    cargo run --package premath-cli -- conformance-check
    ;;
  traceability-check)
    cargo run --package premath-cli -- traceability-check
    ;;
  ci-drift-budget-check)
    python3 tools/ci/check_drift_budget.py --json
    ;;
  coherence-check)
    cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json
    ;;
  doctrine-check)
    python3 tools/conformance/check_doctrine_site.py
    python3 tools/conformance/check_runtime_orchestration.py
    python3 tools/conformance/check_doctrine_mcp_parity.py
    python3 tools/conformance/run_fixture_suites.py --suite doctrine-inf
    ;;
  conformance-run)
    python3 tools/conformance/run_fixture_suites.py "$@"
    ;;
  baseline)
    run_baseline
    ;;
  hk-install)
    hk install
    ;;
  hk-check)
    hk run check
    ;;
  hk-fix)
    hk run fix --all --no-stage
    ;;
  hk-pre-commit)
    hk run pre-commit
    ;;
  hk-pre-push)
    hk run pre-push
    ;;
  ci-command-surface-check)
    cargo run --package premath-cli -- command-surface-check
    ;;
  ci-hygiene-check)
    cargo run --package premath-cli -- repo-hygiene-check
    cargo run --package premath-cli -- issue check
    ;;
  ci-pipeline-check)
    cargo run --package premath-cli -- pipeline-wiring-check
    ;;
  ci-wiring-check)
    echo "[ci-wiring-check] deprecated alias; running ci-pipeline-check" >&2
    sh "$ROOT/tools/ci/run_task.sh" ci-pipeline-check
    ;;
  ci-pipeline-test)
    run_ci_pipeline_test
    ;;
  ci-observation-build)
    cargo run --package premath-cli -- observe-build --repo-root .
    ;;
  ci-observation-query)
    cargo run --package premath-cli -- observe --surface artifacts/observation/latest.json --mode latest --json
    ;;
  ci-observation-test)
    cargo test -p premath-surreal observation
    cargo test -p premath-ux
    cargo test -p premath-cli observe
    ;;
  ci-observation-serve)
    cargo run --package premath-cli -- observe-serve --surface artifacts/observation/latest.json --bind 127.0.0.1:43174
    ;;
  ci-observation-check)
    sh "$ROOT/tools/ci/run_task.sh" ci-observation-build
    cargo run --package premath-cli -- observe-check --repo-root .
    ;;
  mcp-serve)
    cargo run --package premath-cli -- mcp-serve --issues .premath/issues.jsonl --issue-query-backend jsonl --mutation-policy instruction-linked --surface artifacts/observation/latest.json --repo-root .
    ;;
  ci-pipeline-required)
    python3 tools/ci/pipeline_required.py "$@"
    ;;
  ci-pipeline-instruction)
    : "${INSTRUCTION:?set INSTRUCTION=instructions/<ts>-<id>.json}"
    python3 tools/ci/pipeline_instruction.py --instruction "$INSTRUCTION" "$@"
    ;;
  ci-required)
    python3 tools/ci/run_required_checks.py "$@"
    ;;
  ci-required-attested|precommit)
    python3 tools/ci/run_required_attested.py "$@"
    ;;
  ci-check)
    sh tools/ci/run_gate.sh hk-check
    ;;
  ci-pre-commit)
    sh tools/ci/run_gate.sh hk-pre-commit
    ;;
  ci-instruction)
    : "${INSTRUCTION:?set INSTRUCTION=instructions/<ts>-<id>.json}"
    sh tools/ci/run_instruction.sh "$INSTRUCTION"
    ;;
  ci-instruction-check)
    if [ -n "${INSTRUCTION:-}" ]; then
      cargo run --package premath-cli -- instruction-check --instruction "$INSTRUCTION"
    else
      found=0
      for instruction in instructions/*.json tests/ci/fixtures/instructions/*.json; do
        [ -f "$instruction" ] || continue
        found=1
        cargo run --package premath-cli -- instruction-check --instruction "$instruction"
      done
      if [ "$found" -eq 0 ]; then
        echo "[instruction-check] FAIL (no instruction envelopes found)" >&2
        exit 1
      fi
    fi
    ;;
  ci-instruction-smoke)
    python3 tools/ci/test_instruction_smoke.py --instruction tests/ci/fixtures/instructions/20260221T010000Z-ci-wiring-golden.json
    ;;
  ci-instruction-example)
    sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
    ;;
  infra-up)
    sh tools/infra/terraform/up.sh
    ;;
  infra-down)
    sh tools/infra/terraform/down.sh
    ;;
  ci-check-tf)
    sh tools/ci/run_gate_terraform.sh ci-required-attested
    ;;
  ci-pre-commit-tf)
    sh tools/ci/run_gate_terraform.sh hk-pre-commit
    ;;
  ci-check-tf-local)
    TF_VAR_cheese_profile=local sh tools/ci/run_gate_terraform.sh ci-required-attested
    ;;
  ci-check-tf-microvm)
    TF_VAR_cheese_profile=darwin_microvm_vfkit sh tools/ci/run_gate_terraform.sh ci-required-attested
    ;;
  jj-alias-install)
    sh tools/jj/install_aliases.sh
    ;;
  pf-start)
    pitchfork start docs-preview observation-api
    ;;
  pf-stop)
    if pitchfork supervisor status >/dev/null 2>&1; then
      pitchfork stop --all
    else
      echo "pitchfork supervisor not running"
    fi
    ;;
  pf-status)
    pitchfork list
    ;;
  pf-gate-loop-start)
    pitchfork start gate-check-loop
    ;;
  pf-gate-loop-stop)
    pitchfork stop gate-check-loop
    ;;
  *)
    echo "unknown task: $TASK" >&2
    exit 2
    ;;
esac
