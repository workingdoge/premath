---
slug: draft
shortname: CHANGE-SITE
title: workingdoge.com/premath/CHANGE-SITE
name: Doctrine-Site Change Morphisms
status: draft
category: Standards Track
tags:
  - premath
  - site
  - change-management
  - doctrine
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

This spec defines the category of doctrine-site changes. Each morphism is a
typed mutation on `SITE-PACKAGE.json` with descent conditions.

This spec instantiates `draft/CHANGE-INF` for the doctrine-site concern,
in the same way that `draft/DOCTRINE-SITE` instantiates `draft/DOCTRINE-INF`.

Normative when capability `capabilities.site_change` is claimed.

Dependencies:

- `draft/CHANGE-INF` (category structure, commuting-square rule),
- `draft/DOCTRINE-SITE` (site object, canonical map artifacts, GC1),
- `draft/DOCTRINE-INF` (morphism classes).

## 2. SiteMutation vocabulary

A `SiteMutation` is an atomic operation on a `SITE-PACKAGE.json`. The closed
vocabulary is:

```text
SiteMutation =
  | AddNode(id, path, kind, requiresDeclaration)
  | RemoveNode(id)
  | AddEdge(id, from, to, morphisms)
  | RemoveEdge(id)
  | AddCover(id, over, parts)
  | RemoveCover(id)
  | UpdateCover(id, parts)
  | AddOperation(id, edgeId, path, kind, morphisms, operationClass,
                 routeEligibility?)
  | RemoveOperation(id)
  | ReparentOperations(newParentNodeId)
  | UpdateBaseCoverParts(parts)
  | UpdateWorldRouteBinding(routeFamilyId, operationIds)
```

Each variant is defined below.

### 2.1 Node mutations

- `AddNode(id, path, kind, requiresDeclaration)`: Add a node to `site.nodes`.
  Precondition: no node with `id` exists.
- `RemoveNode(id)`: Remove a node from `site.nodes`.
  Precondition: node with `id` exists. No edge or cover references `id`.

### 2.2 Edge mutations

- `AddEdge(id, from, to, morphisms)`: Add an edge to `site.edges`.
  Precondition: no edge with `id` exists. Both `from` and `to` nodes exist.
  All `morphisms` MUST be valid `draft/DOCTRINE-INF` morphism class IDs.
- `RemoveEdge(id)`: Remove an edge from `site.edges`.
  Precondition: edge with `id` exists. No operation references `edgeId = id`.

### 2.3 Cover mutations

- `AddCover(id, over, parts)`: Add a cover to `site.covers`.
  Precondition: no cover with `id` exists. Node `over` exists. All `parts`
  nodes exist.
- `RemoveCover(id)`: Remove a cover from `site.covers`.
  Precondition: cover with `id` exists.
- `UpdateCover(id, parts)`: Replace the parts of an existing cover.
  Precondition: cover with `id` exists. All `parts` nodes exist.

### 2.4 Operation mutations

- `AddOperation(id, edgeId, path, kind, morphisms, operationClass,
  routeEligibility?)`: Add an operation to
  `operationRegistry.operations`. Precondition: no operation with `id` exists.
  Edge `edgeId` exists. `operationClass` MUST be one of `route_bound`,
  `read_only_projection`, `tooling_only`. When `operationClass = route_bound`,
  `routeEligibility` MUST be provided and MUST satisfy `draft/DOCTRINE-SITE`
  §3.5 GC1.
- `RemoveOperation(id)`: Remove an operation from the registry.
  Precondition: operation with `id` exists.

### 2.5 Registry-level mutations

- `ReparentOperations(newParentNodeId)`: Change
  `operationRegistry.parentNodeId`. Precondition: node `newParentNodeId` exists.
- `UpdateBaseCoverParts(parts)`: Replace
  `operationRegistry.baseCoverParts`. All `parts` nodes MUST exist.
- `UpdateWorldRouteBinding(routeFamilyId, operationIds)`: Add or replace
  a row in `worldRouteBindings.rows` for the given `routeFamilyId`.
  All `operationIds` MUST reference existing `route_bound` operations.

## 3. SiteChangeRequest

The morphism in the change category for the doctrine-site concern:

```text
SiteChangeRequest {
  schema: 1,
  changeKind: "premath.site_change.v1",
  changeId: string,
  concernId: "doctrine_site_topology",
  fromDigest: string,
  toDigest: string,
  morphismKind: "vertical" | "horizontal" | "mixed",
  mutations: list<SiteMutation>,
  preservationClaims: list<string>,
}
```

Field semantics:

- `changeId`: deterministic from canonical serialization of the request
  excluding debug-only payloads. Computed as SHA256 of the canonical JSON
  encoding of `(changeKind, concernId, fromDigest, mutations,
  preservationClaims)`.
- `concernId`: always `"doctrine_site_topology"` for this spec.
- `fromDigest`: SHA256 of the source `SITE-PACKAGE.json` before mutations.
- `toDigest`: SHA256 of the target `SITE-PACKAGE.json` after mutations.
  Computed after apply, not declared by the submitter.
- `morphismKind`: classification per `draft/CHANGE-INF` §5.
  - `vertical`: mutations affect only operations/edges (fibre) while nodes
    (base) remain unchanged.
  - `horizontal`: mutations affect nodes (base) and edges/operations adjust
    accordingly.
  - `mixed`: both base and fibre change without clean vertical/horizontal
    decomposition.
- `mutations`: ordered list of `SiteMutation` values from §2.
- `preservationClaims`: subset of `draft/CHANGE-INF` §6 claim IDs
  that this change asserts it preserves.

A `SiteChangeRequest` is a `ChangeRecord` (per `draft/CHANGE-INF` §3)
with:

- `concernId = "doctrine_site_topology"`,
- `normativeRef` = path to `SITE-PACKAGE.json`,
- `fromRef = fromDigest`, `toRef = toDigest`,
- `contextMapRef`, `totalMapRef` derived from mutation application.

## 4. Apply semantics

`apply(package, request) → (package', witness)`:

1. **Digest validation.** Compute SHA256 of `package` (canonical JSON). If it
   does not equal `request.fromDigest`, reject with
   `site_change_digest_mismatch`.

2. **Sequential mutation application.** For each mutation in
   `request.mutations`, in order:
   - Validate the mutation's preconditions against the current state.
   - If any precondition fails, reject with the appropriate failure class
     (see §7) and stop. No partial application.
   - Apply the mutation to produce the next state.

3. **Post-condition validation.** After all mutations:
   - Reachability: every operation node MUST have a directed path from the
     doctrine root (`draft/DOCTRINE-INF`). Reject unreachable operations with
     `site_change_operation_unreachable`.
   - GC1 total-binding: every `route_bound` operation with
     `worldRouteRequired=true` MUST appear in exactly one
     `worldRouteBindings` row. Reject violations per `draft/DOCTRINE-SITE` §3.5.

4. **Digest computation.** Compute SHA256 of the resulting `package'`
   (canonical JSON). Set `request.toDigest` to this value.

5. **Commutation check.** Validate the commuting-square rule per
   `draft/CHANGE-INF` §4. Set `commutationCheck` to `accepted` or
   `rejected`.

6. **Return.** `(package', witness)` where `witness` includes the computed
   digests, mutation trace, and commutation verdict.

Order matters: mutations within a single request are applied sequentially.
This is intentional — `AddNode` followed by `AddEdge` referencing that node
is valid; the reverse is not.

## 5. Composition semantics

`compose(r1, r2) → r12`:

Given two `SiteChangeRequest` values `r1` and `r2`:

1. **Composability check.** `r1.toDigest` MUST equal `r2.fromDigest`.
   If not, the requests are not composable.

2. **Mutation concatenation.** `r12.mutations = r1.mutations ++ r2.mutations`.

3. **Digest propagation.** `r12.fromDigest = r1.fromDigest`,
   `r12.toDigest = r2.toDigest`.

4. **Preservation claims.** `r12.preservationClaims` = intersection of
   `r1.preservationClaims` and `r2.preservationClaims`. Both components must
   hold for the composition to claim a preservation.

5. **Morphism kind.** If `r1.morphismKind = r2.morphismKind`, inherit.
   Otherwise `r12.morphismKind = "mixed"`.

6. **Obstruction check.** Validate `r12` against the `r1.fromDigest` state:
   apply the composed mutation list from `r1.fromDigest`. If this fails, the
   composition has an obstruction (see §7).

7. **Change ID.** `r12.changeId` is computed from the composed request's
   canonical content per §3 rules.

Composition satisfies associativity per `draft/CHANGE-INF` §3.1:
`compose(compose(r1, r2), r3) = compose(r1, compose(r2, r3))` because
mutation concatenation is associative and digest threading is transitive.

## 6. Descent on composed changes

A composed `SiteChangeRequest` satisfies descent iff:

1. **Component validity.** Each component `ri` applies cleanly from its
   `fromDigest` state.

2. **Composition validity.** The composition `r12` applies cleanly from
   `r1.fromDigest`.

3. **Glue witness existence.** Applying the composition produces the same
   `toDigest` as applying components sequentially. That is:
   `apply(apply(package, r1).package', r2).toDigest = apply(package, r12).toDigest`.

4. **Contractibility.** There is exactly one way to decompose the composition
   into the given components — no ambiguity in the factorization. When two
   different component decompositions produce the same composed result, they
   MUST be equivalent (same mutation sequences modulo identity mutations).

When descent fails, the composition MUST be rejected. The glue witness is
the proof that sequential application and composed application agree.

This is the descent condition on the change fibre: a cover of a composed
change by component changes satisfies descent iff each component has accepted
commutation and the glue (composition witness) exists and is contractible.

## 7. Failure classes

Implementations MUST use the following failure class identifiers:

```text
site_change_digest_mismatch        — fromDigest doesn't match current state
site_change_node_not_found         — RemoveNode on nonexistent node
site_change_node_already_exists    — AddNode with existing id
site_change_edge_not_found         — RemoveEdge on nonexistent edge
site_change_edge_dangling          — edge references nonexistent node
site_change_cover_not_found        — RemoveCover/UpdateCover on nonexistent cover
site_change_cover_part_missing     — cover references nonexistent node
site_change_operation_not_found    — RemoveOperation on nonexistent operation
site_change_operation_already_exists — AddOperation with existing id
site_change_operation_unreachable  — operation has no path from doctrine root
site_change_route_binding_invalid  — route_bound operation missing routeEligibility
                                     or GC1 violation
site_change_morphism_id_unknown    — edge morphism not in DOCTRINE-INF
site_change_composition_dangling   — composed mutations leave dangling refs
site_change_composition_ambiguous  — overlapping mutations on same object
site_change_glue_obstruction       — sequential apply ≠ composed apply
```

Each failure MUST include:

- `class`: one of the above identifiers,
- `message`: human-readable diagnostic.

Implementations MUST be fail-closed: any failure aborts the entire request.

## 8. Self-application

The `SiteChangeRequest` that registers `op/site.apply_change` in the doctrine
graph MUST itself be expressible as a valid `SiteChangeRequest` with accepted
commutation.

This is the structural fixed-point requirement: the operation that applies
site changes must be installable via the same mechanism it implements.

Concretely, the bootstrap `SiteChangeRequest` MUST:

1. `AddNode` for any new nodes required by the operation (if not already
   present).
2. `AddEdge` from the appropriate doctrine ancestor to the operation.
3. `AddOperation` for `op/site.apply_change` with:
   - `operationClass = "route_bound"`,
   - `routeEligibility.routeFamilyId = "route.site_change"`,
   - `morphisms` including `dm.identity`, `dm.profile.execution`,
     `dm.commitment.attest`.
4. `UpdateWorldRouteBinding` for `route.site_change` binding the new operation.

The bootstrap request MUST apply cleanly and produce accepted commutation
when run through the `site.apply_change` transport action itself.

## 9. Transportability

When a downstream site pulls back from this site's `SITE-PACKAGE.json`, a
`SiteChangeRequest` on the upstream induces a `SiteChangeRequest` on the
downstream fibre via the pullback functor.

Requirements:

1. **Pullback preservation.** For a pullback `p: Downstream → Upstream`,
   a `SiteChangeRequest` `r` on Upstream induces `p*(r)` on Downstream.
   `p*(r)` MUST be a valid `SiteChangeRequest` on the downstream
   `SITE-PACKAGE.json`.

2. **Descent preservation.** If `r` satisfies descent (§6), then `p*(r)` MUST
   satisfy descent on the downstream fibre.

3. **Functoriality.** `p*(compose(r1, r2)) = compose(p*(r1), p*(r2))`.
   The pullback functor preserves composition.

4. **Failure propagation.** If `r` is rejected upstream, `p*(r)` MUST also
   be rejected downstream. Failure classes propagate through the pullback.

## 10. Operation surface: `site.apply_change`

### 10.1 Transport action contract

Request shape:

```json
{
  "action": "site.apply_change",
  "payload": {
    "changeRequest": "<SiteChangeRequest>",
    "repoRoot": "string (optional)",
    "dryRun": "bool (optional, default false)"
  }
}
```

Response shape (accepted):

```json
{
  "result": "accepted",
  "changeId": "string",
  "fromDigest": "string",
  "toDigest": "string",
  "commutationCheck": "accepted",
  "artifactDigests": {
    "siteInput": "string",
    "siteMap": "string",
    "operationRegistry": "string"
  },
  "witnessRefs": ["string"]
}
```

Response shape (rejected):

```json
{
  "result": "rejected",
  "failureClasses": ["string"],
  "diagnostics": [
    { "class": "string", "message": "string" }
  ]
}
```

When `dryRun = true`, the implementation MUST validate and compute digests but
MUST NOT write any artifacts to disk.

### 10.2 World-route binding

The `site.apply_change` operation binds to a world-route family:

- `worldId`: `"world.site_change.v1"`
- `morphismRowId`: `"wm.control.site_change.mutation"`
- `routeFamilyId`: `"route.site_change"`
- `requiredMorphisms`: `["dm.identity", "dm.profile.execution",
  "dm.commitment.attest"]`
- `failureClassUnbound`: `"world_route_unbound"`

This row MUST appear in `worldRouteBindings.rows` after bootstrap (§8).

### 10.3 CLI surface

`premath site-apply --change <path> [--dry-run] [--json]`

Delegates to the `site.apply_change` transport action:

- `--change <path>`: path to a JSON file containing a `SiteChangeRequest`.
- `--dry-run`: sets `dryRun = true` in the payload.
- `--json`: emit response as JSON (default for machine consumers).

Exit codes:

- `0`: accepted.
- `1`: rejected (failure classes in stderr or JSON output).
- `2`: invalid input (malformed request, missing file).

## 11. Non-goals

This document does not prescribe:

- branch or forge policy for change requests,
- review workflow for approving changes,
- specific serialization format for mutation payloads beyond JSON,
- migration strategy from manual site-package editing to change-request flow.

It prescribes only the category structure, apply/compose semantics, descent
conditions, and operation surface for doctrine-site changes.

## 12. Operation surface: `site.current_digest`

### 12.1 Transport action contract

Read-only projection. Returns the canonical digest and summary counts of the
current `SITE-PACKAGE.json`.

Request shape:

```json
{
  "action": "site.current_digest",
  "payload": {
    "repoRoot": "string (optional)"
  }
}
```

Response shape (accepted):

```json
{
  "result": "accepted",
  "digest": "string",
  "summary": {
    "nodeCount": "number",
    "edgeCount": "number",
    "operationCount": "number",
    "coverCount": "number",
    "worldRouteBindingRowCount": "number"
  }
}
```

Response shape (rejected):

```json
{
  "result": "rejected",
  "failureClasses": ["string"],
  "diagnostic": "string"
}
```

### 12.2 Operation classification

- `operationClass`: `read_only_projection`
- No world-route binding required.

### 12.3 CLI surface

`premath site-digest [--json] [--repo-root <path>]`

Exit codes: `0` accepted, `2` invalid input.

## 13. Operation surface: `site.build_change`

### 13.1 Transport action contract

Constructs a complete, valid `SiteChangeRequest` from a mutation list.
Auto-computes: `fromDigest` (from current package), `toDigest` (tentative
apply on clone), `morphismKind` (from mutation types), `changeId`. Validates
each mutation precondition. Returns the built request — does NOT write.

Request shape:

```json
{
  "action": "site.build_change",
  "payload": {
    "mutations": ["<SiteMutation>"],
    "preservationClaims": ["string (optional)"],
    "repoRoot": "string (optional)"
  }
}
```

Response shape (accepted):

```json
{
  "result": "accepted",
  "changeRequest": "<SiteChangeRequest>",
  "fromDigest": "string",
  "toDigest": "string",
  "morphismKind": "vertical | horizontal | mixed",
  "mutationCount": "number"
}
```

Response shape (rejected):

```json
{
  "result": "rejected",
  "failureClasses": ["string"],
  "diagnostics": [
    { "class": "string", "message": "string" }
  ]
}
```

### 13.2 MorphismKind auto-classification

- `vertical`: mutations only touch operations/edges (fibre), no node mutations.
- `horizontal`: mutations touch nodes (base) but not operations/edges.
- `mixed`: both node and operation/edge mutations present.

### 13.3 World-route binding

- `operationClass`: `route_bound`
- `routeFamilyId`: `"route.site_change"`
- `requiredMorphisms`: `["dm.identity", "dm.profile.execution",
  "dm.commitment.attest"]`

### 13.4 CLI surface

`premath site-build --mutations <path> [--json] [--repo-root <path>]`

- `--mutations <path>`: path to a JSON file containing a mutation list.

Exit codes: `0` accepted, `1` rejected (validation failure), `2` invalid input.

## 14. Operation surface: `site.compose_changes`

### 14.1 Transport action contract

Composes two `SiteChangeRequest` values per §5. Validates composability
(`r1.toDigest == r2.fromDigest`), returns composed request.

Request shape:

```json
{
  "action": "site.compose_changes",
  "payload": {
    "request1": "<SiteChangeRequest>",
    "request2": "<SiteChangeRequest>"
  }
}
```

Response shape (accepted):

```json
{
  "result": "accepted",
  "composedRequest": "<SiteChangeRequest>",
  "fromDigest": "string",
  "toDigest": "string",
  "morphismKind": "vertical | horizontal | mixed",
  "mutationCount": "number"
}
```

Response shape (rejected):

```json
{
  "result": "rejected",
  "failureClasses": ["string"],
  "diagnostic": "string"
}
```

### 14.2 World-route binding

Same as `site.build_change` (§13.3).

### 14.3 CLI surface

`premath site-compose --request1 <path> --request2 <path> [--json]`

Exit codes: `0` accepted, `1` rejected, `2` invalid input.

## 15. Digest-chain enforcement

### 15.1 Change log

`.premath/site-change-log.jsonl` records each applied change as a single
JSONL row:

```json
{
  "changeId": "string",
  "fromDigest": "string",
  "toDigest": "string",
  "timestamp": "string (RFC 3339)"
}
```

The `site.apply_change` transport action MUST append a row on successful
non-dry-run apply.

### 15.2 Chain coherence check

`site-change-chain-check` verifies that the current `SITE-PACKAGE.json`
canonical digest matches the `toDigest` of the last entry in the change log.

### 15.3 Genesis

If the log file is missing or empty, the check seeds from the current state:
it computes the current digest and writes a genesis entry with
`changeId = "genesis"` and `fromDigest = toDigest = <current digest>`.

### 15.4 Failure class

`site_change_chain_break` — current digest does not match last `toDigest`.

### 15.5 CLI surface

`premath site-change-chain-check [--json] [--repo-root <path>]`

Exit codes: `0` chain valid, `1` chain broken, `2` invalid input.

### 15.6 Baseline integration

`site-change-chain-check` MUST be included in `mise run baseline`.
