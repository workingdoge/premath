---
slug: draft
shortname: OBSERVATION-INF
title: workingdoge.com/premath/OBSERVATION-INF
name: Observation Functor and Deterministic Projection Discipline
status: draft
category: Standards Track
tags:
  - premath
  - conformance
  - observation
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

This specification defines observation as a deterministic, read-only functor
from witness state and issue memory to projection surfaces:

```text
Obs : WitnessState × IssueMemory → Surface
```

**Non-authority boundary.** Observation MUST NOT introduce independent semantic
authority. The observation surface is a projection — it cannot reject, gate, or
attest. Any acceptance/rejection consumed by downstream surfaces MUST remain
checker/Gate-discharged and factor through Unified Evidence routing
(`draft/EVIDENCE-INF` §1).

Observation factors through `eta_Obs : Obs => Ev` (the Unified Evidence Plane;
`draft/EVIDENCE-INF` §1).

This document is normative when capability
`capabilities.observation_semantics` is claimed.

## 2. Observation functor

The observation functor `Obs` is defined abstractly over:

- **Inputs.**
  - `W`: a witness family — the set of CI witness artifacts indexed by source
    identifier and run identifier.
  - `I`: issue memory — the set of issue records with lifecycle state and
    dependency structure.

- **Output.**
  - `S`: an observation surface — a deterministic, read-only composite
    projection of the input state.

**Determinism.** For fixed `(W, I)`, `Obs` MUST produce a unique `S` up to
canonical equality. Implementations MUST NOT introduce non-determinism through
ordering sensitivity, environment variation, or cache state.

**Read-only boundary.** The observation surface `S` is a projection, not an
authority. It MUST NOT:

- reject or accept witness artifacts,
- gate downstream operations,
- attest or produce commitment evidence,
- mutate witness state or issue memory.

## 3. State derivation lattice

State values form a finite lattice over the set
`{accepted, rejected, running, error, empty}`.

### 3.1 Priority ordering

The priority ordering determines which evidence source wins when multiple
sources exist for a single concern:

```text
decision > required > instruction > empty
```

Where:

- `decision`: a terminal state from a checker/gate verdict (`accepted` or
  `rejected`).
- `required`: a state from a required CI witness (`running`, `accepted`,
  `rejected`, `error`).
- `instruction`: a state from an instruction-typed artifact.
- `empty`: no evidence source present.

### 3.2 Derivation fold

State derivation is a monotone fold over evidence sources:

```text
derive : EvidenceFamily → State
```

Given an evidence family `{e_1, ..., e_n}` for a concern, `derive` selects the
state from the highest-priority evidence source. When multiple sources share the
same priority tier, the fold MUST be deterministic — implementations MUST use a
stable ordering (e.g., lexicographic on source identifier) to break ties.

**Monotonicity.** Adding a higher-priority evidence source MUST NOT lower the
derived state in the lattice. Removing evidence sources MUST NOT promote a
lower-priority source's state above the previous derived state unless the
removed source was the sole contributor at its priority tier.

### 3.3 Lattice operations

- `join(s1, s2)`: the state from the higher-priority evidence source.
- `meet(s1, s2)`: the state from the lower-priority evidence source.
- `bottom`: `empty`.
- `top`: any `decision`-tier state.

## 4. Coherence as natural transformation

Observation coherence is a natural transformation:

```text
η : Obs ∘ π_i => Obs_total
```

checking compatibility of sub-projections with the total surface. Each
sub-projection is independently derivable from the same inputs — coherence
verifies they agree with the composed total surface.

### 4.1 Sub-projection families

The following sub-projection families are defined. Each is a deterministic
projection for fixed inputs `(W, I)`:

1. **Policy drift.** Compares the observation surface's policy digest against
   the canonical policy source. Detects when policy has changed since the last
   observation build.

2. **Instruction typing.** Projects instruction-typed artifacts and validates
   their type annotations against the instruction doctrine.

3. **Issue partition.** Projects issue memory into a partitioned view by
   lifecycle state (open, closed, blocked, in-progress).

4. **Dependency integrity.** Projects issue dependency structure and validates
   acyclicity, dangling references, and blocking relationships.

5. **Lease health.** Projects active lease state and validates lease
   expiry/renewal invariants.

6. **Worker throughput.** Projects worker assignment and throughput metrics
   from witness artifacts.

### 4.2 Coherence check

For each sub-projection `π_i`, the coherence check verifies:

```text
η_i : Obs(π_i(W, I)) = π_i(Obs(W, I))
```

That is, projecting first then observing MUST produce the same result as
observing first then projecting. When any `η_i` fails, the coherence check
MUST report the failing sub-projection family and diagnostic message.

## 5. Attention derivation

### 5.1 `needsAttention`

`needsAttention` is a derived boolean computed as the join of:

- **State-level attention.** `true` when the derived state (§3) is `rejected`,
  `error`, or `running` for any concern that has a `decision`-tier or
  `required`-tier evidence expectation.
- **Coherence-level attention.** `true` when any sub-projection coherence check
  (§4) fails.

`needsAttention` MUST be deterministic for a fixed surface `S`.

### 5.2 `topFailureClass`

`topFailureClass` is the first failure class under a fixed priority ordering
over failure class families (§8). When no failures exist, `topFailureClass`
MUST be `null`.

The priority ordering is:

1. Schema violations (highest priority).
2. Build failures.
3. Projection mismatches (lowest priority).

`topFailureClass` MUST be deterministic for a fixed surface `S`.

## 6. Projection match discipline

Two match modes form a refinement:

```text
typed ⊑ compatibility_alias
```

### 6.1 Typed mode (default)

Typed mode matches on canonical typed digests only. A typed digest includes the
schema version, content hash, and type annotation. Typed mode MUST be the
default for all projection matching.

### 6.2 Compatibility mode

Compatibility mode matches on typed OR legacy (untyped) digests. Legacy digests
are content hashes without type annotations, produced by earlier versions of the
observation infrastructure.

Implementations MUST default to typed mode. Compatibility mode MUST be
explicitly requested by the caller. When compatibility mode is used, the
observation surface MUST annotate the match result with the mode used.

### 6.3 Refinement relationship

Typed mode is a refinement of compatibility mode: any match accepted by typed
mode is also accepted by compatibility mode, but not vice versa. That is:

```text
match_typed(d) = true  ⟹  match_compat(d) = true
match_compat(d) = true  ⟹  match_typed(d) = true ∨ match_typed(d) = false
```

## 7. Evidence factoring

`eta_Obs : Obs => Ev` MUST factor through the Unified Evidence Plane
(`draft/EVIDENCE-INF` §1).

### 7.1 Factoring uniqueness

For fixed canonical inputs `(W, I)`, the factoring MUST be unique. That is,
there is exactly one way to map the observation surface into the evidence plane
for a given input state.

### 7.2 Typed evidence fields

Observation surfaces carry typed evidence fields that MUST be consistent with
upstream witness artifacts:

- Evidence digests in the observation surface MUST match the digests of the
  source witness artifacts.
- Evidence type annotations MUST match the declared types in the witness family.
- When an upstream witness artifact is updated, the corresponding evidence field
  in the observation surface MUST reflect the update on the next observation
  build.

## 8. Failure classification algebra

Failure classes form a closed vocabulary. Each failure has:

- `class: string` — a stable failure class identifier,
- `message: string` — a human-readable diagnostic.

### 8.1 Fail-closed discipline

The checker MUST be fail-closed: unknown states are failures, not
pass-throughs. Any state not explicitly classified as accepted MUST be treated
as a failure.

### 8.2 Failure class families

Failure classes partition into three families:

**Schema violations:**

```text
observation_schema_invalid           — surface does not conform to expected schema
observation_schema_version_unknown   — unrecognized schema version
observation_evidence_type_mismatch   — evidence type annotation mismatch
```

**Build failures:**

```text
observation_build_witness_missing    — required witness artifact not found
observation_build_issue_load_failed  — issue memory could not be loaded
observation_build_normalize_failed   — normalization of input state failed
observation_build_derive_failed      — state derivation produced inconsistent result
```

**Projection mismatches:**

```text
observation_projection_digest_mismatch — projection digest does not match tracked
observation_projection_coherence_failed — sub-projection coherence check failed
observation_projection_mode_invalid    — invalid projection match mode requested
```

## 9. Doctrine morphism preservation

Observation preserves the following doctrine morphisms
(per `draft/DOCTRINE-INF`):

- `dm.identity` — identity morphism is preserved (observing the same state
  twice produces the same surface).
- `dm.presentation.projection` — presentation-projection morphism is preserved
  (observation is itself a deterministic projection).

Observation does NOT preserve:

- `dm.policy.rebind` — observation does not rebind policy.
- `dm.transport.world` — observation does not transport across worlds.
- `dm.commitment.attest` — observation does not produce attestation evidence.

This is consistent with the non-authority boundary (§1): observation is
read-only projection, not authority.

## 10. Non-goals

This document does not prescribe:

- storage backend or persistence mechanism,
- specific CI vendor integration or witness format,
- event format or streaming infrastructure,
- mutation semantics or write operations,
- caching, refresh, or invalidation strategy.

It prescribes only the functor structure, state derivation lattice, coherence
discipline, and non-authority boundary for observation.
