---
slug: draft
shortname: SITE-RESOLVE
title: workingdoge.com/premath/SITE-RESOLVE
name: Deterministic Site Resolve Contract
status: draft
category: Standards Track
tags:
  - premath
  - site
  - resolver
  - world
  - kcir
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

This spec defines one deterministic resolver contract that selects
operation-to-world routing through one authority path:

`DOCTRINE-INF -> DOCTRINE-SITE -> WORLD-REGISTRY -> KCIR handoff`.

Purpose:

- define one canonical `SitePackage` projection shape used by resolvers,
- define deterministic request/response envelopes for route resolution,
- enforce fail-closed unbound/ambiguous outcomes,
- keep resolver output stable and witness-linkable for KCIR authority handoff.

This spec does not introduce a second semantic authority lane. Semantic
admissibility remains kernel/Gate owned (`draft/PREMATH-KERNEL`,
`draft/GATE`, `draft/BIDIR-DESCENT`).

## 2. Canonical projection object (`SitePackage`)

Resolvers MUST evaluate one canonical projected object:

```text
SitePackage {
  schema: 1
  packageKind: "premath.site_package.v1"
  sourceRefs: {
    doctrineSiteInput: string
    doctrineSite: string
    doctrineOperationRegistry: string
    controlPlaneContract: string
  }
  siteTopology: {
    siteId: string
    nodes: list<string>
    covers: list<string>
    edges: list<string>
  }
  operationRows: list<OperationRow>
  worldRouteRows: list<WorldRouteRow>
  kcirMappingRows: list<KcirMappingRow>
}
```

Rules:

- `SitePackage` MUST be deterministically projected from authoritative
  declaration surfaces:
  - `draft/DOCTRINE-SITE-INPUT.json`,
  - generated `draft/DOCTRINE-SITE.json`,
  - generated `draft/DOCTRINE-OP-REGISTRY.json`,
  - `draft/CONTROL-PLANE-CONTRACT.json`.
- projection MUST be canonicalized with stable row ordering and duplicate
  elimination (lexicographic by stable identity fields).
- projection rows MUST be treated as authority references, not wrapper-owned
  semantics.
- generated/derived rows MUST remain replay-stable under input permutation.

## 3. Resolver envelopes

### 3.1 Request

```text
SiteResolveRequest {
  schema: 1
  requestKind: "premath.site_resolve.request.v1"
  operationId: string
  routeFamilyHint?: string
  claimedCapabilities: list<string>
  policyDigest: string
  profileId: string
  contextRef: string
}
```

Rules:

- `operationId` MUST reference one row in projected operation rows.
- `claimedCapabilities` MUST be interpreted against
  `draft/CAPABILITY-REGISTRY.json`.
- `policyDigest` and `profileId` MUST be treated as filter inputs, never as
  authority bypass flags.

### 3.2 Response

```text
SiteResolveResponse {
  schema: 1
  responseKind: "premath.site_resolve.response.v1"
  result: "accepted" | "rejected"
  failureClasses: list<string>
  selected?: SelectedBinding
  projection: SiteResolveProjection
}
```

```text
SelectedBinding {
  operationId: string
  routeFamilyId: string
  siteNodeId: string
  coverId: string
  worldId: string
  morphismRowId: string
  requiredMorphisms: list<string>
}
```

```text
SiteResolveProjection {
  projectionKind: "premath.site_resolve.projection.v1"
  requestDigest: string
  sitePackageDigest: string
  doctrineSiteDigest: string
  doctrineOpRegistryDigest: string
  worldRouteDigest: string
  policyDigest: string
  kcirMappingRef?: {
    sourceKind: string
    targetDomain: string
    targetKind: string
    identityFields: list<string>
  }
}
```

## 4. Deterministic selection order (normative)

Resolver implementations MUST execute this order exactly:

`candidate gather -> capability/policy filter -> world-route validation -> overlap/glue decision`.

### 4.1 Candidate gather

- gather candidate rows by `operationId` and optional `routeFamilyHint`.
- if no candidate exists, resolver MUST reject fail closed with
  `site_resolve_unbound`.

### 4.2 Capability/policy filter

- drop rows not admitted by claimed capabilities and policy/profile constraints.
- if all rows are removed, resolver MUST reject fail closed with:
  - `site_resolve_capability_missing`, or
  - `site_resolve_policy_denied`.

### 4.3 World-route validation

- remaining rows MUST be validated against world-route semantics from
  `draft/WORLD-REGISTRY` and `worldRouteBindings`.
- implementations SHOULD execute this through the canonical command lane
  (`premath world-registry-check`) or equivalent kernel-backed API.
- rows failing world validation MUST reject with canonical world classes (for
  example `world_route_unbound`, `world_route_unknown_world`,
  `world_route_unknown_morphism`, `world_route_morphism_drift`).

### 4.4 Overlap/glue decision

- remaining rows MUST be checked for site overlap compatibility and deterministic
  glue outcome against `draft/DOCTRINE-SITE` topology claims.
- incompatibility MUST reject with `site_overlap_mismatch`.
- missing glue evidence MUST reject with `site_glue_missing`.
- non-contractible glue MUST reject with `site_glue_non_contractible`.

### 4.5 Tie-break and ambiguity

When multiple candidates survive overlap/glue checks, implementations MUST sort
by this deterministic key tuple:

1. exact `routeFamilyHint` match first,
2. cover specificity (more specific cover first),
3. lexical tuple:
   `(routeFamilyId, operationId, worldId, morphismRowId, siteNodeId, coverId)`.

If more than one distinct candidate remains equal under this key, resolver MUST
reject fail closed with `site_resolve_ambiguous`.

## 5. Fail-closed outcomes

Resolvers MUST fail closed for both classes below:

- unbound: no admissible candidate path (`site_resolve_unbound`),
- ambiguous: multiple non-equivalent candidates after deterministic ordering
  (`site_resolve_ambiguous`).

Resolvers MUST NOT emit a synthetic default binding to recover either case.

## 6. KCIR handoff contract

Accepted resolver output MUST include stable route/site/world references
sufficient for KCIR authority handoff:

- operation identity (`operationId`, `routeFamilyId`),
- site identity (`siteNodeId`, `coverId`),
- world identity (`worldId`, `morphismRowId`, `requiredMorphisms`),
- deterministic projection refs (`requestDigest`, package/spec digests, policy
  digest),
- optional KCIR mapping row (`sourceKind`, `targetDomain`, `targetKind`,
  `identityFields`) when the mapping exists in
  `controlPlaneKcirMappings.mappingTable`.

This handoff is projection material only; final admissibility remains
kernel/Gate owned.

## 7. Conformance and integration

Minimum execution surfaces for this contract:

- `mise run doctrine-check` (doctrine-site + route closure integrity),
- `python3 tools/conformance/run_world_core_vectors.py`
  (world-route semantic invariance),
- `mise run docs-coherence-check` (docs/spec parity).

Implementations SHOULD keep resolver envelopes witness-linkable to the same
deterministic lineage used by control-plane KCIR mappings.

