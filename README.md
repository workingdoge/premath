# Premath — Total Spec (greenfield bundle)

**Bundle version:** `0.1.0` (parked)

This repository bundle contains a coherent, backend-generic Premath spec set.

Design goals:
- **Maximum expressiveness**: semantic structure lives in the kernel (reindexing + descent) and optional extensions.
- **Minimal encoding**: when interop is desired, normalization and equality reduce to deterministic *reference equality* (via `project_ref`) rather than large proof objects.
- **Backend-generic**: commitment backends (hash, Merkle, lattice, etc.) are profiles that implement `project_ref` + `verify_ref`. The kernel never hardcodes a scheme.

## System in 30 seconds

- **Semantic authority**: kernel + gate specs decide admissibility (`PREMATH-KERNEL`, `GATE`, `BIDIR-DESCENT`).
- **Control-plane consistency**: coherence checker enforces spec/docs/contract parity and emits deterministic checker witnesses.
- **Operational runtime**: harness contracts govern typed runtime loops, typestate closure, and retry/escalation behavior without adding semantic authority.
- **Regression discipline**: claim-gated conformance vectors and doctrine/coherence checks keep behavior stable as capabilities evolve.

## What We Are Building

Premath is a worldized semantic control plane:

- repository/control states are contexts,
- specs/contracts/witnesses are definables indexed by those contexts,
- route families are bound to explicit world profiles with deterministic
  morphism rows,
- lease orchestration (including BEAM/Rustler adapters) is bound to
  `world.lease.v1` (`route.issue_claim_lease`) and checked through the core
  `premath world-registry-check` surface.

North-star rule:

- one admissibility authority lane (kernel/Gate + checker contracts),
- adapters/wrappers are transport and execution IO only,
- optional overlays (for example torsor/extension interpretation) stay
  evidence-only and never become direct acceptance authority.

## Canonical Frontend Flow (One Authority Lane)

All frontend/runtime entrypoints follow the same path:

```text
Frontend adapter (Steel | Rhai | CLI | MCP | optional NIF)
  -> host action
  -> site resolver decision (INF -> SITE -> WORLD)
  -> typed transport envelope
  -> world-route kernel check
  -> mutation/evidence projection
```

Boundary command surfaces:

| Boundary | Canonical surface |
| --- | --- |
| Frontend host-action execution | `premath scheme-eval`; `premath rhai-eval`; `premath mcp-serve` |
| Site resolver decision | `premath site-resolve` |
| Typed transport dispatch | `premath transport-dispatch`; `premath transport-check` |
| World-route admissibility | `premath world-registry-check`; `premath world-gate-check` |
| Mutation/evidence emission | `premath issue ...`; `premath instruction-*`; `premath required-*` |

Rhai/Steel/MCP/NIF are adapter-only frontends over this lane. They do not
introduce independent mutation authority.

## INF/SITE/WORLD Resolver Map

Primary newcomer mental model:

- `INF`: semantic obligations and preservation classes
  (`specs/premath/draft/DOCTRINE-INF.md`).
- `SITE`: operation topology and route eligibility
  (`specs/premath/draft/DOCTRINE-SITE-INPUT.json`,
  `specs/premath/draft/DOCTRINE-OP-REGISTRY.json`).
- `WORLD`: route-family to world/morphism bindings
  (`specs/premath/draft/WORLD-REGISTRY.md`).
- `RESOLVER`: deterministic selection over INF/SITE/WORLD
  (`premath site-resolve`, `specs/premath/draft/SITE-RESOLVE.md`).

Generated entrypoint for this map:

- `docs/design/generated/DOCTRINE-SITE-INVENTORY.md`
  (`site -> operations -> route families -> world bindings -> command surfaces`).

Why this exists:

- keep multi-agent runtime evolution expressive without adding parallel
  semantics,
- keep CI/control behavior auditable through typed route and witness bindings,
- keep refactors safe via executable golden/adversarial/invariance closure.

## Newcomer Path (20 Minutes)

Read these in order:

1. `README.md` (this page) for boundary shape and command surface.
2. `docs/design/generated/DOCTRINE-SITE-INVENTORY.md` for the generated
   INF/SITE/WORLD navigation index.
3. `specs/premath/draft/SPEC-INDEX.md` for what is normative vs optional.
4. `specs/premath/draft/WORLD-REGISTRY.md` for world/morphism/route binding.
5. `specs/premath/draft/PREMATH-KERNEL.md` and `specs/premath/draft/GATE.md`
   for admissibility authority.
6. `docs/design/ARCHITECTURE-MAP.md` for implementation placement.

Then run:

- `mise run doctrine-check`
- `mise run coherence-check`

## Authority Map

| Concern | Authoritative spec(s) | Executable checker/runner | Command |
| --- | --- | --- | --- |
| Semantic admissibility | `specs/premath/draft/PREMATH-KERNEL.md`, `specs/premath/draft/GATE.md`, `specs/premath/draft/BIDIR-DESCENT.md` | Coherence + gate/toy vectors | `mise run coherence-check` |
| World/route bindings | `specs/premath/draft/WORLD-REGISTRY.md`, `specs/premath/draft/DOCTRINE-SITE-INPUT.json`, `specs/premath/draft/CONTROL-PLANE-CONTRACT.json` | Core world-registry command + world-core conformance parity + runtime adapter parity checker | `cargo run --package premath-cli -- world-registry-check ... --json`; `python3 tools/conformance/run_world_core_vectors.py`; `python3 tools/conformance/check_runtime_orchestration.py --json` |
| Control-plane parity | `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`, `specs/premath/draft/PREMATH-COHERENCE.md` | Premath coherence checker | `mise run coherence-check` |
| Docs/spec linkage | `specs/premath/draft/SPEC-INDEX.md`, `specs/premath/draft/SPEC-TRACEABILITY.md` | Docs + traceability checks | `mise run docs-coherence-check` |
| Regression vectors | `specs/premath/draft/CONFORMANCE.md`, `specs/premath/draft/CAPABILITY-VECTORS.md` | Fixture-suite + capability vector runners | `mise run conformance-run` |

### Canonical World Semantics Map

World semantics live in one executable lane:

1. `crates/premath-kernel/src/world_registry.rs`:
   route-family/world/morphism validation semantics + canonical failure classes.
2. `crates/premath-cli/src/commands/world_registry_check.rs`:
   core command surface (`premath world-registry-check`) and control-plane-derived
   required world-route bindings.
3. `tools/conformance/run_world_core_vectors.py`:
   semantic conformance lane (golden/adversarial/invariance) that replays
   fixture expectations against core command outputs.

Wrapper surfaces are non-authority adapters:

- `tools/conformance/check_runtime_orchestration.py` aggregates contract/runtime
  checks and invokes the core world command.
- `tests/conformance/fixtures/runtime-orchestration/` now carries runtime-route
  adapter parity vectors (world semantics are centralized in
  `tests/conformance/fixtures/world-core/`).

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
- `specs/premath/draft/WORLD-REGISTRY.md` — canonical world/morphism/route
  binding contract (`world == premath`).
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
- `crates/premath-doctrine`:
  - Canonical doctrine/control-plane contract parsing + world-descent requirement
    derivation used by kernel/CLI surfaces.
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
- `crates/premath-transport`:
  - transport-facing lease bridge over canonical issue-memory semantics.
  - Optional `rustler_nif` feature exports a generic NIF dispatcher
    (`dispatch`) over canonical `action + payload` transport envelopes while
    preserving world-route binding metadata
    (`route.issue_claim_lease` -> `world.lease.v1`,
    `route.fiber.lifecycle` -> `world.fiber.v1`).
  - `premath transport-check` validates typed transport action registry closure
    (`action`/`actionId`/route/world/morphism + semantic digest).
  - `premath transport-dispatch` executes typed transport envelopes and emits
    deterministic dispatch metadata (`dispatchKind`, `profileId`, `actionId`,
    `semanticDigest`) for lease actions plus structured-concurrency actions
    (`fiber.spawn|join|cancel`).
  - Additional transports (for example gRPC request/response wrappers) should
    reuse the same dispatcher contract and remain adapter-only.
  - Default build remains Erlang-free; BEAM runtime is required only when
    loading the produced NIF into Elixir/Erlang.
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
- coherence-contract obligation discharge validation,
- docs-to-executable coherence validation,
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
mise run ci-observation-serve
mise run mcp-serve
mise run ci-observation-check
mise run ci-drift-budget-check
mise run ci-required
mise run ci-verify-required
mise run ci-verify-required-strict
mise run ci-verify-required-strict-native
mise run ci-decide-required
mise run ci-verify-decision
mise run ci-required-verified
mise run ci-required-attested
mise run ci-pipeline-required
mise run coherence-check
mise run doctrine-check
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
- projection now routes through one core command surface:
  - `cargo run --package premath-cli -- observe-build --repo-root .`
- `mise run ci-observation-query` returns judgment-oriented views
  (`latest`, `needs_attention`, `instruction`, `projection`).
- `mise run ci-observation-serve` starts a tiny UX HTTP read API over the same
  semantics (`GET /latest`, `GET /needs-attention`,
  `GET /instruction?id=<instruction_id>`,
  `GET /projection?digest=<projection_digest>[&match=typed|compatibility_alias]`).
  Projection lookup defaults to typed authority matching.
- `mise run ci-observation-check` enforces that observation output is a pure
  projection of CI witness artifacts (no semantic drift).
- `docs/observation/index.html` is a lightweight human-facing dashboard view
  over the same API.
- This projection layer is where a Surreal-backed UI/read API should attach;
  semantic truth remains in CI witnesses and gate envelopes.

Dashboard quickstart:

```bash
mise run ci-observation-build
mise run ci-observation-serve
python3 -m http.server 43173 --directory docs
```

Open `http://127.0.0.1:43173/observation/` (default API:
`http://127.0.0.1:43174`).

One-command orchestration alternative:

```bash
mise run pf-start
```

This starts both `docs-preview` and `observation-api`.

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
- `observation-api`: runs the Observation Surface HTTP API on
  `http://127.0.0.1:43174` (with a deterministic pre-build step)
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

`premath-cli` now includes runtime-facing commands for `premath-tusk` and
`premath-ux`, plus Beads-style issue-memory operations:

- `premath init [path] [--json]`
  - initializes `.premath/issues.jsonl` (migrates legacy `.beads/issues.jsonl` when present) with text or deterministic JSON output.
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
  - runs deterministic issue-memory contract checks (`epic` typing, active acceptance/verification sections, note-size warnings).
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

### Evaluator Metadata Precedence

`premath scheme-eval` and `premath rhai-eval` share one metadata model:

- scalar defaults (`issueId`, `policyDigest`, `instructionRef`) resolve by
  precedence: `call-level > CLI flags > program defaults`.
- capability claims resolve by deterministic union + dedupe across
  `program-level`, `CLI --capability-claim`, and `call-level` claims.
- mutation-capable actions still require the same evidence/capability checks
  (`policyDigest`, `instructionRef`, and action claims) regardless of frontend.

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
        "cd <ABS_REPO_ROOT> && mise run mcp-serve"
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
  "cd <ABS_REPO_ROOT> && mise run mcp-serve"
]
startup_timeout_sec = 180
```

After updating client config:

```bash
mise install
mise run mcp-serve
```

Then restart the MCP client so it re-reads configuration.
