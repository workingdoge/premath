# Premath — Total Spec (greenfield bundle)

**Bundle version:** `0.1.0` (parked)

This repository bundle contains a coherent, backend-generic Premath spec set.

Design goals:
- **Maximum expressiveness**: semantic structure lives in the kernel (reindexing + descent) and optional extensions.
- **Minimal encoding**: when interop is desired, normalization and equality reduce to deterministic *reference equality* (via `project_ref`) rather than large proof objects.
- **Backend-generic**: commitment backends (hash, Merkle, lattice, etc.) are profiles that implement `project_ref` + `verify_ref`. The kernel never hardcodes a scheme.

## System in 30 seconds

- **Semantic authority**: kernel + obligation/gate specs decide admissibility (`PREMATH-KERNEL`, `OBLIGATION-DISCHARGE`, `GATE`).
- **Control-plane consistency**: coherence checker enforces spec/docs/contract parity and emits deterministic checker witnesses.
- **Operational runtime**: harness contracts govern typed runtime loops, typestate closure, and retry/escalation behavior without adding semantic authority.
- **Regression discipline**: claim-gated conformance vectors and doctrine/coherence checks keep behavior stable as capabilities evolve.

## Layout

- `specs/premath/draft/` — promoted draft contracts (normative for active claims)
- `specs/premath/profile/` — optional claim-scoped overlays
- `specs/premath/raw/` — raw (experimental/informational) documents
- `specs/process/` — process docs (COSS lifecycle)
- `docs/foundations/` — explanatory foundations notes (non-normative)
- `docs/design/` — implementation-facing notes grouped by runtime,
  transport, control-plane, and operations lanes (non-normative)

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

Harness typestate closure/mutation-gate conformance is currently exercised under
`capabilities.change_morphisms` (intentional bundling; not an independent
capability claim today).

## Toy suites

This repo includes two small, executable suites that exercise the **Gate laws**:

- **Semantic toy suite**: `premath toy-gate-check` + `tests/toy/fixtures/`
  - Fastest way to sanity-check stability/locality/descent through the Rust kernel.
  - Run: `cargo test -p premath-kernel --test toy_vectors`

- **KCIR toy suite**: `tools/kcir_toy/` + `tests/kcir_toy/fixtures/`
  - Compiles the semantic cases into **KCIR/NF-shaped fixtures**, then runs a
    minimal KCIR verifier + the Rust-native Gate check.
  - Compile: `python tools/kcir_toy/compile_kcir_toy_fixtures.py --in tests/toy/fixtures --out tests/kcir_toy/fixtures`
  - Run: `python tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures`

> Note: the `tools/kcir_toy` fixture generator uses a SHA-256 toy binder for reproducibility only.
> The normative kernel remains commitment-backend agnostic via `draft/REF-BINDING`.

Python tooling dependency convention:

- `requirements.txt` is the authoritative dependency list for `tools/` scripts.
- It is currently stdlib-only (intentionally empty), but any future third-party imports must be declared there.

## Dev Environment (Nix + Direct Scripts)

This repo supports a Nix-first setup:

- `direnv` + `nix develop` provide system/native dependencies and shell tooling.
- `tools/ci/run_task.sh` provides repo-local task entrypoints.
- `tools/ci/baseline_tasks.json` is the baseline composition manifest.

Tracked files:

- `flake.nix` (system layer)
- `.envrc` (repo-root-aware `use flake`)
- `tools/ci/run_task.sh` (task alias layer)
- `tools/ci/baseline_tasks.json` (baseline manifest)

Direnv setup:

```bash
direnv allow
```

Typical workflows:

```bash
# Nix-first lane (after direnv allow)
sh tools/ci/run_task.sh baseline

# One-shot lane without entering the shell
direnv exec . sh tools/ci/run_task.sh baseline

# Non-Nix lane
sh tools/ci/run_task.sh baseline
```

The flake shell uses `devenv.root`, so raw `nix develop` needs the same root
override that `.envrc` supplies automatically. Prefer `direnv exec . <cmd>` for
one-shot commands outside an activated shell.

## Workspace layering

Runtime crates are split by responsibility:

- `crates/premath-kernel`:
  - Generic laws only (contexts, covers, reindexing, descent, witnesses).
  - No storage or backend policy.
- `crates/premath-coherence`:
  - Typed coherence-obligation evaluator used by `premath coherence-check`.
  - Emits deterministic checker witness output over the coherence contract.
- `crates/premath-tusk`:
  - Minimal `tusk-core` runtime surface (run identity, descent pack artifacts,
    Gate-class mapping, witness envelope emission).
- `crates/premath-bd`:
  - Canonical memory/storage model (`Issue`, `Dependency`, JSONL, `MemoryStore`).
  - Projection-only spec-IR lane (`spec_ir`) for typed statement entity/edge
    indexing from draft artifacts.
  - No orchestration with VCS or query backends.
- `crates/premath-surreal`:
  - Query/index adapters (issue graph cache + observation-surface indexing).
- `crates/premath-ux`:
  - UX composition layer over query adapters (`latest`, `needs_attention`,
    `instruction`, `projection` views).
- `crates/premath-jj`:
  - JJ snapshot/status adapter.
- `crates/premath-cli`:
  - Composition point for workflows, verification commands, UX queries, and
    harness/control-plane command surfaces.

This keeps the kernel backend-generic while allowing Beads-style workflows to
compose runtime (`tusk`) + storage (`bd`) + query adapters (`surreal`) + UX
composition (`ux`) + versioning (`jj`) at the edges.

Work-memory authority model (current default profile):

- canonical long-running memory: `.premath/issues.jsonl` via `premath-bd`
- mutation path: instruction-mediated writes (`mutation_policy=instruction-linked`)
  with policy-scoped + capability-scoped authorization from instruction witness
  (`capabilityClaims`, `policyDigest`)
- operational mutation helpers: `issue_claim`/`issue_lease_renew`/`issue_lease_release`
  (deterministic multiagent lease protocol) and `issue_discover`
  (non-loss discovered work capture)
- write evidence: mutation witness with optional JJ snapshot attribution
- query/read acceleration: `premath-surreal` projection/cache (rebuildable, non-authoritative)

### Kernel vs KCIR note

Premath semantics and KCIR-style representation should stay decoupled:

- kernel semantics (`premath-kernel`) define laws and witness interfaces,
- KCIR is an optional representation profile for normalization/witness portability,
- any KCIR implementation should live behind an optional bridge profile rather than inside the kernel.

## Baseline gate

Run the local baseline closure gate before commit:

```bash
sh tools/ci/run_task.sh baseline
```

Recommended pre-commit gate (includes format check):

```bash
sh tools/ci/run_task.sh precommit
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
- coherence-contract obligation discharge validation,
- authority-map/traceability validation,
- drift-budget sentinel validation across docs/contracts/checkers/cache-closure,
- doctrine-to-operation site coherence validation (including MCP
  doctrine-operation parity),
- executable capability conformance vectors
  (`capabilities.normal_forms`, `capabilities.kcir_witnesses`,
  `capabilities.commitment_checkpoints`, `capabilities.squeak_site`,
  `capabilities.ci_witnesses`, `capabilities.instruction_typing`,
  `capabilities.adjoints_sites`, `capabilities.change_morphisms`).

Optional `hk` hook runner (configured in `hk.pkl`):

```bash
sh tools/ci/run_task.sh hk-install
```

Manual runs:

```bash
sh tools/ci/run_task.sh hk-pre-commit
sh tools/ci/run_task.sh hk-pre-push
sh tools/ci/run_task.sh hk-check
sh tools/ci/run_task.sh ci-command-surface-check
sh tools/ci/run_task.sh ci-pipeline-check
sh tools/ci/run_task.sh ci-pipeline-test
sh tools/ci/run_task.sh ci-observation-test
sh tools/ci/run_task.sh ci-observation-build
sh tools/ci/run_task.sh ci-observation-query
sh tools/ci/run_task.sh ci-observation-serve
sh tools/ci/run_task.sh mcp-serve
sh tools/ci/run_task.sh ci-observation-check
sh tools/ci/run_task.sh ci-drift-budget-check
sh tools/ci/run_task.sh ci-required
sh tools/ci/run_task.sh ci-verify-required
sh tools/ci/run_task.sh ci-verify-required-strict
sh tools/ci/run_task.sh ci-verify-required-strict-native
sh tools/ci/run_task.sh ci-decide-required
sh tools/ci/run_task.sh ci-verify-decision
sh tools/ci/run_task.sh ci-required-verified
sh tools/ci/run_task.sh ci-required-attested
sh tools/ci/run_task.sh ci-pipeline-required
sh tools/ci/run_task.sh coherence-check
sh tools/ci/run_task.sh doctrine-check
sh tools/ci/run_task.sh ci-check
sh tools/ci/run_task.sh ci-instruction-check
sh tools/ci/run_task.sh ci-instruction-smoke
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json sh tools/ci/run_task.sh ci-pipeline-instruction
```

`hk` keeps fast hygiene checks in `pre-commit` and runs the required projected
closure gate (`sh tools/ci/run_task.sh ci-required-attested`) on `pre-push`/`check`. This is optional and can coexist
with `.githooks`-based local hooks.

`sh tools/ci/run_task.sh ci-required` is the canonical SqueakSite gate entrypoint:

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
- `sh tools/ci/run_task.sh ci-verify-required` verifies witness determinism/binding
- `sh tools/ci/run_task.sh ci-required-verified` runs both execution and verification
- `sh tools/ci/run_task.sh ci-decide-required` emits deterministic `accept|reject` from verified witness
- `sh tools/ci/run_task.sh ci-required-attested` runs the authoritative local/CI gate chain
  (`ci-required` + strict verify + decision + decision attestation)

- default: local execution (`PREMATH_SQUEAK_SITE_PROFILE=local`)
- optional external runner: set
  - `PREMATH_SQUEAK_SITE_PROFILE=external`
  - `PREMATH_SQUEAK_SITE_RUNNER=<executable path>`
  - legacy aliases still accepted:
    `PREMATH_EXECUTOR_PROFILE` / `PREMATH_EXECUTOR_RUNNER`

See `tools/ci/README.md` for runner protocol details.

The current repo CI binding runs:

- `sh tools/ci/run_task.sh ci-pipeline-check`
- `sh tools/ci/run_task.sh ci-pipeline-test`
- `python3 tools/ci/pipeline_required.py`

Provider-specific required-check mappings are documented in
`docs/design/control-plane/CI-PROVIDER-BINDINGS.md`.

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

- `sh tools/ci/run_task.sh ci-observation-build` builds
  - `artifacts/observation/latest.json` (deterministic read model),
  - `artifacts/observation/events.jsonl` (append-friendly projection feed).
- projection now routes through one core command surface:
  - `cargo run --package premath-cli -- observe-build --repo-root .`
- `sh tools/ci/run_task.sh ci-observation-query` returns judgment-oriented views
  (`latest`, `needs_attention`, `instruction`, `projection`).
- `sh tools/ci/run_task.sh ci-observation-serve` starts a tiny UX HTTP read API over the same
  semantics (`GET /latest`, `GET /needs-attention`,
  `GET /instruction?id=<instruction_id>`,
  `GET /projection?digest=<projection_digest>[&match=typed|compatibility_alias]`).
  Projection lookup defaults to typed authority matching.
- `sh tools/ci/run_task.sh ci-observation-check` enforces that observation output is a pure
  projection of CI witness artifacts through `premath observe-check`.
- `docs/observation/index.html` is a lightweight human-facing dashboard view
  over the same API.
- This projection layer is where a Surreal-backed UI/read API should attach;
  semantic truth remains in CI witnesses and gate envelopes.

Dashboard quickstart:

```bash
sh tools/ci/run_task.sh ci-observation-build
sh tools/ci/run_task.sh ci-observation-serve
python3 -m http.server 43173 --directory docs
```

Open `http://127.0.0.1:43173/observation/` (default API:
`http://127.0.0.1:43174`).

One-command orchestration alternative:

```bash
sh tools/ci/run_task.sh pf-start
```

This starts both `docs-preview` and `observation-api`.

`sh tools/ci/run_task.sh ci-check` is retained as a compatibility task for fixed full-gate
execution via `hk-check`.

Instruction-envelope flow:

```bash
sh tools/ci/run_task.sh ci-instruction-check
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json sh tools/ci/run_task.sh ci-pipeline-instruction
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json sh tools/ci/run_task.sh ci-instruction
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
sh tools/ci/run_task.sh ci-instruction-smoke
```

This executes requested checks through the same gate surface and writes a CI
witness artifact to `artifacts/ciwitness/<instruction-id>.json`.

GitHub manual dispatch workflow: `.github/workflows/instruction.yml`
(`instruction_path`, optional `allow_failure`) validates envelope shape first,
then runs the instruction and uploads the witness artifact.

Optional Terraform/OpenTofu provisioning shape:

```bash
sh tools/ci/run_task.sh infra-up
sh tools/ci/run_task.sh ci-check-tf
sh tools/ci/run_task.sh infra-down
```

This keeps admissibility/gate semantics in `hk` while moving substrate startup
into a separate infra plane (`tools/infra/terraform/`).

Default infra profile is `local` (same semantics, Terraform-bound runner).
An experimental Darwin microVM runtime profile is available:

```bash
sh tools/ci/run_task.sh ci-check-tf-local
sh tools/ci/run_task.sh ci-check-tf-microvm
```

Treat `darwin_microvm_vfkit` as an optional runtime adapter path, not baseline
CI required flow. Current microvm profile is prototype-level.

Design framing for this control loop: `docs/design/control-plane/HIGHER-ORDER-CI-CD.md`.

### Optional Pitchfork Runtime Orchestration

`pitchfork` is optional and used as an orchestration layer for local long-lived
or scheduled dev processes; it does not replace hk gate semantics.

```bash
sh tools/ci/run_task.sh pf-start
sh tools/ci/run_task.sh pf-status
sh tools/ci/run_task.sh pf-stop
```

Optional scheduled gate loop:

```bash
sh tools/ci/run_task.sh pf-gate-loop-start
sh tools/ci/run_task.sh pf-gate-loop-stop
```

Current `pitchfork.toml` daemons:

- `docs-preview`: serves `docs/` on `http://127.0.0.1:43173`
- `observation-api`: runs the Observation Surface HTTP API on
  `http://127.0.0.1:43174` (with a deterministic pre-build step)
- `gate-check-loop`: optional local closure loop (`sh tools/ci/run_task.sh ci-required-attested`, then sleep 30m)

### JJ Glue (control plane)

If you want JJ-native command flow while keeping the same gate semantics:

```bash
sh tools/ci/run_task.sh jj-alias-install
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

`premath-cli` now includes runtime-facing commands for `premath-tusk` and
`premath-ux`, plus Beads-style issue-memory operations:

- `premath init [path]`
  - initializes `.premath/issues.jsonl` (migrates legacy `.beads/issues.jsonl` when present).
- `premath mock-gate --json`
  - emits a deterministic Gate witness envelope from synthetic failures.
- `premath tusk-eval --identity <run_identity.json> --descent-pack <descent_pack.json> --json`
  - evaluates a `DescentPack` with a deterministic v0 policy and emits:
    - Gate witness envelope
    - optional `GlueResult` when admissible.
- `premath observe --surface artifacts/observation/latest.json --mode latest --json`
  - queries Observation Surface v0 through `premath-ux` (backed by
    `premath-surreal` observation index adapter).
- `premath observe-serve --surface artifacts/observation/latest.json --bind 127.0.0.1:43174`
  - serves the same query contract over HTTP for frontend consumption.
- `premath mcp-serve --issues .premath/issues.jsonl --issue-query-backend jsonl --mutation-policy instruction-linked --surface artifacts/observation/latest.json --repo-root .`
  - serves MCP tools over stdio for agent integration.
  - `.premath/issues.jsonl` remains canonical memory; `surreal` backend mode is a query projection layer.
  - under `instruction-linked`, issue/dep writes require an accepted instruction
    witness with allowed `policyDigest` plus action capability claims
    (`capabilities.change_morphisms` + per-action claim or
    `capabilities.change_morphisms.all`).
  - data-plane tools: `init_tool`, `issue_ready`, `issue_list`,
    `issue_check`, `issue_backend_status`, `issue_blocked`, `issue_add`, `issue_claim`,
    `issue_lease_renew`, `issue_lease_release`, `issue_lease_projection`,
    `issue_discover`, `issue_update`, `dep_add`, `dep_remove`, `dep_replace`,
    `dep_diagnostics`,
    `observe_latest`, `observe_needs_attention`, `observe_instruction`,
    `observe_projection`.
  - operator flow (dependency integrity):
    - pre-dispatch check: call `dep_diagnostics` with `graphScope=active` and
      schedule work only when `integrity.hasCycle=false`.
    - forensic check: call `dep_diagnostics` with `graphScope=full` to inspect
      historical closed-cycle noise separately from active scheduling.
  - doctrine-gated tools: `instruction_check`, `instruction_run`
    (runs `tools/ci/pipeline_instruction.py` and emits CI witness artifacts).
- `premath issue add "Title" --issues .premath/issues.jsonl --json`
  - appends a new issue entry into JSONL-backed memory.
- `premath issue claim <issue-id> --assignee <name> --issues .premath/issues.jsonl --json`
  - atomically claims work by setting assignee and `in_progress` status.
- `premath issue discover <parent-issue-id> "Title" --issues .premath/issues.jsonl --json`
  - records discovered follow-up work and links it with `discovered-from`.
- `premath issue backend-status --issues .premath/issues.jsonl --repo . --projection .premath/surreal_issue_cache.json --json`
  - reports backend integration state (canonical JSONL refs/errors, surreal query projection provenance/freshness, and JJ availability/head metadata).
- `premath issue list --issues .premath/issues.jsonl --json`
  - lists issues with optional status/assignee filters.
- `premath issue check --issues .premath/issues.jsonl --json`
  - runs deterministic issue-memory contract checks (`epic` typing, active acceptance/verification sections, note-size warnings, compactness drift).
- `premath issue ready --issues .premath/issues.jsonl --json`
  - returns open issues with no unresolved blocking dependencies.
- `premath issue blocked --issues .premath/issues.jsonl --json`
  - returns non-closed issues with unresolved blocking dependencies.
- `premath issue update <issue-id> --status in_progress --issues .premath/issues.jsonl --json`
  - updates mutable issue fields and persists JSONL.
- `premath dep add <issue-id> <depends-on-id> --type blocks --issues .premath/issues.jsonl --json`
  - adds a typed dependency edge between existing issues.
- `premath dep remove <issue-id> <depends-on-id> --type blocks --issues .premath/issues.jsonl --json`
  - removes one typed dependency edge.
- `premath dep replace <issue-id> <depends-on-id> --from-type blocks --to-type related --issues .premath/issues.jsonl --json`
  - replaces one dependency edge type without manual JSONL edits.
- `premath dep diagnostics --issues .premath/issues.jsonl --graph-scope active|full --json`
  - reports scoped dependency graph integrity diagnostics (`graphScope`, `hasCycle`, `cyclePath`), defaulting to `active`.

### MCP Client Config Snippets

Use absolute paths in client configs so the server starts deterministically.

Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "premath": {
      "command": "sh",
      "args": [
        "-lc",
        "cd <ABS_REPO_ROOT> && sh tools/ci/run_task.sh mcp-serve"
      ]
    }
  }
}
```

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.premath]
command = "sh"
args = [
  "-lc",
  "cd <ABS_REPO_ROOT> && sh tools/ci/run_task.sh mcp-serve"
]
startup_timeout_sec = 180
```

After updating client config:

```bash
sh tools/ci/run_task.sh mcp-serve
```

Then restart the MCP client so it re-reads configuration.
