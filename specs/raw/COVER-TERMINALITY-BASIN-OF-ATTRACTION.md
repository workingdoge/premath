# Cover Terminality: Basin Of Attraction

## Status

This document is an active raw Premath draft.

It depends on `COVER-TERMINALITY.md`,
`COVER-TERMINALITY-FIXED-POINT.md`, `COVER-TERMINALITY-LIMIT.md`, and
`COVER-TERMINALITY-ATTRACTOR.md` and defines the next member law in that
family.

## Position in the family

`basin_of_attraction` is the first downstream relation stage linking an
attractor classifier back to an aligned source limit line.

```text
Gamma |- s : Stabilization
Gamma, s : Stabilization |- f : FixedPoint(s)
Gamma, s : Stabilization, f : FixedPoint(s) |- l : Limit(f)
Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f) |- a : Attractor(l)
Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f), a : Attractor(l)
  |- BasinOfAttraction(a, l) : Type
```

No `basin_of_attraction` term may be treated as semantically prior to the
attractor classifier and aligned source limit line on which it depends.

## Intent

`basin_of_attraction` relates one attractor classifier to the exact source
limit summaries in the aligned limit line that belong to that classifier.

It does not introduce a new witness, summary, or classifier. It exposes the
membership relation between a signature-stable attractor class and the source
limit ids that realize it.

## Relation carrier

A `basin_of_attraction` relation over `a` and `l` SHALL determine:

```text
record BasinOfAttractionRelation (a : Attractor(l), l : LimitLine) : U where
  source_attractor_id : AttractorId(a)
  terminal_signature_id : TerminalSignatureId(a)
  member_limit_ids : List(LimitId(l))
  member_limit_count : Nat
```

These names are provisional raw-spec names for the relation boundary. They may
be normalized later without changing the law defined here.

## Canonical source

`BasinOfAttraction(a, l)` is only well-formed when the attractor classifier `a`
and the source limit line `l` are aligned:

- the attractor classifier is derived from `l`,
- source profile agrees,
- context coordinates agree.

The member limits of the basin are exactly those source limit summaries in `l`
whose terminal signature equals the terminal signature carried by `a`.

## Relation law

Given `Gamma |- s : Stabilization`, `Gamma, s : Stabilization |- f :
FixedPoint(s)`, `Gamma, s : Stabilization, f : FixedPoint(s) |- l : Limit(f)`,
`Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f) |- a :
Attractor(l)`, and
`Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f), a : Attractor(l) |- b :
BasinOfAttraction(a, l)`, the relation `b` determines all of the following.

### 1. Attractor derivation

`b` is derived from the attractor classifier `a`.

```text
b.source_attractor_id = id(a)
```

### 2. Signature preservation

`b` preserves the terminal signature carried by `a`.

```text
b.terminal_signature_id = a.terminal_signature_id
```

### 3. Membership realization

`b.member_limit_ids` are exactly the ids of those source limit summaries in `l`
whose terminal signature matches `b.terminal_signature_id`.

```text
b.member_limit_ids = matchingLimitIds(b.terminal_signature_id, l)
```

### 4. Membership cardinality

`b.member_limit_count` SHALL equal the cardinality of `b.member_limit_ids`.

```text
b.member_limit_count = card(b.member_limit_ids)
b.member_limit_count > 0
```

### 5. Alignment persistence

`BasinOfAttraction(a, l)` is only valid while the attractor classifier and
source limit line remain aligned in source identity, source profile, and
context coordinates.

The basin therefore depends on that aligned pair, not on `a` alone.

### 6. Context persistence

`BasinOfAttraction(a, l)` remains in the same Premath context as its source
classifier and source limit line.

Any reindexing or context change must be explicit. A basin relation may not
silently replace the context of its source pair.

### 7. Downstream suitability

`attractor_family` may depend on the ordered basin relations as the next
downstream classifier-family stage.

It SHALL not use `b` to retroactively define the source attractor classifier or
the source limit line from which `b` is derived.

## Non-relation boundary

A term is not a valid `basin_of_attraction` relation if any of the following
holds.

- It does not depend on both a prior attractor classifier and an aligned source
  limit line.
- It fails to preserve the source attractor id.
- It fails to preserve the attractor terminal signature.
- Its member limit ids are not exactly the matching source limit ids for that
  signature.
- Its member count fails to agree with the number of member limit ids.
- It is built from a source attractor classifier and limit line that are not
  aligned.
- It is used under an untracked context change.
- It is treated as a fresh classifier rather than as a relation over an
  existing one.

This draft names the semantic non-relation boundary only. It does not yet
assign runtime failure classes.

## Relation to later stages

`attractor_family` is the next downstream stage that may group basin relations
into a higher family surface.

## Implementation boundary

This document defines relation meaning only.

It does **not** yet define:

- an executable checker algorithm,
- serialized carrier payloads,
- runtime projection rules,
- concrete failure codes.

Those belong to later carried-runtime drafts.
