---
slug: draft
shortname: DOCTRINE-INF
title: workingdoge.com/premath/DOCTRINE-INF
name: Meta Doctrine and Infinity-Layer Preservation
status: draft
category: Standards Track
tags:
  - premath
  - doctrine
  - meta-institution
  - infinity
  - preservation
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

This specification defines Premath's meta layer (`infty` layer): a doctrine of
morphisms over institutions, obligations, runtime transport, and projections.

Purpose:

- keep architecture changes composable,
- require explicit preservation claims under change,
- prevent semantic drift when implementations evolve.

`draft/DOCTRINE-INF` does not replace kernel laws. It constrains how lower
layers claim they preserve meaning.

## 2. Layer stack

Conforming stacks SHOULD be read as:

1. Doctrine layer (`draft/DOCTRINE-INF`)
2. Institution/kernel layer (`draft/PREMATH-KERNEL`, `draft/GATE`)
3. Obligation/runtime layer (`raw/TUSK-CORE`, `raw/SQUEAK-CORE`, `raw/SQUEAK-SITE`)
4. CI/projection layer (`raw/PREMATH-CI`, `raw/CI-TOPOS`)

Lower layers MUST publish which doctrine morphism classes they preserve.

## 3. Doctrine morphism registry (v0)

v0 defines the following morphism classes.

- `dm.identity`: identity/pure relabeling morphism preserving semantic object
  and verdict class.
- `dm.refine.context`: context/lineage refinement morphism preserving meaning
  under reindex/refinement.
- `dm.refine.cover`: cover refinement morphism preserving admissibility class.
- `dm.transport.world`: inter-world transport morphism preserving declared
  payload meaning and non-bypass constraints.
- `dm.transport.location`: runtime-location/site transport morphism preserving
  run-frame semantics and classification boundaries.
- `dm.profile.execution`: execution-substrate/profile morphism preserving Gate
  class outcomes for fixed semantic inputs.
- `dm.profile.evidence`: evidence-representation/profile morphism preserving
  kernel verdict and Gate failure classes.
- `dm.policy.rebind`: explicit policy/normalizer rebinding morphism requiring a
  new run boundary and explicit witness attribution.
- `dm.presentation.projection`: projection/view/UI morphism that MAY change
  representation but MUST NOT change semantic authority.
- `dm.commitment.attest`: commitment/witness attestation morphism binding
  obligations or instruction envelopes to deterministic evidence objects.

## 4. Preservation claim format

Lower specs MUST include a declaration section with this shape:

```text
Doctrine Preservation Declaration (v0)
Preserved morphisms:
- dm....
- dm....
Not preserved:
- dm.... (reason)
```

A spec MUST NOT claim preservation for a morphism class unless this spec's
required invariants make that preservation checkable.

## 5. Satisfaction preservation rule

Let `Sat_W(phi)` mean statement `phi` is satisfied in world/profile `W`.

A declared doctrine morphism `m : W1 -> W2` preserves `phi` only if:

```text
Sat_W1(phi) => Sat_W2(m(phi))
```

where `m(phi)` is the mapped statement under the declared morphism class.

For profile morphisms, preserving verdict class and Gate failure class set is a
sufficient v0 criterion.

## 6. Unknown and partial classification

Unknown classification is first-class.

`unknown(reason)` MUST be modeled as an explicit state, not implicit failure.

Morphisms from `unknown(reason)` MAY be restricted by policy (for example:
clarify-only, plan-only, escalation-required), but this restriction MUST be
explicitly declared in runtime/profile policy material.

## 7. Conformance expectations

Conforming implementations SHOULD:

- publish doctrine-preservation declarations across lower specs,
- include doctrine morphism attribution in witness diagnostics where relevant,
- reject or downgrade claims when preservation cannot be established.

## 8. Security and robustness

Implementations MUST treat doctrine declarations as auditable contract material.

Implementations SHOULD:

- keep doctrine morphism IDs stable,
- version declaration changes with explicit decision-log entries,
- fail closed when a required preservation claim is missing.
