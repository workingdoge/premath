---
slug: draft
shortname: PREMATH-KERNEL
title: workingdoge.com/premath/PREMATH-KERNEL
name: Premath Kernel (Definability by Contractible Descent)
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - definability
  - descent
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

## Statement IDs (normative indexing surface)

Kernel clauses intended for machine binding SHOULD carry stable statement IDs.
IDs are indexing metadata only and MUST NOT alter semantic meaning.

ID form:

- `KERNEL.DEF.*` for definitions.
- `KERNEL.AX.*` for axioms/laws.
- `KERNEL.REQ.*` for explicit requirements.
- `KERNEL.REJ.*` for rejection criteria.

Stability rule:

- IDs SHOULD remain stable across wording-only edits.
- IDs MUST change only when the indexed semantic claim changes.

## 1. Overview

Premath is the kernel doctrine of definability: a notion is admissible exactly when it is
**stable under context change** and **glues uniquely from locally compatible data**.

Premath is **ontology-agnostic**: it does not prescribe what definables *are* (sets,
groupoids, ∞-groupoids, …). It only prescribes how they MUST **behave** under reindexing
and descent.

Informative note:

- A canonical model of this kernel in higher topos theory is described in
  `raw/SEMANTICS-INFTOPOS`.
- Stronger “final dial turn” variants (hyperdescent, universes/comprehension) are OPTIONAL
  extensions and are specified separately (`raw/HYPERDESCENT`, `raw/UNIVERSE`).

### 1.1 The Premath shape (informative)

Premath’s semantic kernel can be viewed in the classical *fibre space* / *fibration* style:

- a **site** `(𝒞, Cov)` of contexts and covers, and
- a **total space** `E` of definables-in-context,

with projection:

- `p₀ : E → 𝒞` (“forget the definable, keep the context”).

Concretely, giving an indexed assignment `Def : 𝒞^{op} → 𝒱` (a pseudo/∞-functor, depending on the
chosen coherence level `𝒱`) is equivalent (via the **Grothendieck construction**) to giving a
Grothendieck fibration (a “fibre space” in Grothendieck’s Kansas notes) `p₀ : ∫ Def → 𝒞` whose fiber
over `Γ` is `Def(Γ)`.

- Reindexing `f^*` is induced by cartesian liftings in the fibration.
- Choosing a *split cleavage* corresponds to a strict implementation presentation (see
  `raw/SPLIT-PRESENTATION`).

Implementations additionally choose an **external host base** `B` in which Premath meanings are
realized and/or checked (for example: a typechecker/proof-kernel world, an ∞-cosmos world, etc.).
We treat “typechecker” and “∞-cosmos” as *examples of external host bases `B`*; whether `B` is
presented as a “type”, “term”, or other meta-object is an implementation detail and not part of
this kernel.

A standard realization picture is the commuting square:

\[
\begin{matrix}
E & \xrightarrow{F} & E_B \\
\downarrow p_0 & & \downarrow p_B \\
\mathcal C & \xrightarrow{f} & B
\end{matrix}
\]

where `E_B → B` is a Premath-shaped bundle over the host and `F` preserves kernel meaning
(reindexing/descent/refinement) inside the host.

### 1.2 Kernel axioms as obligations (informative compilation boundary)

The kernel is semantic. To *operate* it in a host, implementations typically compile semantic
requirements into **finite, explicit obligations** that can be checked and discharged.

Canonical obligation kinds include:

- Reindexing coherence: unit and composition.
- Descent coherence: overlap compatibility and cocycle laws.
- Contractible gluing: effective descent.
- Refinement invariance: agreement under cover refinement.

The recommended operational interface is specified in `draft/BIDIR-DESCENT`, with deterministic
interop support specified in `draft/NF`, `draft/NORMALIZER`, and `draft/REF-BINDING`.

Repository conformance additionally exports a canonical obligation-authority
registry from kernel code (`crates/premath-kernel/src/obligation_registry.rs`)
and machine output (`premath obligation-registry --json`). Downstream layers
MUST treat that export as read-only authority for obligation->Gate mapping.

## 2. Ambient coherence level

Premath is parameterized by an ambient “sameness level” \(\mathcal V\), e.g.

\[
\mathcal V \in \{\mathbf{Set},\ \mathbf{Gpd},\ \mathbf{Cat},\ \mathcal S_\infty,\ \ldots\}.
\]

Implementations MUST choose an ambient level \(\mathcal V\) and interpret sameness \(\approx\)
as equivalence in \(\mathcal V\).

- [KERNEL.REQ.AMBIENT.1] Implementations MUST choose an ambient level
  \(\mathcal V\) and interpret sameness \(\approx\) as equivalence in
  \(\mathcal V\).
- In \(\mathbf{Set}\): \(\approx\) is equality.
- In \(\mathbf{Gpd}/\mathbf{Cat}\): \(\approx\) is isomorphism.
- In \(\mathcal S_\infty\): \(\approx\) is equivalence.

\(\equiv\) is RESERVED for definitional/algorithmic equality in an implementation.

## 3. Contexts and covers

A Premath world MUST provide:

1. [KERNEL.DEF.CTX.1] **Contexts:** a category \(\mathcal C\) with objects
   \(\Gamma\) and morphisms \(f:\Gamma'\to\Gamma\).
2. [KERNEL.DEF.COV.1] **Covers:** a coverage / Grothendieck pretopology
   \(\mathrm{Cov}\).

- [KERNEL.DEF.COVER.1] A cover of \(\Gamma\) is a family
  \(U=\{u_i:\Gamma_i\to\Gamma\}\triangleright\Gamma\).

- [KERNEL.REQ.PULLBACK.1] Implementations MUST make pullbacks of covering maps
  available so overlaps \(\Gamma_i\times_\Gamma\Gamma_j\) exist for descent
  checks.

## 4. Indexed definables

A Premath world MUST provide an indexed assignment

\[
\mathrm{Def}:\mathcal C^{op}\to\mathcal V
\]

together with reindexing (pullback) maps \(f^*\) for each morphism \(f\).

- [KERNEL.DEF.INDEXED.1] A Premath world MUST provide indexed assignment
  \(\mathrm{Def}:\mathcal C^{op}\to\mathcal V\) with reindexing maps \(f^*\).
- [KERNEL.REQ.INDEXED_COHERENCE.1] **Indexed coherence is REQUIRED:**
  \(\mathrm{Def}\) MUST be a pseudo/∞-functor appropriate to \(\mathcal V\).
  Reindexing coherence is structure, not optional.

## 5. Stability (reindexing coherence)

For every morphism \(f:\Gamma'\to\Gamma\) and \(g:\Gamma''\to\Gamma'\), and every
\(A\in\mathrm{Def}(\Gamma)\):

- [KERNEL.AX.STABILITY_UNIT.1] **Unit:** \((id_\Gamma)^*A \approx A\)
- [KERNEL.AX.STABILITY_COMPOSITION.1] **Composition:**
  \((f\circ g)^*A \approx g^*(f^*A)\)

with the standard coherence laws demanded by \(\mathcal V\).

## 6. Locality (restriction)

- [KERNEL.REQ.LOCALITY.1] For every cover
  \(U=\{u_i\}\triangleright\Gamma\), restrictions \(u_i^*A\) MUST exist.

## 7. Descent data

- [KERNEL.DEF.DESCENT_DATUM.1] A descent datum over
  \(U\triangleright\Gamma\) consists of:

- local definables \(A_i\in\mathrm{Def}(\Gamma_i)\), and
- overlap compatibilities \(\phi_{ij}:p_1^*A_i\approx p_2^*A_j\)

satisfying cocycle coherence on triple overlaps (in the sense of \(\mathcal V\)).

- [KERNEL.DEF.DESCENT_OBJECT.1] Let \(\mathrm{Desc}_U(\Gamma)\) denote the
  \(\mathcal V\)-object of descent data, and
  \(\mathrm{res}_U:\mathrm{Def}(\Gamma)\to\mathrm{Desc}_U(\Gamma)\) the
  restriction map.

## 8. Coherence axiom: contractible descent

- [KERNEL.AX.CONTRACTIBLE_DESCENT.1] **Contractible Descent is REQUIRED:** for
  every cover \(U\triangleright\Gamma\), \(\mathrm{res}_U\) MUST be an
  equivalence in \(\mathcal V\).

- [KERNEL.AX.GLUE_FIBER.1] Equivalently, for each datum
  \(d\in\mathrm{Desc}_U(\Gamma)\), the homotopy fiber
  \(\mathrm{Glue}(d):=\mathrm{fib}_d(\mathrm{res}_U)\) MUST be contractible.

This axiom specializes as:

- sheaf condition when \(\mathcal V=\mathbf{Set}\),
- stack condition when \(\mathcal V=\mathbf{Gpd}\),
- higher stack condition when \(\mathcal V=\mathcal S_\infty\).

## 9. Refinement closure

- [KERNEL.AX.REFINEMENT_CLOSURE.1] Let \(J\) be the Grothendieck topology
  generated by \(\mathrm{Cov}\). Premath descent MUST hold for all \(J\)-covers,
  not just generating families.

## 10. Rejection criteria

A notion is NOT Premath-admissible if it is:

- [KERNEL.REJ.NON_STABLE.1] non-stable (fails reindexing coherence),
- [KERNEL.REJ.NON_LOCAL.1] non-local (cannot restrict to covers),
- [KERNEL.REJ.NON_GLUABLE.1] non-gluable (descent existence fails),
- [KERNEL.REJ.NON_UNIQUE.1] non-unique (glue-space non-contractible),
- [KERNEL.REJ.REFINEMENT_SENSITIVE.1] refinement-sensitive (meaning changes
  under refinement without \(\approx\)-identification).

## 11. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.refine.context`
- `dm.refine.cover`
- `dm.policy.rebind` (only with explicit new run boundary and rebinding evidence)

Not preserved:

- `dm.transport.world` (handled by `raw/SQUEAK-CORE`)
- `dm.transport.location` (handled by `raw/SQUEAK-SITE`)
- `dm.profile.execution` (handled by runtime/CI layer)
- `dm.presentation.projection` (handled by projection layer)
