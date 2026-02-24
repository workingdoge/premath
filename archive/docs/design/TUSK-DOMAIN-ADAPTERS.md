# Tusk Domain Adapters

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Tusk keeps Premath generic by separating:

- kernel semantics (reindexing + descent),
- context/version lineage,
- replayable memory substrate,
- domain interpretation,
- control policy.

Beads is not the kernel. Beads is one domain interpretation.

## 2. World vs adapter responsibility

`DomainAdapter` responsibilities:

- propose domain projection,
- propose cover strategy,
- produce local states,
- produce overlap compatibility evidence,
- propose glue structures.

`PremathWorld` / `KernelRuntime` responsibilities:

- instantiate admissible covers,
- define overlap obligations,
- judge locality/descent/contractibility/refinement invariance,
- emit Gate-class witnesses.

Adapter proposals are never self-admissibility proofs.

## 3. Premath world assembly

A Premath world is a full constructor shape with:

- contexts + covers,
- indexed definables,
- reindexing structure,
- optional adjoint structure,
- law-checking + witness emission.

Adapters provide host/domain payloads for this world to check.

## 4. Reference split

Use explicit refs:

- `context_id` for the context object (`Gamma`) key in `C`.
- `ctx_ref` for context/version lineage.
- `data_head_ref` for EventStore progression.

Avoid ambiguous `head_ref` in new contracts.

## 5. Generic memory substrate

EventStore is canonical write surface.

Required properties:

- deterministic event identity,
- event identity includes idempotency key material,
- monotone `data_head_ref`,
- linearizable append semantics with deterministic total order,
- deterministic replay/fold,
- causal provenance links.

Logical interface:

```text
append(events, at_data_head_ref) -> data_head_ref
read(range_or_filter, at_data_head_ref) -> event_stream
fold(event_stream, reducer_id) -> state_snapshot
checkpoint(data_head_ref) -> snapshot_ref
```

## 6. Context/version abstraction

ContextProvider abstraction (JJ is one implementation):

```text
resolve_context_id(scope) -> context_id
resolve_ctx_ref(context_id, scope) -> ctx_ref
parents(ctx_ref) -> list<ctx_ref>
snapshot(ctx_ref) -> ContextSnapshot
diff(ctx_ref_a, ctx_ref_b) -> ContextDelta
```

## 7. Domain adapter contract

Logical interface:

```text
adapter_id() -> string
adapter_version() -> string

project(context_id, ctx_ref, data_head_ref, event_stream) -> DomainProjection
cover_strategy(projection, intent) -> CoverStrategy
restrict(projection, cover_part_id) -> LocalState
compatibility(local_i, local_j, overlap_id) -> CompatWitness
propose_glue(core) -> GlueProposalSet

encode_intent(domain_command) -> EventBatch
summarize(glue_result) -> Summary
obligations(glue_result) -> ObligationSet
```

Rules:

- `project` must be deterministic for fixed inputs.
- adapter must not mutate history directly.
- adapter must not invent world coverage.
- adapter must not select final glue semantics.

## 8. World cover ownership

`Cover` is world-owned doctrine, not adapter-owned optimization state.

Recommended split:

- adapter: `cover_strategy(...)`
- world: `choose_cover(context_id, strategy) -> Cover`
- world/audit layer may emit `cover_strategy_digest` for trace diagnostics

World also owns overlap enumeration used in admissibility checks.

World-owned glue selection:

- adapter proposes `GlueProposalSet`,
- world selects `GlueResult` (or emits Gate failures such as `descent_failure` / `glue_non_contractible`),
- adapter computes summaries/obligations from world-selected `GlueResult`.
- adapter must not compute summaries/obligations from raw proposals.

## 9. DescentCore/DescentPack boundary

Adapters should emit `DescentCore` and `GlueProposalSet` structures (defined in `TUSK-DESCENT-PACKS.md`).
`DescentPack` is the assembled trace artifact (`DescentCore + GlueProposalSet`).

World checks `DescentCore` against kernel laws and emits witnesses.

## 10. Interface ownership

Recommended crate ownership:

- `premath-kernel`: laws + witness semantics,
- `premath-tusk` / `tusk-core`: integration interfaces,
- adapter crates: concrete implementations,
- projection crates: query/presentation views.

## 11. Domain examples

Task graph adapter:

- entities: tasks/dependencies/messages,
- locals: partitioned subgraphs,
- compat: boundary consistency,
- glue: global graph assembly.

Accounting adapter:

- entities: journals/accounts/postings,
- locals: partitioned posting sets,
- compat: overlap balance/identity consistency,
- glue: global journal/ledger assembly.

## 12. Control policy placement

GTD/scheduling/swarm strategies belong to control plane.

They may select intent and refinement order.
They may not alter admissibility laws or write derived projections directly.
