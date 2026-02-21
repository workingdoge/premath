# KCIR Toy Gate Suite

This directory contains **non-normative tooling** that compiles the semantic toy
Gate cases (`tests/toy/fixtures/*/case.json`) into **KCIR-shaped fixtures** and
then runs a minimal verifier + Gate checker.

## Why this exists

The semantic toy suite (`tools/toy/*`) is the fastest way to sanity-check the
Gate laws.

This KCIR toy suite is the next step: it demonstrates that the same Gate
failures can be detected when the inputs are carried by the **KCIR/NF** layer
(commitment-addressed objects) instead of raw JSON values.

## What is (and is not) covered

Covered:
- a minimal KCIR node codec (legacy fixed32 wire)
- minimal opcode verification for:
  - `C_LITERAL`
  - `M_LITERAL`
  - `O_PRIM`
  - `O_MKGLUE` (a first proof-carrying descent witness trace)
  - `O_ASSERT_OVERLAP` / `O_ASSERT_TRIPLE` (proof-carrying overlap + cocycle checks)
  - `O_ASSERT_CONTRACTIBLE` (proof-carrying contractible gluing witness)
- ToyViews Base API: mapId encoding + coverSig hashing
- Gate checks (stability/locality/descent/contractible glue) evaluated in the
  toy semantic worlds

Not covered (yet):
- full opcode registry
- Σ/Π / Beck–Chevalley
- non-enumerative ("compressed") proofs of contractibility (e.g. lattice / norm-style witnesses).
  The opcode slot supports a schemeId+proofBytes hook, but the default toy scheme
  still enumerates internally.

## Commands

Compile KCIR fixtures from the semantic toy fixtures:

```bash
python tools/kcir_toy/compile_kcir_toy_fixtures.py \
  --in tests/toy/fixtures \
  --out tests/kcir_toy/fixtures
```

Run the KCIR toy suite:

```bash
python tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures
```
### Commitment backend note (informative)

The kernel specs are commitment-backend agnostic.
The KCIR toy suite uses `tools/kcir_toy/toy_ref.py` (SHA-256 + fixed32 digest encoding)
ONLY to make fixtures reproducible.
This does not constrain production backends (Merkle, lattice/R_q, etc.).

