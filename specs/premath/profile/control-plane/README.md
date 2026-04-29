# Premath Control-Plane Profile Map

This directory is the profile map for governance, runtime, and projection
contracts that apply Premath to repository control surfaces.

Current authority files remain in `../../draft/` for path stability:

- `PREMATH-COHERENCE.md`
- `COHERENCE-CONTRACT.json`
- `CONTROL-PLANE-CONTRACT.json`
- `DOCTRINE-SITE.md`
- `DOCTRINE-SITE-INPUT.json`
- `DOCTRINE-SITE.json`
- `DOCTRINE-OP-REGISTRY.json`
- `LLM-INSTRUCTION-DOCTRINE.md`
- `LLM-PROPOSAL-CHECKING.md`
- `HARNESS-RUNTIME.md`
- `HARNESS-TYPESTATE.md`
- `HARNESS-RETRY-ESCALATION.md`
- `CHANGE-MORPHISMS.md`
- `SPAN-SQUARE-CHECKING.md`
- control-plane sections of `UNIFICATION-DOCTRINE.md`

Profile rule:

- This profile owns deterministic projection, route parity, lifecycle,
  instruction/proposal typing, harness behavior, and coherence evidence for the
  implementation control plane.
- It does not own Premath Core admissibility.
- Tusk owns operator instruments and workflow surfaces over these contracts.
  Premath owns only the checker/kernel contracts those surfaces compile to.

Physical moves into this directory should be a separate migration with updated
contract paths, generated doctrine-site artifacts, traceability rows,
conformance fixtures, and decision-log evidence.
