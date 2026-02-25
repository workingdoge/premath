---
slug: draft
shortname: SPEC-INDEX
title: workingdoge.com/premath/SPEC-INDEX
name: Spec Index and Conformance Profiles
status: informational
category: Informational
tags:
  - premath
  - kernel
  - conformance
  - index
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 0. North Star (Canonical Target State)

This section is the single-source statement of target state and phase posture.

### 0.1 Target state sentence

Premath targets one self-hosted, fail-closed decision spine:
`compile -> KCIR -> Gate -> witness`, with semantic admissibility owned only by
`draft/PREMATH-KERNEL`, `draft/GATE`, and `draft/BIDIR-DESCENT`.

### 0.2 Non-negotiable invariants

1. Single semantic authority lane: no checker/CI/runtime wrapper may introduce
   a parallel admissibility path.
2. Single mutation-authority lane: operational mutation is instruction-mediated
   by default and fail-closed when authority evidence is missing.
3. Claim-gated normativity: optional capability and profile-overlay requirements
   are normative only when explicitly claimed.
4. Deterministic evidence lineage: control-plane/runtime outcomes remain
   witness-linked, digest-bound, and replay-stable.
5. Coherence-before-convenience: docs/spec/contracts/checkers must stay
   synchronized under deterministic parity gates.
6. Constructor-first worldization: route/world/evidence binding for
   control-plane flows is derived from one deterministic constructor lane, not
   from wrapper-local semantics.

### 0.3 Current phase and active epic IDs

Current phase (as of 2026-02-25):

- KCIR self-hosting phase 3 closure is complete (`bd-287` closed).
- Follow-on closure for statement-ID/KCIR projection indexing is complete
  (`bd-294` closed).
- Doctrine-site resolver unification closure is complete (`bd-332` closed).

Active epic IDs:

- none currently open/in-progress.

Recently closed epic IDs:

- `bd-287`: KCIR self-hosting phase 3.
- `bd-294`: Kernel Statement-ID + KCIR Projection Index v1.
- `bd-332`: Doctrine-Site Resolver Unification (INF/SITE/WORLD selection).

Phase-3 dependency spine (ordered):

- `bd-288`: architecture contract (target-state vs transition-state; closed).
- `bd-289`: spec/index glue (closed).
- `bd-234`: host-action mapping contract/checker binding (gates `bd-290`; closed).
- `bd-290`: control-plane parity (closed).
- `bd-235`: local REPL lease-op parity boundary (gates `bd-291`; closed).
- `bd-291`: implementation (closed).
- `bd-292`: conformance (closed).
- `bd-293`: docs/traceability closure (closed).

This section records stable phase milestones only; mutable execution status
remains issue-memory authority (`premath issue ready|list|blocked`).

Active non-epic blocker:

- `bd-67` (`blocked`, manual): governance reviewer-pool readiness.

Live status authority:

- `.premath/issues.jsonl` via `premath issue list|ready|blocked`.

## 1. Purpose

This file is the **front door**. It answers:

- what is normative vs. informative,
- what claims an implementation may make,
- and how the ecosystem compiles to a small, checkable kernel.

Premath is designed to be **host-agnostic**. We treat “typechecker” and “∞-cosmos”
as *examples of external host bases* `B` in which Premath meanings are realized and/or checked.
Whether `B` is presented as a “type”, “term”, or other meta-object is an implementation detail.

Normative conformance requirements live in `draft/CONFORMANCE`.

## 2. The Premath shape (one total, two bases)

Premath’s semantic kernel is the fibre-space projection:

- `p₀ : E → 𝒞` where `𝒞` is the context world (covers/refinements are declared on it),
  and `E` is the total space of definables-in-context.

Implementations additionally choose an external host base `B` and a realization `F`
into a Premath-shaped bundle over `B`. See `draft/PREMATH-KERNEL` for the exact diagram and roles.

## 3. Claims and profiles (capability-based)

Premath avoids prescribing a single internal architecture. Instead, conformance is
defined by **claims**. An implementation MUST satisfy the requirements of every claim it asserts.

Claims are grouped into profiles:

- **Kernel**: semantic law only (reindexing coherence + contractible descent + refinement invariance).
- **Interop Core**: deterministic, exchangeable artifacts (KCIR + NF + ref binding + wire/errors).
- **Interop Full**: `Interop Core` + deterministic normalization, obligations, and gate enforcement.

Implementations MAY additionally claim profile overlays published under
`specs/premath/profile/`. Profile overlays are additive to base claims and are
normative only when explicitly claimed.

Details and required vectors are defined in `draft/CONFORMANCE` and `draft/CAPABILITY-VECTORS`.

Interop profiles should be read as evidence/representation profiles over one kernel.
The unifying feature is the kernel law outcome, not the wire representation.
Profile choice may change portability and artifact form, but must not change kernel meaning.

## 4. Reference architecture (optional)

The diagram below is a **reference pathway** for implementations that want deterministic,
portable interop artifacts (`ObjNF/MorNF`, `cmpRef`, `project_ref`, wire formats). It is not
the only valid architecture.

Conformance is judged at the boundaries an implementation exposes (parsers, normalizers,
verifiers, obligation checkers), not by whether it contains these exact boxes internally.

```
   source syntax / IR
 (DSL, KCIR builders, etc.)
            |
            | elaborate / compile (optional)
            v
   obligations Ω  +  candidate meaning
            |              |
            | discharge     | canonicalize / key (optional)
            v              v
      host checker      NF/Normalizer/Refs
            \              /
             \            /
              v          v
            kernel laws (p₀:E→𝒞)
     (stability + contractible descent + refinement invariance)
```

## 5. What is normative (by claim)

### 5.1 Always normative (Kernel claim)

- `draft/DOCTRINE-INF` — doctrine/infinity-layer preservation contract;
  governance-flywheel preservation profile (§9) is conditional normative only
  when `profile.doctrine_inf_governance.v0` is claimed.
- `draft/PREMATH-KERNEL` — semantic kernel (contexts/covers + contractible descent).

### 5.2 Normative for Interop Core (only if claimed)

- `draft/KCIR-CORE`
- `draft/REF-BINDING`
- `draft/NF`
- `draft/WIRE-FORMATS`
- `draft/ERROR-CODES`

### 5.3 Normative for Interop Full (only if claimed)

Everything in Interop Core, plus:

- `draft/NORMALIZER`
- `draft/BIDIR-DESCENT`
- `draft/GATE`

### 5.4 Normative for optional evidence capabilities (only if claimed)

For capability identifiers and vectors defined in `draft/CAPABILITY-VECTORS`:

- `capabilities.normal_forms`
- `capabilities.kcir_witnesses`
- `capabilities.commitment_checkpoints`
- `capabilities.squeak_site`
- `capabilities.ci_witnesses`
- `capabilities.instruction_typing`
- `capabilities.adjoints_sites`
- `capabilities.change_morphisms`

Capability-specific normative specs include:

- `raw/SQUEAK-SITE` (for `capabilities.squeak_site`)
- `raw/PREMATH-CI` (for `capabilities.ci_witnesses`)
- `draft/LLM-INSTRUCTION-DOCTRINE` (for `capabilities.instruction_typing`)
- `draft/LLM-PROPOSAL-CHECKING` (for `capabilities.instruction_typing`)
- `profile/ADJOINTS-AND-SITES` (for `capabilities.adjoints_sites`)
- `draft/CHANGE-MORPHISMS` (for `capabilities.change_morphisms`)
- `draft/HARNESS-TYPESTATE` (for `capabilities.change_morphisms`)

Normative requirements apply only when the corresponding capability is claimed.

Worker-operation doctrine-site routing note:

- Mutation/session operation surfaces for
  `capabilities.change_morphisms` are mapped in
  `draft/DOCTRINE-OP-REGISTRY.json` / `draft/DOCTRINE-SITE.json`
  (`op/mcp.issue_add`, `op/mcp.issue_update`, `op/mcp.issue_claim`,
  `op/mcp.issue_lease_renew`, `op/mcp.issue_lease_release`,
  `op/mcp.issue_discover`, `op/mcp.dep_add`, `op/mcp.dep_remove`,
  `op/mcp.dep_replace`,
  `op/harness.session_read`, `op/harness.session_write`,
  `op/harness.session_bootstrap`).
- Operation rows in `draft/DOCTRINE-OP-REGISTRY.json` MUST carry explicit
  `operationClass` as declared by
  `draft/DOCTRINE-SITE-INPUT.json` policy rows
  (`route_bound`, `read_only_projection`, `tooling_only`).
- Only `route_bound` operations are resolver/world-route eligible and MUST bind
  through declared `worldRouteBindings`; non-route classes MUST remain
  resolver-ineligible non-authority surfaces.
- Read-only dependency integrity projection route is also mapped in
  `draft/DOCTRINE-OP-REGISTRY.json` / `draft/DOCTRINE-SITE.json`
  (`op/mcp.issue_list`, `op/mcp.issue_ready`, `op/mcp.issue_blocked`,
  `op/mcp.issue_check`, `op/mcp.issue_backend_status`,
  `op/mcp.issue_lease_projection`, `op/mcp.dep_diagnostics`).
- Phase-3 transition boundary: REPL/control overlays MUST treat
  `issue.lease_renew` and `issue.lease_release` as MCP-only host actions until
  a promoted non-MCP authority surface exists; hidden local fallback mutation
  paths are forbidden.
- Promoted harness contract surfaces (`draft/HARNESS-RUNTIME`,
  `draft/HARNESS-TYPESTATE`, `draft/HARNESS-RETRY-ESCALATION`) MUST reuse the
  same routed operation IDs above and MUST NOT introduce parallel
  mutation/session authority paths.
- Instruction/observation/init MCP surfaces are also mapped in
  `draft/DOCTRINE-OP-REGISTRY.json` / `draft/DOCTRINE-SITE.json`
  (`op/mcp.instruction_check`, `op/mcp.instruction_run`,
  `op/mcp.observe_latest`, `op/mcp.observe_needs_attention`,
  `op/mcp.observe_instruction`, `op/mcp.observe_projection`,
  `op/mcp.init_tool`).
- Doctrine-conformance routing also includes explicit runtime orchestration
  parity (`op/conformance.runtime_orchestration`), binding
  `runtimeRouteBindings` contract routes to
  `draft/DOCTRINE-OP-REGISTRY.json` operation nodes, enforcing routed
  operation path boundaries (`tools/ci/*`) and optional
  `controlPlaneKcirMappings` row-shape checks (when mapping rows are present),
  with invariance vectors for profile-permuted route scenarios. Canonical
  semantic authority executes via `premath runtime-orchestration-check`;
  `tools/conformance/check_runtime_orchestration.py` is a wrapper adapter.
- World-route semantic closure is enforced through the core command lane
  (`premath world-registry-check`) with dedicated executable vectors in
  `tests/conformance/fixtures/world-core/` (`run_world_core_vectors.py`).
  Runtime-orchestration vectors are adapter/runtime-route parity checks only;
  they MUST NOT duplicate world semantic authority vectors.
- For multithread worker orchestration, routed operation paths MUST be treated
  as operational cover/refinement execution surfaces only (no semantic
  authority transfer). Any acceptance/rejection consumed by runtime/control
  surfaces MUST remain checker/Gate-discharged and factor through Unified
  Evidence routing (`draft/UNIFICATION-DOCTRINE` §10 and §12).
- Evaluator/REPL transition surfaces (for example `scheme_eval`-style
  host-action loops) MUST remain compatibility/control overlays until they are
  bound to contract rows and doctrine-site routed operation IDs. They MUST NOT
  introduce unrouted mutation/session authority paths.

### 5.5 Informative and optional

The entries below are informative/default reading surfaces unless they are
explicitly claimed under §5.4 or §5.6.

- `draft/DOCTRINE-SITE` — machine-checkable doctrine-to-operation site map
  (`site-packages/<site-id>/SITE-PACKAGE.json` -> generated
  `draft/DOCTRINE-SITE-INPUT.json` -> generated
  `draft/DOCTRINE-SITE.json` + generated `draft/DOCTRINE-OP-REGISTRY.json`,
  including worker mutation and harness-session operation routes, plus
  operation-class policy (`route_bound`, `read_only_projection`,
  `tooling_only`) and route-eligibility gating.
- `draft/DOCTRINE-SITE-CUTOVER.json` — deterministic doctrine-site migration
  contract declaring bounded compatibility window and generated-only cutoff
  phase; checker/generator lanes MUST fail closed when legacy/manual authority
  surfaces are disabled by the active phase.
- `draft/DOCTRINE-SITE-GENERATION-DIGEST.json` — deterministic generation digest
  contract for doctrine site source parity (`site-packages` -> generated input /
  site map / operation registry).
- `draft/SPEC-TRACEABILITY` — spec-to-check/vector coverage matrix with
  explicit gap targets.
- `draft/PREMATH-COHERENCE` — typed coherence-contract checker/witness model
  for repository control-plane surfaces (`draft/COHERENCE-CONTRACT.json`).
- `draft/COHERENCE-CONTRACT.json` — machine coherence contract artifact for
  deterministic checker execution.
- `draft/KERNEL-STATEMENT-BINDINGS.json` — typed projection-only statement
  binding contract linking kernel statement IDs to obligations/checkers/vectors
  (indexing/query/evidence support only; no semantic admissibility authority).
- `draft/WORLD-REGISTRY` — canonical world-profile and inter-world morphism
  table contract (`world == premath`) for route-family to world binding
  declarations, explicit Grothendieck constructor object contract for active
  profiles, and CwF/descent authority boundaries with adapter/non-authority
  constraints.
- `draft/SITE-RESOLVE` — deterministic resolver/projection contract for
  operation-site-world selection order
  (`candidate gather -> capability/policy filter -> world-route validation ->
  overlap/glue decision`) and fail-closed unbound/ambiguous outcomes, with
  stable route/site/world refs for KCIR handoff.
- `draft/HARNESS-RUNTIME` — promoted harness runtime contract for
  `boot/step/stop` and the shared harness surface map
  (`draft/HARNESS-RUNTIME` §1.1) used by typestate and retry/escalation
  contracts.
- `draft/HARNESS-TYPESTATE` — promoted harness typestate closure/mutation gate
  contract for tool-calling turns (normative when
  `capabilities.change_morphisms` is claimed; shared harness partitioning/routes
  are declared in `draft/HARNESS-RUNTIME` §1.1).
- `draft/HARNESS-RETRY-ESCALATION` — promoted classify/retry/escalation control
  contract for CI harness wrappers bound to canonical policy digest and routed
  escalation operations (shared harness partitioning/routes in
  `draft/HARNESS-RUNTIME` §1.1).
- `draft/CONTROL-PLANE-CONTRACT.json` — shared typed control-plane constants
  (projection policy/check order + CI witness kinds + schema lifecycle table
  for contract/witness/projection kind families + harness retry/escalation
  bindings + worker mutation authority policy/routes + runtime route bindings
  (`runtimeRouteBindings`) + Stage 2/Stage 3 typed-authority metadata +
  control-plane bundle profile (`controlPlaneBundleProfile`) declaring
  `C_cp` (repository-state context family), `E_cp` (control-plane artifact
  family), reindexing/coherence and cover/glue obligations, plus explicit
  semantic-authority split (`PREMATH-KERNEL`/`GATE`/`BIDIR-DESCENT` remain
  authority; control-plane is projection/parity only), and canonical KCIR
  control-plane mapping table (`controlPlaneKcirMappings`) for instruction /
  proposal / coherence / doctrine-route / fiber-lifecycle / required-decision
  surfaces, including deterministic digest-lineage fields and legacy non-KCIR
  compatibility deprecation policy)
  consumed by
  CI/coherence adapter
  surfaces; lifecycle semantics follow `draft/UNIFICATION-DOCTRINE` §5.1
  including governance-mode metadata
  (`rollover|freeze`) and process contract in
  `../../process/SCHEMA-LIFECYCLE-GOVERNANCE.md`.
- `draft/CAPABILITY-REGISTRY.json` — shared typed executable-capability +
  profile-overlay-claim registry, including capability-to-normative-doc claim
  bindings (`capabilityDocBindings`) consumed by conformance/docs/coherence
  parity surfaces.
- `draft/LLM-INSTRUCTION-DOCTRINE` — doctrine contract for typed LLM
  instruction flows (normative only when `capabilities.instruction_typing` is
  claimed).
- `draft/LLM-PROPOSAL-CHECKING` — proposal ingestion/checking contract for LLM
  proposal artifacts (normative only when
  `capabilities.instruction_typing` is claimed).
- `draft/UNIFICATION-DOCTRINE` — minimum-encoding/maximum-expressiveness
  architecture doctrine for canonical boundaries and deterministic projections
  (including Unified Evidence Plane contract in §10 and cross-layer obstruction
  algebra in §11, plus Grothendieck operationalization routing contract in
  §12).
- `draft/SPAN-SQUARE-CHECKING` — typed span/square witness contract for
  pipeline/base-change commutation plus composition-law (identity,
  associativity, h/v compatibility, interchange) surfaces in coherence checker
  paths.
- `raw/CTX-SITE` — informational site base (`Ctx`) + coverage (`J`) model for
  context/refinement decomposition.
- `raw/SHEAF-STACK` — informational presheaf/sheaf/stack rendering of
  transport/descent obligations.
- `raw/TORSOR-EXT` — informational torsor/extension/twist-class model for
  non-canonical split behavior; overlay-only interpretation (not an authority
  lane).
- `raw/SEMANTICS-INFTOPOS` — presentation-free model sketch (informational).
- `raw/HYPERDESCENT` — optional strengthening: hyperdescent.
- `raw/UNIVERSE` — optional extension: universe + comprehension (Tarski-style).
- `raw/SPLIT-PRESENTATION` — guidance: strict IR vs. semantic equality.
- `raw/TUSK-CORE` — single-world operational runtime contracts (informational/raw).
- `raw/SQUEAK-CORE` — inter-world transport/composition contracts (informational/raw).
- `raw/FIBER-CONCURRENCY` — structured-concurrency transport profile over
  worldized control lanes (`fiber.spawn|join|cancel`) (informational/raw).
- `raw/WORLD-PROFILES-CONTROL` — raw control-world profile sketches for
  `world.lease.v1`, `world.instruction.v1`, and `world.ci_witness.v1`,
  including route-family/morphism-table candidates against `C_cp`/`E_cp`, plus
  optional torsor/extension overlay posture (`overlay.torsor_ext.v1`) with
  explicit non-authority constraints.
- `raw/SQUEAK-SITE` — runtime-location site contracts for Squeak/Cheese
  (normative only when `capabilities.squeak_site` is claimed).
- `raw/PREMATH-CI` — higher-order CI/CD control-loop contract (normative only
  when `capabilities.ci_witnesses` is claimed).
- `raw/CI-TOPOS` — closure-style CI projection discipline (informational/raw).
- `raw/BEAM-COORDINATION` — BEAM/OTP coordination + lease/sublease profile
  bound to `world.lease.v1` route families (`route.issue_claim_lease`) and
  existing Premath authority/witness lanes (informational/raw).
- `docs/foundations/` — explanatory notes (non-normative).

Raw capability-spec lifecycle policy:

- Raw capability specs MAY be exercised by executable vectors, but remain
  lifecycle-raw until promotion criteria are met.
- Capability claims bind only the capability-scoped normative clauses listed in
  §5.4; raw status still means the full document text is open to iteration.
- Promotion from raw to draft for capability-scoped specs requires:
  1) deterministic golden/adversarial/invariance vectors for every claimed law
     boundary;
  2) deterministic witness/failure-class mapping through checker/run surfaces;
  3) issue-backed migration plan + decision-log entry for lifecycle change.

Current raw-retain posture:

- `raw/SQUEAK-SITE` — retained raw per Decision 0040; promote only when
  criteria are met.
- `raw/TUSK-CORE` — retained raw per Decision 0041; promote only when criteria
  are met.

### 5.6 Normative for profile overlays (only if claimed)

- `profile/ADJOINTS-AND-SITES` — capability-scoped adjoint/site overlay:
  admissible-map allowlist policy, Beck-Chevalley obligations, and deterministic
  `(normalizerId, policyDigest)` discharge binding for profile claims.

Joint capability note:

- Implementations MAY jointly claim `capabilities.adjoints_sites` and
  `capabilities.squeak_site`; composed systems SHOULD also route cross-lane
  pullback/base-change claims through `draft/SPAN-SQUARE-CHECKING` and MUST
  follow lane separation/single-authority encoding rules in
  `draft/UNIFICATION-DOCTRINE` §9 (see `profile/ADJOINTS-AND-SITES` §10 for
  composed overlay routing).

Notation convention:

- Use `SigPi` in prose/identifiers; render the adjoint triple as
  `\Sigma_f -| f* -| \Pi_f` (or shorthand `sig\Pi` when compact notation is
  needed).

Lane ownership note:

- CwF strict substitution/comprehension obligations are checker-core
  (`draft/PREMATH-COHERENCE`) and are not profile-scoped.
- CwF<->sig\Pi bridge mapping is normative in
  `profile/ADJOINTS-AND-SITES` §11 and MUST preserve existing obligation
  vocabularies (no new bridge-owned obligation IDs).
- Span/square commutation is a typed witness contract
  (`draft/SPAN-SQUARE-CHECKING`) that composed profiles MUST route through for
  cross-lane pullback/base-change claims, including composition-law witness
  coverage for identity/associativity/h-v/interchange.
- Unified evidence factoring MUST route control-plane artifact families through
  one attested surface (`draft/UNIFICATION-DOCTRINE` §10, including fail-closed
  factorization boundary in §10.5).
- Grothendieck operationalization of worker concurrency/routing MUST follow
  `draft/UNIFICATION-DOCTRINE` §12: cover-local execution, deterministic
  glue-or-obstruction, and no parallel admissibility path.
- Typed evidence-object migration MUST follow staged internalization gates in
  `draft/UNIFICATION-DOCTRINE` §10.6 (single authority artifact per stage with
  deterministic compatibility/rollback boundaries). Stage 1 typed-core parity
  claims MUST use the fail-closed class boundary in §10.6.2, and Stage 1
  rollback claims MUST use the deterministic rollback witness boundary in
  §10.6.3. Stage 2 typed-authority claims MUST use the clause-to-surface
  mapping in §10.6.4 (including Stage 2 gate-chain parity vectors and
  `capabilities.ci_witnesses` boundary-authority vectors). Stage 3 typed-first
  closure claims MUST use §10.6.5 mapping (direct bidir checker/discharge route
  canonical, sentinel fallback profile-gated only, typed-first consumer
  lineage).

## 6. Suggested reading order

Minimal authority path (default for all implementations):
1) `draft/SPEC-INDEX`
2) `draft/DOCTRINE-INF`
3) `draft/PREMATH-KERNEL`
4) `draft/BIDIR-DESCENT` + `draft/GATE`
5) `draft/NORMALIZER` + `draft/REF-BINDING` (when normalized evidence paths are used)

Surface-reduction guidance:

- Treat the minimal authority path above as canonical.
- Add profile/capability/control-plane docs only when the corresponding claim is
  implemented.
- When composing overlays, route composition semantics through
  `draft/UNIFICATION-DOCTRINE` (single-authority encoding; deterministic
  projections).

If you are proving semantics:
1) `draft/DOCTRINE-INF`
2) `draft/PREMATH-KERNEL`
3) `raw/SEMANTICS-INFTOPOS` (optional)
4) optional extensions (`HYPERDESCENT`, `UNIVERSE`)

If you are implementing Interop Full:
1) `draft/DOCTRINE-INF`
2) `draft/PREMATH-KERNEL`
3) `draft/REF-BINDING` + `draft/KCIR-CORE`
4) `draft/NF` → `draft/NORMALIZER`
5) `draft/BIDIR-DESCENT` + `draft/GATE`
6) `draft/WIRE-FORMATS` + `draft/ERROR-CODES`
7) `draft/CONFORMANCE` + `draft/CAPABILITY-VECTORS`
8) `draft/SPEC-TRACEABILITY`
9) `draft/UNIFICATION-DOCTRINE`

If you are implementing change discipline:
1) `draft/CHANGE-MORPHISMS`
2) `draft/CAPABILITY-VECTORS` (`capabilities.change_morphisms`)
3) conformance fixtures under `tests/conformance/fixtures/capabilities/`

If you are implementing higher-order CI/CD:
1) `draft/DOCTRINE-INF`
2) `draft/DOCTRINE-SITE`
   (`site-packages/<site-id>/SITE-PACKAGE.json` ->
   generated `draft/DOCTRINE-SITE-INPUT.json` ->
   generated `draft/DOCTRINE-SITE.json` + generated
   `draft/DOCTRINE-OP-REGISTRY.json`; migration/cutover authority in
   `draft/DOCTRINE-SITE-CUTOVER.json`)
3) `draft/WORLD-REGISTRY`
4) `draft/SITE-RESOLVE`
5) `draft/LLM-INSTRUCTION-DOCTRINE`
6) `draft/LLM-PROPOSAL-CHECKING`
7) `raw/PREMATH-CI`
8) `raw/CI-TOPOS`
9) `draft/PREMATH-COHERENCE` + `draft/COHERENCE-CONTRACT.json`
10) `draft/UNIFICATION-DOCTRINE` (especially §10 and §12)
11) `raw/WORLD-PROFILES-CONTROL`
12) `raw/TUSK-CORE` + `raw/SQUEAK-CORE`
13) `raw/SQUEAK-SITE`

If you are implementing the adjoints-and-sites overlay:
1) `draft/PREMATH-KERNEL`
2) `draft/GATE`
3) `draft/BIDIR-DESCENT`
4) `profile/ADJOINTS-AND-SITES` (§11 for CwF<->sig\Pi bridge)

If you are integrating SigPi + Squeak + spans in one system:
1) `draft/PREMATH-KERNEL`
2) `draft/BIDIR-DESCENT` + `draft/GATE`
3) `profile/ADJOINTS-AND-SITES` (§10)
4) `raw/SQUEAK-CORE` + `raw/SQUEAK-SITE`
5) `draft/SPAN-SQUARE-CHECKING`
6) `draft/PREMATH-COHERENCE` + `draft/COHERENCE-CONTRACT.json`
7) `draft/UNIFICATION-DOCTRINE` (§9 lane separation)

If you are implementing multithread worker orchestration:
1) `draft/UNIFICATION-DOCTRINE` (§9 lane separation; one authority artifact per boundary)
2) `raw/CTX-SITE` + `raw/SHEAF-STACK` (refinement/cover + glue-or-witness discipline)
3) `profile/ADJOINTS-AND-SITES` (§10/§11) (when `capabilities.adjoints_sites` is claimed)
4) `draft/CHANGE-MORPHISMS` + `draft/CAPABILITY-VECTORS`
   (`capabilities.change_morphisms`)
5) `raw/SQUEAK-SITE` (only when `capabilities.squeak_site` is claimed)
6) `draft/PREMATH-COHERENCE` + `draft/COHERENCE-CONTRACT.json`
7) operational companion: `docs/design/MULTITHREAD-LANE-SITE-ADJOINTS.md`

If you are implementing the Unified Evidence Plane:
1) `draft/UNIFICATION-DOCTRINE` (§10, especially §10.6)
2) `draft/CONTROL-PLANE-CONTRACT.json`
3) `draft/WORLD-REGISTRY`
4) `draft/PREMATH-COHERENCE` + `draft/COHERENCE-CONTRACT.json`
5) `draft/SPAN-SQUARE-CHECKING`
6) `profile/ADJOINTS-AND-SITES` + `raw/SQUEAK-SITE` (only when those capabilities are claimed)

## 7. Notes on restrictiveness

- The kernel is intentionally small and closed.
- Interop is intentionally strict when claimed: it exists to make independent implementations converge.
- Implementations that do not exchange artifacts (e.g., proof-assistant-internal models) MAY omit
  interop machinery, and should simply refrain from making the corresponding interop claims.
