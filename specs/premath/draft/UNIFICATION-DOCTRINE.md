---
slug: draft
shortname: UNIFICATION-DOCTRINE
title: workingdoge.com/premath/UNIFICATION-DOCTRINE
name: Minimum Encoding, Maximum Expressiveness Doctrine
status: draft
category: Standards Track
tags:
  - premath
  - doctrine
  - architecture
  - unification
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

This doctrine defines the architectural rule for Premath evolution:

- minimum canonical encoding at authority boundaries,
- maximum expressiveness via typed projections, overlays, and capability claims.

It applies to instruction/proposal/checking, issue memory, conformance surfaces,
and interop artifacts.

## 2. Core Principle

For any semantic boundary `B`, implementations MUST prefer:

1. one canonical representation at `B`,
2. many deterministic views derived from that representation.

Premath systems SHOULD add expressiveness by adding projections and obligation
routes, not by forking canonical encodings.

## 3. Canonical Boundary Rules

### 3.1 Single authority encoding

Each authority boundary MUST define one canonical payload shape and one
deterministic identity function.

Examples:

- instruction proposals: canonical proposal payload + deterministic
  `proposalDigest`/`proposalKcirRef`,
- issue memory: `issue.event.v1` append-only substrate + deterministic replay.

### 3.2 Derived view discipline

Derived views MUST be deterministic projections of canonical payloads.

Derived views MAY optimize for workflow semantics (execution, GTD, groupoid,
profile overlays), but MUST NOT introduce independent semantic authority.

### 3.3 Binding discipline

Any normalized/evidence-producing route MUST carry deterministic binding
material:

- `normalizerId`,
- `policyDigest`,
- canonical refs where applicable (for example `kcir1_*`, `cmp1_*`, `ev1_*`,
  `iss1_*`).

## 4. Expressiveness Without Forks

Expressiveness SHOULD be introduced by:

- capability-scoped overlays,
- obligation compilation/discharge hints,
- additional deterministic projections,
- richer witness annotations.

Expressiveness MUST NOT be introduced by:

- parallel canonical schemas for the same authority boundary,
- implicit authority in planner/proposal outputs,
- unverifiable side-channel state.

## 5. Migration Rules

When replacing or tightening a boundary representation:

1. implementations SHOULD provide deterministic projection/replay between old
   and new surfaces,
2. compatibility aliases MAY exist temporarily,
3. canonical authority MUST move to one boundary before compatibility aliases
   are removed.

Compatibility fields (for example digest aliases) MUST stay bound to the same
canonical payload while they coexist.

### 5.1 Schema lifecycle policy (contract/witness/projection kinds)

Control-plane implementations MUST publish one deterministic lifecycle table for
schema/versioned kind families (for example `*.contract.v*`, witness kinds, and
projection kinds).

For each kind family:

1. exactly one canonical kind MUST be declared,
2. compatibility aliases MAY be declared with an explicit `supportUntilEpoch`,
3. each alias MUST declare a canonical replacement kind,
4. checkers MUST resolve accepted aliases to canonical kind before downstream
   comparison,
5. compatibility aliases participating in one lifecycle table MUST share one
   deterministic rollover epoch,
6. rollover runway (`supportUntilEpoch - activeEpoch`) MUST be positive and
   bounded (CI implementation profile: max 12 months),
7. lifecycle tables MUST declare governance mode metadata under
   `schemaLifecycle.governance` with at least:
   - `mode` (`rollover` or `freeze`),
   - `decisionRef`,
   - `owner`,
8. when `mode=rollover`, `rolloverCadenceMonths` MUST be explicit and
   compatibility aliases MUST remain within that cadence,
9. when `mode=freeze`, compatibility aliases MUST be absent and `freezeReason`
   MUST be explicit,
10. governance transitions (`rollover <-> freeze`) MUST be decision-logged and
    linked by `decisionRef`.

When a breaking shape change is introduced:

- migration MUST emit witness evidence that the old payload was replayed or
  projected into the canonical replacement,
- the compatibility window MUST be explicit in the lifecycle table.

After `supportUntilEpoch`, checkers MUST reject the alias deterministically
(fail closed) and report the canonical replacement kind.

Process-level governance shape and operator flow are defined in:

- `specs/process/SCHEMA-LIFECYCLE-GOVERNANCE.md`.

## 6. Conformance Expectations

Implementations following this doctrine SHOULD:

- expose deterministic witness lineage from canonical payload to final verdict,
- fail closed on unknown/unbound classifications at authority boundaries,
- run doctrine/traceability/coherence checks in merge-gated command surfaces.

## 7. Relationship to Other Specs

This doctrine constrains how existing specs compose:

- `draft/SPEC-INDEX` (normative scope and claims),
- `draft/LLM-INSTRUCTION-DOCTRINE` and `draft/LLM-PROPOSAL-CHECKING`
  (checking-mode authority split),
- `draft/PREMATH-COHERENCE` (cross-surface parity obligations),
- `draft/SPAN-SQUARE-CHECKING` (typed span/square witness layer for
  pipeline/base-change commutation),
- `draft/CHANGE-MORPHISMS` (deterministic change projections),
- `draft/KCIR-CORE`, `draft/NF`, `draft/NORMALIZER` (interop identity surfaces).

## 8. KCIR Boundary Profile (v0)

This profile pins one KCIR-compatible identity path for proposal-bearing
instruction/checking boundaries.

### 8.1 Canonical proposal KCIR projection

Implementations exposing `proposalKcirRef` MUST derive it from:

```text
KCIRProposalProjection {
  kind: "kcir.proposal.v1",
  canonicalProposal: <Section 2 canonical proposal payload from LLM-PROPOSAL-CHECKING>
}
```

`proposalKcirRef` is:

```text
"kcir1_" + SHA256(JCS(KCIRProposalProjection))
```

### 8.2 Boundary map

| Boundary | Canonical payload | Canonical identity |
| --- | --- | --- |
| instruction envelope proposal field | `LLMProposal` canonical payload | `proposalKcirRef` (preferred) + `proposalDigest` (compatibility alias) |
| proposal ingest witness | canonical proposal + obligation/discharge projection | `proposalKcirRef` in witness lineage |
| coherence parity and migration witnesses | deterministic parity tuple containing proposal identity keys when present | `proposalKcirRef` |
| capability/conformance vectors | deterministic replay payload over the same canonical proposal | `proposalKcirRef` and deterministic reject on mismatch |

Derived profiles MAY add projection metadata, but MUST NOT fork this canonical
proposal KCIR projection.

### 8.3 Duplicate encoding deprecation rule

When multiple code paths validate proposal identity:

1. one shared validator module MUST own canonicalization and declared-ref
   validation,
2. other paths MUST call that module and MUST NOT re-encode independent
   validation semantics,
3. compatibility identities (`proposalDigest`) MAY remain while migration is in
   progress, but MUST stay bound to the same canonical proposal payload as
   `proposalKcirRef`.

## 9. Lane Separation Contract (v0)

To preserve minimum encoding with maximum expressiveness, implementations MUST
keep the following lane split explicit and non-overlapping:

| Lane | Primary role | Canonical references | Non-authority constraints |
| --- | --- | --- | --- |
| Semantic doctrine lane | semantic meaning and obligation authority | `draft/PREMATH-KERNEL`, `draft/BIDIR-DESCENT`, `draft/GATE`, `profile/ADJOINTS-AND-SITES` | MAY compile to obligations; MUST NOT be bypassed by planner/projection outputs |
| Strict checker lane | strict operational equalities (`≡`) for deterministic checking | `draft/PREMATH-COHERENCE`, `draft/COHERENCE-CONTRACT.json` | validates control-plane consistency only; MUST NOT redefine kernel admissibility |
| Witness commutation lane | typed pipeline/base-change commutation artifacts | `draft/SPAN-SQUARE-CHECKING` | checker-facing evidence only; MUST NOT self-authorize acceptance |
| Runtime transport lane | runtime location/world transport surfaces | `raw/SQUEAK-CORE`, `raw/SQUEAK-SITE` | transport/site checks are capability-scoped; MUST remain bound to canonical witness lineage |

### 9.1 SigPi (adjoint) lane rule

When `capabilities.adjoints_sites` is claimed, the SigPi adjoint triple
(`\Sigma_f -| f* -| \Pi_f`, shorthand `sig\Pi`) and
Beck-Chevalley obligations remain in the semantic doctrine lane and MUST
discharge under deterministic bindings (`normalizerId`, `policyDigest`) without
introducing a parallel authority encoding.

### 9.2 Squeak lane rule

When `capabilities.squeak_site` is claimed, Squeak transport/site witnesses MAY
extend expressiveness, but MUST project into the same canonical authority chain
as other evidence surfaces (no side-channel acceptance path).

### 9.3 Composition rule

When SigPi adjoint obligations, span/square commutation obligations, and Squeak
transport obligations are composed in one implementation profile:

1. composition MUST occur via obligation and witness routing, not by creating a
   second semantic authority schema,
2. composed checks MUST remain deterministic and fail closed on unknown/unbound
   lane or capability material,
3. cross-lane pullback/base-change claims MUST project through typed
   span/square witnesses (`draft/SPAN-SQUARE-CHECKING`), including deterministic
   composition-law witnesses (identity/associativity/h-v/interchange) when
   composed routing is claimed,
4. derived projections MAY vary by workflow, but MUST remain replayable to one
   canonical authority artifact.

### 9.4 CwF/SigPi bridge rule

When strict CwF equalities are bridged into SigPi semantic obligations:

1. bridge morphisms MUST compile into existing obligation vocabularies
   (`cwf_*`, `stability`, `locality`, `descent_*`, `adjoint_triple`,
   `beck_chevalley_*`) instead of creating new authority vocabularies,
2. strict (`≡`) and semantic (`~=`) equality notions MUST remain lane-local and
   explicitly labeled in witness lineage,
3. bridge outputs MAY reduce search cost, but MUST NOT reduce discharge
   obligations or bypass checker/Gate authority.

## 10. Unified Evidence Plane Contract (v0)

Implementations claiming the Unified Evidence Plane MUST model attested evidence
as one context-indexed family:

- `Ev : Ctx^op -> V`

where `V` is the selected witness universe (`Set`, `Groupoid`, or `Spaces`).

### 10.1 Canonical evidence object

`Ev` is the canonical attested evidence surface for control-plane outputs.

`Ev` does not replace kernel or Gate authority. Kernel/Gate decide admissibility;
`Ev` is the deterministic attestation/projection surface for accepted/rejected
checker outcomes.

### 10.2 Universal factoring rule

For every control-plane artifact family `F : Ctx^op -> V` that carries
attestable output (instruction/proposal/coherence/CI/observation projections),
there MUST be one deterministic natural transformation:

- `eta_F : F => Ev`

so artifact meaning factors through one evidence surface instead of parallel
authority schemas.

For fixed canonical inputs (contract bytes + repository state + deterministic
binding context), `eta_F` MUST be unique up to canonical projection equality in
`Ev` (no alternate authority encoding for the same artifact family output).

### 10.3 Required law set

A conforming `Ev` route MUST satisfy:

1. Transport law (naturality): reindexing commutes with evidence projection.
2. Descent law: cover-local evidence either glues deterministically or emits
   deterministic obstruction witnesses.
3. Determinism law: equality/comparison claims are bound to
   `normalizerId + policyDigest`.
4. Authority-boundary law: proposals/projections MAY suggest, but MUST NOT
   self-authorize admissibility without checker discharge.

### 10.4 Cross-lane commutation requirement

When cross-lane pullback/base-change claims are surfaced in `Ev`, implementations
MUST route commutation through typed span/square witnesses
(`draft/SPAN-SQUARE-CHECKING`) so lane composition remains explicit and
replayable.

### 10.5 Fail-closed factorization boundary

Implementations MUST reject deterministically when factorization into `Ev`
cannot be established as a unique canonical route.

Minimum fail-closed classes:

- `unification.evidence_factorization.missing` (no typed `eta_F` route),
- `unification.evidence_factorization.ambiguous` (multiple inequivalent routes),
- `unification.evidence_factorization.unbound` (missing deterministic binding
  context for canonical comparison).

Equivalent implementation-local class names are permitted only when a
deterministic mapping to these classes is documented and replay-stable.

### 10.6 Typed evidence-object internalization stages (v0)

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
     (`draft/UNIFICATION-DOCTRINE` §5.1 +
     `draft/CONTROL-PLANE-CONTRACT.json`).
4. Stage 3 (typed-first cleanup):
   - expired compatibility aliases MUST reject fail closed,
   - all control-plane consumers MUST use typed evidence authority directly,
   - no compatibility alias may reintroduce a parallel authority route.

Stage-gate requirements:

1. stage transitions MUST preserve the §10.2 factoring rule (`eta_F : F => Ev`)
   for all claimed artifact families,
2. stage transitions MUST preserve §10.3 deterministic binding
   (`normalizerId + policyDigest`),
3. stage transitions MUST preserve §10.5 fail-closed factorization behavior.

Rollback requirements:

1. if a stage fails deterministic parity or replay checks, rollback to the
   previous accepted stage MUST be deterministic and MUST preserve prior
   canonical identity bindings,
2. rollback MUST NOT introduce a second authority artifact,
3. rollback conditions and target stage MUST be decision-logged and issue-linked
   before re-attempting promotion.

#### 10.6.1 Stage 1 typed-core profile (minimum)

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

#### 10.6.2 Stage 1 dual-projection parity contract

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

#### 10.6.3 Stage 1 deterministic rollback witness contract

When Stage 1 rollback is claimed, implementations MUST define one deterministic
rollback witness profile bound to:

- source/target stages (`stage1 -> stage0`),
- deterministic binding tuple (`normalizerId`, `policyDigest`),
- authority digest refs for pre-rollback and rollback-target authority
  checkpoints.

Rollback trigger metadata MUST include at least all Stage 1 parity classes from
§10.6.2 (`missing`, `mismatch`, `unbound`), so rollback admission remains
deterministic and replayable.

Implementations MUST reject fail closed on Stage 1 rollback witness errors with
at least:

- `unification.evidence_stage1.rollback.precondition`
  (missing/invalid rollback preconditions),
- `unification.evidence_stage1.rollback.identity_drift`
  (rollback witness identity comparison indicates authority drift),
- `unification.evidence_stage1.rollback.unbound`
  (missing deterministic binding context for rollback witness comparison).

#### 10.6.4 Stage 2 authority mapping table (normative)

When Stage 2 is active, implementations MUST bind doctrine clauses to
deterministic checker/vector surfaces as follows.

| Stage 2 clause | Typed contract surface | Checker surface | Executable vectors |
| --- | --- | --- | --- |
| typed core is authority; alias is projection-only | `draft/CONTROL-PLANE-CONTRACT.json` `evidenceStage2Authority` (`activeStage=stage2`, `aliasRole=projection_only`) | `mise run coherence-check` (`gate_chain_parity` Stage 2 checks) | `tests/conformance/fixtures/coherence-site/*/gate_chain_parity_stage2_*` |
| alias-window fail-closed enforcement | lifecycle table in `draft/CONTROL-PLANE-CONTRACT.json` + §5.1 governance | `gate_chain_parity` Stage 2 alias-window checks | `gate_chain_parity_stage2_alias_window_reject` |
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

## 11. Cross-layer Obstruction Algebra (v0)

Implementations MAY project failure classes into one typed obstruction algebra
for cross-layer analysis. This algebra is secondary metadata; it MUST NOT
replace source-layer failure-class authority.

### 11.1 Constructor set

Minimum constructor families:

- `semantic(tag)` for Gate/BIDIR semantic failures,
- `structural(tag)` for checker-structure/parity failures,
- `lifecycle(tag)` for lifecycle/attestation/authority-chain failures,
- `commutation(tag)` for typed cross-lane commutation failures.

### 11.2 Deterministic projection pair

Conforming projections MUST provide deterministic functions:

- `project_obstruction(sourceFailureClass) -> constructor`
- `canonical_obstruction_class(constructor) -> canonicalFailureClass`

For fixed contract bytes + repository state + deterministic bindings, both
functions MUST be replay-stable.

### 11.3 Compatibility rule

Existing failure vocabularies remain unchanged:

- Gate/BIDIR classes remain source of semantic admissibility failure truth.
- Coherence classes remain source of checker parity/shape failure truth.
- CI/lifecycle classes remain source of control-plane lifecycle failure truth.

The obstruction algebra MUST be vocabulary-preserving:

- it MAY add typed constructor metadata,
- it MUST NOT rename, suppress, or reinterpret source failure classes.

### 11.4 Initial mapping table (minimum)

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

### 11.5 Witness and issue-memory projection

If an implementation emits obstruction constructors:

- witness payloads SHOULD include constructor metadata next to source classes,
- issue discovery flows SHOULD project deterministic tags
  `obs.<family>.<tag>` for long-running memory/indexing.

These projections are informative indexing aids and MUST NOT affect admissibility
decisions.
