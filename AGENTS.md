# Repository Guidelines

## Project Structure & Module Organization

- Core crates live in `crates/`: `premath-kernel` (laws/gate/witnesses), `premath-tusk` (runtime identity/descent/witness envelope), `premath-bd` (JSONL memory), `premath-jj`, `premath-surreal`, and `premath-cli`.
- Specs are lifecycle-scoped:
  - `specs/premath/draft/` for promoted contract specs
  - `specs/premath/raw/` for exploratory/informational specs
  - `specs/process/` for governance (`coss.md`, `decision-log.md`)
- Tests and vectors live in `crates/*/tests`, `tests/toy/`, `tests/kcir_toy/`, and `tests/conformance/`.
- Tooling scripts live in `tools/` (`toy`, `kcir_toy`, `conformance`).

## Environment (Nix-First)

- Preferred developer entrypoint: `nix develop`.
- One-shot commands:
  - `nix develop -c just baseline`
  - `nix develop -c just precommit`
  - `nix build .#default` (build CLI package)
  - `nix run .#default -- --help` (run CLI app)
- If not using Nix, install Rust + Python 3 and run the equivalent `cargo`/`python3` commands directly.
- Python tooling dependency policy: declare third-party script deps in root `requirements.txt` (currently intentionally empty/stdlib-only).
- Hybrid runtime/task layer uses `mise`:
  - pinned versions in `.mise.toml`
  - tasks via `mise run <task>`
  - optional auto-activation with `.envrc` (`use flake` + `use mise`)
  - one-time helper: `mise direnv activate > ~/.config/direnv/lib/use_mise.sh`

## Build, Test, and Development Commands

- `cargo build --workspace` — build all crates.
- `cargo test --workspace` — run Rust tests.
- `just baseline` — full local closure gate (`py-setup` + fmt + clippy + build/tests + toy + KCIR toy + conformance checks + doctrine-site check).
- `mise run baseline` — equivalent baseline gate for non-Nix or mixed workflows (includes `rust-setup` for `rustfmt`/`clippy` components).
- `mise run hk-install` — install optional `hk`-managed git hooks using `hk.pkl`.
- `mise run hk-pre-commit` / `mise run hk-pre-push` — run hk hook profiles manually.
- `mise run hk-check` / `mise run hk-fix` — run hk baseline check or fast local fixes (`hk-fix` runs on all files with no auto-stage).
- `mise run ci-check` — canonical gate entrypoint through `tools/ci/run_gate.sh` (SqueakSite profile switch: `PREMATH_SQUEAK_SITE_PROFILE=local|external`; legacy `PREMATH_EXECUTOR_PROFILE` still accepted).
- `mise run ci-instruction` — run one instruction envelope (`INSTRUCTION=instructions/<ts>-<id>.json`) and emit `artifacts/ciwitness/<instruction-id>.json`.
- `sh tools/ci/run_instruction.sh instructions/<ts>-<id>.json` — run an instruction envelope and emit `artifacts/ciwitness/<instruction-id>.json`.
- `mise run infra-up` / `mise run infra-down` — optional Terraform/OpenTofu provisioning profile for external runner binding (`tools/infra/terraform/`).
- `mise run ci-check-tf` — gate execution via Terraform/OpenTofu-resolved external runner (`tools/ci/run_gate_terraform.sh`, default profile `local`).
- `mise run ci-check-tf-local` — Terraform/OpenTofu path pinned to local runner profile.
- `mise run ci-check-tf-microvm` — experimental/prototype Terraform/OpenTofu path using `darwin_microvm_vfkit` profile.
- `mise run jj-alias-install` — install repo-local JJ aliases (`jj gate-fast|gate-fix|gate-check|gate-pre-commit`) that delegate to hk/mise gates.
- `mise run pf-start` / `mise run pf-status` / `mise run pf-stop` — optional pitchfork orchestration for local daemons in `pitchfork.toml`.
- `mise run pf-gate-loop-start` / `mise run pf-gate-loop-stop` — optional background `ci-check` loop via pitchfork (`ci-check` every 30m).
- `just conformance-run` — run executable capability vectors (`capabilities.normal_forms`, `capabilities.kcir_witnesses`, `capabilities.commitment_checkpoints`, `capabilities.squeak_site`, `capabilities.ci_witnesses`, `capabilities.instruction_typing`).
- `just doctrine-check` — validate doctrine declarations and doctrine-to-operation site reachability (`specs/premath/draft/DOCTRINE-SITE.json`).
- `just precommit` — same as baseline.
- `python3 tools/conformance/check_stub_invariance.py` — validate capability fixture stubs/invariance pairs.
- `cargo run --package premath-cli -- <args>` — run CLI commands locally.
- `cargo run --package premath-cli -- mock-gate --json` — emit a mock Gate witness envelope.
- `cargo run --package premath-cli -- tusk-eval --identity <run_identity.json> --descent-pack <descent_pack.json> --json` — evaluate a Tusk descent pack and emit envelope + glue result.

## Coding Style & Naming Conventions

- Rust style: `cargo fmt --all`; lint with `cargo clippy --workspace --all-targets -- -D warnings`.
- Keep modules focused; avoid duplicate “v2” naming in greenfield paths (prefer canonical names like `KCIR-CORE`).
- Use clear, domain-specific names (`*_witness`, `*_ref`, `policy_digest`, `normalizer_id`) that match spec terminology.
- Specs: update references to `draft/...` for promoted specs; keep `raw/...` references only for non-promoted docs.

## Testing Guidelines

- Treat `just baseline` as the minimum local merge gate.
- If using `hk`, keep `pre-push`/`check` mapped to the same baseline closure gate.
- For spec/conformance fixture edits, run `python3 tools/conformance/check_stub_invariance.py`.
- For executable capability vectors, run `python3 tools/conformance/run_capability_vectors.py`.
- For kernel/gate edits, run:
  - `cargo test --workspace`
  - `python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures`
  - `python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures`

## Commit & Pull Request Guidelines

- Use Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`).
- Keep PRs scoped to one concern (code, specs, or conformance fixtures).
- For spec changes, include updated vectors/fixtures when behavior changes.
- Add a decision-log entry in `specs/process/decision-log.md` for lifecycle or boundary changes.
