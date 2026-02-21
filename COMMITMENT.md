# Premath bundle commitment (v0.1)

This repository is structured to avoid drifting into "definability for definability".

**Commitment:** the baseline Premath kernel bundle is *minimal, backend-generic, and totalized operationally by a normalizer + deterministic descent obligations*.

The binding decision is recorded in:
- `specs/process/decision-log.md` (Decision 0001, dated 2026-02-19)

## What is committed

### Normative kernel bundle
These documents define the committed baseline:

- `specs/premath/draft/PREMATH-KERNEL.md` — definability = coherent reindexing + contractible descent
- `specs/premath/draft/GATE.md` — admissibility gate + failure classes
- `specs/premath/draft/REF-BINDING.md` — backend-generic reference binding (`project_ref`/`verify_ref`)
- `specs/premath/draft/NF.md` — canonical ObjNF/MorNF byte grammars
- `specs/premath/raw/NORMALIZER.md` — canonicalization + comparison keys (`cmpRef`)
- `specs/premath/draft/BIDIR-DESCENT.md` — synthesis/checking + obligation emission/discharge
- `specs/premath/draft/KCIR-CORE.md` — reference model + store + profile interface
- `specs/premath/raw/OPCODES.md` — opcode contracts phrased via canonical bytes + `project_ref`
- `specs/premath/raw/DSL.md`, `specs/premath/draft/ERROR-CODES.md`, `specs/premath/draft/WITNESS-ID.md`, `specs/premath/draft/WIRE-FORMATS.md`, `specs/premath/draft/CONFORMANCE.md`

### Optional, non-kernel dials
These documents are explicitly **non-binding** on the kernel profile:

- `specs/premath/raw/SEMANTICS-INFTOPOS.md` — Premath∞ semantics (informational)
- `specs/premath/raw/HYPERDESCENT.md` — optional capability: hyperdescent
- `specs/premath/raw/UNIVERSE.md` — optional extension: universe + comprehension
- `specs/premath/raw/SPLIT-PRESENTATION.md` — implementation guidance (strict IR vs semantic equality)

## Next step

The next step is **running code** + **vectors** that exercise the baseline bundle:

1. Implement `project_ref` for at least one backend profile.
2. Implement NF parsing + deterministic normalization (`NORMALIZER`).
3. Implement Gate checks via deterministic obligations (`BIDIR-DESCENT` + `GATE`).
4. Build golden/adversarial fixtures that prove determinism and correct failure classes.
