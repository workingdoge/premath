---
slug: draft
shortname: SIGPI-SITE
title: workingdoge.com/premath/SIGPI-SITE
name: SigPi Adjoint Triple Site Instantiation
status: draft
category: Standards Track
tags:
  - premath
  - site
  - sigpi
  - adjoint
  - stepping
  - doctrine
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

This spec instantiates `draft/SIGPI-INF` for premath's concrete concern
fibres.

Dependencies:

- `draft/SIGPI-INF` (parent adjoint triple)
- `draft/DOCTRINE-SITE` (operation topology)
- `draft/DOCTRINE-INF` (morphism preservation)
- `draft/CHANGE-INF` (Σ_f instantiation)
- `draft/OBSERVATION-INF` (f* instantiation)

This spec is normative when `capabilities.sigpi_stepping` is claimed.

## 2. Concern fibre registry

Concrete concern identifiers for this site:

```text
issue_memory
witnesses
change_log
site_topology
working_copy
observation_surfaces
```

Dependency DAG (edges denote "depends on"):

```text
issue_memory → witnesses
change_log → site_topology
working_copy → issue_memory
working_copy → change_log
observation_surfaces → witnesses
observation_surfaces → issue_memory
```

The DAG MUST be acyclic. The registry is extensible: new concern identifiers
MAY be added via the registry protocol, provided the resulting DAG remains
acyclic.

## 3. Sigma decomposition

Concrete `Σ_i` shapes per fibre:

| Fibre | Shape | Identity |
| --- | --- | --- |
| `issue_memory` | JSONL store keyed by issue ID | Content-addressed via `content_hash` / `structure_hash` |
| `witnesses` | Directory indexed by `(source_id, run_id)` | Directory listing digest |
| `change_log` | JSONL digest chain | Chain head digest |
| `site_topology` | `SITE-PACKAGE.json` digest | Package digest |
| `working_copy` | Staged-but-uncommitted mutations across all fibres | Working copy digest |
| `observation_surfaces` | Latest observation surface digest | Surface digest |

Total Σ digest = Merkle composition over per-fibre digests in lexicographic
concern order:

```text
digest(Σ) = SHA256(
  digest(Σ_{change_log}) ||
  digest(Σ_{issue_memory}) ||
  digest(Σ_{observation_surfaces}) ||
  digest(Σ_{site_topology}) ||
  digest(Σ_{witnesses}) ||
  digest(Σ_{working_copy})
)
```

## 4. Reindexing instantiation

Concrete `f*_i` surfaces per fibre:

| Fibre | Reindexing surface | Source |
| --- | --- | --- |
| `issue_memory` | QueryCache projections (`ready_open_ids`, `blockers_of`, `dependents_of`) | `premath-observe` |
| `witnesses` | Observation functor (`draft/OBSERVATION-INF` §2) | `premath-observe` |
| `change_log` | Chain coherence check (`draft/CHANGE-SITE` §15) | `premath-transport` |
| `site_topology` | Site digest projection | `premath-kernel` |
| `observation_surfaces` | Observation build as `f*` (deterministic pullback of current Σ) | `premath-observe` |
| `working_copy` | Staged mutation projection | `premath-bd` |

`draft/OBSERVATION-INF` and `draft/OBSERVATION-SITE` are the `f*`
instantiation for the `{witnesses, issue_memory}` fibres.

## 5. Pi decomposition

Concrete `Π_i(Σ)` vocabularies per concern:

**`issue_memory`**: MCP operations — `issue_add`, `issue_update`,
`issue_claim`, `lease_renew`, `lease_release`, `dep_add`, `dep_remove`,
`dep_replace`.

**`witnesses`**: creation/update of witness records.

**`change_log`**: `SiteChangeRequest` application (`draft/CHANGE-SITE` §3).

**`site_topology`**: `SiteMutation` vocabulary (`draft/CHANGE-SITE` §2).

**`working_copy`**: stage/unstage/commit.

**`observation_surfaces`**: build/rebuild.

Cross-fibre Π dependencies:

- `issue_claim` in `Π_{issue_memory}` depends on `Σ_{issue_memory}` (ready
  computation) and `Σ_{witnesses}` (evidence state).
- `commit` in `Π_{working_copy}` depends on `Σ_{working_copy}` and all
  mutation-target fibres.
- `build` in `Π_{observation_surfaces}` depends on `Σ_{witnesses}` and
  `Σ_{issue_memory}`.

## 6. Content-addressed identity

Concrete digest scheme:

- **Per-fibre (JSON fibres)**: SHA256 of canonical JSON (keys sorted, no
  trailing whitespace, UTF-8 normalized).
- **Per-fibre (witness directory)**: SHA256 of canonical directory listing
  (sorted paths with per-file SHA256 digests).
- **Total**: SHA256 of concatenated per-fibre digests in lexicographic concern
  order (§3).

Convergence: identical total digests imply identified states regardless of
stepping path. Two different sequences of operations producing the same total
digest MUST be treated as arriving at the same state.

## 7. Commuting-square instantiation

Concrete cross-fibre squares for this site's DAG:

For each dependency edge `i → j` in the concern DAG (§2), the naturality of
`f*` requires:

```text
f*_total ; π_j = π_j ; f*_j
```

That is, per-fibre reindexing commutes with stepping.

Example: `SiteChangeRequest` (`Σ_f` on `site_topology`) + observation build
(`f*` on `witnesses`) must commute. Applying a site change then observing MUST
produce the same result as observing what the site change would produce.

Failure classes for violations:

```text
sigpi_site_commuting_square_violation
sigpi_site_naturality_failure
```

## 8. Descent instantiation

Glue condition: sequential per-fibre application and composed total application
MUST produce identical total Σ digests.

Contractibility: the decomposition of a composed step into per-fibre component
steps MUST be unique up to content-addressed identity (§6).

Failure classes:

```text
sigpi_site_glue_obstruction
sigpi_site_descent_component_rejected
sigpi_site_descent_contractibility_failed
```

## 9. Adjoint component mapping

How existing specs map to the triple:

| Existing spec | Adjoint component | Concern fibres |
| --- | --- | --- |
| `draft/CHANGE-INF` + `draft/CHANGE-SITE` | `Σ_f` | `{change_log, site_topology}` |
| `draft/OBSERVATION-INF` + `draft/OBSERVATION-SITE` | `f*` | `{witnesses, issue_memory}` |
| Issue MCP operations | `Π_f` | `{issue_memory}` |

Each existing spec remains authoritative for its concern fibres. SIGPI-SITE
provides the cross-fibre adjoint coherence that ties them together.

## 10. Doctrine-site routing

`Σ_f` ops (mutations) register as `route_bound` with morphisms
`[dm.identity, dm.commitment.attest]`.

`f*` ops (projections) register as `read_only_projection` with morphisms
`[dm.identity, dm.presentation.projection]`.

`Π_f` ops (gating/queries) register as `read_only_projection`.

World-route binding follows `draft/DOCTRINE-SITE` conventions.

## 11. Checker contract

The checker validates:

1. **Total Σ digest coherence**: recomputed total digest matches declared
   digest.
2. **Per-fibre digest coherence**: each per-fibre digest matches recomputed
   value.
3. **Concern DAG acyclicity**: the concern dependency graph (§2) contains no
   cycles.
4. **Cross-fibre commuting squares**: `f*` naturality (§7) holds for all
   dependency edges.
5. **Descent on composed step chains**: glue and contractibility conditions
   (§8) hold for multi-step compositions.

Exit codes:

- `0`: accepted (all checks pass).
- `1`: rejected (one or more checks fail).
- `2`: invalid input (malformed arguments or missing required data).

## 12. Non-goals

- No concern-specific mutation semantics beyond existing specs.
- No concurrency/locking models.
- No working-copy staging UI.
- No beads-specific operational details.
