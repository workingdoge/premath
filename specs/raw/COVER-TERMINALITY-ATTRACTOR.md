# Cover Terminality: Attractor

## Status

This document is an active raw Premath draft.

It depends on `COVER-TERMINALITY.md`,
`COVER-TERMINALITY-FIXED-POINT.md`, and `COVER-TERMINALITY-LIMIT.md`
and defines the next member law in that family.

## Position in the family

`attractor` is the first classifier stage downstream of `limit`.

```text
Gamma |- s : Stabilization
Gamma, s : Stabilization |- f : FixedPoint(s)
Gamma, s : Stabilization, f : FixedPoint(s) |- l : Limit(f)
Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f) |- Attractor(l) : Type
```

No `attractor` term may be treated as semantically prior to the `limit`
summaries on which it depends.

## Intent

`attractor` classifies the ordered terminal summaries carried by `limit` into
terminal-signature classes.

It does not introduce a new terminal witness or a new terminal summary. It
exposes one classifier object for a signature-stable family of source limit
summaries.

## Classifier carrier

An `attractor` classifier over `l` SHALL determine:

```text
record AttractorClassifier (l : Limit(f)) : U where
  terminal_signature_id          : TerminalSignatureId(l)
  first_limit_at_convergence_depth : Nat
  last_limit_at_convergence_depth  : Nat
  stabilized_class_count           : Nat
  total_terminal_count             : Nat
  total_member_count               : Nat
  max_terminal_cover_count         : Nat
  member_limit_count               : Nat
```

These names are provisional raw-spec names for the classifier boundary. They
may be normalized later without changing the law defined here.

## Canonical source

For a source limit line `l`, each source limit summary carries a terminal
signature determined by:

- `stabilized_class_count`
- `total_terminal_count`
- `total_member_count`
- `max_terminal_cover_count`

An `attractor` classifier is derived from one signature-stable member family of
source limit summaries.

Multiple attractor classifiers may therefore be derived from one limit line
when that line contains multiple terminal signatures.

## Classifier law

Given `Gamma |- s : Stabilization`, `Gamma, s : Stabilization |- f :
FixedPoint(s)`, `Gamma, s : Stabilization, f : FixedPoint(s) |- l : Limit(f)`,
and `Gamma, s : Stabilization, f : FixedPoint(s), l : Limit(f) |- a :
Attractor(l)`, the classifier `a` determines all of the following.

### 1. Signature derivation

`a` is derived from one signature-stable member family inside `l`.

All source limit summaries classified by `a` share `a.terminal_signature_id`.

### 2. Signature coherence

The summary carried by `a` SHALL agree with the terminal signature it
classifies.

```text
a.terminal_signature_id
  = signature(
      a.stabilized_class_count,
      a.total_terminal_count,
      a.total_member_count,
      a.max_terminal_cover_count,
    )
```

### 3. Depth-span coherence

The depth span carried by `a` SHALL cover the member limit summaries classified
by `a`.

```text
a.first_limit_at_convergence_depth = minDepth(memberLimits(a, l))
a.last_limit_at_convergence_depth = maxDepth(memberLimits(a, l))
a.first_limit_at_convergence_depth <= a.last_limit_at_convergence_depth
```

### 4. Membership cardinality

`a.member_limit_count` SHALL equal the cardinality of the source limit
summaries classified by `a`.

```text
a.member_limit_count = card(memberLimits(a, l))
a.member_limit_count > 0
```

### 5. Source-summary preservation

The source limit summaries classified by `a` SHALL preserve the same:

- `stabilized_class_count`
- `total_terminal_count`
- `total_member_count`
- `max_terminal_cover_count`

The classifier therefore preserves the terminal summary invariants already
carried by the source limit summaries while adding classification structure.

### 6. Context persistence

`Attractor(l)` remains in the same Premath context as its source limit line and
upstream witness chain.

Any reindexing or context change must be explicit. An attractor classifier may
not silently replace the context of its source summaries.

### 7. Downstream suitability

`basin_of_attraction` may depend on `a` together with its source limit line as
the next downstream classifier relation.

It SHALL not use `a` to retroactively define the source limit summaries from
which `a` is derived.

## Non-classifier boundary

A term is not a valid `attractor` classifier if any of the following holds.

- It does not depend on a prior limit line.
- It mixes source limit summaries with different terminal signatures.
- Its carried signature fails to agree with its summary fields.
- Its depth span fails to cover the member limit summaries it classifies.
- Its member count fails to agree with the classified source family.
- It is used under an untracked context change.
- It is treated as a fresh witness or summary rather than as a classifier over
  source limit summaries.

This draft names the semantic non-classifier boundary only. It does not yet
assign runtime failure classes.

## Relation to later stages

`basin_of_attraction` is the next downstream stage that may relate attractor
classifiers back to the wider limit line.

`attractor_family` remains a follow-up draft and is not defined here.

## Implementation boundary

This document defines classifier meaning only.

It does **not** yet define:

- an executable checker algorithm,
- serialized carrier payloads,
- runtime projection rules,
- concrete failure codes.

Those belong to later carried-runtime drafts.
