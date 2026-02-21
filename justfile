# Premath development commands

# Build all crates
build:
    cargo build --workspace

# Run Rust tests (prefers nextest when available)
test-rust:
    @if cargo nextest --version >/dev/null 2>&1; then \
        cargo nextest run --workspace; \
    else \
        cargo test --workspace; \
    fi

# Run all tests
test:
    just test-rust

# Run tests with insta snapshot updates
test-update:
    cargo insta test --workspace

# Check all crates (fast, no codegen)
check:
    cargo check --workspace

# Run semantic toy vectors
test-toy:
    python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures

# Run KCIR toy vectors
test-kcir-toy:
    python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures

# Validate conformance capability fixture stubs and invariance pairs
conformance-check:
    python3 tools/conformance/check_stub_invariance.py

# Validate doctrine-to-operation site map and declaration coherence
doctrine-check:
    python3 tools/conformance/check_doctrine_site.py

# Run executable capability vectors
conformance-run:
    python3 tools/conformance/run_capability_vectors.py

# Install Python tooling dependencies declared for repository scripts
py-setup:
    python3 -m pip install -r requirements.txt

# Baseline closure gate for local development
baseline:
    just py-setup
    just fmt-check
    just lint
    cargo build --workspace
    cargo test --workspace
    just test-toy
    just test-kcir-toy
    just conformance-check
    just doctrine-check
    just conformance-run

# Required closure gate projected from current delta
ci-required:
    python3 tools/ci/run_required_checks.py

# Verify required-gate witness against deterministic projection contract
ci-verify-required:
    python3 tools/ci/verify_required_witness.py

# Verify required-gate witness and compare against detected delta (strict CI mode)
ci-verify-required-strict:
    python3 tools/ci/verify_required_witness.py --compare-delta

# Strict verify + require native witness source for selected checks (phase-in)
ci-verify-required-strict-native:
    python3 tools/ci/verify_required_witness.py --compare-delta --require-native-check baseline

# Deterministic accept/reject decision from the verified required witness
ci-decide-required:
    python3 tools/ci/decide_required.py --compare-delta --out artifacts/ciwitness/latest-decision.json

# Verify decision attestation chain (delta snapshot + witness + decision)
ci-verify-decision:
    python3 tools/ci/verify_decision.py

# Run required gate and enforce witness verification
ci-required-verified:
    just ci-required
    just ci-verify-required

# Recommended local gate before commit
precommit:
    just ci-required-verified

# Clippy lint
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format
fmt:
    cargo fmt --all

# Format check (CI)
fmt-check:
    cargo fmt --all -- --check

# Run the CLI
run *ARGS:
    cargo run --package premath-cli -- {{ARGS}}

# Run contractibility check
check-contract ID LEVEL="set":
    cargo run --package premath-cli -- check {{ID}} --level {{LEVEL}}

# Run full axiom verification
verify ID LEVEL="set":
    cargo run --package premath-cli -- verify {{ID}} --level {{LEVEL}}

# Count lines of code
loc:
    tokei crates/

# Watch for changes and rebuild
watch:
    cargo watch -x "check --workspace"

# Bring up Terraform/OpenTofu infra profile and print runner binding
infra-up:
    sh tools/infra/terraform/up.sh

# Tear down Terraform/OpenTofu infra profile resources
infra-down:
    sh tools/infra/terraform/down.sh

# Run closure gate through Terraform/OpenTofu-resolved runner
ci-check-tf:
    sh tools/ci/run_gate_terraform.sh ci-required-verified

# Run one instruction envelope and emit a CI witness artifact
ci-instruction INSTRUCTION:
    sh tools/ci/run_instruction.sh {{INSTRUCTION}}

# Run closure gate through Terraform/OpenTofu with local runner profile
ci-check-tf-local:
    TF_VAR_cheese_profile=local sh tools/ci/run_gate_terraform.sh ci-required-verified

# Run closure gate through Terraform/OpenTofu with experimental microvm profile
ci-check-tf-microvm:
    TF_VAR_cheese_profile=darwin_microvm_vfkit sh tools/ci/run_gate_terraform.sh ci-required-verified
