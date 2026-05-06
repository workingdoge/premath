# Repository Guidelines

## Project Structure & Module Organization

- Core crates live in `crates/`: `premath-kernel` (laws/gate/witnesses), `premath-coherence`, and `premath-cli`.
- Premath should not grow transitional standalone crates without an explicit promoted checker use. KCIR substrate authority belongs to `fish/sites/kcir`; executable KCIR carriage belongs in Kurma.
- Tracker runtime, ready queues, issue mutation, and dependency graph editing belong to Tusk or downstream operator surfaces. Premath keeps normalized work-tracker claim checking only as a profile/control-plane checker surface, not as kernel law.
- Specs are lifecycle-scoped:
  - `specs/premath/draft/` for promoted contract specs
  - `specs/premath/raw/` for exploratory/informational specs
  - `specs/process/` for governance (`coss.md`, `decision-log.md`)
- Tests and vectors live in `crates/*/tests`, `tests/toy/`, `tests/kcir_toy/`, and checker fixture directories under `tests/`.
- Tooling scripts live in `tools/` (`toy`, `kcir_toy`).

## Environment (Nix-First)

- Preferred developer entrypoint: `direnv allow`, then the direnv-activated shell.
- One-shot commands:
  - `direnv exec . cargo test --workspace`
  - `nix build .#default` (build CLI package)
  - `nix run .#default -- --help` (run CLI app)
- If not using Nix, install Rust + Python 3 and run the equivalent `cargo`/`python3` commands directly.
- Python tooling dependency policy: declare third-party script deps in root `requirements.txt` (currently intentionally empty/stdlib-only).
- Repo checks are direct Cargo, Python toy, and native Premath CLI commands.
- `.envrc` is repo-root-aware `use flake` only.

## Build, Test, and Development Commands

- `cargo build --workspace` — build all crates.
- `cargo test --workspace` — run Rust tests.
- `cargo fmt --all -- --check` — check Rust formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` — lint Rust.
- `cargo test --workspace` — run Rust tests.
- `python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures` — run KCIR toy vectors.
- `cargo run --package premath-cli -- traceability-check` — validate promoted draft spec coverage matrix integrity and authority-class parity (`specs/premath/draft/SPEC-TRACEABILITY.md`, `specs/premath/AUTHORITY-MAP.json`).
- `cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json` — evaluate typed coherence obligations and emit deterministic checker witness output.
- `cargo run --package premath-cli -- drift-budget-check --json` — run native drift-budget sentinels across SPEC-INDEX/CAPABILITY-REGISTRY maps, control-plane lane bindings, KCIR mapping shape, coherence required obligation sets, SigPi notation, and coherence-cache input closure.
- `cargo run --package premath-cli -- command-surface-check` — enforce direct command-surface references.
- `cargo run --package premath-cli -- repo-hygiene-check` — enforce repository hygiene guardrails.
- `cargo run --package premath-cli -- <args>` — run CLI commands locally.
- `cargo run --package premath-cli -- proposal-check --proposal <proposal.json> --json` — validate/canonicalize one proposal payload, compile obligations, and emit deterministic discharge output.
- `cargo run --package premath-cli -- required-projection --input <projection_input.json> --json` — project `changedPaths` to deterministic local checker IDs through core semantics.
- `cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json` — evaluate typed coherence obligations and emit deterministic coherence witness output.
- `cargo run --package premath-cli -- ref project --profile policies/ref/sha256_detached_v1.json --domain kcir.node --payload-hex <hex> --json` — project deterministic backend refs via profile-bound `project_ref`.
- `cargo run --package premath-cli -- ref verify --profile policies/ref/sha256_detached_v1.json --domain kcir.node --payload-hex <hex> --evidence-hex <hex> --ref-scheme-id <id> --ref-params-hash <hash> --ref-domain <domain> --ref-digest <digest> --json` — verify provided refs via profile-bound `verify_ref`.

## Workflow Instructions

- Provider workflow orchestration is outside Premath.
- Run the specific native checker affected by an edit; there is no repo-local task adapter.

## GitHub Ops Conventions

- `main` is protected and PR-only. Do not attempt direct pushes to `main`; use topic branch + PR.
- For governance/ops rollouts, record command evidence and resulting URLs in `.premath/OPERATIONS.md`; tracker note linkage belongs to the owning Tusk/downstream tracker.
- Forge branch-protection administration is external to the Premath checker contract.

## Memory Lane Discipline

- Keep work memory split across three lanes:
  - tracker lane: external Tusk/downstream tracker authority,
  - operations lane: `.premath/OPERATIONS.md` (runbooks and rollout evidence),
  - doctrine/decision lane: `specs/*` + `specs/process/decision-log.md` (boundary/lifecycle authority).
- Keep issue notes compact and reference operations/spec artifacts instead of pasting large transcripts.
- Use `docs/design/control-plane/MEMORY-LANES-CONTRACT.md` as the canonical write-discipline reference.

## Development Meta Workflow

- Do not re-derive process shape per task; use:
  - `docs/design/control-plane/DEVELOPMENT-META-LOOP.md`
  - `.premath/OPERATIONS.md` (`Development Meta Loop (Default)`)
- For non-trivial epics, keep dependency order explicit:
  1. architecture contract
  2. spec/index + doctrine-site glue
  3. control-plane parity
  4. implementation
  5. checker fixtures
  6. docs/traceability closure
- Maintain one bounded tracker item per worker session by default; discovered work must
  be captured in the owning Tusk/downstream tracker.
- Keep mutation authority instruction-linked for agent workers unless an
  explicit, auditable override mode is selected.

## Coding Style & Naming Conventions

- Rust style: `cargo fmt --all`; lint with `cargo clippy --workspace --all-targets -- -D warnings`.
- Keep modules focused; avoid duplicate “v2” naming in greenfield paths (prefer canonical names like `KCIR-CORE`).
- Use clear, domain-specific names (`*_witness`, `*_ref`, `policy_digest`, `normalizer_id`) that match spec terminology.
- Specs: update references to `draft/...` for promoted specs; keep `raw/...` references only for non-promoted docs.

## Testing Guidelines

- Treat the direct checker sequence in `README.md` as the minimum local merge gate.
- For spec/checker fixture edits, run `cargo test --workspace`, `premath traceability-check`, `premath coherence-check`, and `premath drift-budget-check`.
- For kernel/gate edits, run:
  - `cargo test --workspace`
  - `python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures`
  - `python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures`

## Commit & Pull Request Guidelines

- Use Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`).
- Keep PRs scoped to one concern (code, specs, or checker fixtures).
- For spec changes, include updated vectors/fixtures when behavior changes.
- Add a decision-log entry in `specs/process/decision-log.md` for lifecycle or boundary changes.
