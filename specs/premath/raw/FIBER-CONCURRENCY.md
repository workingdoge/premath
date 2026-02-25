---
slug: raw
shortname: FIBER-CONCURRENCY
title: workingdoge.com/premath/FIBER-CONCURRENCY
name: Fiber Concurrency Profile (Structured Concurrency over Worlds)
status: raw
category: Standards Track
tags:
  - premath
  - fiber
  - concurrency
  - transport
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

This specification defines a structured-concurrency profile for Premath using
fiber lifecycle actions.

Boundary:

- semantic admissibility remains kernel/Gate authority,
- fiber lifecycle is operational control-plane/runtime structure,
- transport adapters carry typed fiber action envelopes and witnesses.

This document does not introduce a parallel semantic authority lane.

## 2. Fiber model

A fiber is a bounded operational section over a worldized execution context:

```text
Fiber {
  fiberId
  parentFiberId?
  taskRef
  scopeRef?
  state
  witnessRefs[]
}
```

State progression is:

```text
spawned -> running -> joined
        \-> cancelled
```

Operational decomposition SHOULD treat child fibers as a cover over one parent
execution objective; `join` is the deterministic glue boundary.

## 3. Typed transport actions

Fiber lifecycle actions are carried through typed transport dispatch envelopes:

- `fiber.spawn`
- `fiber.join`
- `fiber.cancel`

Expected payload minimums:

- `fiber.spawn`: `taskRef`, optional `fiberId`, optional `parentFiberId`,
  optional `scopeRef`.
- `fiber.join`: `fiberId`, non-empty `joinSet`, optional `resultRef`.
- `fiber.cancel`: `fiberId`, optional `reason`.

Dispatch responses MUST be deterministic and include:

- `dispatchKind`,
- `profileId`,
- `actionId`,
- `semanticDigest`,
- `worldBinding`,
- one fiber witness reference on accepted lifecycle actions.

## 4. World-route binding

Fiber lifecycle actions MUST bind as follows:

| Action | Operation ID | Route family | World ID | Morphism row |
| --- | --- | --- | --- | --- |
| `fiber.spawn` | `op/transport.fiber_spawn` | `route.fiber.lifecycle` | `world.fiber.v1` | `wm.control.fiber.lifecycle` |
| `fiber.join` | `op/transport.fiber_join` | `route.fiber.lifecycle` | `world.fiber.v1` | `wm.control.fiber.lifecycle` |
| `fiber.cancel` | `op/transport.fiber_cancel` | `route.fiber.lifecycle` | `world.fiber.v1` | `wm.control.fiber.lifecycle` |

Route bindings are operational mapping rows and MUST NOT be treated as semantic
admissibility proofs.

## 5. Descent interpretation

Fiber trees can be interpreted as covers/refinements:

1. `spawn` creates local sections on refined contexts,
2. overlap compatibility is checked by existing checker/contract surfaces,
3. `join` is glue-or-obstruction at the control plane boundary,
4. unresolved joins/cancellations remain operational failures, not Gate-class
   substitutions.

## 6. Runtime neutrality

Fiber semantics are runtime-neutral:

- Effect-style structured concurrency MAY be used for control semantics.
- OTP supervision MAY be used for process/runtime execution.
- Other runtimes MAY be used when they preserve the same typed action and
  witness contracts.

Runtime choice MUST NOT change accepted/rejected outcomes for fixed
action+payload inputs at the transport authority boundary.

## 7. Failure classes (minimum)

Implementations SHOULD classify at least:

- `fiber_invalid_payload`
- `fiber_missing_field`
- `transport_unknown_action`

Fiber lifecycle failures MUST remain transport/control-plane failures and MUST
NOT be relabeled as semantic Gate failures.

## 8. Non-bypass rule

Fiber transport actions are adapters over canonical authority lanes.

Implementations MUST NOT:

- write semantic admissibility outcomes directly from fiber runtime,
- bypass instruction/mutation policy boundaries for issue-memory authority,
- treat projection artifacts as authority substitutions.

## 9. Executable surfaces

Current command surfaces:

- `premath transport-check --json`
- `premath transport-dispatch --action fiber.spawn --payload '<json>' --json`
- `premath transport-dispatch --action fiber.join --payload '<json>' --json`
- `premath transport-dispatch --action fiber.cancel --payload '<json>' --json`

These surfaces provide typed transport envelopes and deterministic metadata for
runtime adapters (CLI, MCP, NIF, or RPC wrappers).
