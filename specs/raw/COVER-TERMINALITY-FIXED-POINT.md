# Cover Terminality: Fixed Point

## Status

This document is an active raw Premath draft.

It depends on `COVER-TERMINALITY.md` and defines the first member law in that
family.

## Position in the family

`fixed_point` is a dependent witness stage.

```text
Gamma |- s : Stabilization
Gamma, s : Stabilization |- FixedPoint(s) : Type
```

No `fixed_point` term may be treated as semantically prior to the
`stabilization` witness on which it depends.

## Intent

`fixed_point` is the first bounded terminal witness in the cover terminality
family.

It turns the ordered stabilization progression into one terminal handoff object
that later stages may depend on without replaying the full progression.

## Witness carrier

A `fixed_point` witness over `s` SHALL determine:

```text
record FixedPointWitness (s : Stabilization) : U where
  terminal_level_id          : StabilizationLevelId(s)
  fixed_at_convergence_depth : Nat
  stabilized_class_count     : Nat
  total_terminal_count       : Nat
  total_member_count         : Nat
  max_terminal_cover_count   : Nat
```

These names are provisional raw-spec names for the witness boundary. They may
be normalized later without changing the law defined here.

## Canonical source

For each ordered stabilization witness `s`, there is a distinguished terminal
stabilization level `terminalLevel(s)`.

`fixed_point` witnesses SHALL be extracted from `terminalLevel(s)`, not from an
arbitrary earlier stabilization level.

## Witness law

Given `Gamma |- s : Stabilization` and `Gamma, s : Stabilization |- f :
FixedPoint(s)`, the witness `f` certifies all of the following.

### 1. Terminal derivation

`f` is derived from the terminal stabilization level of `s`.

```text
f.terminal_level_id = id(terminalLevel(s))
```

### 2. Bounded terminality

`f` certifies that the cover line represented by `s` has reached a bounded
terminal witness at `f.fixed_at_convergence_depth`.

This law does not yet define a global finality claim outside that bounded cover
line.

### 3. Summary coherence

The summary carried by `f` SHALL agree with the terminal stabilization level of
`s`.

```text
f.fixed_at_convergence_depth = up_to_convergence_depth(terminalLevel(s))
f.stabilized_class_count = stabilized_class_count(terminalLevel(s))
f.total_terminal_count = cumulative_terminal_count(terminalLevel(s))
f.total_member_count = cumulative_member_count(terminalLevel(s))
f.max_terminal_cover_count = cumulative_max_terminal_cover_count(terminalLevel(s))
```

### 4. Context persistence

`FixedPoint(s)` remains in the same Premath context as `s`.

Any reindexing or context change must be explicit. A `fixed_point` witness may
not silently replace the context of its source stabilization witness.

### 5. Handoff suitability

Later members of the cover terminality family may depend on `f` as a terminal
witness handoff object.

They SHALL not retroactively define the terminality of `f`; they consume it.

## Non-witness boundary

A term is not a valid `fixed_point` witness if any of the following holds.

- It does not depend on a prior stabilization witness.
- It is derived from a non-terminal or non-distinguished stabilization level.
- Its carried summary fails to agree with the terminal stabilization level.
- It is used under an untracked context change.
- It is treated as semantically sufficient without the source stabilization
  dependence it requires.

This draft names the semantic non-witness boundary only. It does not yet assign
runtime failure classes.

## Relation to later stages

`limit` is the first downstream terminal-summary stage that may summarize from a
`fixed_point` witness.

`attractor`, `basin_of_attraction`, and `attractor_family` remain follow-up
drafts and are not defined here.

## Implementation boundary

This document defines witness meaning only.

It does **not** yet define:

- an executable checker algorithm,
- serialized carrier payloads,
- runtime projection rules,
- concrete failure codes.

Those belong to later carried-runtime drafts.
