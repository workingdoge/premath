# Premath Draft Specs

This directory contains promoted **draft** specifications.

Draft specs are active contracts between editor and implementers and should be
treated as normative for claimed conformance profiles/capabilities.

Minimal authority path (read first):

- `SPEC-INDEX.md` — front door for claims, profiles, and normative scope.
- `DOCTRINE-INF.md` — doctrine/infinity-layer preservation contract.
- `PREMATH-KERNEL.md` — semantic kernel contract.
- `BIDIR-DESCENT.md` + `GATE.md` — obligation/discharge and admissibility gate
  authority.
- `NORMALIZER.md` + `REF-BINDING.md` — deterministic normalized comparison and
  identity binding in evidence-bearing paths.

Additive control-plane overlays (only when needed):

- `DOCTRINE-SITE.md` — doctrine-to-operation site map contract
  (`DOCTRINE-SITE-SOURCE.json` + `DOCTRINE-OP-REGISTRY.json` ->
  `DOCTRINE-SITE.json`).
- `LLM-INSTRUCTION-DOCTRINE.md` — instruction typing/binding doctrine for
  LLM-driven control loops.
- `LLM-PROPOSAL-CHECKING.md` — proposal ingestion contract binding LLM outputs
  into checking/discharge (never self-authorizing).
- `PREMATH-COHERENCE.md` — typed coherence contract + checker witness model for
  repository control-plane consistency.
- `COHERENCE-CONTRACT.json` — machine contract artifact consumed by
  `premath coherence-check`.
- `CONTROL-PLANE-CONTRACT.json` — shared typed control-plane contract consumed
  by CI projection and coherence parity checks (including schema lifecycle
  alias-window policy for contract/witness/projection kinds).
- `CAPABILITY-REGISTRY.json` — shared typed executable-capability registry
  consumed by conformance/docs/coherence parity checks.
- `UNIFICATION-DOCTRINE.md` — minimum-encoding/maximum-expressiveness
  architecture doctrine for canonical boundaries.
- `SPAN-SQUARE-CHECKING.md` — typed span/square witness contract for
  pipeline/base-change commutation checks.
- `SPEC-TRACEABILITY.md` — spec-to-check/vector coverage matrix with explicit
  gap targets.

Surface-reduction rule:

- Treat the minimal authority path as canonical.
- Add overlays only when a claimed capability/profile requires them.
- Do not create parallel authority paths in docs; route back to `SPEC-INDEX.md`
  and `UNIFICATION-DOCTRINE.md` for composition rules.

Related:

- `../raw/` — experimental and informational specs not yet promoted.
- `../profile/` — optional profile overlays (normative only when claimed).
- `../../process/coss.md` — lifecycle/process policy.
- `../../process/decision-log.md` — promotion and architectural decisions.
