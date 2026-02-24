---
slug: draft
shortname: EVIDENCE-INF
title: workingdoge.com/premath/EVIDENCE-INF
name: Unified Evidence Plane Abstract Discipline
status: draft
category: Standards Track
tags:
  - premath
  - evidence
  - sigpi
  - conformance
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## Language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
specification are to be interpreted as described in RFC 2119 (and RFC 8174 for
capitalization).

## 1. Scope

This specification defines the abstract evidence discipline for attested
control-plane evidence. It instantiates `draft/SIGPI-INF` f* for the evidence
fibres.

Implementations claiming the Unified Evidence Plane MUST model attested evidence
as one context-indexed family:

- `Ev : Ctx^op -> V`

where `V` is the selected witness universe (`Set`, `Groupoid`, or `Spaces`).

This document is normative when capability `capabilities.unified_evidence` is
claimed.

### 1.1 Canonical evidence object

`Ev` is the canonical attested evidence surface for control-plane outputs.

`Ev` does not replace kernel or Gate authority. Kernel/Gate decide admissibility;
`Ev` is the deterministic attestation/projection surface for accepted/rejected
checker outcomes.

### 1.2 Universal factoring rule

For every control-plane artifact family `F : Ctx^op -> V` that carries
attestable output (instruction/proposal/coherence/CI/observation projections),
there MUST be one deterministic natural transformation:

- `eta_F : F => Ev`

so artifact meaning factors through one evidence surface instead of parallel
authority schemas.

For fixed canonical inputs (contract bytes + repository state + deterministic
binding context), `eta_F` MUST be unique up to canonical projection equality in
`Ev` (no alternate authority encoding for the same artifact family output).

### 1.3 Required law set

A conforming `Ev` route MUST satisfy:

1. Transport law (naturality): reindexing commutes with evidence projection.
2. Descent law: cover-local evidence either glues deterministically or emits
   deterministic obstruction witnesses.
3. Determinism law: equality/comparison claims are bound to
   `normalizerId + policyDigest`.
4. Authority-boundary law: proposals/projections MAY suggest, but MUST NOT
   self-authorize admissibility without checker discharge.

### 1.4 Cross-lane commutation requirement

When cross-lane pullback/base-change claims are surfaced in `Ev`, implementations
MUST route commutation through typed span/square witnesses
(`draft/SPAN-SQUARE-CHECKING`) so lane composition remains explicit and
replayable.

### 1.5 Fail-closed factorization boundary

Implementations MUST reject deterministically when factorization into `Ev`
cannot be established as a unique canonical route.

Minimum fail-closed classes:

- `unification.evidence_factorization.missing` (no typed `eta_F` route),
- `unification.evidence_factorization.ambiguous` (multiple inequivalent routes),
- `unification.evidence_factorization.unbound` (missing deterministic binding
  context for canonical comparison).

Equivalent implementation-local class names are permitted only when a
deterministic mapping to these classes is documented and replay-stable.

### 1.6 Typed evidence-object internalization stages (v0)

Implementations MAY migrate from payload-first witness surfaces to Premath-typed
`Ev` objects, but MUST do so through deterministic stage gates.

At every stage, there MUST be exactly one authority artifact for admissibility
outcomes. Derived compatibility payloads MAY exist, but MUST remain projections
of that one authority artifact.

Stage contract:

1. Stage 0 (projection-locked):
   - existing witness payloads remain the authority artifact,
   - candidate typed `Ev` projections MAY be emitted for parity checking only,
   - candidate typed projections MUST roundtrip deterministically to the current
     authority payload comparison surface.
2. Stage 1 (typed-core dual projection):
   - implementations define a minimal typed evidence core profile
     (`ev1_*`-style deterministic identity binding is RECOMMENDED),
   - both authority artifact and typed core MUST be linked by deterministic
     replayable projections,
   - typed-core parity failures MUST fail closed before stage promotion.
3. Stage 2 (canonical typed authority with compatibility alias):
   - the typed evidence core becomes the authority artifact,
   - legacy payloads MAY remain as compatibility aliases only,
   - alias windows MUST be governed by one lifecycle table
     (`profile/UNIFICATION-GOVERNANCE` §5.1 +
     `draft/CONTROL-PLANE-CONTRACT.json`).
4. Stage 3 (typed-first cleanup):
   - expired compatibility aliases MUST reject fail closed,
   - all control-plane consumers MUST use typed evidence authority directly,
   - no compatibility alias may reintroduce a parallel authority route.

Stage-gate requirements:

1. stage transitions MUST preserve the §1.2 factoring rule (`eta_F : F => Ev`)
   for all claimed artifact families,
2. stage transitions MUST preserve §1.3 deterministic binding
   (`normalizerId + policyDigest`),
3. stage transitions MUST preserve §1.5 fail-closed factorization behavior.

Rollback requirements:

1. if a stage fails deterministic parity or replay checks, rollback to the
   previous accepted stage MUST be deterministic and MUST preserve prior
   canonical identity bindings,
2. rollback MUST NOT introduce a second authority artifact,
3. rollback conditions and target stage MUST be decision-logged and issue-linked
   before re-attempting promotion.

#### 1.6.1 Stage 1 typed-core profile (minimum)

When Stage 1 is claimed, implementations MUST define one deterministic
typed-core projection profile with:

1. one profile kind identifier (for example `ev.stage1.core.v1`),
2. deterministic binding fields:
   - `normalizerId`
   - `policyDigest`
3. one canonical typed-core identity function over canonicalized profile bytes
   (an `ev1_*`-style prefix is RECOMMENDED),
4. one deterministic projection from current authority payload to the Stage 1
   typed-core profile.

The Stage 1 typed-core profile is checker-facing parity material; it MUST NOT
be treated as a second authority artifact while Stage 1 is active.

#### 1.6.2 Stage 1 dual-projection parity contract

Stage 1 parity checks MUST evaluate one deterministic comparison tuple for fixed
canonical inputs:

- authority payload digest,
- typed-core projection digest/ref,
- deterministic binding tuple (`normalizerId`, `policyDigest`).

Implementations MUST reject fail closed on Stage 1 parity errors with at least:

- `unification.evidence_stage1.parity.missing`
  (missing authority->typed-core route),
- `unification.evidence_stage1.parity.mismatch`
  (deterministic parity comparison failed),
- `unification.evidence_stage1.parity.unbound`
  (missing deterministic binding context for parity comparison).

Equivalent implementation-local class names are permitted only when a
deterministic mapping to these classes is documented and replay-stable.

#### 1.6.3 Stage 1 deterministic rollback witness contract

When Stage 1 rollback is claimed, implementations MUST define one deterministic
rollback witness profile bound to:

- source/target stages (`stage1 -> stage0`),
- deterministic binding tuple (`normalizerId`, `policyDigest`),
- authority digest refs for pre-rollback and rollback-target authority
  checkpoints.

Rollback trigger metadata MUST include at least all Stage 1 parity classes from
§1.6.2 (`missing`, `mismatch`, `unbound`), so rollback admission remains
deterministic and replayable.

Implementations MUST reject fail closed on Stage 1 rollback witness errors with
at least:

- `unification.evidence_stage1.rollback.precondition`
  (missing/invalid rollback preconditions),
- `unification.evidence_stage1.rollback.identity_drift`
  (rollback witness identity comparison indicates authority drift),
- `unification.evidence_stage1.rollback.unbound`
  (missing deterministic binding context for rollback witness comparison).

#### 1.6.4 Stage 2 authority mapping table (normative)

When Stage 2 is active, implementations MUST bind doctrine clauses to
deterministic checker/vector surfaces as follows.

| Stage 2 clause | Typed contract surface | Checker surface | Executable vectors |
| --- | --- | --- | --- |
| typed core is authority; alias is projection-only | `draft/CONTROL-PLANE-CONTRACT.json` `evidenceStage2Authority` (`activeStage=stage2`, `aliasRole=projection_only`) | `mise run coherence-check` (`gate_chain_parity` Stage 2 checks) | `tests/conformance/fixtures/coherence-site/*/gate_chain_parity_stage2_*` |
| alias-window fail-closed enforcement | lifecycle table in `draft/CONTROL-PLANE-CONTRACT.json` + `profile/UNIFICATION-GOVERNANCE` §5.1 governance | `gate_chain_parity` Stage 2 alias-window checks | `gate_chain_parity_stage2_alias_window_reject` |
| alias-as-authority rejection | `evidenceStage2Authority.requiredFailureClasses.authorityAliasViolation` | `gate_chain_parity` Stage 2 alias-role checks | `gate_chain_parity_stage2_alias_role_reject` |
| unbound typed authority rejection | `evidenceStage2Authority.requiredFailureClasses.unbound` | `gate_chain_parity` Stage 2 unbound checks | `gate_chain_parity_stage2_unbound_binding_reject` |
| direct BIDIR evidence route parity | `evidenceStage2Authority.bidirEvidenceRoute` + canonical obligation set (`routeKind=direct_checker_discharge`, `obligationFieldRef=bidirCheckerObligations`) | `gate_chain_parity` Stage 2 bidir-route checks | `gate_chain_parity_stage2_kernel_missing_reject`, `gate_chain_parity_stage2_kernel_drift_reject` |
| typed-first consumer lineage (CI/instruction/decision/observation) | typed authority fields carried in witness/decision/snapshot payloads (`typedCoreProjectionDigest`, `authorityPayloadDigest`, `normalizerId`, `policyDigest`) | CI witness validators + observation projection selection | `capabilities.ci_witnesses` boundary-authority vectors (`golden/boundary_authority_lineage_accept`, adversarial/invariance pairs) |

Equivalent implementation-local routes are permitted only when replay-stable
and mapped deterministically to these clause-level surfaces.

Observation/projection consumers MUST treat typed digests as canonical by
default. Compatibility alias lookup MAY exist only behind an explicit
compatibility mode (for example `match=compatibility_alias`), and MUST NOT be
the default projection-selection path.

#### 1.6.5 Stage 3 typed-first closure mapping (normative)

When Stage 3 cleanup is claimed complete, implementations MUST bind closure
clauses to deterministic checker/vector surfaces as follows.

| Stage 3 clause | Typed contract surface | Checker surface | Executable vectors/tests |
| --- | --- | --- | --- |
| direct checker/discharge route is canonical authority route | `evidenceStage2Authority.bidirEvidenceRoute` (`routeKind=direct_checker_discharge`, `obligationFieldRef=bidirCheckerObligations`) | `gate_chain_parity` Stage 2 bidir-route checks | `gate_chain_parity_stage2_kernel_missing_reject`, `gate_chain_parity_stage2_kernel_drift_reject` |
| transitional sentinel path is compatibility-only and profile-gated | optional `kernelComplianceSentinel` + `bidirEvidenceRoute.fallback.mode=profile_gated_sentinel` with current `profileKind` in `fallback.profileKinds` | Stage 2 authority fallback-gating checks in `gate_chain_parity` | `cargo test -p premath-coherence` (`check_gate_chain_parity_rejects_stage2_bidir_route_obligation_mismatch`, `check_gate_chain_parity_rejects_stage2_bidir_route_failure_class_mismatch`) |
| typed-first consumer lineage is canonical across CI/CLI/MCP/observation | typed authority fields (`typedCoreProjectionDigest`, `normalizerId`, `policyDigest`) are canonical; alias fields are compatibility metadata only | CI witness lineage validators + observation typed-default projection selection | `capabilities.ci_witnesses` boundary-authority vectors + CLI/observation compatibility-mode checks |

Stage 3 closure MUST NOT introduce a second authority artifact. If fallback is
temporarily re-enabled for a profile, rollback/re-promotion MUST follow
`§1.6` rollback requirements with issue + decision-log linkage.

## 2. Cross-layer Obstruction Algebra (v0)

Implementations MAY project failure classes into one typed obstruction algebra
for cross-layer analysis. This algebra is secondary metadata; it MUST NOT
replace source-layer failure-class authority.

### 2.1 Constructor set

Minimum constructor families:

- `semantic(tag)` for Gate/BIDIR semantic failures,
- `structural(tag)` for checker-structure/parity failures,
- `lifecycle(tag)` for lifecycle/attestation/authority-chain failures,
- `commutation(tag)` for typed cross-lane commutation failures.

### 2.2 Deterministic projection pair

Conforming projections MUST provide deterministic functions:

- `project_obstruction(sourceFailureClass) -> constructor`
- `canonical_obstruction_class(constructor) -> canonicalFailureClass`

For fixed contract bytes + repository state + deterministic bindings, both
functions MUST be replay-stable.

### 2.3 Compatibility rule

Existing failure vocabularies remain unchanged:

- Gate/BIDIR classes remain source of semantic admissibility failure truth.
- Coherence classes remain source of checker parity/shape failure truth.
- CI/lifecycle classes remain source of control-plane lifecycle failure truth.

The obstruction algebra MUST be vocabulary-preserving:

- it MAY add typed constructor metadata,
- it MUST NOT rename, suppress, or reinterpret source failure classes.

### 2.4 Initial mapping table (minimum)

| Source failure class | Constructor | Canonical class |
| --- | --- | --- |
| `stability_failure` | `semantic(stability)` | `stability_failure` |
| `locality_failure` | `semantic(locality)` | `locality_failure` |
| `descent_failure` | `semantic(descent)` | `descent_failure` |
| `glue_non_contractible` | `semantic(contractibility)` | `glue_non_contractible` |
| `adjoint_triple_coherence_failure` | `semantic(adjoint_triple)` | `adjoint_triple_coherence_failure` |
| `coherence.cwf_substitution_identity.violation` | `structural(cwf_substitution_identity)` | `coherence.cwf_substitution_identity.violation` |
| `coherence.cwf_substitution_composition.violation` | `structural(cwf_substitution_composition)` | `coherence.cwf_substitution_composition.violation` |
| `coherence.span_square_commutation.violation` | `commutation(span_square_commutation)` | `coherence.span_square_commutation.violation` |
| `decision_witness_sha_mismatch` | `lifecycle(decision_attestation)` | `decision_witness_sha_mismatch` |
| `decision_delta_sha_mismatch` | `lifecycle(decision_delta_attestation)` | `decision_delta_sha_mismatch` |
| `unification.evidence_factorization.missing` | `lifecycle(evidence_factorization_missing)` | `unification.evidence_factorization.missing` |
| `unification.evidence_factorization.ambiguous` | `lifecycle(evidence_factorization_ambiguous)` | `unification.evidence_factorization.ambiguous` |
| `unification.evidence_factorization.unbound` | `lifecycle(evidence_factorization_unbound)` | `unification.evidence_factorization.unbound` |

Implementations MAY extend this table, but extensions MUST stay deterministic
and MUST NOT change existing row mappings.

### 2.5 Witness and issue-memory projection

If an implementation emits obstruction constructors:

- witness payloads SHOULD include constructor metadata next to source classes,
- issue discovery flows SHOULD project deterministic tags
  `obs.<family>.<tag>` for long-running memory/indexing.

These projections are informative indexing aids and MUST NOT affect admissibility
decisions.

## 3. Grothendieck Operationalization Contract (v0)

This section defines the canonical operational reading of the discipline for
worker orchestration and control-plane artifact routing.

### 3.1 One fibration, many deterministic projections

Implementations claiming this contract MUST treat the semantic authority shape
as one indexed family/fibration over contexts:

- semantic authority projection: `p0 : E -> C` (kernel/Gate path),
- attested control-plane evidence family: `Ev : Ctx^op -> V` (§1).

Operational surfaces (instruction, coherence, CI witness, harness trajectory,
observation projections, issue-memory projections) MAY vary by workflow, but
MUST remain deterministic projections over one authority path and MUST NOT
introduce a second admissibility schema.

### 3.2 Worker orchestration as site cover/descent

Concurrent work decomposition MUST be modeled as admissible covers on `Ctx`:

1. coordinator selects an admissible cover
   `{rho_i : Gamma_i -> Gamma}` over one active work context `Gamma`,
2. each worker computes local candidate evidence over one refinement `Gamma_i`,
3. overlap compatibility is checked on pullbacks
   `Gamma_i x_Gamma Gamma_j`,
4. coordinator performs deterministic glue-or-obstruction discharge.

Global acceptance from concurrent work is valid only when glue/discharge is
accepted through checker/Gate authority. Otherwise, deterministic obstruction
witnesses MUST be emitted.

### 3.3 Universal evidence factoring requirement

For each attestable operational family `F : Ctx^op -> V` in one execution
profile, implementations MUST provide one deterministic factoring route:

- `eta_F : F => Ev`

such that:

1. all acceptance/rejection outputs consumed by runtime/control surfaces are
   obtained through `Ev`,
2. no surface may self-authorize admissibility without checker/Gate discharge,
3. parallel inequivalent factorization routes fail closed under §1.5.

### 3.4 Commutation and deterministic binding

Cross-lane pullback/base-change claims in this operationalization MUST:

1. route through typed span/square witnesses
   (`draft/SPAN-SQUARE-CHECKING`),
2. bind equality/comparison claims to `normalizerId + policyDigest`,
3. preserve lane ownership in `profile/UNIFICATION-GOVERNANCE` §9
   (semantic/checker/commutation/runtime lanes).

### 3.5 Operational theorem shape (normative reading)

For fixed canonical contract bytes, repository state, and deterministic binding
context:

1. local worker outputs that factor through `Ev` and satisfy overlap
   compatibility obligations produce one deterministic global accepted route up
   to canonical projection equality in `Ev`, or
2. produce deterministic rejection with obstruction/failure witnesses.

Implementations MUST treat this as an authority boundary rule, not a heuristic.

### 3.6 Minimum implementation checklist

Conforming implementations SHOULD:

1. encode worker decomposition in cover/refinement terms (`Ctx`, `J`),
2. verify overlap/base-change claims via span/square witnesses,
3. keep harness/coherence/CI surfaces as projections into `Ev`,
4. keep admissibility decisions checker/Gate-owned only,
5. reject deterministically on missing/ambiguous/unbound evidence factorization
   routes.

### 3.7 Explicit constructor binding requirement (GC profile)

Implementations SHOULD materialize one explicit constructor object for active
control-plane worldization profiles (see `draft/WORLD-REGISTRY` §2.5), with:

- one declared context/cover base,
- one deterministic world/morphism/route binding family digest closure,
- one evidence-factorization route set into `Ev`.

Doctrine-site route class policy and route-world bindings MUST provide total,
unambiguous constructor input (`draft/DOCTRINE-SITE` §3.4-§3.5).

Interpretation overlays (for example torsor/extension overlays) MAY decorate
constructor outputs but MUST remain non-authority attachments and MUST NOT
introduce alternate admissibility routes.
