# Premath Profile Overlays

This directory contains optional **profile overlays**.

Profile overlays are additive to the base draft claims:

- they are **normative only when explicitly claimed**,
- they do not replace `draft/PREMATH-KERNEL`,
- and they must compile to deterministic checker/discharge behavior.

Current overlays:

- `ADJOINTS-AND-SITES.md` — capability-scoped adjoint/site obligations over the
  kernel context-site base, discharged under `(normalizerId, policyDigest)`.

Related:

- `../draft/SPEC-INDEX.md` — claim/profile front door.
- `../draft/CONFORMANCE.md` — executable conformance contract.
- `../../process/coss.md` — lifecycle/process policy.
