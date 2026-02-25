# Conformance Tools

This directory contains lightweight conformance validation helpers.

## `run_fixture_suites.py`

Runs the executable conformance fixture suites through one command surface:

- `interop-core` (`run_interop_core_vectors.py`)
- `gate` (`run_gate_vectors.py`)
- `witness-id` (`run_witness_id_vectors.py`)
- `kernel-profile` (`run_kernel_profile_vectors.py`)
- `statement-index` (`check_statement_index.py`)
- `statement-bindings` (`check_statement_bindings.py`)
- `statement-kcir` (`run_statement_kcir_vectors.py`)
- `statement-projection-lane` (`check_statement_projection_lane.py`)
- `doctrine-inf` (`run_doctrine_inf_vectors.py`)
- `coherence-contract` (`premath coherence-check`)
- `tusk-core` (`run_tusk_core_vectors.py`)
- `harness-typestate` (`run_harness_typestate_vectors.py`)
- `runtime-orchestration` (`run_runtime_orchestration_vectors.py`)
- `frontend-parity` (`run_frontend_parity_vectors.py`)
- `world-core` (`run_world_core_vectors.py`)
- `capabilities` (`run_capability_vectors.py`)

The runner computes a deterministic KCIR-style cache binding per suite using:

- `schemeId = kcir.cache.fixture-suite.v1`
- `domain = conformance.<suite>`
- `paramsHash` over runner parameters/command/python version
- `digest` over hashed suite input files

Cache artifacts live under `.premath/cache/conformance/` and are keyed by
`kcir1_<digest>`.

Run:

```bash
python3 tools/conformance/run_fixture_suites.py
```

Disable cache for one run:

```bash
PREMATH_CONFORMANCE_CACHE=0 python3 tools/conformance/run_fixture_suites.py --no-cache
```

## `check_stub_invariance.py`

Validates capability fixture stubs in:

- `tests/conformance/fixtures/capabilities/`

Checks include:

- `manifest.json` integrity and vector membership,
- `case.json` / `expect.json` existence and JSON validity,
- consistency (`capabilityId`, `vectorId`),
- invariance pair completeness (`semanticScenarioId` grouped pairs),
- invariance assertion presence for kernel verdict and Gate class stability.

Run:

```bash
python3 tools/conformance/check_stub_invariance.py
```

## `check_spec_traceability.py`

Validates promoted draft spec coverage matrix integrity using:

- `specs/premath/draft/SPEC-TRACEABILITY.md`
- `specs/premath/draft/` promoted draft spec set

Checks include:

- every promoted draft spec appears exactly once in the matrix,
- status is one of `covered|instrumented|gap`,
- `gap` rows carry target IDs (`T-*-*`),
- matrix rows do not reference unknown draft specs.

Run:

```bash
python3 tools/conformance/check_spec_traceability.py
```

## `check_docs_coherence.py`

Validates critical docs-to-executable coherence invariants:

- executable capability list parity across:
  - `specs/premath/draft/CAPABILITY-REGISTRY.json` (`executableCapabilities`)
  - `README.md`
  - `tools/conformance/README.md`
  - `specs/premath/draft/SPEC-INDEX.md` (§5.4)
- baseline gate task parity between:
  - `.mise.toml` (`[tasks.baseline]`)
  - `docs/design/CI-CLOSURE.md` baseline task list
- projected check ID parity between:
  - `tools/ci/change_projection.py` (`CHECK_ORDER`)
  - `docs/design/CI-CLOSURE.md` projected check list
- capability-scoped normative-vs-informative consistency in
  `specs/premath/draft/SPEC-INDEX.md` (§5.4/§5.5 conditional clauses).

Run:

```bash
python3 tools/conformance/check_docs_coherence.py
```

## `premath coherence-check`

Runs the typed coherence contract checker surface (`draft/PREMATH-COHERENCE`)
using machine artifact `specs/premath/draft/COHERENCE-CONTRACT.json`.

Checks include:

- scope non-contradiction (including bidir checker vocabulary alignment),
- capability parity across executable/docs/manifest surfaces,
- baseline and projected gate-chain parity,
- doctrine operation reachability for required operation paths,
- profile overlay traceability,
- transport functoriality vectors (identity/composition/naturality),
- coverage base-change/transitivity vectors over admissible covers,
- glue-or-witness contractibility vectors for descent outcomes.

Run via task:

```bash
mise run coherence-check
```

## `run_capability_vectors.py`

Runs executable capability vectors from typed registry:

- `specs/premath/draft/CAPABILITY-REGISTRY.json`

Current set:

- `capabilities.normal_forms`
- `capabilities.kcir_witnesses`
- `capabilities.commitment_checkpoints`
- `capabilities.squeak_site`
- `capabilities.ci_witnesses`
- `capabilities.instruction_typing`
- `capabilities.adjoints_sites`
- `capabilities.change_morphisms`

Checks include:

- deterministic accept/reject outcomes for each vector,
- optional deterministic rejection-class checks via
  `expect.json.expectedFailureClasses`,
- normalizer/policy binding behavior for normalized-mode vectors,
- SqueakSite location descriptor/overlap/glue behavior,
- instruction-envelope to CI witness determinism checks,
- typed/unknown instruction classification determinism checks,
- adjoint/site obligation compilation and discharge checks
  (`adjoint_triangle`, `beck_chevalley_sigma`, `beck_chevalley_pi`,
  `refinement_invariance`),
- deterministic `Delta -> requiredChecks` projection behavior,
- deterministic `ci.required` witness verification behavior,
- strict delta-compare witness verification behavior,
- boundary-authority lineage parity checks (kernel obligation registry ->
  proposal discharge -> coherence scope -> CI semantic witness classes),
- stale generated doctrine-site digest rejection checks,
- invariance pairing (`kernelVerdict` and Gate failure classes) across evidence profiles.

Run:

```bash
python3 tools/conformance/run_capability_vectors.py
```

## `run_interop_core_vectors.py`

Runs executable Interop Core vectors in:

- `tests/conformance/fixtures/interop-core/`

Checks include:

- deterministic ref projection/verification slices (`draft/REF-BINDING`),
- required KCIR domain table coverage (`draft/KCIR-CORE`),
- ObjNF/MorNF parser/constructor contracts (`draft/NF`),
- registered wire-format parse behavior (`draft/WIRE-FORMATS`),
- known error-code registry membership checks (`draft/ERROR-CODES`).

Ref projection/verification vectors run through the canonical CLI command
surface (`premath ref project` / `premath ref verify`) using profile artifact
`policies/ref/sha256_detached_v1.json`; no Python-side digest shim is used.

Run:

```bash
python3 tools/conformance/run_interop_core_vectors.py
```

## `run_gate_vectors.py`

Runs executable Gate vectors in:

- `tests/conformance/fixtures/gate/`

Checks include deterministic Gate witness behavior for:

- `stability_failure`
- `locality_failure`
- `descent_failure`
- `glue_non_contractible`

Run:

```bash
python3 tools/conformance/run_gate_vectors.py
```

## `run_witness_id_vectors.py`

Runs executable Witness-ID vectors in:

- `tests/conformance/fixtures/witness-id/`

Checks include deterministic witness-id behavior for:

- stability under excluded field changes (`message`, `sources`, `details`)
- sensitivity to canonical key fields (`class`, `lawRef`, `tokenPath`, `context`)

Run:

```bash
python3 tools/conformance/run_witness_id_vectors.py
```

## `run_harness_typestate_vectors.py`

Runs executable harness typestate closure vectors in:

- `tests/conformance/fixtures/harness-typestate/`

Checks include:

- deterministic accept/reject outcomes for each vector,
- deterministic `expectedFailureClasses` matching,
- deterministic `expectedJoinClosed` matching, and
- failure-class coverage parity guard:
  - every fail-closed class emitted by
    `crates/premath-cli/src/commands/harness_join_check.rs` MUST appear in at
    least one fixture `expect.json.expectedFailureClasses`,
  - fixture `expectedFailureClasses` entries MUST NOT contain classes not
    emitted by `harness-join-check`.

Run:

```bash
python3 tools/conformance/run_harness_typestate_vectors.py
```

## `run_kernel_profile_vectors.py`

Runs canonical cross-model kernel profile vectors in:

- `tests/conformance/fixtures/kernel-profile/`

Checks include deterministic semantic outcome parity for shared scenarios across:

- semantic toy fixtures (`tests/toy/fixtures`)
- KCIR toy fixtures (`tests/kcir_toy/fixtures`)

The stable compared projection is:

- `result`
- `failures[].{class, lawRef, witnessId}`

Run:

```bash
python3 tools/conformance/run_kernel_profile_vectors.py
```

## `run_tusk_core_vectors.py`

Runs deterministic Tusk runtime-contract vectors in:

- `tests/conformance/fixtures/tusk-core/`

Checks include stable runtime-eval boundary behavior for:

- accepted single-glue selection path,
- `locality_failure` mapping for missing locals,
- `locality_failure` mapping for missing multi-local compatibility evidence,
- `descent_failure` mapping for missing glue proposals,
- `descent_failure` mapping for missing mode bindings,
- `glue_non_contractible` mapping for multi-proposal ambiguity.

Run:

```bash
python3 tools/conformance/run_tusk_core_vectors.py
```

## `run_harness_typestate_vectors.py`

Runs deterministic harness typestate closure vectors in:

- `tests/conformance/fixtures/harness-typestate/`

Checks include deterministic reject/accept behavior for:

- `ToolUse -> JoinClosed` closure classes (`tool.use_missing`, `tool.join_incomplete`),
- protocol stop-reason handling (`protocol.stop_reason_unhandled`),
- protocol parallel transport ordering (`protocol.parallel_transport_order_invalid`),
- truncation metadata policy (`tool.response_truncation_policy_violation`),
- handoff artifact gating (`handoff.required_artifact_missing`),
- machine-readable error-envelope requirements for tool failures (`tool.schema_invalid`).

Run:

```bash
python3 tools/conformance/run_harness_typestate_vectors.py
```

## `run_runtime_orchestration_vectors.py`

Runs deterministic runtime orchestration vectors in:

- `tests/conformance/fixtures/runtime-orchestration/`

Checks include deterministic accept/reject behavior for:

- required route presence for Harness/Squeak runtime bindings,
- required morphism coverage on routed operation nodes,
- operation-path boundary enforcement for routed CI operations (`tools/ci/*`),
- required handoff-shape contract markers for `HARNESS-RUNTIME`,
- optional `controlPlaneKcirMappings` row-shape coverage when mapping rows are
  present,
- constructor world-route family coverage including
  `route.transport.dispatch` bound/missing reject paths,
- invariance scenario parity across profile-permuted vectors.

Run:

```bash
python3 tools/conformance/run_runtime_orchestration_vectors.py
```

## `run_frontend_parity_vectors.py`

Runs cross-frontend host-action parity vectors in:

- `tests/conformance/fixtures/frontend-parity/`

Checks include deterministic accept/reject behavior for:

- required frontend coverage for Steel/Rhai/MCP/CLI rows,
- core verdict/failure authority from `premath site-resolve --json`,
- kernel verdict parity across required frontend rows relative to core outputs,
- failure-class and witness-ref parity across equivalent host-action scenarios,
- adversarial world-route drift and transport-profile mismatch detection,
- invariance parity across profile-permuted vectors.

Run:

```bash
python3 tools/conformance/run_frontend_parity_vectors.py
```

## `run_world_core_vectors.py`

Runs deterministic world-core vectors in:

- `tests/conformance/fixtures/world-core/`

Checks include deterministic accept/reject behavior for:

- world-route family coverage and operation binding completeness,
- morphism-drift reject behavior on bound world routes,
- control-plane host-action to world-route binding closure through
  `premath world-registry-check --control-plane-contract ...`,
- resolver overlap/glue/ambiguity fail-closed behavior through
  `premath site-resolve --json`,
- KCIR projection invariance on equivalent site-package inputs
  (`sitePackageDigest` / `worldRouteDigest` / resolver witness semantic digest),
- invariance parity across profile-permuted vectors.

Run:

```bash
python3 tools/conformance/run_world_core_vectors.py
```

## `check_doctrine_site.py`

Validates doctrine-to-operation site coherence using:

- `specs/premath/draft/DOCTRINE-INF.md` (morphism registry),
- `specs/premath/draft/DOCTRINE-SITE-INPUT.json` (single input authority),
- `specs/premath/draft/DOCTRINE-SITE-CUTOVER.json` (migration/cutover phase
  authority),
- generated `specs/premath/draft/DOCTRINE-SITE.json` (canonical site map),
- generated `specs/premath/draft/DOCTRINE-OP-REGISTRY.json` (operation-node view),
- declaration-bearing spec sections (`Doctrine Preservation Declaration (v0)`),
- operation entrypoints referenced in the site map.

Checks include:

- generation roundtrip (`generated == tracked`),
- cutover contract validity and fail-closed generated-only posture
  (legacy source fallback/override disabled in active phase),
- declaration presence and exact set coherence (`preserved`/`notPreserved`),
- edge morphism validity against doctrine registry,
- cover/node references,
- reachability from doctrine root to all operation nodes.

Run:

```bash
python3 tools/conformance/check_doctrine_site.py
```

## `generate_doctrine_site.py`

Generates canonical doctrine-site artifacts from input contract +
declaration-bearing specs.

Run:

```bash
python3 tools/conformance/generate_doctrine_site.py
```

## `generate_doctrine_site_inventory.py`

Generates one canonical doctrine-site navigation index from site-package source
and tracked doctrine artifacts.

Outputs:

- `docs/design/generated/DOCTRINE-SITE-INVENTORY.json`
- `docs/design/generated/DOCTRINE-SITE-INVENTORY.md`

Inventory shape includes deterministic:

- site summary (`siteId`, topology counts),
- route-family -> world/morphism bindings,
- operation rows with route/world binding and command-surface refs
  (`path:*`, `hostAction:*`, `runtimeRoute:*`),
- cutover posture from `DOCTRINE-SITE-CUTOVER.json`.

Run (write):

```bash
python3 tools/conformance/generate_doctrine_site_inventory.py
```

Run (drift check):

```bash
python3 tools/conformance/generate_doctrine_site_inventory.py --check
```

Drift check (no write):

```bash
python3 tools/conformance/generate_doctrine_site.py --check
```

## `check_runtime_orchestration.py`

Validates Harness+Squeak runtime orchestration bindings using:

- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json` (`runtimeRouteBindings`),
- `specs/premath/draft/DOCTRINE-OP-REGISTRY.json` (operation-node bindings),
- `specs/premath/draft/HARNESS-RUNTIME.md` (required handoff-shape contract).

Canonical semantic authority lane:

- `premath runtime-orchestration-check`
  (`cargo run --package premath-cli -- runtime-orchestration-check ... --json`).
- `check_runtime_orchestration.py` is an adapter wrapper that invokes the
  canonical command and preserves doctrine-check command-surface compatibility.

Checks include:

- required runtime route presence in doctrine operation bindings,
- required morphism coverage on bound operation routes,
- routed operation path boundary enforcement (`tools/ci/*`),
- required Harness/Squeak handoff-section shape markers in runtime contract,
- optional `controlPlaneKcirMappings` row-shape validation when mapping rows are
  provided,
- world-route checks delegated to the core command lane
  (`premath world-registry-check`) rather than duplicated wrapper semantics,
- wrapper/runtime faults return non-semantic wrapper errors (`result=error`)
  and do not synthesize semantic failure classes.

Run:

```bash
python3 tools/conformance/check_runtime_orchestration.py
```

Direct core command:

```bash
cargo run --package premath-cli -- runtime-orchestration-check \
  --control-plane-contract specs/premath/draft/CONTROL-PLANE-CONTRACT.json \
  --doctrine-op-registry specs/premath/draft/DOCTRINE-OP-REGISTRY.json \
  --harness-runtime specs/premath/draft/HARNESS-RUNTIME.md \
  --doctrine-site-input specs/premath/draft/DOCTRINE-SITE-INPUT.json \
  --json
```

## `run_doctrine_inf_vectors.py`

Runs executable doctrine-inf semantic boundary vectors in:

- `tests/conformance/fixtures/doctrine-inf/`

Checks include deterministic reject/accept behavior for:

- edge morphisms within destination `preserved` declarations,
- edge morphisms outside destination `preserved` declarations,
- edge morphisms explicitly listed under destination `notPreserved`,
- overlap violations between `preserved` and `notPreserved`,
- route-consolidation closure through kernel world-route checks
  (`premath doctrine-inf-check` -> `world_route_*` fail-closed classes),
- claim-gated governance-profile (`profile.doctrine_inf_governance.v0`) checks
  for policy provenance pinning + digest mismatch, guardrail stage
  presence/order, eval gate threshold success, eval lineage evidence fields,
  observability mode validity, risk-tier control binding, and self-evolution
  declaration requirements (retry/escalation/rollback).

Governance claimed vectors are repository-claim-gated via
`specs/premath/draft/CAPABILITY-REGISTRY.json` `profileOverlayClaims`.
When the repository does not claim `profile.doctrine_inf_governance.v0`, vectors
with `governanceProfile.claimed=true` are skipped unless
`--ignore-repo-claims` is set.

Run:

```bash
python3 tools/conformance/run_doctrine_inf_vectors.py
```
