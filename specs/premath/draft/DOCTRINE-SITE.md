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
- `draft/DOCTRINE-OP-REGISTRY.json` (generated operation-node + CI edge view).

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
- kernel/gate/conformance contract nodes (`draft/*`),
- instruction doctrine nodes when instruction-envelope control loops are exposed
  (for example `draft/LLM-INSTRUCTION-DOCTRINE` and
  `draft/LLM-PROPOSAL-CHECKING`),
- runtime transport/site nodes (`raw/TUSK-CORE`, `raw/SQUEAK-CORE`,
  `raw/SQUEAK-SITE`),
- CI/projection nodes (`raw/PREMATH-CI`, `raw/CI-TOPOS`),
- operational entrypoint nodes (`tools/ci/*`, `tools/conformance/*`,
  `crates/premath-cli/src/commands/*` for worker-memory and harness session
  surfaces).

Operational nodes are not semantic authorities. They are execution/projection
surfaces bound to upstream declarations.

When implementations expose multithread worker orchestration, repositories
SHOULD include route guidance linking these operation nodes to:

- cover/refinement decomposition semantics (`raw/CTX-SITE`),
- deterministic glue-or-obstruction boundary (`raw/SHEAF-STACK`),
- Unified Evidence factoring and lane ownership (`draft/UNIFICATION-DOCTRINE`
  §10 and §12).

Repository v0 note:

- CI operation nodes currently include `tools/ci/run_gate.sh`,
  `tools/ci/run_gate_terraform.sh`, `tools/ci/run_instruction.sh`,
  `tools/ci/verify_required_witness.py`, and `tools/ci/decide_required.py`.
  Squeak runtime transport/placement routing for gate execution is explicit on
  `run_gate*` operation nodes via `dm.transport.world` +
  `dm.transport.location`.
- worker-memory operation nodes include MCP mutation paths for
  `issue_add`, `issue_update`, `issue_claim`, `issue_lease_renew`,
  `issue_lease_release`, `issue_discover`, `dep_add`, `dep_remove`, and
  `dep_replace` in `crates/premath-cli/src/commands/mcp_serve.rs`.
- worker-memory read/projection nodes include
  `issue_list`, `issue_ready`, `issue_blocked`, `issue_check`,
  `issue_backend_status`, `issue_lease_projection`, and `dep_diagnostics`
  in `crates/premath-cli/src/commands/mcp_serve.rs`.
- MCP instruction/doctrine and observation projection nodes include
  `instruction_check`, `instruction_run`, `observe_latest`,
  `observe_needs_attention`, `observe_instruction`, and
  `observe_projection` in `crates/premath-cli/src/commands/mcp_serve.rs`.
- MCP initialization node includes `init_tool` in
  `crates/premath-cli/src/commands/mcp_serve.rs`.
- harness-session operation nodes include `read`, `write`, and `bootstrap`
  paths in `crates/premath-cli/src/commands/harness_session.rs`.
- doctrine-conformance operation nodes currently include
  `tools/conformance/check_doctrine_site.py`,
  `tools/conformance/check_runtime_orchestration.py`,
  `tools/conformance/check_doctrine_mcp_parity.py`, and
  `tools/conformance/run_doctrine_inf_vectors.py` (including claim-gated
  governance-profile vectors).

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

For routed worker-memory and harness operation paths
(`op/mcp.issue_*`, `op/mcp.dep_*`, `op/harness.session_*`):

1. decomposition/routing MUST remain operational projection material only,
2. semantic admissibility MUST remain checker/Gate-owned,
3. control-plane acceptance/rejection outputs MUST remain bound to one
   deterministic evidence route (no parallel authority path).

Cross-lane pullback/base-change commutation claims SHOULD be routed through the
typed span/square witness surface (`draft/SPAN-SQUARE-CHECKING`) when surfaced
by control-plane tooling.

## 7. Conformance tooling

Repositories SHOULD provide a deterministic checker that validates:

- generated map roundtrip against tracked map artifacts,
- declaration presence and morphism ID validity,
- declaration set coherence with `draft/DOCTRINE-SITE.json`,
- edge and cover coherence,
- doctrine-to-operation reachability.

In this repository, that checker is:

- `tools/conformance/check_doctrine_site.py`
- `tools/conformance/check_runtime_orchestration.py`
- `tools/conformance/check_doctrine_mcp_parity.py` (MCP operation parity
  against `draft/DOCTRINE-OP-REGISTRY.json`)
- `tools/conformance/run_doctrine_inf_vectors.py` (semantic-boundary +
  claim-gated governance-profile vectors)

And the canonical map generator is:

- `tools/conformance/generate_doctrine_site.py`

## 8. Security and robustness

Implementations MUST treat map artifacts and spec text as untrusted input.

Implementations SHOULD:

- fail closed on missing declaration-bearing nodes,
- reject unknown morphism IDs,
- keep map and declarations in lockstep under review/CI.
