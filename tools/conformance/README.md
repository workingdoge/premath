# Conformance Tools

This directory contains lightweight conformance validation helpers.

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
