# Atlas: Site Of Sites

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Atlas is the staged site-of-sites model for cross-repo placement and seam
coherence.

Atlas does not own local semantics. It owns the map of sites, authority
surfaces, seams, covers, and descent obligations that make a multi-site slice
coherent.

The motivating failure mode is visible today:

- Premath carries checker/kernel content, but also started carrying placement
  doctrine.
- Tusk carries operator workflow topology, but also carries general placement
  rules.
- Nerve carries simplex substrate semantics that other repos need, but those
  consumers do not yet share one explicit cover contract.

Atlas exists to prevent those roles from collapsing into each other.

## 2. Non-Goals

Atlas is not:

- a replacement for Nerve simplex semantics,
- a replacement for Premath kernel, law, checker, or checker surfaces,
- a replacement for Tusk runtime, tracker, daemon, MCP, CLI, or UI behavior,
- a reusable executable carrier implementation,
- a downstream product or proof repo.

Atlas is only the cross-site placement and gluing calculus.

## 3. Core Objects

### 3.1 `SemanticSite`

A `SemanticSite` is a bounded authority host such as `nerve`, `premath`,
`tusk`, `kurma`, `bridge`, `home`, or `aac`.

A site may host many documents and tools, but Atlas only cares about the
authority surfaces the site is allowed to decide.

### 3.2 `AuthoritySurface`

An `AuthoritySurface` is a named family of claims a site is allowed to decide.

Examples:

- `simplex.substrate`
- `premath.kernel.law`
- `premath.work_checker.admissibility_check`
- `tusk.operator.instrument`
- `kurma.carriage.method`

An authority surface must have one owner in a given cover.

### 3.3 `ProjectionSurface`

A `ProjectionSurface` is a derived view over authority.

Examples:

- ready/blocked boards,
- dependency graphs,
- dashboards,
- issue lists,
- command transcripts,
- status summaries.

Projection surfaces may be operationally useful, but they must not authorize
mutation or redefine law.

### 3.4 `SiteMorphism`

A `SiteMorphism` is a typed seam from one site surface to another.

Examples:

- `simplex.substrate -> work.semantic_state`
- `work.semantic_state -> premath.work_checker.input_nf`
- `premath.work_checker.decision -> tusk.tracker_instrument`
- `premath.checker.witness -> tusk.receipt.projection`
- `bridge.secret_domain.contract -> tusk.bridge_adapter.compatibility`

Every morphism should declare:

- source site and surface,
- target site and surface,
- what is transported,
- what is forbidden to transport,
- verification obligation.

### 3.5 `SiteCover`

A `SiteCover` is a finite set of sites and morphisms required for one coherent
architecture slice.

A cover is the right object for multi-repo work. It prevents a feature from
being assigned to one repo just because one repo currently hosts the visible
tool.

### 3.6 `DescentCondition`

A `DescentCondition` is the obligation that makes the cover coherent.

Typical obligations:

- authority does not leak into projections,
- instrument layers do not define law,
- law layers do not smuggle runtime policy,
- substrate vocabulary is referenced rather than redefined,
- compatibility adapters cannot become canonical domain contracts,
- all boundary inputs used by a higher-level claim are accepted under their
  owner sites.

### 3.7 `GlobalSection`

A `GlobalSection` is an accepted implementation, proof, or design slice over a
cover.

In practical terms, a global section says:

```text
this cross-site feature is coherent under the declared cover and descent
conditions
```

If no global section exists, the result is topology drift, even when each repo
looks locally reasonable.

## 4. Shape Layers

Atlas uses the shape layers from `TOPOLOGY-V2.md`:

| Layer | Role | Typical host |
| --- | --- | --- |
| substrate | primitive geometry/state language | `nerve` |
| law | admissibility, contracts, witnesses, failure classes | `premath` |
| carriage | reusable replay, normalization, verification method | `kurma` |
| instrument | operator/runtime tool surface | `tusk` |
| projection | derived human/product/query views | `tusk` or downstream |
| proof | live domain instance under real constraints | downstream repo |

These layers are not repositories. They are roles that repositories may host.

## 5. Correctness Rules

1. One cover must have one owner for each authority surface.
2. A projection surface must not authorize mutation.
3. An instrument surface must not define substrate or law.
4. A law surface must not define runtime deployment policy.
5. A compatibility adapter must not become the canonical domain contract.
6. A site morphism must name what is transported and what is forbidden.
7. A global section must cite the cover and descent condition it satisfies.
8. Drift is failure to produce an accepted global section, not merely textual
   disagreement between docs.

## 6. Worked Example: Simplex-Native Work Tracker

The simplex-native work tracker is not owned by one repo. It is a cover.

```text
Cover work-tracker.v0 = {
  simplex,
  work,
  premath,
  tusk
}
```

Morphism sketch:

```text
simplex.substrate
  -> work.semantic_state
  -> premath.work_checker
  -> tusk.tracker_instrument
  -> tusk/downstream.projections
```

Authority split:

| Surface | Owner |
| --- | --- |
| simplex, boundary, patch, local-to-global vocabulary | candidate `simplex` site; provisionally `nerve` |
| work-state meaning, acceptance, verification, handoff/session semantics | candidate `work` site; no Premath host; current Premath surface is checker-only |
| `WorkClaimNF`, admissibility decision, failure classes, checker vectors | `premath` checker surface; see `specs/premath/raw/WORK-TRACKER-CHECKER-PROFILE.md` |
| CLI/MCP/daemon/UI, worker loop, repo binding, compatibility adapter | `tusk` |
| ready/blocked/dependency graph/board views | projection surfaces |
| `bd` import/export | compatibility projection |

Descent conditions:

- `tusk` may expose `claim`, `close`, `discover`, and lease operations as
  tracker-owned instruments that can emit normalized Premath acceptances.
- ready/blocked/dependency graph views are derived projections, not authority.
- `bd` state can seed or compare tracker state but cannot define the
  simplex-native authority object.
- Premath may define work-tracker checker rules but must reference simplex
  substrate vocabulary and candidate work semantics; it must not own
  work-state meaning.
- Nerve is a provisional host for reusable simplex/patch vocabulary; Nerve's
  protocol-specific coding stack is not part of the generic work-tracker cover
  unless explicitly added.

A valid work-tracker implementation is a global section over `work-tracker.v0`.

## 7. Existing Sources And Destination

| Current source | Atlas classification | Destination |
| --- | --- | --- |
| `docs/design/control-plane/TOPOLOGY-V2.md` | Premath-local staging note for shape-layer routing | move essence into Atlas; leave Premath pointer |
| Tusk `design/notes/tusk-workflow-topology.md` | instrument-local workflow topology | keep in Tusk; reference Atlas for placement doctrine |
| Tusk `design/notes/tusk-topology-reconciliation.md` | repo-scoped placement audit | migrate general placement rules into Atlas; keep Tusk-specific decisions local |
| Nerve draft simplex specs | substrate authority | keep in Nerve; Atlas references, not copies |
| Premath kernel/checker/claim specs | checker authority | keep in Premath |

This table is a migration map, not a move command.

## 8. Atlas Site

`fish/sites/atlas` is now the intended durable home for site-of-sites placement
and cover doctrine.

This document remains a Premath staging pointer until the stable content is
fully moved and Premath keeps only checker/kernel-specific references.

## 9. First Promotion Path

1. Keep the dedicated Atlas site small and placement-only.
2. Move the stable parts of this document and `TOPOLOGY-V2.md` there.
3. Replace Premath topology text with pointers plus Premath-specific checker
   boundaries.
4. Replace Tusk general placement text with pointers plus Tusk-specific
   instrument/runtime boundaries.
5. Use `fish/sites/atlas/specs/raw/ATLAS-0002-WORK-TRACKER-COVER.md` as the
   first worked site cover (`work-tracker.v0`).
6. Use `fish/sites/atlas/specs/raw/ATLAS-0003-SIMPLEX-FACTOR-CANDIDATE.md` as
   the extraction test before creating any durable `simplex` site.

## 10. Verification

This design note is coherent if:

- Premath remains the owner of checker/kernel surfaces only,
- Tusk remains the owner of instrument/projection runtime only,
- Nerve remains only the provisional host for reusable simplex/patch vocabulary
  until a `simplex` site is factored, and Nerve-specific protocol/coding
  semantics are not imported into generic work tracking,
- candidate `work` semantics are not collapsed into Tusk instrumentation or
  Premath checker implementation,
- Atlas is the placement owner for cross-site placement and gluing rules.
