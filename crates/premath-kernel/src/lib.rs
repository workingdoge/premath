//! # Premath Kernel
//!
//! The kernel doctrine of definability: a notion is admissible exactly when
//! it is stable under context change and glues uniquely from locally
//! compatible data.
//!
//! This crate is **ontology-agnostic**: it does not prescribe what definables
//! are (sets, groupoids, ∞-groupoids, …). It only prescribes how they must
//! behave under reindexing and descent.
//!
//! ## Architecture
//!
//! ```text
//! Coherence<V>          ← Ambient sameness level (Set, Gpd, S∞)
//!     │
//! Context               ← Objects of C, with morphisms f: Γ' → Γ
//!     │
//! Cover                 ← Families {u_i: Γ_i → Γ} ▷ Γ
//!     │
//! Definable<V>          ← Fibers Def(Γ) with reindexing f*
//!     │
//! DescentDatum<V>       ← Local definables + overlap compatibilities
//!     │
//! ContractibleDescent   ← The axiom: res_U is an equivalence
//! ```

pub mod coherence;
pub mod context;
pub mod cover;
pub mod definable;
pub mod descent;
pub mod error;
pub mod gate;
pub mod obligation_registry;
pub mod runtime_orchestration;
pub mod site_resolve;
pub mod toy;
pub mod witness;
pub mod world_registry;

pub use coherence::CoherenceLevel;
pub use context::{Context, ContextId, Morphism};
pub use cover::Cover;
pub use definable::{ContentHash, Definable, FiberSignature};
pub use descent::{
    ContractibilityResult, DescentDatum, OverlapWitness, check_refinement_invariance,
};
pub use error::PremathError;
pub use gate::{GateCheck, World};
pub use obligation_registry::{
    ObligationGateMapping, failure_class_to_law_ref, obligation_gate_registry,
    obligation_gate_registry_json, obligation_to_failure_class,
};
pub use runtime_orchestration::{
    KcirMappingCheckRow, Phase3CommandSurfaceCheckRow, RuntimeOrchestrationReport,
    RuntimeOrchestrationSummary, RuntimeRouteCheckRow, WorldRouteBindingCheckRow,
    evaluate_runtime_orchestration, failure_class as runtime_orchestration_failure_class,
};
pub use site_resolve::{
    SitePackageKcirMappingRow, SitePackageOperationRow, SitePackageProjection,
    SitePackageSourceRefs, SitePackageTopology, SitePackageWorldRouteRow,
    SiteResolveKcirMappingRef, SiteResolveProjection, SiteResolveRequest, SiteResolveResponse,
    SiteResolveSelectedBinding, SiteResolveWitness, failure_class as site_resolve_failure_class,
    resolve_site_request,
};
pub use witness::{GateFailure, GateResult};
pub use world_registry::{
    OperationRouteRow, RequiredRouteBinding, RouteBindingRow, ValidationIssue, ValidationReport,
    WorldMorphismRow, WorldRegistry, WorldRouteBindingRow, WorldRow,
    failure_class as world_failure_class, parse_operation_route_rows,
    parse_world_route_binding_rows, resolve_operation_binding, resolve_route_family,
    validate_world_bindings_against_operations, validate_world_registry,
    validate_world_route_bindings, validate_world_route_bindings_with_required_families,
    validate_world_route_bindings_with_requirements,
};
