# Premath Draft Specs — Fibre Space Surface

This directory contains the **normative fibre space** specifications.
Everything here is an active contract between editor and implementers.

## Four Pillars

### Doctrine
- `DOCTRINE-INF.md` — doctrine/infinity-layer preservation contract.

### Change
- `CHANGE-INF.md` — change discipline: record shape, fibred commuting-square
  rule, composition, change category, and descent conditions.
- `CHANGE-SITE.md` — doctrine-site change morphisms: typed mutations on
  `SITE-PACKAGE.json` with apply/compose semantics, descent conditions,
  self-application (bootstrap fixed-point), and transport action contract.

### Site
- `DOCTRINE-SITE.md` — doctrine-to-operation site map contract.
- `SITE-RESOLVE.md` — deterministic resolver/projection contract for
  operation-site-world selection and stable KCIR handoff refs.
- `WORLD-REGISTRY.md` — canonical world-profile and inter-world morphism
  table contract.

### Kernel
- `PREMATH-KERNEL.md` — semantic kernel contract.
- `GATE.md` — admissibility gate authority.
- `BIDIR-DESCENT.md` — obligation/discharge contract.
- `NORMALIZER.md` — deterministic normalized comparison.

## KCIR Integrity Layer

Deterministic identity, normal forms, and evidence lineage for the fibre space:

- `KCIR-CORE.md` — KCIR core identity contract.
- `REF-BINDING.md` — identity binding in evidence-bearing paths.
- `NF.md` — normal form contract.
- `WIRE-FORMATS.md` — wire format contract.
- `ERROR-CODES.md` — error code registry.

## Supporting Fibre Space

- `SPAN-SQUARE-CHECKING.md` — typed span/square witness contract.
- `EVIDENCE-INF.md` — abstract evidence discipline (Unified Evidence Plane).
- `EVIDENCE-SITE.md` — concrete site instantiation of evidence discipline.
- `WITNESS-ID.md` — witness identity contract.

## Navigation

- `SPEC-INDEX.md` — front door for claims, profiles, and normative scope.

## Minimal Authority Path

Read first: `SPEC-INDEX.md` → `DOCTRINE-INF.md` → `PREMATH-KERNEL.md` →
`BIDIR-DESCENT.md` + `GATE.md` → `NORMALIZER.md` + `REF-BINDING.md`.

## Related Directories

- `../contracts/` — JSON machine contracts (operational, compiled into
  binaries via `include_str!`).
- `../raw/` — demoted specs awaiting fibre space reworking.
- `../archive/` — retired specs with no fibre space analog.
- `../profile/` — optional profile overlays (normative only when claimed).
  - `ADJOINTS-AND-SITES.md` — adjoint/site overlay.
- `../../process/` — lifecycle/process policy and decision log.
