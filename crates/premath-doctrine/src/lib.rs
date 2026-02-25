pub mod world_descent;

pub use world_descent::{
    DerivedWorldRequirements, DoctrineRequiredRouteBinding, DoctrineValidationIssue,
    WorldDescentContractProjection, derive_world_descent_requirements_for_runtime_orchestration,
    derive_world_descent_requirements_for_world_registry_check,
    validate_world_descent_contract_projection,
};
