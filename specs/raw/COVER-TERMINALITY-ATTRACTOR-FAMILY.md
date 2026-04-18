# Cover Terminality: Attractor Family

## Status

This document is an active raw Premath draft.

It depends on the prior `cover terminality` member-law drafts and defines the
final family-stage law in that line.

## Position in the family

`attractor_family` is the higher family stage downstream of
`basin_of_attraction`.

```text
Gamma |- s : Stabilization
Gamma, s : Stabilization |- f : FixedPoint(s)
Gamma, s : Stabilization, f : FixedPoint(s) |- l : Limit(f)
Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f) |- a : Attractor(l)
Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f), a : Attractor(l)
  |- b : BasinOfAttraction(a, l)
Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f), a : Attractor(l),
  b : BasinOfAttraction(a, l) |- AttractorFamily(b) : Type
```

No `attractor_family` term may be treated as semantically prior to the
basin-of-attraction relations on which it depends.

## Intent

`attractor_family` groups basin-of-attraction relations that share the same
realized member-limit set into one higher family object.

It does not introduce a new witness, summary, classifier, or relation. It
exposes the family shape induced by shared realized membership across prior
basin relations.

## Family carrier

An `attractor_family` object SHALL determine:

```text
record AttractorFamily (b : BasinOfAttraction(a, l)) : U where
  member_basin_ids : List(BasinId)
  member_attractor_ids : List(AttractorId)
  terminal_signature_ids : List(TerminalSignatureId)
  shared_member_limit_ids : List(LimitId)
  attractor_count : Nat
  shared_member_limit_count : Nat
```

These names are provisional raw-spec names for the family boundary. They may
be normalized later without changing the law defined here.

## Canonical source

One `attractor_family` is derived from a source basin collection by grouping
basin relations with equal `member_limit_ids`.

The canonical shared key is therefore the realized member-limit set itself.

Basins with different realized member-limit sets belong to different family
objects, even if other fields partially overlap.

## Family law

Given the upstream witness chain and
`Gamma, ... , b : BasinOfAttraction(a, l) |- g : AttractorFamily(b)`, the
family object `g` determines all of the following.

### 1. Basin-family derivation

`g` is derived from one source family of basin relations whose
`member_limit_ids` agree exactly.

All member basins of `g` therefore share the same realized member-limit set.

### 2. Shared-membership preservation

The shared member-limit ids carried by `g` SHALL equal that common realized
member-limit set.

```text
g.shared_member_limit_ids = sharedMemberLimits(g)
```

### 3. Membership aggregation

The member basin ids, attractor ids, and terminal signature ids carried by `g`
are exactly those contributed by the source basin relations in the family.

```text
g.member_basin_ids = familyBasinIds(g)
g.member_attractor_ids = familyAttractorIds(g)
g.terminal_signature_ids = familyTerminalSignatures(g)
```

### 4. Count coherence

`g.attractor_count` SHALL equal the cardinality of the member basin,
attractor, and terminal-signature lists.

`g.shared_member_limit_count` SHALL equal the cardinality of
`g.shared_member_limit_ids`.

```text
g.attractor_count = card(g.member_basin_ids)
g.attractor_count = card(g.member_attractor_ids)
g.attractor_count = card(g.terminal_signature_ids)
g.shared_member_limit_count = card(g.shared_member_limit_ids)
g.attractor_count > 0
g.shared_member_limit_count > 0
```

### 5. Context persistence

`AttractorFamily(b)` remains in the same Premath context as its source basin
relations and their upstream source chain.

Any reindexing or context change must be explicit. An attractor family may not
silently replace the context of its source family.

### 6. Closure of the current family line

`attractor_family` is the final named member in the current `cover terminality`
family draft.

Later work may derive further doctrines from it, but such work is not part of
the currently named family line unless added by an explicit replacement draft.

## Non-family boundary

A term is not a valid `attractor_family` object if any of the following holds.

- It does not depend on prior basin-of-attraction relations.
- It groups basin relations with different realized `member_limit_ids`.
- Its member basin, attractor, and terminal-signature lists do not agree in
  cardinality.
- Its attractor count fails to agree with those member lists.
- Its shared member-limit count fails to agree with the shared member-limit id
  list.
- It is used under an untracked context change.
- It is treated as a fresh classifier or relation rather than as a higher
  family object over existing basin relations.

This draft names the semantic non-family boundary only. It does not yet assign
runtime failure classes.

## Implementation boundary

This document defines higher-family meaning only.

It does **not** yet define:

- an executable checker algorithm,
- serialized carrier payloads,
- runtime projection rules,
- concrete failure codes.

Those belong to later carried-runtime drafts.
