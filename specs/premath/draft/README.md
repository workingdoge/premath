# Premath Draft Specs

This directory contains promoted **draft** specifications.

Draft specs are active contracts between editor and implementers and should be
treated as normative for claimed conformance profiles/capabilities.

Minimal authority path (read first):

- `SPEC-INDEX.md` — front door for claims, profiles, and normative scope.
  - canonical North Star target/invariants/phase posture lives in
    `SPEC-INDEX.md` §0.
- `DOCTRINE-INF.md` — doctrine/infinity-layer preservation contract.
- `PREMATH-KERNEL.md` — semantic kernel contract.
- `BIDIR-DESCENT.md` + `GATE.md` — obligation/discharge and admissibility gate
  authority.
- `NORMALIZER.md` + `REF-BINDING.md` — deterministic normalized comparison and
  identity binding in evidence-bearing paths.

Additive control-plane overlays (only when needed):

- `DOCTRINE-SITE.md` — doctrine-to-operation site map contract
  (`site-packages/<site-id>/SITE-PACKAGE.json` -> generated
  `DOCTRINE-SITE-INPUT.json` -> generated `DOCTRINE-SITE.json` +
  generated `DOCTRINE-OP-REGISTRY.json`), including explicit runtime
  orchestration route bindings (`op/conformance.runtime_orchestration`) and
  operation class policy rows (`route_bound`, `read_only_projection`,
  `tooling_only`).
- `DOCTRINE-SITE-CUTOVER.json` — deterministic migration/cutover contract
  declaring bounded compatibility window and generated-only cutoff phase.
- `DOCTRINE-SITE-GENERATION-DIGEST.json` — deterministic generation digest
  contract for doctrine site input/map/operation-registry artifacts.
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
- `WORLD-REGISTRY.md` — canonical world-profile and inter-world morphism table
  contract (`world == premath`) for route-to-world binding discipline with
  explicit adapter/non-authority boundaries.
- `SITE-RESOLVE.md` — deterministic resolver/projection contract for
  operation-site-world selection (`candidate gather -> capability/policy filter
  -> world-route validation -> overlap/glue decision`) and stable KCIR handoff
  refs.
- `HARNESS-RUNTIME.md` — promoted harness runtime contract for
  `boot/step/stop` plus the shared harness surface map used by typestate and
  retry/escalation contracts.
- `HARNESS-TYPESTATE.md` — promoted typestate closure/mutation gate contract
  for tool-calling turns and fail-closed mutation admissibility (shared harness
  partitioning/routes in `HARNESS-RUNTIME.md` §1.1).
- `HARNESS-RETRY-ESCALATION.md` — promoted classify/retry/escalation policy
  contract for harness CI wrappers (shared harness partitioning/routes in
  `HARNESS-RUNTIME.md` §1.1).
- `CONTROL-PLANE-CONTRACT.json` — shared typed control-plane contract consumed
  by CI projection and coherence parity checks (including schema lifecycle
  alias-window policy for contract/witness/projection kinds, governance-mode
  metadata, runtime-route parity bindings under `runtimeRouteBindings`, and
  explicit control-plane bundle profile fields under
  `controlPlaneBundleProfile` (`C_cp`/`E_cp`, reindex/cover-glue obligations,
  and authority split boundaries), plus canonical KCIR control-plane mapping
  fields under `controlPlaneKcirMappings` (instruction/proposal/coherence/
  doctrine-route/fiber-lifecycle/required-decision mappings, digest-lineage
  bindings, and non-KCIR compatibility deprecation policy). Phase-3 authority
  boundary:
  governance/KCIR mapping CI gates execute through premath core CLI
  (`governance-promotion-check`, `kcir-mapping-check`) while wrappers remain
  adapter-only transports; evaluator/REPL host-action surfaces remain
  design-level compatibility overlays until a promoted non-overlay route is
  contract-bound and doctrine-site routed.
- `CAPABILITY-REGISTRY.json` — shared typed executable-capability +
  profile-overlay-claim registry, including capability-to-normative-doc claim
  bindings (`capabilityDocBindings`) consumed by conformance/docs/coherence
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
- `../profile/` — optional profile overlays (normative only when claimed).
- `../../process/coss.md` — lifecycle/process policy.
- `../../process/SCHEMA-LIFECYCLE-GOVERNANCE.md` — lifecycle rollover/freeze
  governance contract.
- `../../process/decision-log.md` — promotion and architectural decisions.
