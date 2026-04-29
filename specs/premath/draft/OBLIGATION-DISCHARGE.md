---
slug: draft
shortname: OBLIGATION-DISCHARGE
title: workingdoge.com/premath/OBLIGATION-DISCHARGE
name: Core Obligation and Discharge Interface
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - gate
  - obligation
  - discharge
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

This specification defines the Premath Core obligation/discharge interface.

It is intentionally small:

- `draft/PREMATH-KERNEL` owns semantic admissibility laws,
- this document owns finite obligation records and deterministic discharge
  verdicts,
- `draft/GATE` owns accepted/rejected Gate outcomes and failure classes, and
- `draft/WITNESS-ID` owns deterministic witness identity.

Full bidirectional verifier orchestration, normalized comparison machinery, and
LLM proposal ingestion are profile-scoped. The current full-profile
orchestration document is `profile/interop/BIDIR-DESCENT`.

## 2. Result Model

A discharge boundary evaluates a finite obligation set `O` and MUST produce one
of:

- `accepted`, or
- `rejected` with deterministically ordered Gate witnesses.

Acceptance is discharge-determined. Proposal, planner, or projection artifacts
MUST NOT self-authorize acceptance.

## 3. Obligation Kinds

<!-- premath:anchor obligation-discharge.core-obligation-kinds.start -->
The Core obligation/discharge interface MUST support obligations covering at
least:

1. `stability` - functorial reindexing (GATE-3.1)
2. `locality` - cover restriction existence (GATE-3.2)
3. `descent_exists` - gluing existence (GATE-3.3)
4. `descent_contractible` - contractible glue space (GATE-3.4)
5. `adjoint_triple` - Sigma/f*/Pi coherence (GATE-3.5) **only if advertised**

Implementations MAY use the following operational obligations, which MUST map
into Gate classes deterministically:

6. `ext_gap` - no derivation/transport path for a required target context
7. `ext_ambiguous` - multiple incomparable maximal derivations
   (non-contractible choice)
<!-- premath:anchor obligation-discharge.core-obligation-kinds.end -->

Unsupported obligation forms MUST be rejected deterministically unless an
explicitly claimed profile defines their semantics.

## 4. Obligation Record Format

Each obligation MUST have deterministic serialization sufficient to compute a
stable obligation ID. At minimum, each record MUST bind:

- `kind`,
- `ctx` (serialized context),
- `subject` (at minimum `kind` plus a committed reference when available), and
- `details` (kind-specific data).

Implementations MAY add fields, but MUST keep canonical serialization stable for
the same semantic input and declared profile bindings.

## 5. Discharge Requirements

Discharge MUST be deterministic for fixed inputs, policy bindings, stores, and
declared profile parameters.

For every obligation set, an implementation MUST:

1. accept only when all required obligations discharge successfully,
2. reject when any required obligation fails,
3. reject unsupported obligation kinds deterministically,
4. emit the correct Gate failure class for each rejection, and
5. order witness records deterministically as specified by `draft/GATE`.

Profiles MAY define additional comparison modes or evidence payloads. Those
profiles MUST preserve the Core accepted/rejected result and Gate failure
classes for fixed semantic inputs and fixed bindings.

## 6. Mapping to Gate

The following mapping is normative:

- `stability` failures -> `stability_failure` (`GATE-3.1`)
- `locality` failures -> `locality_failure` (`GATE-3.2`)
- `descent_exists` / `ext_gap` -> `descent_failure` (`GATE-3.3`)
- `descent_contractible` / `ext_ambiguous` -> `glue_non_contractible`
  (`GATE-3.4`)
- `adjoint_triple` -> `adjoint_triple_coherence_failure` (`GATE-3.5`)

Rejected checks MUST emit Gate witness payloads as specified by `draft/GATE`
§4.1. `witnessId` values MUST be computed per `draft/WITNESS-ID`.

## 7. Replay and Profile Invariance

At exposed boundaries, a Core implementation MUST preserve:

- the accepted/rejected result,
- the Gate failure classes, and
- deterministic witness identity for the same rejected obligation records.

Artifact form, transport metadata, and profile-local evidence MAY differ across
profiles. Those differences MUST NOT create a second admissibility authority.

## 8. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.refine.context`
- `dm.refine.cover`
- `dm.profile.evidence` (verdict class and failure-class invariance only)
- `dm.policy.rebind` (requires explicit binding mismatch/rebind handling)

Not preserved:

- `dm.transport.world` (handled by `raw/SQUEAK-CORE`)
- `dm.transport.location` (handled by `raw/SQUEAK-SITE`)
- `dm.profile.execution` (handled by runtime/CI layer)
- `dm.presentation.projection` (handled by projection layer)
