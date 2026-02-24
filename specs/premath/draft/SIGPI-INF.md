---
slug: draft
shortname: SIGPI-INF
title: workingdoge.com/premath/SIGPI-INF
name: SigPi Adjoint Triple and Dependent Stepping Discipline
status: draft
category: Standards Track
tags:
  - premath
  - conformance
  - sigpi
  - adjoint
  - stepping
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

This spec defines the concern-abstract adjoint triple `Σ_f ⊣ f* ⊣ Π_f`
applied to stepping over concern fibres. The categorical structure follows
`profile/ADJOINTS-AND-SITES` (context reindexing), instantiated for
time-stepping.

The three components of the triple:

- **`Σ_f`** (left adjoint): pushes accumulated state forward
  (memory/commitment).
- **`f*`** (reindexing): pulls back state (observation/projection).
- **`Π_f`** (right adjoint): computes available actions (future/choice).

This spec is a higher-level parent doctrine. Existing and future concern specs
are all instantiations:

- `draft/CHANGE-INF` instantiates `Σ_f` (mutation/accumulation side).
- `draft/OBSERVATION-INF` instantiates `f*` (observation/projection side).
- Future concern specs instantiate additional components of the triple.

This spec is normative when `capabilities.sigpi_stepping` is claimed.

## 2. Concern fibre index

The index category `I` has:

- **Objects**: concern identifiers (finite set of named concerns).
- **Morphisms**: declared dependency edges between concerns.

`I` MUST be a finite DAG (well-founded, no cycles). This is the decomposition
of total state into independent but potentially dependent concern fibres.

Each concern `i ∈ I` contributes:

- A fibre `Σ_i` to accumulated state.
- A reindexing surface `f*_i`.
- A fibre `Π_i` to the action space.

Cross-fibre dependencies in `I` constrain which actions and projections are
available: if concern `j` depends on concern `i`, then `Π_j(Σ)` may depend on
`Σ_i`.

## 3. The adjoint triple

Given a stepping morphism `f`, the triple `Σ_f ⊣ f* ⊣ Π_f` is defined as
follows:

**`Σ_f`** (left adjoint — dependent sum): pushes state forward, accumulating
into the past. `draft/CHANGE-INF` mutations live here. Σ_f takes a local
concern-fibre state and packages it into the total accumulated state.

**`f*`** (reindexing — pullback): projects current state along `f`.
`draft/OBSERVATION-INF` projections live here. Read-only, non-authority.
f* takes a total state and restricts/projects it to a concern-fibre view.

**`Π_f`** (right adjoint — dependent product): computes available actions given
current state. Ready computation, dependency-gated action spaces. Π_f takes
current state and produces the space of compatible action choices.

The unit `η : Id → f* ∘ Σ_f` embeds current state into the pullback of its own
extension: observing what you just accumulated returns the original.

The counit `ε : Σ_f ∘ f* → Id` is the stepping action: applying accumulated
evidence to produce new state.

**Adjunction of adjunctions** (nLab Prop 1.2): the triple is equivalently
`(Σ_f ⊣ f*) ⊣ (f* ⊣ Π_f)` — a second-order adjunction between the mutation
pair and the gating pair.

**Limit/colimit preservation** (nLab Note 2.1): `f*` preserves all limits and
colimits — observation preserves all structural constructions in both
directions.

## 4. Sigma: accumulated state

`Σ = Σ_{i ∈ I} Σ_i` — total accumulated state as dependent sum over concern
fibres.

Each `Σ_i` is the past-state fibre for concern `i`.

Content-addressed identity: the digest of total state is the Merkle composition
of per-fibre digests. Two states with identical digests MUST be identified.

"Uncommitted Σ" (working copy) is valid Σ not yet bound to a witness record.
It participates in the stepping monoid but does not produce witness evidence
until committed.

## 5. Reindexing: observation as pullback

`f* : Σ' → Σ` — observation/projection is the pullback along stepping morphism
`f`.

Deterministic and read-only: for fixed `f` and `Σ'`, the pullback `f*(Σ')`
MUST be unique.

`draft/OBSERVATION-INF` is the instantiation of `f*` for the
`{witnesses, issue_memory}` fibres.

Coherence: `f*` MUST commute with per-fibre projections. That is, the following
diagram commutes for all `i ∈ I`:

```text
Σ' ---f*---> Σ
 |            |
 π_i          π_i
 |            |
 v            v
Σ'_i --f*_i-> Σ_i
```

This is the naturality condition for `f*` as a natural transformation.

## 6. Pi: available actions

`Π = Π_{i ∈ I} Π_i(Σ)` — total action space as dependent product over current
Σ.

Each `Π_i(Σ)` is the set of available morphisms for concern `i` given
accumulated state. The dependency on Σ is the key dependent-type aspect: what
actions are available depends on what state has accumulated.

Cross-fibre dependencies in `I` constrain which `Π_j` fibres depend on which
`Σ_k` fibres. For example, ready computation in `Π_{issue_memory}` depends on
the dependency DAG in `Σ_{issue_memory}`.

A section of Π is a compatible choice of one action per concern fibre,
respecting all cross-fibre constraints. The space of sections is:

```text
Sections(Π) = { s : Π_{i ∈ I} Π_i(Σ) | s respects I-dependencies }
```

## 7. Derived adjoint pairs

From the triple, derive (per nLab Note 2.2):

**(a)** `f* ∘ Σ_f ⊣ f* ∘ Π_f` — the "observe-after-mutate" monad is left
adjoint to the "observe-after-choosing" comonad on Σ. The monad `f* ∘ Σ_f` is
the stepping monoid: accumulate state, then observe the result.

**(b)** `Σ_f ∘ f* ⊣ Π_f ∘ f*` — "mutate-what-you-observe" comonad,
"gate-what-you-observe" monad.

The stepping monoid `(Sections(Π), ∘, id)` acts on Σ via the counit
`ε : Σ_f ∘ f* → Id`. Associativity holds up to content-addressed equality
(§4).

**Algebra/coalgebra duality** (nLab Prop 2.6 proof): algebras of the stepping
monad `f* ∘ Σ_f` are isomorphic to coalgebras of the observation comonad
`f* ∘ Π_f`.

Interpretation: states closed under stepping are exactly the states determined
by their observations. A state that cannot be stepped further is one whose
observation fully determines it.

## 8. Fully faithful and idempotent properties

Per nLab Props 2.3, 2.6:

If `Σ_f` is fully faithful (no information loss when accumulating state), then
`Π_f` is fully faithful (actions are fully determined by state). This is the
content-addressed convergence property: faithful Σ (deterministic digests)
forces faithful Π.

If either adjunction (`Σ_f ⊣ f*` or `f* ⊣ Π_f`) is idempotent, both are.
Observing twice equals observing once forces mutation and gating idempotency.

**Final lifts** (nLab Prop 2.4): in the fully faithful case, `f*` admits final
lifts for small sinks — for any collection of steps targeting the same
observation, a canonical "weakest" step structure exists.

**Opfibration** (nLab Cor 2.5): `f*` is a Street opfibration — mutations of Σ
lift uniquely to opcartesian arrows in observation space. Every state change has
a canonical minimal observation update.

Implementations SHOULD target the fully faithful case. When `Σ_f` is not
faithful (lossy accumulation, e.g., compaction), the spec degrades gracefully
with declared faithfulness boundaries.

## 9. Content-addressed convergence

Different Π paths reaching equivalent Σ states are identified.

If `step(Σ, s1) = step(Σ, s2)` under content-addressed identity (§4), then
`s1` and `s2` are convergent from Σ.

This is not required globally — it is a property of specific path pairs.
Generalizes the beads content-hash merge pattern.

When the triple is Frobenius (`Σ_f ≅ Π_f`), mutation and action space are
identified by content hash — the ambidextrous/Wirthmüller case (nLab §3). In
this case, accumulating state and computing available actions are dual
operations on the same structure.

## 10. Cross-fibre commuting square

When a step touches fibres `i` and `j` where `j` depends on `i` in `I`, the
reindexing `f*` MUST commute with per-fibre projection.

Projecting total step to `j` then observing MUST agree with projecting to `i`,
stepping `i`, computing induced `j`-state:

```text
Σ ---step---> Σ'
|              |
π_j            π_j
|              |
v              v
Σ_j --ind_j-> Σ'_j
```

where `ind_j` is the induced `j`-step from the `i`-step via the dependency
edge `i → j` in `I`.

This generalizes `draft/CHANGE-INF` §4 to cross-fibre coherence via the
naturality of `f*`.

## 11. Descent on composed steps

A cover of a composed step by component steps satisfies descent if and only if:

1. Each component step has accepted commutation (per §10).
2. A glue witness exists: sequential application of component steps produces
   the same total Σ as the composed step.
3. The glue is contractible: the decomposition into component steps is unique
   up to content-addressed identity (§4).

This generalizes `draft/CHANGE-INF` §4.2 from single-concern to multi-concern
fibred descent. The contractibility condition ensures that there is essentially
one way to decompose a composed step, preventing ambiguity in step replay.

## 12. Instantiation contract

A concrete spec instantiating this doctrine MUST specify:

**(a)** Concern index subset: which concern identifiers from `I` this spec
addresses, and which dependency edges are relevant.

**(b)** Adjoint component(s): which component(s) of the triple this spec
addresses:
- `Σ_f` (mutation/accumulation),
- `f*` (projection/observation), or
- `Π_f` (action space/gating).

**(c)** Fibre shapes: concrete data structures for each `Σ_i` and `Π_i` fibre
the spec governs.

**(d)** Commuting-square specialization: concrete cross-fibre commuting squares
(§10) for the concern subset, with failure classes for violations.

**(e)** Content-addressed identity: concrete digest scheme for fibre state
identity.

Current instantiations:

| Spec | Adjoint component | Concern fibres |
| --- | --- | --- |
| `draft/CHANGE-INF` | `Σ_f` | `{change_log, site_topology}` |
| `draft/OBSERVATION-INF` | `f*` | `{witnesses, issue_memory}` |

Future instantiations (non-normative sketch):

| Spec | Adjoint component | Concern fibres |
| --- | --- | --- |
| Beads | Full triple | `{work_units}` |

## 13. Beads instantiation sketch

Non-normative.

- Σ fibres = content-addressed work artifacts (issues with hash identity).
- Π sections = dependency-gated task completions (ready computation).
- f* = observation projections of work-unit state.
- Convergence = content-hash merge: two stepping paths producing the same
  artifact digests are identified.
- Descent = composed chains glue when sequential and parallel produce the same
  digest.

## 14. Doctrine morphism preservation

### Doctrine Preservation Declaration (v0)

```text
preserved:
  - dm.identity
  - dm.commitment.attest
  - dm.presentation.projection
notPreserved: []
```

Preservation semantics:

- `dm.identity`: identity step is a no-op. `step(Σ, id) = Σ` for all Σ.
- `dm.commitment.attest`: `Σ_f` preserves attestation. Steps produce witness
  evidence that factors through the commitment discipline.
- `dm.presentation.projection`: `f*` preserves projection. Observation is a
  pure projection that does not introduce authority.
- `Π_f` preservation is concern-specific and delegated to instantiating specs.

## 15. Non-goals

- No concern-specific mutation vocabularies or state schemas.
- No CI/forge integration.
- No working-copy persistence or staging mechanics.
- No specific digest algorithms (only deterministic + content-addressed).
- No concurrency/parallelism models.
