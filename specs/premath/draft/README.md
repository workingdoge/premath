# Premath Draft Specs

This directory contains promoted **draft** specifications.

Draft specs are active contracts between editor and implementers and should be
treated as normative for claimed conformance profiles/capabilities.

Start here:

- `SPEC-INDEX.md` — front door for claims, profiles, and normative scope.
- `DOCTRINE-INF.md` — doctrine/infinity-layer preservation contract.
- `DOCTRINE-SITE.md` — doctrine-to-operation site map contract
  (`DOCTRINE-SITE.json`).
- `LLM-INSTRUCTION-DOCTRINE.md` — instruction typing/binding doctrine for
  LLM-driven control loops.
- `LLM-PROPOSAL-CHECKING.md` — proposal ingestion contract binding LLM outputs
  into checking/discharge (never self-authorizing).
- `PREMATH-COHERENCE.md` — typed coherence contract + checker witness model for
  repository control-plane consistency.
- `NORMALIZER.md` — deterministic normalization and comparison-key policy for
  normalized evidence modes.
- `COHERENCE-CONTRACT.json` — machine contract artifact consumed by
  `premath coherence-check`.
- `CONTROL-PLANE-CONTRACT.json` — shared typed control-plane contract consumed
  by CI projection and coherence parity checks.
- `CAPABILITY-REGISTRY.json` — shared typed executable-capability registry
  consumed by conformance/docs/coherence parity checks.
- `PREMATH-KERNEL.md` — semantic kernel contract.
- `UNIFICATION-DOCTRINE.md` — minimum-encoding/maximum-expressiveness
  architecture doctrine for canonical boundaries.
- `SPEC-TRACEABILITY.md` — spec-to-check/vector coverage matrix with explicit
  gap targets.

Related:

- `../raw/` — experimental and informational specs not yet promoted.
- `../profile/` — optional profile overlays (normative only when claimed).
- `../../process/coss.md` — lifecycle/process policy.
- `../../process/decision-log.md` — promotion and architectural decisions.
