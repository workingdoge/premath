---
slug: draft
shortname: SPEC-INDEX
title: workingdoge.com/premath/SPEC-INDEX
name: Spec Index and Conformance Profiles
status: draft
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

- `draft/DOCTRINE-INF` — doctrine/infinity-layer preservation contract.
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
- Read-only dependency integrity projection route is also mapped in
  `draft/DOCTRINE-OP-REGISTRY.json` / `draft/DOCTRINE-SITE.json`
  (`op/mcp.issue_list`, `op/mcp.issue_ready`, `op/mcp.issue_blocked`,
  `op/mcp.issue_check`, `op/mcp.issue_backend_status`,
  `op/mcp.issue_lease_projection`, `op/mcp.dep_diagnostics`).
- Promoted harness contract surfaces (`draft/HARNESS-RUNTIME`,
  `draft/HARNESS-RETRY-ESCALATION`) MUST reuse the same routed operation IDs
  above and MUST NOT introduce parallel mutation/session authority paths.
- Instruction/observation/init MCP surfaces are also mapped in
  `draft/DOCTRINE-OP-REGISTRY.json` / `draft/DOCTRINE-SITE.json`
  (`op/mcp.instruction_check`, `op/mcp.instruction_run`,
  `op/mcp.observe_latest`, `op/mcp.observe_needs_attention`,
  `op/mcp.observe_instruction`, `op/mcp.observe_projection`,
  `op/mcp.init_tool`).

### 5.5 Informative and optional

The entries below are informative/default reading surfaces unless they are
explicitly claimed under §5.4 or §5.6.

- `draft/DOCTRINE-SITE` — machine-checkable doctrine-to-operation site map
  (`draft/DOCTRINE-SITE-SOURCE.json` + `draft/DOCTRINE-OP-REGISTRY.json` ->
  `draft/DOCTRINE-SITE.json`, including worker mutation and harness-session
  operation routes).
- `draft/SPEC-TRACEABILITY` — spec-to-check/vector coverage matrix with
  explicit gap targets.
- `draft/PREMATH-COHERENCE` — typed coherence-contract checker/witness model
  for repository control-plane surfaces (`draft/COHERENCE-CONTRACT.json`).
- `draft/COHERENCE-CONTRACT.json` — machine coherence contract artifact for
  deterministic checker execution.
- `draft/HARNESS-RUNTIME` — promoted harness runtime contract for
  `boot/step/stop`, session/feature/trajectory artifacts, and deterministic
  worker/coordinator loop behavior over existing routed operations.
- `draft/HARNESS-RETRY-ESCALATION` — promoted classify/retry/escalation control
  contract for CI harness wrappers bound to canonical policy digest and routed
  escalation operations.
- `draft/CONTROL-PLANE-CONTRACT.json` — shared typed control-plane constants
  (projection policy/check order + CI witness kinds + schema lifecycle table
  for contract/witness/projection kind families + harness retry/escalation
  bindings + worker mutation authority policy/routes + Stage 2/Stage 3
  typed-authority metadata) consumed by
  CI/coherence adapter
  surfaces; lifecycle semantics follow `draft/UNIFICATION-DOCTRINE` §5.1
  including governance-mode metadata
  (`rollover|freeze`) and process contract in
  `../../process/SCHEMA-LIFECYCLE-GOVERNANCE.md`.
- `draft/CAPABILITY-REGISTRY.json` — shared typed executable-capability
  registry consumed by conformance/docs/coherence parity surfaces.
- `draft/LLM-INSTRUCTION-DOCTRINE` — doctrine contract for typed LLM
  instruction flows (normative only when `capabilities.instruction_typing` is
  claimed).
- `draft/LLM-PROPOSAL-CHECKING` — proposal ingestion/checking contract for LLM
  proposal artifacts (normative only when
  `capabilities.instruction_typing` is claimed).
- `draft/UNIFICATION-DOCTRINE` — minimum-encoding/maximum-expressiveness
  architecture doctrine for canonical boundaries and deterministic projections
  (including Unified Evidence Plane contract in §10 and cross-layer obstruction
  algebra in §11).
- `draft/SPAN-SQUARE-CHECKING` — typed span/square witness contract for
  pipeline/base-change commutation plus composition-law (identity,
  associativity, h/v compatibility, interchange) surfaces in coherence checker
  paths.
- `raw/CTX-SITE` — informational site base (`Ctx`) + coverage (`J`) model for
  context/refinement decomposition.
- `raw/SHEAF-STACK` — informational presheaf/sheaf/stack rendering of
  transport/descent obligations.
- `raw/TORSOR-EXT` — informational torsor/extension/twist-class model for
  non-canonical split behavior.
- `raw/SEMANTICS-INFTOPOS` — presentation-free model sketch (informational).
- `raw/HYPERDESCENT` — optional strengthening: hyperdescent.
- `raw/UNIVERSE` — optional extension: universe + comprehension (Tarski-style).
- `raw/SPLIT-PRESENTATION` — guidance: strict IR vs. semantic equality.
- `raw/TUSK-CORE` — single-world operational runtime contracts (informational/raw).
- `raw/SQUEAK-CORE` — inter-world transport/composition contracts (informational/raw).
- `raw/SQUEAK-SITE` — runtime-location site contracts for Squeak/Cheese
  (normative only when `capabilities.squeak_site` is claimed).
- `raw/PREMATH-CI` — higher-order CI/CD control-loop contract (normative only
  when `capabilities.ci_witnesses` is claimed).
- `raw/CI-TOPOS` — closure-style CI projection discipline (informational/raw).
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

Current tracked promotion queue:

- `raw/SQUEAK-SITE` — tracked by issue `bd-44` (raw-retain path recorded in
  Decision 0040; promote only when criteria are met).
- `raw/TUSK-CORE` — tracked by issue `bd-45` (raw-retain path recorded in
  Decision 0041; promote only when criteria are met).

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
   (`draft/DOCTRINE-SITE-SOURCE.json` + `draft/DOCTRINE-OP-REGISTRY.json` ->
   `draft/DOCTRINE-SITE.json`)
3) `draft/LLM-INSTRUCTION-DOCTRINE`
4) `draft/LLM-PROPOSAL-CHECKING`
5) `raw/PREMATH-CI`
6) `raw/CI-TOPOS`
7) `draft/PREMATH-COHERENCE` + `draft/COHERENCE-CONTRACT.json`
8) `raw/TUSK-CORE` + `raw/SQUEAK-CORE`
9) `raw/SQUEAK-SITE`

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
3) `draft/PREMATH-COHERENCE` + `draft/COHERENCE-CONTRACT.json`
4) `draft/SPAN-SQUARE-CHECKING`
5) `profile/ADJOINTS-AND-SITES` + `raw/SQUEAK-SITE` (only when those capabilities are claimed)

## 7. Notes on restrictiveness

- The kernel is intentionally small and closed.
- Interop is intentionally strict when claimed: it exists to make independent implementations converge.
- Implementations that do not exchange artifacts (e.g., proof-assistant-internal models) MAY omit
  interop machinery, and should simply refrain from making the corresponding interop claims.
