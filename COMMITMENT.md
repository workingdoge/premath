# Premath commitment (current posture)

This file is a concise posture summary. Normative authority remains in
`specs/premath/draft/SPEC-INDEX.md` and related promoted specs.

The original baseline binding decision is recorded in:
- `specs/process/decision-log.md` (Decision 0001)

## What remains committed

### Kernel and interop authority

The repository keeps one semantic authority lane:

- kernel admissibility and discharge authority stays in
  `specs/premath/draft/PREMATH-KERNEL.md`,
  `specs/premath/draft/GATE.md`, and
  `specs/premath/draft/BIDIR-DESCENT.md`;
- interop identity/checking surfaces remain deterministic and profile-scoped via
  `specs/premath/draft/REF-BINDING.md`,
  `specs/premath/draft/NF.md`,
  `specs/premath/draft/NORMALIZER.md`,
  `specs/premath/draft/KCIR-CORE.md`,
  `specs/premath/draft/WITNESS-ID.md`,
  `specs/premath/draft/WIRE-FORMATS.md`,
  `specs/premath/draft/ERROR-CODES.md`.

### Control-plane and harness discipline

Control-plane execution remains deterministic and fail-closed through:

- `specs/premath/draft/PREMATH-COHERENCE.md` +
  `specs/premath/draft/COHERENCE-CONTRACT.json`,
- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`,
- `specs/premath/draft/HARNESS-RUNTIME.md`,
  `specs/premath/draft/HARNESS-TYPESTATE.md`,
  `specs/premath/draft/HARNESS-RETRY-ESCALATION.md`.

### Doctrine and unification program

Architecture/lifecycle commitments are governed by:

- `specs/premath/draft/UNIFICATION-DOCTRINE.md`,
- `specs/premath/draft/DOCTRINE-INF.md`,
- `specs/premath/draft/CONFORMANCE.md` (claim-gated capability/profile lanes).

## Current state tracking

For active status and sequence, use:

- issue authority: `.premath/issues.jsonl`,
- lifecycle decisions: `specs/process/decision-log.md`,
- roadmap pointer surface: `ROADMAP.md`.
