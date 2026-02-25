pub mod world_descent;

pub use world_descent::{
    DerivedWorldRequirements, DoctrineRequiredRouteBinding, DoctrineValidationIssue,
    derive_world_descent_requirements_for_runtime_orchestration,
    derive_world_descent_requirements_for_world_registry_check,
};
