---
slug: draft
shortname: DOCTRINE-SITE
title: workingdoge.com/premath/DOCTRINE-SITE
name: Doctrine to Operation Site Map
status: draft
category: Standards Track
tags:
  - premath
  - doctrine
  - site
  - operation
  - conformance
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

This spec defines a site-shaped, auditable path from doctrine declarations to
operational gate entrypoints.

Purpose:

- make the doctrine-to-operation path explicit,
- keep declarations and operational entrypoints coherent under change,
- enforce that runtime tools remain downstream of declared semantic authority.

This spec does not introduce new doctrine morphism classes.
It binds existing classes from `draft/DOCTRINE-INF` to an auditable path map.

## 2. Site object

Implementations SHOULD model this path as:

```text
DoctrineOperationSite = (Nodes, Covers, Edges, Entrypoints)
```

where:

- `Nodes` are specification and operation objects,
- `Covers` are admissible decompositions over doctrine/layer nodes,
- `Edges` are declared doctrine-preserving relations,
- `Entrypoints` are operational executables (`tools/...`) reachable from doctrine.

## 3. Canonical map artifact

The canonical machine-readable artifact for this site is:

- `draft/DOCTRINE-SITE.json`

Conforming repositories MUST keep this artifact coherent with:

- `draft/DOCTRINE-INF` morphism registry,
- per-spec `Doctrine Preservation Declaration (v0)` sections,
- operational tool paths referenced by the map.

## 4. Required node classes

The site map MUST include at least:

- doctrine root (`draft/DOCTRINE-INF`),
- kernel/gate/conformance contract nodes (`draft/*`),
- instruction doctrine nodes when instruction-envelope control loops are exposed
  (for example `draft/LLM-INSTRUCTION-DOCTRINE`),
- runtime transport/site nodes (`raw/TUSK-CORE`, `raw/SQUEAK-CORE`,
  `raw/SQUEAK-SITE`),
- CI/projection nodes (`raw/PREMATH-CI`, `raw/CI-TOPOS`),
- operational entrypoint nodes (`tools/ci/*`, `tools/conformance/*`).

Operational nodes are not semantic authorities. They are execution/projection
surfaces bound to upstream declarations.

## 5. Edge discipline

Every edge in `draft/DOCTRINE-SITE.json` MUST:

- reference known node IDs,
- reference morphism IDs from `draft/DOCTRINE-INF`,
- terminate at a node whose declaration preserves the listed morphisms (when the
  destination node is declaration-bearing).

This keeps doctrine path claims checkable.

## 6. Reachability requirement

For each operation node, there MUST exist at least one directed path from
`draft/DOCTRINE-INF`.

This ensures every operational gate/projection entrypoint has an explicit
doctrine ancestry.

## 7. Conformance tooling

Repositories SHOULD provide a deterministic checker that validates:

- declaration presence and morphism ID validity,
- declaration set coherence with `draft/DOCTRINE-SITE.json`,
- edge and cover coherence,
- doctrine-to-operation reachability.

In this repository, that checker is:

- `tools/conformance/check_doctrine_site.py`

## 8. Security and robustness

Implementations MUST treat map artifacts and spec text as untrusted input.

Implementations SHOULD:

- fail closed on missing declaration-bearing nodes,
- reject unknown morphism IDs,
- keep map and declarations in lockstep under review/CI.
