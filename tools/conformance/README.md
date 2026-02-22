# Conformance Tools

This directory contains lightweight conformance validation helpers.

## `run_fixture_suites.py`

Runs the executable conformance fixture suites through one command surface:

- `interop-core` (`run_interop_core_vectors.py`)
- `gate` (`run_gate_vectors.py`)
- `witness-id` (`run_witness_id_vectors.py`)
- `kernel-profile` (`run_kernel_profile_vectors.py`)
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
  - `tools/conformance/run_capability_vectors.py` (`DEFAULT_EXECUTABLE_CAPABILITIES`)
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

## `run_capability_vectors.py`

Runs executable capability vectors (current set):

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

## `check_doctrine_site.py`

Validates doctrine-to-operation site coherence using:

- `specs/premath/draft/DOCTRINE-INF.md` (morphism registry),
- `specs/premath/draft/DOCTRINE-SITE.json` (site map),
- declaration-bearing spec sections (`Doctrine Preservation Declaration (v0)`),
- operation entrypoints referenced in the site map.

Checks include:

- declaration presence and exact set coherence (`preserved`/`notPreserved`),
- edge morphism validity against doctrine registry,
- cover/node references,
- reachability from doctrine root to all operation nodes.

Run:

```bash
python3 tools/conformance/check_doctrine_site.py
```

## `run_doctrine_inf_vectors.py`

Runs executable doctrine-inf semantic boundary vectors in:

- `tests/conformance/fixtures/doctrine-inf/`

Checks include deterministic reject/accept behavior for:

- edge morphisms within destination `preserved` declarations,
- edge morphisms outside destination `preserved` declarations,
- edge morphisms explicitly listed under destination `notPreserved`,
- overlap violations between `preserved` and `notPreserved`.

Run:

```bash
python3 tools/conformance/run_doctrine_inf_vectors.py
```
