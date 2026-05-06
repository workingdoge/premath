# Design Glossary

Status: draft
Scope: design-level, non-normative

## Terms

`Premath world`
- Full constructor environment: contexts, covers, indexed definables, admissibility checks, witnesses.

`Tusk`
- Downstream runtime/integration layer that consumes Premath checker and witness
  artifacts.

`Tusk unit`
- Recursive local solver with downward spawning and upward summary/obligation/witness return.

`tusk-sigpi`
- Inter-world transport/composition layer.

`SigPi`
- External doctrine for world-to-world composition/transport.

`DoctrineOperationSite`
- Site-shaped map from doctrine declarations to operational entrypoints.
- In this repo: `specs/premath/draft/DOCTRINE-SITE.{md,json}` validated by
  doctrine-site checks (`crates/premath-cli/src/commands/traceability_check.rs`).

`LLM Instruction Doctrine`
- Doctrine-level constraints for typed instruction handling, unknown classification, and deterministic instruction-to-witness binding.
- In this repo: `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`.

`ctx_ref`
- Context/version reference from `ContextProvider` lineage.

`context_id`
- Stable key for context object `Gamma` in world `C`.

`data_head_ref`
- Canonical EventStore head reference for append-only replay state.

`ContextProvider`
- Interface for resolving lineage (`ctx_ref`, parents, snapshots, diffs).

`EventStore`
- Canonical append/read/fold/checkpoint substrate.
- Event identity includes idempotency key material for retry-safe append.

`DomainAdapter`
- Domain interpreter that proposes projection, local states, compatibility evidence, and glue proposals.

`PremathWorld` / `KernelRuntime`
- World-level checker that chooses covers, enforces law checks, and emits Gate witnesses.

`Cover`
- World-owned local decomposition over a context.

`CoverStrategy`
- Adapter-proposed strategy for world cover selection.

`OverlapId`
- World-defined overlap obligation identifier between cover parts.

`QueryProjection`
- Rebuildable read model/index layer.

`PresentationProjection`
- UI/API-facing view model derived from read projections.

`Control policy`
- Scheduling/refinement policy that does not alter admissibility semantics.

`executor_profile`
- Control-plane selector for where checks execute (`local`, `external`, ...).
- Must not change required check semantics or Gate-class outcomes.

`executor_runner`
- Executable adapter used by `executor_profile=external` to provision/target host substrate.
- Responsible for startup/teardown/routing diagnostics; not admissibility semantics.

`intent_id`
- Stable identifier for declared run intent.
- Computed from canonical `IntentSpec`, not raw natural-language text.

`IntentSpec`
- Canonical structured intent representation used for deterministic `intent_id` derivation.

`cover_strategy_digest`
- Deterministic digest of cover-strategy request material used for audit by default.
- May be promoted to identity material under explicit hardening policy.

`normalizer_id`
- Identifier of comparison-relevant normalization behavior.

`policy_digest`
- Digest of all semantic parameters affecting comparability/admissibility.

`GateWitness`
- Local-world admissibility witness (Gate-class failures or accept).

`TransportWitness`
- SigPi-layer witness for cross-world transport compatibility.

`Cheese` / `SqueakCheese`
- Squeak runtime unit for substrate execution/orchestration (local, remote, microvm, etc.).
- Operational transport/runtime object; not a semantic admissibility authority.

`Sheafification/stackification`
- Semantic forcing/validation of descent behavior; not implied by backend choice.
