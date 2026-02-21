//! # premath-tusk
//!
//! Minimal `tusk-core` runtime skeleton:
//! - deterministic run/intent identity
//! - `DescentCore` / `DescentPack` artifacts
//! - world-owned glue selection result surface
//! - Gate-class mapping + gate witness envelope emission

pub mod descent;
pub mod eval;
pub mod identity;
pub mod mapping;
pub mod witness;

pub use descent::{
    CompatWitness, ContractibilityBasis, DescentCore, DescentPack, GlueMethod, GlueProposal,
    GlueProposalSet, GlueResult, GlueSelectionFailure, ModeBinding,
};
pub use eval::{EvalOutcome, evaluate_descent_pack};
pub use identity::{IntentSpec, RunIdOptions, RunIdentity, compute_intent_id};
pub use mapping::{
    TuskDiagnosticFailure, TuskFailureKind, map_glue_selection_failure, map_tusk_failure_kind,
};
pub use witness::GateWitnessEnvelope;
