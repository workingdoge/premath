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
   span/square witnesses (`draft/SPAN-SQUARE-CHECKING`),
4. derived projections MAY vary by workflow, but MUST remain replayable to one
   canonical authority artifact.

## 10. Unified Evidence Plane Contract (v0)

To keep one authority surface while increasing expressiveness, implementations
SHOULD model attested evidence as one context-indexed family:

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
there SHOULD be a deterministic natural transformation:

- `eta_F : F => Ev`

so artifact meaning factors through one evidence surface instead of parallel
authority schemas.

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
