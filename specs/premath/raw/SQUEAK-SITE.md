---
slug: raw
shortname: SQUEAK-SITE
title: workingdoge.com/premath/SQUEAK-SITE
name: Squeak Site on Runtime Locations
status: raw
category: Standards Track
tags:
  - premath
  - squeak
  - sigpi
  - site
  - runtime
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

## 1. Scope

This specification defines a site structure over runtime locations for
`squeak-core`.

Intent:

- model runtime placement/orchestration as a Squeak/SigPi object,
- preserve kernel/Tusk admissibility semantics under location changes,
- support deterministic witness gluing across location covers.

Boundary:

- `raw/TUSK-CORE` governs local admissibility checks inside one world.
- `raw/SQUEAK-CORE` governs transport/composition and handoff.
- `raw/SQUEAK-SITE` governs runtime location objects, covers, overlaps, and
  glue contracts for runtime evidence.

## 2. Site object

Implementations SHOULD model runtime location orchestration as:

```text
SqueakSite = (Loc, Covers, Transport, WitnessPresheaf, GlueLaw)
```

where:

- `Loc` is the category of runtime locations,
- `Covers` defines admissible covering families over a run frame,
- `Transport` provides deterministic location-to-location maps,
- `WitnessPresheaf` assigns location-scoped evidence sections,
- `GlueLaw` defines existence/uniqueness expectations for global witness
  bundles.

## 3. Runtime locations (`Loc`)

A location descriptor MUST be deterministic:

```text
LocationDescriptor {
  loc_id
  world_id
  runtime_profile
  capability_vector
  substrate_binding_ref
}
```

`loc_id` MUST be deterministic for fixed descriptor contents.

`runtime_profile` MAY include values such as `local`, `ci`, `remote-worker`,
`darwin_microvm_vfkit`, or implementation-defined equivalents.

## 4. Morphisms and transport on locations

Location morphisms represent admissible runtime relocation/transport operations.

A conforming transport map over locations MUST preserve:

- run-frame identity material required for replay/audit,
- source lineage attribution material,
- deterministic error classification on mismatch.

Transport failures remain transport/runtime failures and MUST NOT be relabeled
as kernel Gate failures.

## 5. Run frame and admissible covers

Covers are formed over a fixed run frame:

```text
RunFrame {
  delta_ref
  policy_digest
  projection_digest
  required_checks
}
```

A family of locations is an admissible cover of a run frame only if:

- each location can execute all required checks under bound policy,
- capability requirements for those checks are satisfied,
- overlap obligations can be computed deterministically.

## 6. Overlap and agreement relation

For a cover family `{loc_i}`, implementations MUST define deterministic overlap
obligations `overlap(loc_i, loc_j, run_frame)`.

Agreement on overlaps SHOULD require at least:

- same Gate-class outcome vector,
- same required check identifier set,
- same policy/projection digest bindings.

Additional witness-level equivalence requirements MAY be imposed by profile.

## 7. Witness presheaf and glue law

`WitnessPresheaf` maps each location to a location-scoped evidence section.

For families agreeing on all required overlaps, implementations SHOULD construct
a glued global evidence object:

```text
SqueakSiteWitnessBundle {
  run_frame
  cover_loc_ids
  section_refs
  overlap_refs
  glue_mode
}
```

If glue fails, implementations MUST emit deterministic site-class failures such
as:

- `site_overlap_mismatch`
- `site_glue_missing`
- `site_glue_non_contractible`

## 8. Authority split (load-bearing)

Squeak owns:

- location descriptors and covers,
- runtime orchestration (`Cheese`),
- transport and site-level witness glue.

Tusk owns:

- destination-local admissibility checks and Gate-class witnesses.

Infrastructure providers (Terraform/OpenTofu, microvm profiles, etc.) are
location constructors/binders only. They MUST NOT define semantic admissibility.

## 9. Invariance requirements

For fixed `RunFrame` and fixed policy bindings:

- location/profile choice MUST NOT change Gate-class outcomes,
- representation/profile changes MAY change evidence shape but not Gate class,
- retry/replay across locations MUST remain deterministic and auditable.

## 10. Cheese contract (runtime unit term)

This specification uses `Cheese` as the Squeak runtime unit term.

Minimal logical interface:

```text
resolve_cheese(world, runtime_profile) -> CheeseDescriptor
run_on_cheese(cheese, task_ref, env_ref) -> ExecutionResult
```

`CheeseDescriptor` SHOULD include `loc_id` material and substrate binding
attribution.

## 11. v0 profile guidance

A reasonable v0 profile set is:

- required/default: `local`,
- optional: CI-hosted runner profile,
- experimental: microvm-backed runtime profile.

Experimental profiles MUST remain non-required until site invariance vectors
prove stable outcomes under declared bindings.

## 12. Security and robustness

Implementations MUST treat all runtime bindings and transported payloads as
untrusted.

Implementations SHOULD:

- pin profile bindings for replayability,
- retain location/transport lineage logs,
- fail closed when required checks cannot be executed at a location.

## 13. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.transport.location`
- `dm.refine.cover` (location-cover refinement inside one run frame)
- `dm.profile.execution` (profile choice must preserve Gate-class outcomes)
- `dm.profile.evidence` (representation/profile changes must preserve Gate class)

Not preserved:

- `dm.transport.world` (handled by `raw/SQUEAK-CORE`)
- `dm.refine.context` (handled by kernel/Tusk layer)
- `dm.presentation.projection` (handled by projection layer)
