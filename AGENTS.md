# Repository Guidelines

## Project Structure & Module Organization

- Core crates live in `crates/`: `premath-kernel` (laws/gate/witnesses), `premath-tusk` (runtime identity/descent/witness envelope), `premath-bd` (JSONL memory), `premath-jj`, `premath-surreal` (query/index adapters), `premath-ux` (frontend/query composition surface), and `premath-cli`.
- Specs are lifecycle-scoped:
  - `specs/premath/draft/` for promoted contract specs
  - `specs/premath/raw/` for exploratory/informational specs
  - `specs/process/` for governance (`coss.md`, `decision-log.md`, `GITHUB-BRANCH-POLICY.json`)
- Tests and vectors live in `crates/*/tests`, `tests/toy/`, `tests/kcir_toy/`, and `tests/conformance/`.
- Tooling scripts live in `tools/` (`toy`, `kcir_toy`, `conformance`).

## Environment (Nix-First)

- Preferred developer entrypoint: `nix develop`.
- One-shot commands:
  - `nix develop -c mise run baseline`
  - `nix develop -c mise run precommit`
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
- `mise run baseline` — full local closure gate (`py-setup` + fmt + clippy + build/tests + toy + KCIR toy + conformance checks + traceability matrix check + coherence-check + docs-coherence check + doctrine-site check; includes `rust-setup` for `rustfmt`/`clippy` components).
- `mise run hk-install` — install optional `hk`-managed git hooks using `hk.pkl`.
- `mise run hk-pre-commit` / `mise run hk-pre-push` — run hk hook profiles manually.
- `mise run hk-check` / `mise run hk-fix` — run hk baseline check or fast local fixes (`hk-fix` runs on all files with no auto-stage).
- `mise run ci-command-surface-check` — enforce `mise`-only command-surface references (reject legacy task-runner command/file surfaces).
- `mise run ci-hygiene-check` — enforce repository hygiene guardrails for private/local-only surfaces and required ignore entries.
- `mise run ci-branch-policy-check` — validate tracked GitHub `main` branch policy contract against deterministic effective-rules fixture.
- `mise run ci-branch-policy-check-live` — validate tracked GitHub `main` branch policy contract against live server rules API (`GITHUB_TOKEN`/admin-read token required).
- `mise run ci-pipeline-check` — validate provider workflow wrappers call canonical provider-neutral pipeline entrypoints.
- `mise run ci-pipeline-test` — run deterministic unit tests for provider-neutral pipeline summary/digest emission.
- `mise run ci-observation-test` — run deterministic reducer/query tests for `Observation Surface v0`.
- `mise run ci-observation-build` — build `artifacts/observation/latest.json` + `artifacts/observation/events.jsonl` from CI witness artifacts.
- `mise run ci-observation-query` — query the latest observation surface (`latest`, `needs_attention`, `instruction`, `projection`).
- `mise run ci-observation-serve` — run a tiny HTTP read API over Observation Surface v0 (`GET /latest`, `/needs-attention`, `/instruction`, `/projection`).
- `mise run ci-observation-check` — enforce semantic projection invariance (observation output must equal deterministic reducer output from CI witness artifacts).
- `python3 -m http.server 43173 --directory docs` — serve docs locally (includes `docs/observation/index.html` dashboard).
- `mise run ci-pipeline-required` — run provider-neutral required-gate pipeline (`tools/ci/pipeline_required.py`).
- `mise run ci-pipeline-instruction` — run provider-neutral instruction pipeline (`INSTRUCTION=instructions/<ts>-<id>.json`).
- `mise run ci-check` — canonical gate entrypoint through `tools/ci/run_gate.sh` (SqueakSite profile switch: `PREMATH_SQUEAK_SITE_PROFILE=local|external`; legacy `PREMATH_EXECUTOR_PROFILE` still accepted).
- `mise run ci-instruction` — run one instruction envelope (`INSTRUCTION=instructions/<ts>-<id>.json`) and emit `artifacts/ciwitness/<instruction-id>.json`.
- `sh tools/ci/run_instruction.sh instructions/<ts>-<id>.json` — run an instruction envelope and emit `artifacts/ciwitness/<instruction-id>.json`.
- `mise run infra-up` / `mise run infra-down` — optional Terraform/OpenTofu provisioning profile for external runner binding (`tools/infra/terraform/`).
- `mise run ci-check-tf` — gate execution via Terraform/OpenTofu-resolved external runner (`tools/ci/run_gate_terraform.sh`, default profile `local`).
- `mise run ci-check-tf-local` — Terraform/OpenTofu path pinned to local runner profile.
- `mise run ci-check-tf-microvm` — experimental/prototype Terraform/OpenTofu path using `darwin_microvm_vfkit` profile.
- `mise run jj-alias-install` — install repo-local JJ aliases (`jj gate-fast|gate-fix|gate-check|gate-pre-commit`) that delegate to hk/mise gates.
- `mise run pf-start` / `mise run pf-status` / `mise run pf-stop` — optional pitchfork orchestration for local daemons in `pitchfork.toml` (`pf-start` starts both `docs-preview` and `observation-api`).
- `mise run pf-gate-loop-start` / `mise run pf-gate-loop-stop` — optional background `ci-check` loop via pitchfork (`ci-check` every 30m).
- `mise run mcp-serve` — run the stdio MCP server surface over premath issue/dep/observe/doctrine tools (JSONL-authoritative memory, `instruction-linked` mutation policy).
- `mise run conformance-run` — run executable fixture suites (Interop Core + Gate + Witness-ID + cross-model kernel profile + Tusk runtime contract vectors + capability vectors) through the cached suite runner.
- `mise run doctrine-check` — validate doctrine declarations/site reachability plus doctrine-inf semantic boundary vectors (`specs/premath/draft/DOCTRINE-SITE.json`, `tests/conformance/fixtures/doctrine-inf/`).
- `mise run traceability-check` — validate promoted draft spec coverage matrix integrity (`specs/premath/draft/SPEC-TRACEABILITY.md`).
- `mise run coherence-check` — evaluate typed coherence obligations from `specs/premath/draft/COHERENCE-CONTRACT.json` and emit deterministic checker witness output.
- `mise run docs-coherence-check` — validate deterministic docs-to-executable coherence invariants (capability lists, baseline/projection check surfaces, and SPEC-INDEX capability-scoped normativity clauses).
- `mise run precommit` — same as baseline.
- `python3 tools/conformance/check_stub_invariance.py` — validate capability fixture stubs/invariance pairs.
- `cargo run --package premath-cli -- <args>` — run CLI commands locally.
- `cargo run --package premath-cli -- init .` — initialize local `.premath/issues.jsonl` (migrates legacy `.beads/issues.jsonl` when present).
- `cargo run --package premath-cli -- mock-gate --json` — emit a mock Gate witness envelope.
- `cargo run --package premath-cli -- tusk-eval --identity <run_identity.json> --descent-pack <descent_pack.json> --json` — evaluate a Tusk descent pack and emit envelope + glue result.
- `cargo run --package premath-cli -- proposal-check --proposal <proposal.json> --json` — validate/canonicalize one proposal payload, compile obligations, and emit deterministic discharge output.
- `cargo run --package premath-cli -- instruction-check --instruction <instruction.json> --repo-root . --json` — validate/canonicalize one instruction envelope and emit typed execution decision metadata.
- `cargo run --package premath-cli -- instruction-witness --instruction <instruction.json> --runtime <runtime.json> --repo-root . --json` — finalize one CI instruction witness from typed instruction semantics plus executed check runtime payload.
- `cargo run --package premath-cli -- required-witness --runtime <runtime.json> --json` — finalize one CI required witness from projection/check/gate-ref runtime payload.
- `cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json` — evaluate typed coherence obligations and emit deterministic coherence witness output.
- `cargo run --package premath-cli -- ref project --profile policies/ref/sha256_detached_v1.json --domain kcir.node --payload-hex <hex> --json` — project deterministic backend refs via profile-bound `project_ref`.
- `cargo run --package premath-cli -- ref verify --profile policies/ref/sha256_detached_v1.json --domain kcir.node --payload-hex <hex> --evidence-hex <hex> --ref-scheme-id <id> --ref-params-hash <hash> --ref-domain <domain> --ref-digest <digest> --json` — verify provided refs via profile-bound `verify_ref`.
- `cargo run --package premath-cli -- observe --mode latest --json` — query Observation Surface v0 through the UX composition layer.
- `cargo run --package premath-cli -- observe-serve --bind 127.0.0.1:43174` — serve Observation Surface v0 through the UX HTTP API.
- `cargo run --package premath-cli -- mcp-serve --issues .premath/issues.jsonl --issue-query-backend jsonl --mutation-policy instruction-linked --surface artifacts/observation/latest.json --repo-root .` — run MCP tools over stdio (includes doctrine-gated `instruction_check` and `instruction_run`).
- `cargo run --package premath-cli -- issue add \"Title\" --issues .premath/issues.jsonl --json` — add an issue to JSONL-backed memory.
- `cargo run --package premath-cli -- issue claim <issue-id> --assignee <name> --issues .premath/issues.jsonl --json` — atomically claim work (`assignee` + `in_progress`).
- `cargo run --package premath-cli -- issue discover <parent-issue-id> \"Title\" --issues .premath/issues.jsonl --json` — create discovered follow-up work linked by `discovered-from`.
- `cargo run --package premath-cli -- issue list --issues .premath/issues.jsonl --json` — list issues with optional filters.
- `cargo run --package premath-cli -- issue ready --issues .premath/issues.jsonl --json` — return unblocked open issues.
- `cargo run --package premath-cli -- issue blocked --issues .premath/issues.jsonl --json` — return non-closed issues blocked by unresolved dependencies.
- `cargo run --package premath-cli -- issue update <issue-id> --status in_progress --issues .premath/issues.jsonl --json` — update issue fields.
- `cargo run --package premath-cli -- dep add <issue-id> <depends-on-id> --type blocks --issues .premath/issues.jsonl --json` — add a dependency edge.

## CI Workflow Instructions

- Keep provider workflows as thin wrappers only:
  - required gate: `python3 tools/ci/pipeline_required.py`
  - instruction gate: `python3 tools/ci/pipeline_instruction.py --instruction "$INSTRUCTION_PATH"`
- Do not place split gate-chain commands or inline Python summary blocks in `.github/workflows/*.yml`; keep orchestration in `tools/ci/pipeline_*.py`.
- Run both checks after CI/workflow edits:
  - `mise run ci-pipeline-check`
  - `mise run ci-pipeline-test`
  - `mise run ci-wiring-check`

## GitHub Ops Conventions

- `main` is protected and PR-only. Do not attempt direct pushes to `main`; use topic branch + PR.
- Keep local and server policy checks aligned:
  - local fixture contract check: `mise run ci-branch-policy-check`
  - live server check: `mise run ci-branch-policy-check-live` (requires admin-read token in `GITHUB_TOKEN` or explicit `--token-env`)
- `branch-policy` workflow requires repository secret `PREMATH_BRANCH_POLICY_TOKEN`; keep it populated before expecting live workflow success.
- For governance/ops rollouts, record command evidence and resulting URLs in `.premath/OPERATIONS.md` and in the relevant issue notes (`.premath/issues.jsonl`).
- When protection settings are changed, expect pushes to fail until work goes through PR and required status checks (`ci-required`) report on the PR head.

## Coding Style & Naming Conventions

- Rust style: `cargo fmt --all`; lint with `cargo clippy --workspace --all-targets -- -D warnings`.
- Keep modules focused; avoid duplicate “v2” naming in greenfield paths (prefer canonical names like `KCIR-CORE`).
- Use clear, domain-specific names (`*_witness`, `*_ref`, `policy_digest`, `normalizer_id`) that match spec terminology.
- Specs: update references to `draft/...` for promoted specs; keep `raw/...` references only for non-promoted docs.

## Testing Guidelines

- Treat `mise run baseline` as the minimum local merge gate.
- If using `hk`, keep `pre-push`/`check` mapped to the same baseline closure gate.
- For spec/conformance fixture edits, run `python3 tools/conformance/check_stub_invariance.py`.
- For executable interop-core vectors, run `python3 tools/conformance/run_interop_core_vectors.py`.
- For executable gate vectors, run `python3 tools/conformance/run_gate_vectors.py`.
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
