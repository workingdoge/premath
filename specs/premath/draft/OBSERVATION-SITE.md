---
slug: draft
shortname: OBSERVATION-SITE
title: workingdoge.com/premath/OBSERVATION-SITE
name: Observation Site Instantiation and Projection Surface Taxonomy
status: draft
category: Standards Track
tags:
  - premath
  - site
  - observation
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

This spec instantiates `draft/OBSERVATION-INF` for the observation site
concern. It defines the site object, query algebra, build contract, and
projection surface taxonomy.

This spec instantiates `draft/OBSERVATION-INF` in the same way that
`draft/CHANGE-SITE` instantiates `draft/CHANGE-INF`.

Normative when capability `capabilities.observation_semantics` is claimed.

Dependencies:

- `draft/OBSERVATION-INF` (functor structure, state derivation lattice,
  coherence discipline, non-authority boundary),
- `draft/DOCTRINE-SITE` (site object conventions, operation registry, GC1),
- `draft/DOCTRINE-INF` (morphism classes).

## 2. Site object

```text
ObservationSite = (Nodes, Covers, Edges, Entrypoints)
```

### 2.1 Nodes

- `observation-doctrine`: root node representing the observation doctrine
  concern.
- `witness-sources`: node for the witness family `W` (CI witness artifacts
  indexed by source and run).
- `issue-memory`: node for issue memory `I` (issue records with lifecycle
  state and dependency structure).
- `coherence`: node for the coherence sub-projection family (§4 of
  OBSERVATION-INF).

### 2.2 Edges

Edges are doctrine-preserving from `observation-doctrine` to operational
surface nodes:

- `observation-doctrine → witness-sources`: carries morphisms
  `[dm.identity, dm.presentation.projection]`.
- `observation-doctrine → issue-memory`: carries morphisms
  `[dm.identity, dm.presentation.projection]`.
- `observation-doctrine → coherence`: carries morphisms
  `[dm.identity, dm.presentation.projection]`.

All edges MUST carry only morphisms that observation preserves (per
OBSERVATION-INF §9). No edge MAY carry `dm.policy.rebind`,
`dm.transport.world`, or `dm.commitment.attest`.

### 2.3 Covers

- A base cover over `observation-doctrine` with parts
  `[witness-sources, issue-memory, coherence]`.

### 2.4 Entrypoints

- **Checker entrypoint**: the canonical observation checker
  (`observation.check`; see §7).
- **Query entrypoints**: the query algebra surfaces (§4).

## 3. Build contract

```text
build : (RepoRoot, WitnessDir, IssueMemory?) → Surface
```

### 3.1 Inputs

- `RepoRoot: path` — repository root directory.
- `WitnessDir: path` — CI witness directory containing witness artifacts.
- `IssueMemory: path? (optional)` — path to issue memory file. When absent,
  the build MUST proceed with an empty issue memory (all issue-derived state
  is `empty`).

### 3.2 Build steps

1. **Load.** Read witness artifacts from `WitnessDir`. Read issue records from
   `IssueMemory` (if provided). Validate input schemas. On schema failure,
   reject with `observation_schema_invalid` (OBSERVATION-INF §8).

2. **Normalize.** Normalize input state to canonical form. Witness artifacts
   MUST be keyed by `(source_id, run_id)`. Issue records MUST be keyed by
   issue ID. On normalization failure, reject with
   `observation_build_normalize_failed`.

3. **Derive state.** For each concern, derive the observation state using the
   state derivation lattice (OBSERVATION-INF §3). The fold MUST respect
   the priority ordering `decision > required > instruction > empty` with
   stable tie-breaking.

4. **Compute coherence.** Evaluate each sub-projection coherence check
   (OBSERVATION-INF §4). Record coherence results per sub-projection
   family.

5. **Compose surface.** Assemble the observation surface from derived states
   and coherence results. Compute `needsAttention` and `topFailureClass`
   (OBSERVATION-INF §5).

### 3.3 Determinism

For identical inputs `(RepoRoot, WitnessDir, IssueMemory)`, the build MUST
produce an identical surface. This is the instantiation of
OBSERVATION-INF §2 determinism for the build contract.

## 4. Query algebra

A closed vocabulary of read-only queries:

```text
Query =
  | Latest
  | NeedsAttention
  | Instruction(id)
  | Projection(digest, mode)
```

Each query is a projection from the surface — no mutation, no side effects.

### 4.1 `Latest`

Returns the most recently built observation surface. Response:

```text
LatestResponse {
  surface: Surface,
  buildTimestamp: string (RFC 3339),
  inputDigests: { witnesses: string, issues: string? }
}
```

### 4.2 `NeedsAttention`

Returns the attention derivation (OBSERVATION-INF §5). Response:

```text
NeedsAttentionResponse {
  needsAttention: bool,
  topFailureClass: string?,
  failureCount: number,
  coherenceFailures: list<string>
}
```

### 4.3 `Instruction(id)`

Returns the observation state for a specific instruction by ID. Response:

```text
InstructionResponse {
  instructionId: string,
  state: State,
  evidenceSources: list<EvidenceSource>,
  derivationTrace: string
}
```

When the instruction ID is not found, the response MUST indicate
`state = empty` with an empty evidence source list.

### 4.4 `Projection(digest, mode)`

Returns a specific projection surface matched by digest. `mode` is one of
`typed` (default) or `compatibility_alias` (per OBSERVATION-INF §6).
Response:

```text
ProjectionResponse {
  matched: bool,
  matchMode: "typed" | "compatibility_alias",
  projection: ProjectionSurface?,
  digest: string
}
```

When no match is found, `matched` MUST be `false` and `projection` MUST be
`null`.

## 5. Projection surface taxonomy

Three projection surfaces, all read-only:

### 5.1 Structured query surface

The query algebra (§4) as a programmatic API. Consumers invoke queries and
receive typed responses. This is the primary surface for machine consumers.

### 5.2 HTTP projection surface

An HTTP endpoint surface projecting the observation surface:

- **Routes.** Each query variant (§4) maps to an HTTP route.
- **Error mapping.** Failure classes (OBSERVATION-INF §8) map to HTTP
  status codes. Schema violations map to `400`. Build failures map to `500`.
  Projection mismatches map to `404` or `409` depending on the mismatch type.
- **CORS.** The HTTP surface MUST support configurable CORS headers for
  cross-origin consumers.

### 5.3 Tool projection surface

An MCP (or equivalent tool protocol) surface delegating to the query algebra.
Each query variant (§4) is exposed as a tool invocation. The tool surface
MUST delegate to the same underlying query algebra — it MUST NOT implement
independent query logic.

### 5.4 Surface agreement

All three projection surfaces MUST agree on the same underlying observation
surface. That is, for the same inputs and build state, querying via the
structured API, HTTP endpoint, or tool surface MUST return equivalent results.

## 6. Event projection

The observation surface MUST project to a deterministic event sequence for
downstream consumers.

### 6.1 Event kinds

- **Per-source summary.** One event per witness source, summarizing the derived
  state and evidence sources for that source.
- **Surface-level summary.** One event summarizing the total surface state,
  `needsAttention`, and `topFailureClass`.
- **Coherence events.** One event per sub-projection coherence check result
  (pass or fail with diagnostic).

### 6.2 Ordering

Event ordering MUST be stable for fixed inputs. The ordering is:

1. Per-source summaries in lexicographic order by source identifier.
2. Surface-level summary.
3. Coherence events in sub-projection family order (per OBSERVATION-INF
   §4.1).

## 7. Checker contract

The canonical observation checker builds a surface from source artifacts,
compares against a tracked surface, and validates schema invariants.

### 7.1 Checker steps

1. Build a fresh surface from source artifacts using the build contract (§3).
2. Load the tracked (previously built) surface.
3. Compare the fresh surface against the tracked surface using the projection
   match discipline (OBSERVATION-INF §6).
4. Validate schema invariants on the fresh surface.
5. Report results with failure classes per OBSERVATION-INF §8.

### 7.2 Exit codes

- `0`: accepted — fresh surface matches tracked surface and all schema
  invariants hold.
- `1`: rejected — mismatch or schema violation detected. Failure classes are
  reported per OBSERVATION-INF §8.
- `2`: invalid input — source artifacts could not be loaded or parsed.

## 8. Doctrine-site routing

Observation operations register in the doctrine operation registry as
`route_bound` with morphisms `[dm.identity, dm.presentation.projection]`.

### 8.1 Operation registration

Observation operations MUST be registered in the doctrine operation registry
(`contracts/DOCTRINE-OP-REGISTRY.json`) with:

- `operationClass`: `route_bound` for checker and query surfaces.
- `morphisms`: `[dm.identity, dm.presentation.projection]`.
- `routeEligibility`: following `draft/DOCTRINE-SITE` conventions.

### 8.2 World-route binding

World-route binding follows `draft/DOCTRINE-SITE` conventions:

- Observation operations bind to the observation route family.
- Route-family binding MUST satisfy GC1 (total binding per
  `draft/DOCTRINE-SITE` §3.5).
- Non-route-eligible surfaces (e.g., informational projections) MUST be
  registered as `read_only_projection` and MUST NOT appear in world-route
  bindings.

## 9. Non-goals

This document does not prescribe:

- UI or frontend rendering of observation surfaces,
- event streaming infrastructure or pub/sub mechanisms,
- caching, refresh, or invalidation strategy,
- mutation semantics or write operations.

It prescribes only the site object, query algebra, build contract, projection
surface taxonomy, and doctrine-site integration for observation.
