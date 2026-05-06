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
  - checker-claims
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
- keep worker-orchestration routing aligned with cover/refinement and
  Unified Evidence factoring boundaries from `draft/UNIFICATION-DOCTRINE`.

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

## 3. Canonical map artifacts

The canonical machine-readable artifacts for this site are:

- `draft/DOCTRINE-SITE-INPUT.json` (single authoritative input contract),
- `draft/DOCTRINE-SITE.json` (generated canonical map),
- `draft/DOCTRINE-OP-REGISTRY.json` (operation-node + checker edge view).

Conforming repositories MUST generate `draft/DOCTRINE-SITE.json` and
`draft/DOCTRINE-OP-REGISTRY.json`
deterministically from:

- `draft/DOCTRINE-SITE-INPUT.json`,
- declaration-bearing spec sections (`Doctrine Preservation Declaration (v0)`).

Generated views (`draft/DOCTRINE-SITE.json`,
`draft/DOCTRINE-OP-REGISTRY.json`) MUST roundtrip to exactly the same generated
output under deterministic canonicalization.

## 4. Required node classes

The site map MUST include at least:

- doctrine root (`draft/DOCTRINE-INF`),
- kernel/gate/checker claim nodes (`draft/*`),
- instruction doctrine nodes when instruction-envelope control loops are exposed
  (for example `draft/LLM-INSTRUCTION-DOCTRINE` and
  `draft/LLM-PROPOSAL-CHECKING`),
- runtime transport/site nodes (`raw/SQUEAK-CORE`, `raw/SQUEAK-SITE`),
- operational entrypoint nodes
  (`crates/premath-cli/src/commands/*` for checker command surfaces).

Operational nodes are not semantic authorities. They are execution/projection
surfaces bound to upstream declarations.

When adjacent runtime sites expose multithread worker orchestration, those sites
SHOULD include route guidance linking their operation nodes to:

- cover/refinement decomposition semantics (`raw/CTX-SITE`),
- deterministic glue-or-obstruction boundary (`raw/SHEAF-STACK`),
- Unified Evidence factoring and lane ownership (`draft/UNIFICATION-DOCTRINE`
  §10 and §12).

Repository v0 note:

- Gate execution and instruction execution are runtime/control-site concerns,
  not Premath doctrine-site operation nodes.
- Hook management, retry/escalation, and provider artifact publication are
  runtime/control-site concerns, not Premath doctrine-site operation nodes.
- issue/dependency CLI commands are tracker utilities owned outside Premath,
  not doctrine-site operation nodes.
- doctrine checker operation nodes currently include
  `crates/premath-cli/src/commands/coherence_check.rs`,
  `crates/premath-cli/src/commands/traceability_check.rs`,
  and `crates/premath-cli/src/commands/drift_budget_check.rs`.

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

### 6.1 Operational cover/refinement routing boundary

Cross-lane pullback/base-change commutation claims SHOULD be routed through the
typed span/square witness surface (`draft/SPAN-SQUARE-CHECKING`) when surfaced
by control-plane tooling.

## 7. Checker Tooling

Repositories SHOULD provide a deterministic checker that validates:

- generated map roundtrip against tracked map artifacts,
- declaration presence and morphism ID validity,
- declaration set coherence with `draft/DOCTRINE-SITE.json`,
- edge and cover coherence,
- doctrine-to-operation reachability.

In this repository, checker entrypoints are:

- `crates/premath-cli/src/commands/coherence_check.rs`
- `crates/premath-cli/src/commands/traceability_check.rs`
- `crates/premath-cli/src/commands/drift_budget_check.rs`

## 8. Security and robustness

Implementations MUST treat map artifacts and spec text as untrusted input.

Implementations SHOULD:

- fail closed on missing declaration-bearing nodes,
- reject unknown morphism IDs,
- keep map and declarations in lockstep under review.
