# Cover Terminality: Limit

## Status

This document is an active raw Premath draft.

It depends on `COVER-TERMINALITY.md` and
`COVER-TERMINALITY-FIXED-POINT.md` and defines the next member law in that
family.

## Position in the family

`limit` is the first downstream terminal-summary stage.

```text
Gamma |- s : Stabilization
Gamma, s : Stabilization |- f : FixedPoint(s)
Gamma, s : Stabilization, f : FixedPoint(s) |- Limit(f) : Type
```

No `limit` term may be treated as semantically prior to the `fixed_point`
witness on which it depends.

## Intent

`limit` turns a prior fixed-point witness into a terminal summary object that
later stages may consume directly.

`limit` does not re-certify terminality. It preserves and re-exposes the
bounded terminal summary already certified by `fixed_point`.

## Summary carrier

A `limit` summary over `f` SHALL determine:

```text
record LimitSummary (f : FixedPoint(s)) : U where
  source_fixed_point_id     : FixedPointId(f)
  limit_at_convergence_depth : Nat
  stabilized_class_count     : Nat
  total_terminal_count       : Nat
  total_member_count         : Nat
  max_terminal_cover_count   : Nat
```

These names are provisional raw-spec names for the summary boundary. They may
be normalized later without changing the law defined here.

## Canonical source

For each fixed-point witness `f`, there is a distinguished source summary given
by the terminal data already carried by `f`.

`limit` summaries SHALL be derived from that fixed-point summary and SHALL keep
their dependence on `f` explicit.

## Summary law

Given `Gamma |- s : Stabilization`, `Gamma, s : Stabilization |- f :
FixedPoint(s)`, and `Gamma, s : Stabilization, f : FixedPoint(s) |- l :
Limit(f)`, the summary `l` certifies all of the following.

### 1. Fixed-point derivation

`l` is derived from the fixed-point witness `f`.

```text
l.source_fixed_point_id = id(f)
```

### 2. Terminal-summary preservation

`l` preserves the bounded terminal summary carried by `f`.

`limit` therefore depends on prior terminality certification instead of
replacing it.

### 3. Summary coherence

The summary carried by `l` SHALL agree with the summary already carried by `f`.

```text
l.limit_at_convergence_depth = f.fixed_at_convergence_depth
l.stabilized_class_count = f.stabilized_class_count
l.total_terminal_count = f.total_terminal_count
l.total_member_count = f.total_member_count
l.max_terminal_cover_count = f.max_terminal_cover_count
```

### 4. Context persistence

`Limit(f)` remains in the same Premath context as `f` and its source
stabilization witness.

Any reindexing or context change must be explicit. A `limit` summary may not
silently replace the context of its source witness chain.

### 5. Downstream suitability

Later members of the cover terminality family may depend on `l` as the first
downstream terminal-summary object.

They SHALL not use `l` to retroactively define the fixed-point witness from
which it is derived.

## Non-summary boundary

A term is not a valid `limit` summary if any of the following holds.

- It does not depend on a prior fixed-point witness.
- It fails to preserve the source fixed-point identity.
- Its carried summary fails to agree with the fixed-point summary.
- It is used under an untracked context change.
- It is treated as a fresh terminal witness rather than as a downstream summary
  of one.

This draft names the semantic non-summary boundary only. It does not yet assign
runtime failure classes.

## Relation to later stages

`attractor` is the next downstream stage that may classify structure induced by
the terminal summary carried by `limit`.

`basin_of_attraction` and `attractor_family` remain follow-up drafts and are
not defined here.

## Implementation boundary

This document defines downstream terminal-summary meaning only.

It does **not** yet define:

- an executable checker algorithm,
- serialized carrier payloads,
- runtime projection rules,
- concrete failure codes.

Those belong to later carried-runtime drafts.
