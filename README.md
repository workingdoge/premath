# Premath — Total Spec (greenfield bundle)

**Bundle version:** `0.1.0` (parked)

This repository bundle contains a coherent, backend-generic Premath spec set.

Design goals:
- **Maximum expressiveness**: semantic structure lives in the kernel (reindexing + descent) and optional extensions.
- **Minimal encoding**: when interop is desired, normalization and equality reduce to deterministic *reference equality* (via `project_ref`) rather than large proof objects.
- **Backend-generic**: commitment backends (hash, Merkle, lattice, etc.) are profiles that implement `project_ref` + `verify_ref`. The kernel never hardcodes a scheme.

## Layout

- `specs/premath/draft/` — promoted draft contracts (normative for active claims)
- `specs/premath/raw/` — raw (experimental/informational) documents
- `specs/process/` — process docs (COSS lifecycle)
- `docs/foundations/` — explanatory foundations notes (non-normative)
- `docs/design/` — implementation-facing architecture notes (non-normative)

## Start here

- `specs/premath/draft/SPEC-INDEX.md` — what is normative, what claims exist (Kernel vs Interop), and suggested reading orders.
- `specs/premath/draft/DOCTRINE-INF.md` — doctrine/infinity-layer preservation contract.
- `specs/premath/draft/DOCTRINE-SITE.md` — doctrine-to-operation site map
  (`specs/premath/draft/DOCTRINE-SITE.json`).
- `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md` — typed instruction
  doctrine for LLM-driven control loops.
- `specs/premath/draft/PREMATH-KERNEL.md` — definability kernel (contractible descent).

## Conformance

Conformance is claim-based (profiles). See:

- `specs/premath/draft/CONFORMANCE.md`
- `specs/premath/draft/CAPABILITY-VECTORS.md`

Interop documents (NF/normalizer/refs/wire/errors) are normative **only when their corresponding interop claims are asserted**.

## Toy suites

This repo includes two small, executable suites that exercise the **Gate laws**:

- **Semantic toy suite**: `tools/toy/` + `tests/toy/fixtures/`
  - Fastest way to sanity-check stability/locality/descent.
  - Run: `python tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures`

- **KCIR toy suite**: `tools/kcir_toy/` + `tests/kcir_toy/fixtures/`
  - Compiles the semantic cases into **KCIR/NF-shaped fixtures**, then runs a
    minimal KCIR verifier + the same Gate checks.
  - Compile: `python tools/kcir_toy/compile_kcir_toy_fixtures.py --in tests/toy/fixtures --out tests/kcir_toy/fixtures`
  - Run: `python tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures`

> Note: the `tools/kcir_toy` fixture generator uses a SHA-256 toy binder for reproducibility only.
> The normative kernel remains commitment-backend agnostic via `draft/REF-BINDING`.

Python tooling dependency convention:

- `requirements.txt` is the authoritative dependency list for `tools/` scripts.
- It is currently stdlib-only (intentionally empty), but any future third-party imports must be declared there.

## Dev Environment (Nix + mise)

This repo supports a hybrid setup:

- `nix develop` provides system/native dependencies and shell tooling.
- `mise` pins repo runtime versions and provides task entrypoints.

Tracked files:

- `flake.nix` (system layer)
- `.mise.toml` (runtime/task layer)
- `.envrc` (`use flake` + `use mise`)

One-time direnv helper setup:

```bash
mise direnv activate > ~/.config/direnv/lib/use_mise.sh
direnv allow
```

Typical workflows:

```bash
# Nix-first lane
nix develop
mise install
mise run baseline

# Non-Nix lane
mise install
mise run baseline
```

`nix develop` also provides Terraform-compatible tooling (`opentofu`,
`terraform`) for optional infra-profile workflows.

## Workspace layering

Runtime crates are split by responsibility:

- `crates/premath-kernel`:
  - Generic laws only (contexts, covers, reindexing, descent, witnesses).
  - No storage or backend policy.
- `crates/premath-tusk`:
  - Minimal `tusk-core` runtime surface (run identity, descent pack artifacts,
    Gate-class mapping, witness envelope emission).
- `crates/premath-bd`:
  - Canonical memory/storage model (`Issue`, `Dependency`, JSONL, `MemoryStore`).
  - No orchestration with VCS or query backends.
- `crates/premath-surreal`:
  - Query/index cache over `MemoryStore` projections.
- `crates/premath-jj`:
  - JJ snapshot/status adapter.
- `crates/premath-cli`:
  - Composition point for workflows and verification commands.

This keeps the kernel backend-generic while allowing Beads-style workflows to
compose runtime (`tusk`) + storage (`bd`) + query (`surreal`) + versioning (`jj`) at the edges.

### Kernel vs KCIR note

Premath semantics and KCIR-style representation should stay decoupled:

- kernel semantics (`premath-kernel`) define laws and witness interfaces,
- KCIR is an optional representation profile for normalization/witness portability,
- any KCIR implementation should live behind an optional bridge profile rather than inside the kernel.

## Baseline gate

Run the local baseline closure gate before commit:

```bash
mise run baseline
```

Recommended pre-commit gate (includes format check):

```bash
mise run precommit
```

Optional repo-managed git hook:

```bash
git config core.hooksPath .githooks
```

This enforces the current invariant gate:

- Python tooling dependency install from `requirements.txt`,
- format check + clippy (`-D warnings`),
- build + Rust tests,
- toy semantic vectors,
- KCIR toy vectors,
- conformance capability invariance-stub validation,
- doctrine-to-operation site coherence validation,
- executable capability conformance vectors
  (`capabilities.normal_forms`, `capabilities.kcir_witnesses`, `capabilities.commitment_checkpoints`, `capabilities.squeak_site`, `capabilities.ci_witnesses`, `capabilities.instruction_typing`, `capabilities.change_projection`, `capabilities.ci_required_witness`).

Optional `hk` hook runner (configured in `hk.pkl`):

```bash
mise install
mise run hk-install
```

Manual runs:

```bash
mise run hk-pre-commit
mise run hk-pre-push
mise run hk-check
mise run ci-wiring-check
mise run ci-command-surface-check
mise run ci-pipeline-check
mise run ci-pipeline-test
mise run ci-observation-test
mise run ci-observation-build
mise run ci-observation-query
mise run ci-observation-check
mise run ci-required
mise run ci-verify-required
mise run ci-verify-required-strict
mise run ci-verify-required-strict-native
mise run ci-decide-required
mise run ci-verify-decision
mise run ci-required-verified
mise run ci-required-attested
mise run ci-pipeline-required
mise run ci-check
mise run ci-instruction-check
mise run ci-instruction-smoke
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-pipeline-instruction
```

`hk` keeps fast hygiene checks in `pre-commit` and runs the required projected
closure gate (`mise run ci-required-attested`) on `pre-push`/`check`. This is optional and can coexist
with `.githooks`-based local hooks.

`mise run ci-required` is the canonical SqueakSite gate entrypoint:

- computes deterministic change projection (`Delta -> requiredChecks`)
- executes only required checks through `tools/ci/run_gate.sh`
- emits `artifacts/ciwitness/<projection-digest>.json`
- updates `artifacts/ciwitness/latest-required.json` for verification
- writes `artifacts/ciwitness/latest-delta.json` as single-source strict-compare input
- emits per-check gate envelopes under
  `artifacts/ciwitness/gates/<projection-digest>/`
- includes deterministic `gateWitnessRefs` linkage in `ci.required.v1` witnesses
- labels each gate ref with provenance source (`native` or `fallback`)
- prefers native runner/task gate envelope artifacts when present, with
  deterministic fallback emission when unavailable
- `mise run ci-verify-required` verifies witness determinism/binding
- `mise run ci-required-verified` runs both execution and verification
- `mise run ci-decide-required` emits deterministic `accept|reject` from verified witness
- `mise run ci-required-attested` runs the authoritative local/CI gate chain
  (`ci-required` + strict verify + decision + decision attestation)

- default: local execution (`PREMATH_SQUEAK_SITE_PROFILE=local`)
- optional external runner: set
  - `PREMATH_SQUEAK_SITE_PROFILE=external`
  - `PREMATH_SQUEAK_SITE_RUNNER=<executable path>`
  - legacy aliases still accepted:
    `PREMATH_EXECUTOR_PROFILE` / `PREMATH_EXECUTOR_RUNNER`

See `tools/ci/README.md` for runner protocol details.

The current repo CI binding runs:

- `mise run ci-pipeline-check`
- `mise run ci-pipeline-test`
- `python3 tools/ci/pipeline_required.py`

Provider-specific required-check mappings are documented in
`docs/design/CI-PROVIDER-BINDINGS.md`.

`ci-verify-required-strict` uses `--compare-delta` and compares witness
`changedPaths` against `artifacts/ciwitness/latest-delta.json` when present
(fallback: detected VCS delta).
Provider-neutral CI refs:
- `PREMATH_CI_BASE_REF` (optional)
- `PREMATH_CI_HEAD_REF` (optional, default `HEAD`)

CI also publishes:

- `artifacts/ciwitness/latest-required.json`,
- `artifacts/ciwitness/latest-required.sha256`,
- `artifacts/ciwitness/latest-delta.json`,
- `artifacts/ciwitness/latest-delta.sha256`,
- `artifacts/ciwitness/latest-decision.json`,
- `artifacts/ciwitness/latest-decision.sha256`,
- projection-specific witness files (`artifacts/ciwitness/proj1_*.json`),
- a workflow summary row with projection digest, verdict, decision, and digest values.

Observation surface (frontend/query projection):

- `mise run ci-observation-build` builds
  - `artifacts/observation/latest.json` (deterministic read model),
  - `artifacts/observation/events.jsonl` (append-friendly projection feed).
- `mise run ci-observation-query` returns judgment-oriented views
  (`latest`, `needs_attention`, `instruction`, `projection`).
- `mise run ci-observation-check` enforces that observation output is a pure
  projection of CI witness artifacts (no semantic drift).
- This projection layer is where a Surreal-backed UI/read API should attach;
  semantic truth remains in CI witnesses and gate envelopes.

`mise run ci-check` is retained as a compatibility task for fixed full-gate
execution via `hk-check`.

Instruction-envelope flow:

```bash
mise run ci-instruction-check
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-pipeline-instruction
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-instruction
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
mise run ci-instruction-smoke
```

This executes requested checks through the same gate surface and writes a CI
witness artifact to `artifacts/ciwitness/<instruction-id>.json`.

GitHub manual dispatch workflow: `.github/workflows/instruction.yml`
(`instruction_path`, optional `allow_failure`) validates envelope shape first,
then runs the instruction and uploads the witness artifact.

Optional Terraform/OpenTofu provisioning shape:

```bash
mise run infra-up
mise run ci-check-tf
mise run infra-down
```

This keeps admissibility/gate semantics in `hk` while moving substrate startup
into a separate infra plane (`tools/infra/terraform/`).

Default infra profile is `local` (same semantics, Terraform-bound runner).
An experimental Darwin microVM runtime profile is available:

```bash
mise run ci-check-tf-local
mise run ci-check-tf-microvm
```

Treat `darwin_microvm_vfkit` as an optional runtime adapter path, not baseline
CI required flow. Current microvm profile is prototype-level.

Design framing for this control loop: `docs/design/HIGHER-ORDER-CI-CD.md`.

### Optional Pitchfork Runtime Orchestration

`pitchfork` is optional and used as an orchestration layer for local long-lived
or scheduled dev processes; it does not replace hk gate semantics.

```bash
mise install
mise run pf-start
mise run pf-status
mise run pf-stop
```

Optional scheduled gate loop:

```bash
mise run pf-gate-loop-start
mise run pf-gate-loop-stop
```

Current `pitchfork.toml` daemons:

- `docs-preview`: serves `docs/` on `http://127.0.0.1:43173`
- `gate-check-loop`: optional local closure loop (`mise run ci-required-attested`, then sleep 30m)

### JJ Glue (control plane)

If you want JJ-native command flow while keeping the same gate semantics:

```bash
mise run jj-alias-install
```

This installs repo-local aliases:

```bash
jj gate-fast         # hk fix profile (all files, no staging)
jj gate-fix          # hk fix profile (all files, no staging)
jj gate-check        # required projected closure gate
jj gate-pre-commit   # hk pre-commit profile (git-staged flow)
```

This keeps `hk` as the gate engine and uses `jj` as the trigger/orchestration
surface.

## Tusk Runtime Sketch (CLI)

`premath-cli` now includes two runtime-facing commands for `premath-tusk`:

- `premath mock-gate --json`
  - emits a deterministic Gate witness envelope from synthetic failures.
- `premath tusk-eval --identity <run_identity.json> --descent-pack <descent_pack.json> --json`
  - evaluates a `DescentPack` with a deterministic v0 policy and emits:
    - Gate witness envelope
    - optional `GlueResult` when admissible.
