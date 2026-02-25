---
slug: draft
shortname: WORLD-REGISTRY
title: workingdoge.com/premath/WORLD-REGISTRY
name: World Registry and Morphism Table
status: draft
category: Standards Track
tags:
  - premath
  - world
  - cwf
  - descent
  - control-plane
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

This specification defines one canonical registry shape for Premath world
profiles and inter-world morphism declarations.

Purpose:

- keep `world == premath` explicit in control-plane contracts,
- make one explicit Grothendieck constructor authority object for control-plane
  worldization,
- keep semantic authority and adapter execution boundaries non-overlapping,
- make route-to-world binding auditable and fail closed on drift.

This specification does not replace kernel authority (`draft/PREMATH-KERNEL`,
`draft/GATE`, `draft/BIDIR-DESCENT`). It constrains how worldized routes are
declared and bound.

## 2. Canonical registry object

Implementations SHOULD expose one deterministic `WorldRegistry` object:

```text
WorldRegistry {
  schema: 1
  registryKind: "premath.world_registry.v1"
  worlds: list<WorldRow>
  morphisms: list<WorldMorphismRow>
  routeBindings: list<RouteBindingRow>
}
```

### 2.1 World row

```text
WorldRow {
  worldId: string
  role: "semantic_authority" | "control_plane_projection" | "runtime_profile"
  contextFamilyId: string
  definableFamilyId: string
  coverKind: string
  equalityMode: string
  sourceRefs: list<string>
}
```

Rules:

- `worldId` MUST be unique in one registry.
- `sourceRefs` MUST reference canonical contracts/specs for the world row.
- world rows MUST NOT self-authorize outside kernel/checker authority lanes.

### 2.2 World morphism row

```text
WorldMorphismRow {
  morphismRowId: string
  fromWorldId: string
  toWorldId: string
  doctrineMorphisms: list<dm.*>
  preservationClaims: list<string>
}
```

Rules:

- `morphismRowId` MUST be unique.
- `fromWorldId` and `toWorldId` MUST reference declared world rows.
- every `doctrineMorphisms` item MUST be declared in `draft/DOCTRINE-INF`.

### 2.3 Route binding row

```text
RouteBindingRow {
  routeFamilyId: string
  operationIds: list<string>
  worldId: string
  morphismRowId: string
  failureClassUnbound: string
}
```

Rules:

- each `operationId` MUST reference one operation in
  `draft/DOCTRINE-OP-REGISTRY.json`,
- each bound operation MUST resolve to exactly one `worldId` in one active
  profile,
- unresolved or multiply-bound routes MUST reject fail closed.

### 2.4 Kernel execution contract (K0)

World registry rows are not docs-only metadata. Implementations SHOULD expose a
kernel-backed execution interface that all CLI/coherence/adapter paths consume.

Canonical interface shape:

```text
WorldKernelSurface {
  load_registry(input) -> WorldRegistry
  validate_registry(registry) -> RegistryValidation
  resolve_route_family(registry, routeFamilyId) -> RouteBindingRow
  verify_operation_binding(registry, operationRow, bindingRow) -> BindingDecision
}
```

Minimum decision envelope:

```text
BindingDecision {
  result: "accepted" | "rejected"
  failureClasses: list<string>
  derived: {
    worldId: string
    morphismRowId: string
    missingMorphisms: list<string>
  }
}
```

Rules:

- world binding semantics MUST be implemented once in kernel-backed code and
  reused by downstream checker/CLI/CI paths,
- wrappers/adapters MUST treat this interface as authority input and MUST NOT
  re-derive independent acceptance logic,
- output failure classes MUST remain deterministic under input permutation.

### 2.5 Explicit Grothendieck constructor object (GC0)

Implementations SHOULD expose one deterministic constructor object per active
world profile:

```text
WorldGrothendieckConstructor {
  schema: 1
  constructorKind: "premath.world_grothendieck_constructor.v1"
  profileId: string
  sourceRefs: {
    worldRegistry: string
    doctrineSiteInput: string
    doctrineOperationRegistry: string
    controlPlaneContract: string
  }
  base: {
    contextFamilyId: string
    coverKind: string
    doctrineRoot: string
  }
  family: {
    worldRowsDigest: string
    morphismRowsDigest: string
    routeBindingsDigest: string
  }
  evidence: {
    evidenceFamilyId: string
    factorizationRouteKind: string
    factorizationRoutes: list<string>
    binding: {
      normalizerId: string
      policyDigest: string
    }
  }
  overlays: {
    allowed: list<string>
    authorityTargetsForbidden: list<string>
  }
  failureClasses: {
    missingRoute: string
    ambiguousRoute: string
    unboundBinding: string
  }
}
```

Rules:

- constructor derivation MUST be deterministic from the source refs above.
- one active profile MUST resolve to exactly one constructor object up to
  canonical projection equality.
- CLI/checker/CI wrapper lanes MUST consume route/world decisions derived from
  this constructor and MUST NOT synthesize independent semantic route verdicts.
- constructor evidence routes MUST remain aligned with Unified Evidence
  factoring (`draft/UNIFICATION-DOCTRINE` §10 and §12).
- torsor overlays MAY be attached as interpretation metadata only; they MUST
  NOT appear as authority world targets.

## 3. Required worldization rows (bundle v0)

For repository profile `cp.bundle.v0` (`draft/CONTROL-PLANE-CONTRACT.json`),
implementations MUST keep these row IDs reserved:

- `world.kernel.semantic.v1` (semantic kernel/Gate authority),
- `world.control_plane.bundle.v0` (control-plane projection/parity family),
- `world.lease.v1` (claim/lease mutation profile),
- `world.fiber.v1` (structured-concurrency lifecycle profile),
- `world.instruction.v1` (instruction envelope profile),
- `world.ci_witness.v1` (required/decision witness profile).

Capability-gated requirement:

- when `capabilities.change_morphisms` is claimed, mutation route families
  (`issue.claim`, `issue.lease_renew`, `issue.lease_release`,
  `issue.discover`) MUST bind to `world.lease.v1`,
- when `capabilities.instruction_typing` is claimed, instruction route families
  MUST bind to `world.instruction.v1`,
- when `capabilities.ci_witnesses` is claimed, required/decision witness route
  families MUST bind to `world.ci_witness.v1`.

Optional profile reservation:

- structured-concurrency route family `route.fiber.lifecycle` MAY bind to
  `world.fiber.v1` for `fiber.spawn|join|cancel` transport lifecycle actions.

Optional overlay reservation:

- implementations MAY claim torsor/extension interpretation overlay
  `overlay.torsor_ext.v1` for lease/instruction/ci-witness twist classes,
- `overlay.torsor_ext.v1` is not a `WorldRow` authority target and MUST NOT be
  used as `routeBindings.worldId`,
- torsor overlay artifacts MUST remain proposal/evidence-only and route through
  existing checker/Gate authority chains.

## 4. CwF-first and descent ownership

World rows claiming semantic authority MUST preserve CwF/descent ownership
boundaries:

- strict CwF operational equalities remain checker-lane material,
- semantic admissibility remains kernel/Gate material,
- route bindings MUST NOT introduce a second admissibility schema.

Reference boundaries:

- `draft/PREMATH-COHERENCE` (strict CwF checker lane),
- `draft/UNIFICATION-DOCTRINE` §9 and §12 (lane split + Grothendieck
  operationalization).

## 5. Adapter boundary rule

Adapter and transport surfaces remain execution IO. They MUST NOT define
coverage doctrine or final glue semantics.

Reference:

- `raw/TUSK-CORE` adapter/world ownership split,
- §2.4 kernel execution contract (single world semantics authority lane).

## 6. Doctrine-site input embedding

Implementations SHOULD embed world route rows in
`draft/DOCTRINE-SITE-INPUT.json` using a deterministic object:

```text
worldRouteBindings {
  schema: 1
  bindingKind: "premath.world_route_bindings.v1"
  rows: list<RouteBindingRow>
}
```

This object is a declaration/input surface. Generated doctrine site artifacts
remain operation/morphism maps unless and until promotion adds world-route
material to generated outputs.

## 7. Rejection conditions

A worldized route contract MUST reject when any of the following holds:

- `worldId` is unknown,
- `morphismRowId` is unknown,
- `operationIds` include unknown operation IDs,
- one operation is bound to multiple world rows in one active profile,
- declared doctrine morphisms drift from operation route declarations,
- constructor derivation is missing for an active profile,
- constructor derivation is ambiguous (multiple inequivalent constructors), or
- constructor route/evidence bindings are unbound under declared deterministic
  binding material.

## 8. Conformance and traceability

Minimum executable surfaces for this specification are:

- `mise run coherence-check`
- `mise run doctrine-check`
- `mise run docs-coherence-check`

Capability vectors for concrete world profiles (lease/instruction/ci-witness)
remain the authoritative executable closure for profile-specific claims.
