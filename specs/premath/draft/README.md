# Premath Draft Specs

This directory contains promoted **draft** specifications.

Draft specs are active contracts between editor and implementers and should be
treated as normative for claimed checker profiles/capabilities.

Premath Core authority path (read first):

- `SPEC-INDEX.md` — short front door for core/profile/raw/site boundaries.
- `PREMATH-KERNEL.md` — semantic kernel contract.
- `OBLIGATION-DISCHARGE.md` + `GATE.md` — Core obligation/discharge and
  admissibility gate authority.
- `WITNESS-ID.md` — deterministic witness identity.
- `CHECKER-CLAIMS.md` §§1-2.1 — Core checker-claims boundary.

Doctrine preservation guard:

- `DOCTRINE-INF.md` — doctrine/infinity-layer preservation contract; promoted,
  but not part of the minimal Core authority path.

KCIR/Interop authority path (optional):

- `../profile/kcir/README.md` — Premath-side adapter to the KCIR site
  authority in `fish/sites/kcir`.
- `REF-BINDING.md` + `KCIR-CORE.md`
- `NF.md` + `NORMALIZER.md`
- `WIRE-FORMATS.md` + `ERROR-CODES.md`
- `../profile/interop/BIDIR-DESCENT.md` for full-profile verifier
  orchestration.

The files above remain in Premath for path stability while the KCIR site takes
over carrier-substrate authority. They are not part of Premath Core.

Additive control-plane overlays (only when needed):

- `DOCTRINE-SITE.md` — doctrine-to-operation site map contract
  (`DOCTRINE-SITE-INPUT.json` -> generated `DOCTRINE-SITE.json` +
  generated `DOCTRINE-OP-REGISTRY.json`), including explicit runtime
  orchestration route bindings (`op/checker.runtime_orchestration`).
- `LLM-INSTRUCTION-DOCTRINE.md` — instruction typing/binding doctrine for
  LLM-driven control loops.
- `LLM-PROPOSAL-CHECKING.md` — proposal ingestion contract binding LLM outputs
  into checking/discharge (never self-authorizing).
- `PREMATH-COHERENCE.md` — typed coherence contract + checker witness model for
  repository control-plane consistency.
- `COHERENCE-CONTRACT.json` — machine contract artifact consumed by
  `premath coherence-check`.
- `KERNEL-STATEMENT-BINDINGS.json` — projection-only statement binding contract
  from kernel statement IDs to obligations/checkers/vectors (index/query lane;
  not a semantic authority surface).
- `CONTROL-PLANE-CONTRACT.json` — shared typed control-plane contract consumed
  by local-check projection and coherence parity checks (including schema
  lifecycle alias-window policy for contract/projection kinds, governance-mode
  metadata, explicit control-plane bundle profile fields under
  `controlPlaneBundleProfile` (`C_cp`/`E_cp`, reindex/cover-glue obligations,
  and authority split boundaries), plus canonical KCIR control-plane mapping
  fields under `controlPlaneKcirMappings` (proposal/coherence/doctrine-route
  mappings, digest-lineage bindings, and
  non-KCIR compatibility deprecation policy).
- `CAPABILITY-REGISTRY.json` — shared typed executable-capability +
  profile-overlay-claim registry, including capability-to-normative-doc claim
  bindings (`capabilityDocBindings`) consumed by checker/docs/coherence
  parity checks.
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
  Meta-Harness object doctrine lives in its owning downstream site, not in
  Premath.
- `../profile/` — optional profile overlays (normative only when claimed).
- `../../process/coss.md` — lifecycle/process policy.
- `../../process/SCHEMA-LIFECYCLE-GOVERNANCE.md` — lifecycle rollover/freeze
  governance contract.
- `../../process/decision-log.md` — promotion and architectural decisions.
