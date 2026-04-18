# Cover Terminality

## Status

This document is an active raw Premath draft.

It defines the family shape of one terminal cover line. It does not yet define
the full theorem package for every member of that family.

## Purpose

Premath already has the kernel language needed to speak about:

- reindexing under context change,
- admissible covers,
- contractible descent.

This draft names one higher family built over that kernel:

> **cover terminality**
>
> the ordered family of constructions that tracks convergence toward a
> stabilized terminal witness and then classifies the structure induced by that
> witness.

The purpose of this draft is to fix the family shape and dependency order
before stage-specific proof laws are written.

## Family members

The family is:

```text
convergence
  -> stabilization
    -> fixed_point
      -> limit
        -> attractor
          -> basin_of_attraction
            -> attractor_family
```

This order is normative for the draft family shape.

## Dependency discipline

The family SHALL be read in this order:

1. `convergence`
2. `stabilization`
3. `fixed_point`
4. `limit`
5. `attractor`
6. `basin_of_attraction`
7. `attractor_family`

No later member may be treated as semantically prior to an earlier one without
an explicit replacement draft.

## Role split inside the family

The family contains three different kinds of members.

### 1. Progressive source stage

`convergence` is the source progression stage. It records the approach toward
stable classes under increasing cover depth.

### 2. Witness stages

`stabilization` and `fixed_point` are witness-bearing stages.

- `stabilization` records the progressive evidence that terminal behavior is
  settling.
- `fixed_point` is the first bounded terminal witness extracted from that
  progression.

`fixed_point` is therefore the first useful terminal witness in the family, but
it is not the whole family.

### 3. Derived terminal-summary stages

`limit`, `attractor`, `basin_of_attraction`, and `attractor_family` are
terminal-summary or terminal-induced classifier stages.

They depend on the terminal witness side of the family and should not be used
to retroactively define that witness.

## Why `fixed_point` is first

`fixed_point` is the first stage that turns open-ended stabilization data into
one bounded terminal witness.

That matters because it gives the family:

- a stopping point,
- a canonical handoff object for later stages,
- a clean first target for witness-law and checker work.

The later stages may still be semantically richer, but they are not a better
first witness boundary.

## Ownership boundary

For this family:

- Premath owns the semantic family shape and witness meaning.
- Carried runtimes own later executable witness verification and
  interpretation.

This draft therefore defines family order and role, not runtime checker
behavior.

## Law boundary

This document does **not** yet define:

- the `fixed_point` member law inline; see
  `COVER-TERMINALITY-FIXED-POINT.md` for the first member-law draft,
- the `limit` member law inline; see `COVER-TERMINALITY-LIMIT.md` for the
  first downstream terminal-summary draft,
- the theorem packages for `attractor`, `basin_of_attraction`, or
  `attractor_family`,
- executable witness validation rules.

Those belong in follow-up drafts once the family shape is fixed.

## Informative framing

This family can be read profitably as a dependent ladder of witness families:

- later members depend on earlier witness-bearing stages,
- reindexing/context change remains explicit,
- semantic family shape is kept separate from executable interpretation.

Useful framing references:

- nLab, `Categorical semantics of dependent type theory`
- Castellan, Clairambault, Dybjer, `Categories with Families: Unityped, Simply Typed, and Dependently Typed`
- Hofmann, `Syntax and Semantics of Dependent Types`
- Jacobs, `Categorical Logic and Type Theory`

These references are informative only. They do not replace the Premath kernel
or this draft's own authority.
