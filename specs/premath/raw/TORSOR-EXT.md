---
slug: raw
shortname: TORSOR-EXT
title: workingdoge.com/premath/TORSOR-EXT
name: Torsors, Extensions, and Twist Classes
status: raw
category: Informational
tags:
  - premath
  - torsor
  - extension
  - ext
  - uct
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

This informational document defines an internal-geometry view of extension data
using torsors over context-indexed sheaf/stack objects.

It does not introduce a new admissibility checker or witness authority path.

## 2. Group objects and torsors (context-indexed)

Let `G` be an abelian group object over `(Ctx, J)` (set-level or stack-level).
Let `H` be a base object. A `G`-torsor over `H` is a map `p: E -> H` with
fiberwise free/transitive `G`-action, locally trivial on an admissible cover.

Local triviality means:

- on each cover slice `U -> H`, `E|_U` is equivalent to `G x U`.

## 3. Extensions as torsors

In the abelian setting, short exact sequence data

- `0 -> G -> E -> H -> 0`

is represented by a `G`-torsor over `H` with compatible group structure.

Extension equivalence corresponds to torsor isomorphism over `H` preserving the
kernel embedding.

## 4. `Ext` as a moduli class

`Ext(H, G)` is interpreted as equivalence classes of these torsor/extension
objects, with composition law given by the appropriate Baer/torsor sum.

Operationally this is a "twist-class" view: same base data plus an additional
extension class that may be locally trivial but globally non-canonical.

## 5. UCT-style reading (profile-level)

A useful profile pattern is a natural exact sequence of functors:

- `0 -> F -> E -> B -> 0`

where:

- `B` is base invariant data,
- `F` is extension/twist data (`Ext`-like),
- `E` is total cohomological object.

"Non-canonical split" means objectwise splitting may exist, but no global
natural section is chosen in the functor category.

This document keeps that as an interpretation profile; it is not a required
kernel claim.

## 6. Premath integration rule

If torsor/extension artifacts are used in runtime:

- they MUST enter as proposal/obligation evidence under existing checker paths,
- they MUST remain transport-natural under context refinement,
- glue/rejection outcomes MUST still be emitted through deterministic witness
  surfaces.

This keeps maximum expressiveness without increasing authority encodings.
