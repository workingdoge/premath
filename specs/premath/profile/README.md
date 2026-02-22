# Premath Profile Overlays

This directory contains optional **profile overlays**.

Profile overlays are additive to the base draft claims:

- they are **normative only when explicitly claimed**,
- they do not replace `draft/PREMATH-KERNEL`,
- and they must compile to deterministic checker/discharge behavior.

Placement rule:

- Semantic overlay capabilities (for example SigPi adjoint/site obligations)
  belong in `profile/`.
- CwF strict substitution/comprehension equalities remain checker-core
  obligations in `draft/PREMATH-COHERENCE` and are not profile overlays.
- Span/square commutation remains a typed witness contract in
  `draft/SPAN-SQUARE-CHECKING`; profiles MAY reference it when composing
  overlays, but do not own it.

Current overlays:

- `ADJOINTS-AND-SITES.md` — capability-scoped adjoint/site obligations over the
  kernel context-site base, discharged under `(normalizerId, policyDigest)`,
  with composed SigPi + spans + Squeak routing guidance in Section 10.

Related:

- `../draft/SPEC-INDEX.md` — claim/profile front door.
- `../draft/CONFORMANCE.md` — executable conformance contract.
- `../../process/coss.md` — lifecycle/process policy.
