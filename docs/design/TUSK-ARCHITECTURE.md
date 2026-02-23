# Tusk Architecture

Status: draft
Scope: design-level, non-normative

## 1. Premath-first framing

Tusk operates over the Premath kernel shape:

- a context category `C`,
- a total space of definables `E`,
- projection `p0: E -> C`.

For an indexed assignment `Def: C^op -> V`, the Grothendieck construction gives `p0: Integral(Def) -> C`.

A presheaf-like assignment becomes Premath-admissible only when descent and contractible uniqueness obligations are discharged.

Tusk does not redefine Premath laws. It operationalizes them.

## 2. Context and reference model

Tusk uses explicit context identity and two references:

- `context_id`: stable key for the context object `Gamma` in `C`.
- `ctx_ref`: context/version lineage pointer used to materialize that context.
- `data_head_ref`: append-only EventStore head.

Run base is always:

`RunBase = (world_id, context_id, ctx_ref, data_head_ref)`

`context_id` and `ctx_ref` are related but not interchangeable.

## 3. What Tusk is (and is not)

Tusk is an instantiation/runtime layer for a Premath world.

- It owns integration contracts across context/version, memory, domain adapters, projections, control, and witnessing.
- It does not replace `PREMATH-KERNEL`, `GATE`, or `BIDIR-DESCENT`.

## 4. Tusk and SigPi boundary

SigPi describes composition and transport between Premath worlds.

Tusk describes execution inside one world instance.

Non-bypass rule:

- SigPi transport never creates local admissibility.
- Transported artifacts must pass destination-world admissibility checks under destination policy bindings.

## 5. Recursive unit model

A Tusk unit is self-similar at every scale:

- function call,
- subtask,
- swarm subgraph,
- whole workflow.

Each unit has two interfaces:

- downward: spawn local problems under a cover,
- upward: return summaries, obligations, and witnesses.

## 6. Planes and responsibilities

### 6.1 Semantic plane

- Premath law checks and witnesses.
- Optional KCIR/commitment checkpoints.

### 6.2 Context/version plane

- head lineage, parent links, snapshots, diffs.
- backend implementations may use JJ, Git, or others.

### 6.3 Memory/data plane

- append-oriented event substrate,
- deterministic replay/reduction,
- provenance and idempotency fields.

### 6.4 Domain plane

- domain adapters produce domain projections, `DescentCore`, and glue proposals.
- examples: task graph, accounting.

### 6.5 Query projection plane

- rebuildable indexes/materialized views.
- no canonical writes.

### 6.6 Presentation projection plane

- UI/API views over query projections or direct read models.
- read-side only; writes route through command surfaces to EventStore.

### 6.7 Control plane

- planning, scheduling, refinement policy.
- emits intents/events only.
- never mutates projections directly.

### 6.8 Runtime substrate boundary (Squeak-owned)

- runtime placement/orchestration (`Cheese` profiles such as `local`,
  `microvm`, `remote`) is owned by Squeak/SigPi site contracts.
- Tusk consumes destination-local execution context after Squeak placement and
  performs local admissibility checks.

## 7. Authority split

`DomainAdapter` may propose structure and evidence.
`PremathWorld`/`KernelRuntime` decides admissibility and emits Gate-class failures.

This prevents adapters from smuggling merge rules as semantic facts.

### 7.1 Layering note: Premath vs KCIR

Premath kernel semantics and KCIR representation are separate layers.

- `premath-kernel`: semantic laws, admissibility checks, witness interfaces.
- `kcir-core` (optional): canonical IR, normalization, commitment/proof payloads.
- `premath-kcir` bridge (optional): profile that maps Premath witness interfaces to KCIR artifacts.
- `tusk-core`: uses kernel interfaces and optionally loads bridge capabilities.

Dependency rule:

- kernel does not depend on KCIR,
- bridge depends on kernel + KCIR,
- runtime remains correct without KCIR profiles.

## 8. Core interface catalog

`ContextProvider`:

```text
resolve_context_id(scope) -> context_id
resolve_ctx_ref(context_id, scope) -> ctx_ref
parents(ctx_ref) -> list<ctx_ref>
snapshot(ctx_ref) -> ContextSnapshot
diff(ctx_ref_a, ctx_ref_b) -> ContextDelta
```

`EventStore`:

```text
append(events, at_data_head_ref) -> data_head_ref
read(range_or_filter, at_data_head_ref) -> event_stream
fold(event_stream, reducer_id) -> state_snapshot
checkpoint(data_head_ref) -> snapshot_ref
```

EventStore concurrency contract (v0):

- append is linearizable and contributes to a deterministic total event order,
- `at_data_head_ref` is CAS-like precondition material,
- CAS mismatch is a control-plane diagnostic, not a Gate failure.
- deterministic replay is defined over `(data_head_ref, reducer_id)`.
- event identity must include an idempotency key so retries do not create semantically distinct histories.

`DomainAdapter`:

```text
project(context_id, ctx_ref, data_head_ref, event_stream) -> DomainProjection
cover_strategy(projection, intent) -> CoverStrategy
restrict(projection, cover_part_id) -> LocalState
compatibility(local_i, local_j, overlap_id) -> CompatWitness
propose_glue(core) -> GlueProposalSet
encode_intent(domain_command) -> EventBatch
```

`PremathWorld` / `KernelRuntime`:

```text
choose_cover(context_id, cover_strategy) -> Cover
materialize_overlaps(cover, overlap_level) -> OverlapSet
check_descent_core(core, overlap_level) -> GateWitnessSet
select_glue(glue_proposals, mode) -> GlueResult | GlueSelectionFailure
```

Overlap-level requirements and v0 defaults are specified in `TUSK-DESCENT-PACKS.md`.
`GlueSelectionFailure` should map to Gate-compatible failure classes (`descent_failure`, `glue_non_contractible`) with diagnostics.

`QueryProjection`:

```text
rebuild(data_head_ref, projection_spec) -> ViewRef
query(view_ref, expr) -> rows
```

`PresentationProjection`:

```text
render(view_ref, view_model_spec) -> ViewModel
stream(view_ref, cursor) -> ViewDelta
```

`PolicyEngine`:

```text
choose_intent(state, policy) -> (intent_id, intent)
choose_refinement(run_state, failures) -> refinement_step
```

`WitnessEmitter`:

```text
emit_gate_witness(...) -> GateWitness
emit_transport_witness(...) -> TransportWitness
finalize_witness_bundle(...) -> WitnessBundle
```

Runtime placement interfaces and location-cover glue are specified in Squeak
(`raw/SQUEAK-CORE`, `raw/SQUEAK-SITE`), not as Tusk semantic authorities.

## 9. Unit lifecycle contract

```text
open(scope, intent, policy) -> (world_id, context_id, intent_id, ctx_ref, data_head_ref, ContextPack)
choose_cover(ContextPack, strategy) -> Cover
spawn(Cover) -> ChildUnits
collect(ChildUnits) -> DescentCore
check_descent(DescentCore, overlap_level) -> GateWitnessSet
propose_glue(DescentCore) -> GlueProposalSet
assemble_descent_pack(DescentCore, GlueProposalSet) -> DescentPack
select_glue(GlueProposalSet, mode) -> GlueResult | GlueSelectionFailure
close(GlueResult, GateWitnessSet) -> (UpwardSummary, UpwardObligations, WitnessBundle)
```

Harness overlay:
- operational long-running harness hooks (`boot/step/stop`) and durability/
  trajectory contracts are defined in `TUSK-HARNESS-CONTRACT.md`.
- this overlay must remain non-authoritative for semantic admissibility.

## 10. Identity and determinism

Run identity and deterministic ID material are defined in `TUSK-IDENTITY.md`.

Minimum identity fields include:

- `world_id`, `unit_id`, `parent_unit_id`, `context_id`, `intent_id`, `cover_id`,
- `ctx_ref`, `data_head_ref`,
- `adapter_id`, `adapter_version`,
- `policy_digest`, `normalizer_id`.

`cover_strategy_digest` is audit material by default and becomes identity material only under explicit hardening policy.

## 11. Failure model

Local semantic failures should map to Gate classes.
Cross-world failures should map to SigPi transport classes.

Control-plane failures remain diagnostics unless they imply semantic law failures.

## 12. Why this architecture

- Kernel semantics remain stable.
- Domain semantics evolve independently.
- Context/version backends are swappable.
- UIs remain disposable projections.
- Multi-agent operation remains replayable and auditable.
