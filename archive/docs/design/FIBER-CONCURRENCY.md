# Fiber Concurrency (Design)

Status: draft
Scope: design-level, non-normative

## 1. Why this exists

We need an agent-facing concurrency model that is:

- structured (spawn/join/cancel is explicit),
- deterministic at authority boundaries,
- runtime-neutral (not tied to one VM/process model).

Fiber lifecycle gives that shape without creating a second semantic authority
lane.

## 2. Layer split

Fiber concerns belong to control/runtime layers:

- semantic authority: kernel/Gate/checkers,
- operational authority: typed host actions and transport envelopes,
- runtime execution: OTP processes, local fibers, or sidecar workers.

Design rule:

- use effect-style semantics for control flow,
- allow Erlang/OTP or other runtimes as execution backends.

## 3. Current profile

Current typed action set:

- `fiber.spawn`
- `fiber.join`
- `fiber.cancel`

Current world-route binding:

- `route.fiber.lifecycle` -> `world.fiber.v1` via
  `wm.control.fiber.lifecycle`.

Current operation IDs:

- `op/transport.fiber_spawn`
- `op/transport.fiber_join`
- `op/transport.fiber_cancel`

## 4. Transport contract

The transport envelope is the stable interoperability boundary:

- request: `action + payload`
- response: `result`, `failureClasses`, `worldBinding`,
  `dispatchKind/profileId/actionId/semanticDigest`, and `fiberWitnessRef` on
  accepted lifecycle actions.

Adapters (CLI/MCP/NIF/RPC) should reuse this contract directly.

## 5. Descent reading

A fiber tree is operationally read as a cover/refinement tree:

- child fibers are local sections over refined contexts,
- join is the glue boundary,
- unresolved join or cancellation is an operational obstruction.

This is descent-shaped orchestration, not semantic admissibility by itself.

## 6. Relationship to harness

Harness keeps final mutation/witness authority:

- issue lease and instruction policy stay canonical,
- fiber dispatch rows are additional operational witnesses,
- failure handling remains fail-closed.

## 7. What this is not

- not a replacement for world registry semantics,
- not a replacement for issue lease protocol,
- not a bypass around instruction-linked mutation policy.
