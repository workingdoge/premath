# Design Glossary

Status: draft
Scope: design-level, non-normative

## Terms

`Premath world`
- Full constructor environment: contexts, covers, indexed definables, admissibility checks, witnesses.

`Tusk`
- Runtime/integration layer that realizes Premath laws in execution.

`Tusk unit`
- Recursive local solver with downward spawning and upward summary/obligation/witness return.

`tusk-core`
- Single-world execution contracts and interfaces.

`tusk-sigpi`
- Inter-world transport/composition layer.

`SigPi`
- External doctrine for world-to-world composition/transport.

`DoctrineOperationSite`
- Site-shaped map from doctrine declarations to operational entrypoints.
- In this repo: `specs/premath/draft/DOCTRINE-SITE.{md,json}` validated by `tools/conformance/check_doctrine_site.py`.

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

`DescentCore`
- Core presheaf-like package of locals, overlap evidence, and mode (no glue proposals).

`DescentPack`
- `DescentCore` plus `GlueProposalSet`.

`GlueProposal`
- Adapter-proposed global assembly from local states.

`GlueResult`
- World-selected global result under declared mode (or explicit non-contractibility failure).

`GlueSelectionFailure`
- World-side glue selection failure mapped to `descent_failure` or `glue_non_contractible`.

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

`infra_profile`
- Provisioning-plane selector for substrate startup/binding (for example Terraform/OpenTofu).
- Must not alter required check semantics or Gate-class outcomes.

`hk`
- Hook/gate runner used to execute check/fix profiles.
- Executes policy-defined checks; does not define kernel admissibility semantics.

`mise`
- Runtime/tool pinning and task entrypoint layer for repository-local workflows.

`pitchfork`
- Optional local daemon/orchestration runtime for long-lived or scheduled dev processes.
- Operational executor only; does not alter gate semantics.

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
