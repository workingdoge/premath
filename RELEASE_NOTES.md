# Premath kernel bundle — v0.1.0 (parked)

This bundle is the first **parked** milestone: a small, backend-generic Premath kernel
specification with runnable toy suites that exercise the Gate laws.

The goal of this release is to provide a stable base for the next step:
**real backends + conformance vectors**, rather than continued kernel redesign.

## What is committed

### Kernel

- **Definability kernel**: coherent reindexing + locality + **contractible descent**.
- **Fibre-space framing**: the kernel is presented as a Grothendieck fibre space
  (a fibration / “fibre stack”) over contexts.
- **Ontology agnostic**: the sameness level is parameterized by `≈`.

Normative entry points:

- `specs/premath/draft/PREMATH-KERNEL.md`
- `specs/premath/draft/GATE.md`

### Backend seam (commitment agnostic)

The kernel never hardcodes a cryptographic scheme.
All commitment backends (hash, Merkle, lattice, etc.) live behind:

- `specs/premath/draft/REF-BINDING.md`

The toy suites use a deterministic SHA-256 binder **for fixture reproducibility only**.

### Operational totality

- Normalization and deterministic comparison keys (`cmpRef`): `raw/NORMALIZER`.
- Bidirectional checking and obligation discharge: `draft/BIDIR-DESCENT`.
- Deterministic witness IDs: `draft/WITNESS-ID`.

## What is optional

These are included for completeness and reviewer-facing semantics, but do not change
the kernel profile:

- `raw/SEMANTICS-INFTOPOS` (informational)
- `raw/HYPERDESCENT` (capability)
- `raw/UNIVERSE` (extension)
- `raw/SPLIT-PRESENTATION` (implementation guidance)

## Runnable suites

### Semantic toy suite

Exercises stability/locality/descent and witness determinism.

```bash
python tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures
```

### KCIR-shaped toy suite

Compiles toy cases into KCIR/NF-shaped fixtures, verifies a minimal opcode slice,
then applies the Gate checks.

```bash
python tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures
```

Regenerate fixtures deterministically:

```bash
python tools/kcir_toy/compile_kcir_toy_fixtures.py \
  --in tests/toy/fixtures \
  --out tests/kcir_toy/fixtures
```

## Decision log

Project-level constraints are recorded in:

- `specs/process/decision-log.md`

## Next milestone

M1: replace the toy binder with at least one real backend profile implementation
(still behind `project_ref` / `verify_ref`), and expand conformance fixtures to
exercise cross-implementation interoperability.

---

Version: `0.1.0`
