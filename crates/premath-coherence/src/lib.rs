//! Typed coherence-contract checker surface.
//!
//! This crate evaluates a machine contract artifact against repository surfaces
//! and emits deterministic witnesses.

mod instruction;
mod proposal;
mod required;
mod required_decide;
mod required_decision_verify;
mod required_gate_ref;
mod required_projection;
mod required_verify;

pub use instruction::{
    ExecutedInstructionCheck, InstructionError, InstructionProposalIngest, InstructionTypingPolicy,
    InstructionWitness, InstructionWitnessRuntime, ValidatedInstructionEnvelope,
    ValidatedInstructionProposal, build_instruction_witness, build_pre_execution_reject_witness,
    validate_instruction_envelope_payload,
};
pub use proposal::{
    CanonicalProposal, ProposalBinding, ProposalDischarge, ProposalError, ProposalObligation,
    ProposalStep, ProposalTargetJudgment, ValidatedProposal, compile_proposal_obligations,
    compute_proposal_digest, compute_proposal_kcir_ref, discharge_proposal_obligations,
    validate_proposal_payload,
};
pub use required::{
    ExecutedRequiredCheck, RequiredGateWitnessRef, RequiredWitness, RequiredWitnessError,
    RequiredWitnessRuntime, build_required_witness,
};
pub use required_decide::{
    RequiredWitnessDecideRequest, RequiredWitnessDecideResult, decide_required_witness_request,
};
pub use required_decision_verify::{
    RequiredDecisionVerifyDerived, RequiredDecisionVerifyRequest, RequiredDecisionVerifyResult,
    verify_required_decision_request,
};
pub use required_gate_ref::{
    RequiredGateRefFallback, RequiredGateRefRequest, RequiredGateRefResult, build_required_gate_ref,
};
pub use required_projection::{
    PROJECTION_POLICY, PROJECTION_SCHEMA, RequiredProjectionRequest, RequiredProjectionResult,
    normalize_paths as normalize_projection_paths, project_required_checks,
    projection_plan_payload,
};
pub use required_verify::{
    RequiredWitnessVerifyDerived, RequiredWitnessVerifyRequest, RequiredWitnessVerifyResult,
    verify_required_witness_payload, verify_required_witness_request,
};

use premath_kernel::{obligation_gate_registry, obligation_gate_registry_json};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const REQUIRED_OBLIGATION_IDS: &[&str] = &[
    "scope_noncontradiction",
    "capability_parity",
    "gate_chain_parity",
    "operation_reachability",
    "overlay_traceability",
    "transport_functoriality",
    "span_square_commutation",
    "coverage_base_change",
    "coverage_transitivity",
    "glue_or_witness_contractibility",
    "cwf_substitution_identity",
    "cwf_substitution_composition",
    "cwf_comprehension_beta",
    "cwf_comprehension_eta",
];

const REQUIRED_LANE_FAILURE_CLASSES: &[&str] = &[
    "lane_unknown",
    "lane_kind_unbound",
    "lane_ownership_violation",
    "lane_route_missing",
];

const REQUIRED_PULLBACK_ROUTE: &str = "span_square_commutation";
const GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE: &str =
    "coherence.gate_chain_parity.schema_lifecycle_invalid";
const GATE_CHAIN_STAGE1_PARITY_INVALID_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_parity_invalid";
const GATE_CHAIN_STAGE1_PARITY_MISSING_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_parity_missing";
const GATE_CHAIN_STAGE1_PARITY_MISMATCH_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_parity_mismatch";
const GATE_CHAIN_STAGE1_PARITY_UNBOUND_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_parity_unbound";
const STAGE1_PARITY_CLASS_MISSING: &str = "unification.evidence_stage1.parity.missing";
const STAGE1_PARITY_CLASS_MISMATCH: &str = "unification.evidence_stage1.parity.mismatch";
const STAGE1_PARITY_CLASS_UNBOUND: &str = "unification.evidence_stage1.parity.unbound";
const GATE_CHAIN_STAGE1_ROLLBACK_INVALID_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_rollback_invalid";
const GATE_CHAIN_STAGE1_ROLLBACK_PRECONDITION_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_rollback_precondition_missing";
const GATE_CHAIN_STAGE1_ROLLBACK_MISMATCH_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_rollback_failure_class_mismatch";
const GATE_CHAIN_STAGE1_ROLLBACK_UNBOUND_FAILURE: &str =
    "coherence.gate_chain_parity.stage1_rollback_unbound";
const STAGE1_ROLLBACK_CLASS_PRECONDITION: &str =
    "unification.evidence_stage1.rollback.precondition";
const STAGE1_ROLLBACK_CLASS_IDENTITY_DRIFT: &str =
    "unification.evidence_stage1.rollback.identity_drift";
const STAGE1_ROLLBACK_CLASS_UNBOUND: &str = "unification.evidence_stage1.rollback.unbound";
const GATE_CHAIN_STAGE2_AUTHORITY_INVALID_FAILURE: &str =
    "coherence.gate_chain_parity.stage2_authority_invalid";
const GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_VIOLATION_FAILURE: &str =
    "coherence.gate_chain_parity.stage2_authority_alias_violation";
const GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE: &str =
    "coherence.gate_chain_parity.stage2_authority_alias_window_violation";
const GATE_CHAIN_STAGE2_AUTHORITY_UNBOUND_FAILURE: &str =
    "coherence.gate_chain_parity.stage2_authority_unbound";
const GATE_CHAIN_STAGE2_KERNEL_MISSING_FAILURE: &str =
    "coherence.gate_chain_parity.stage2_kernel_compliance_missing";
const GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE: &str =
    "coherence.gate_chain_parity.stage2_kernel_compliance_drift";
const GATE_CHAIN_EVIDENCE_FACTORIZATION_INVALID_FAILURE: &str =
    "coherence.gate_chain_parity.evidence_factorization_invalid";
const GATE_CHAIN_EVIDENCE_FACTORIZATION_MISSING_FAILURE: &str =
    "coherence.gate_chain_parity.evidence_factorization_missing";
const GATE_CHAIN_EVIDENCE_FACTORIZATION_AMBIGUOUS_FAILURE: &str =
    "coherence.gate_chain_parity.evidence_factorization_ambiguous";
const GATE_CHAIN_EVIDENCE_FACTORIZATION_UNBOUND_FAILURE: &str =
    "coherence.gate_chain_parity.evidence_factorization_unbound";
const STAGE2_AUTHORITY_CLASS_ALIAS_VIOLATION: &str =
    "unification.evidence_stage2.authority_alias_violation";
const STAGE2_AUTHORITY_CLASS_ALIAS_WINDOW_VIOLATION: &str =
    "unification.evidence_stage2.alias_window_violation";
const STAGE2_AUTHORITY_CLASS_UNBOUND: &str = "unification.evidence_stage2.unbound";
const STAGE2_KERNEL_CLASS_MISSING: &str = "unification.evidence_stage2.kernel_compliance_missing";
const STAGE2_KERNEL_CLASS_DRIFT: &str = "unification.evidence_stage2.kernel_compliance_drift";
const EVIDENCE_FACTORIZATION_CLASS_MISSING: &str = "unification.evidence_factorization.missing";
const EVIDENCE_FACTORIZATION_CLASS_AMBIGUOUS: &str = "unification.evidence_factorization.ambiguous";
const EVIDENCE_FACTORIZATION_CLASS_UNBOUND: &str = "unification.evidence_factorization.unbound";
const EVIDENCE_FACTORIZATION_ROUTE_KIND: &str = "eta_F_to_Ev";
const STAGE2_AUTHORITY_ALIAS_ROLE: &str = "projection_only";
const STAGE2_BIDIR_ROUTE_KIND: &str = "direct_checker_discharge";
const STAGE2_BIDIR_OBLIGATION_FIELD_REF: &str = "bidirCheckerObligations";
const STAGE2_BIDIR_FALLBACK_MODE: &str = "profile_gated_sentinel";
const WORKER_MUTATION_DEFAULT_MODE: &str = "instruction-linked";
const WORKER_ALLOWED_MUTATION_MODES: &[&str] = &["instruction-linked", "human-override"];
const WORKER_ROUTE_ISSUE_CLAIM: &str = "capabilities.change_morphisms.issue_claim";
const WORKER_ROUTE_ISSUE_LEASE_RENEW: &str = "capabilities.change_morphisms.issue_lease_renew";
const WORKER_ROUTE_ISSUE_LEASE_RELEASE: &str = "capabilities.change_morphisms.issue_lease_release";
const WORKER_ROUTE_ISSUE_DISCOVER: &str = "capabilities.change_morphisms.issue_discover";
const WORKER_CLASS_POLICY_DRIFT: &str = "worker_lane_policy_drift";
const WORKER_CLASS_MUTATION_MODE_DRIFT: &str = "worker_lane_mutation_mode_drift";
const WORKER_CLASS_ROUTE_UNBOUND: &str = "worker_lane_route_unbound";
const GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE: &str =
    "coherence.gate_chain_parity.worker_lane_policy_drift";
const GATE_CHAIN_WORKER_MUTATION_MODE_DRIFT_FAILURE: &str =
    "coherence.gate_chain_parity.worker_lane_mutation_mode_drift";
const GATE_CHAIN_WORKER_ROUTE_UNBOUND_FAILURE: &str =
    "coherence.gate_chain_parity.worker_lane_route_unbound";
const STAGE2_REQUIRED_KERNEL_OBLIGATIONS: &[&str] = &[
    "stability",
    "locality",
    "descent_exists",
    "descent_contractible",
    "adjoint_triple",
    "ext_gap",
    "ext_ambiguous",
];
const REQUIRED_SCHEMA_LIFECYCLE_FAMILIES: &[&str] = &[
    "controlPlaneContractKind",
    "requiredWitnessKind",
    "requiredDecisionKind",
    "instructionWitnessKind",
    "instructionPolicyKind",
    "requiredProjectionPolicy",
    "requiredDeltaKind",
];

#[derive(Debug, Error)]
pub enum CoherenceError {
    #[error("failed to read file: {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid json at {path}: {source}")]
    ParseJson {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("invalid toml at {path}: {source}")]
    ParseToml {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("{0}")]
    Contract(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceBinding {
    pub normalizer_id: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalCapabilityDoc {
    pub doc_ref: String,
    pub capability_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceObligationSpec {
    pub id: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceSurfaces {
    pub capability_registry_path: String,
    pub capability_registry_kind: String,
    #[serde(default = "default_conformance_path")]
    pub conformance_path: String,
    pub capability_manifest_root: String,
    pub readme_path: String,
    pub conformance_readme_path: String,
    pub spec_index_path: String,
    pub spec_index_capability_heading: String,
    pub spec_index_informative_heading: String,
    pub spec_index_overlay_heading: String,
    pub ci_closure_path: String,
    pub ci_closure_baseline_start: String,
    pub ci_closure_baseline_end: String,
    pub ci_closure_projection_start: String,
    pub ci_closure_projection_end: String,
    pub mise_path: String,
    pub mise_baseline_task: String,
    pub control_plane_contract_path: String,
    pub doctrine_site_path: String,
    pub doctrine_root_node_id: String,
    pub profile_readme_path: String,
    pub bidir_spec_path: String,
    pub bidir_spec_section_start: String,
    pub bidir_spec_section_end: String,
    pub coherence_spec_path: String,
    pub coherence_spec_obligation_start: String,
    pub coherence_spec_obligation_end: String,
    pub obligation_registry_kind: String,
    pub informative_clause_needle: String,
    pub transport_fixture_root_path: String,
    pub site_fixture_root_path: String,
}

fn default_conformance_path() -> String {
    "specs/premath/draft/CONFORMANCE.md".to_string()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityRegistry {
    schema: u32,
    registry_kind: String,
    #[serde(default)]
    profile_overlay_claims: Vec<String>,
    executable_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneProjectionContract {
    schema: u32,
    contract_kind: String,
    #[serde(default)]
    schema_lifecycle: Option<ControlPlaneSchemaLifecycle>,
    #[serde(default)]
    evidence_stage1_parity: Option<ControlPlaneStage1Parity>,
    #[serde(default)]
    evidence_stage1_rollback: Option<ControlPlaneStage1Rollback>,
    #[serde(default)]
    evidence_stage2_authority: Option<ControlPlaneStage2Authority>,
    #[serde(default)]
    evidence_factorization: Option<ControlPlaneEvidenceFactorization>,
    #[serde(default)]
    evidence_lanes: Option<ControlPlaneEvidenceLanes>,
    #[serde(default)]
    lane_artifact_kinds: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    lane_ownership: Option<ControlPlaneLaneOwnership>,
    #[serde(default)]
    lane_failure_classes: Option<Vec<String>>,
    #[serde(default)]
    worker_lane_authority: Option<ControlPlaneWorkerLaneAuthority>,
    required_gate_projection: RequiredGateProjection,
    required_witness: ControlPlaneRequiredWitness,
    instruction_witness: ControlPlaneInstructionWitness,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneEvidenceLanes {
    semantic_doctrine: String,
    strict_checker: String,
    witness_commutation: String,
    runtime_transport: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneLaneOwnership {
    #[serde(default)]
    checker_core_only_obligations: Vec<String>,
    #[serde(default)]
    required_cross_lane_witness_route: Option<ControlPlaneCrossLaneWitnessRoute>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneCrossLaneWitnessRoute {
    pullback_base_change: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneWorkerLaneAuthority {
    mutation_policy: ControlPlaneWorkerMutationPolicy,
    mutation_routes: ControlPlaneWorkerMutationRoutes,
    failure_classes: ControlPlaneWorkerLaneFailureClasses,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneWorkerMutationPolicy {
    #[serde(default)]
    default_mode: String,
    #[serde(default)]
    allowed_modes: Vec<String>,
    #[serde(default)]
    compatibility_overrides: Vec<ControlPlaneWorkerMutationOverride>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneWorkerMutationOverride {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    support_until_epoch: String,
    #[serde(default)]
    requires_reason: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneWorkerMutationRoutes {
    #[serde(default)]
    issue_claim: String,
    #[serde(default)]
    issue_lease_renew: String,
    #[serde(default)]
    issue_lease_release: String,
    #[serde(default)]
    issue_discover: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneWorkerLaneFailureClasses {
    #[serde(default)]
    policy_drift: String,
    #[serde(default)]
    mutation_mode_drift: String,
    #[serde(default)]
    route_unbound: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneEvidenceFactorization {
    #[serde(default)]
    profile_kind: String,
    #[serde(default)]
    route_kind: String,
    #[serde(default)]
    factorization_routes: Vec<String>,
    #[serde(default)]
    binding: ControlPlaneEvidenceFactorizationBinding,
    #[serde(default)]
    cross_lane_routes: Option<ControlPlaneCrossLaneWitnessRoute>,
    #[serde(default)]
    failure_classes: ControlPlaneEvidenceFactorizationFailureClasses,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneEvidenceFactorizationBinding {
    #[serde(default)]
    normalizer_id_ref: String,
    #[serde(default)]
    policy_digest_ref: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneEvidenceFactorizationFailureClasses {
    #[serde(default)]
    missing: String,
    #[serde(default)]
    ambiguous: String,
    #[serde(default)]
    unbound: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequiredGateProjection {
    projection_policy: String,
    check_order: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneRequiredWitness {
    witness_kind: String,
    decision_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneInstructionWitness {
    witness_kind: String,
    policy_kind: String,
    policy_digest_prefix: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneSchemaLifecycle {
    active_epoch: String,
    kind_families: BTreeMap<String, ControlPlaneSchemaKindFamily>,
    #[serde(default)]
    governance: Option<ControlPlaneSchemaLifecycleGovernance>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneSchemaKindFamily {
    canonical_kind: String,
    #[serde(default)]
    compatibility_aliases: Vec<ControlPlaneSchemaAlias>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneSchemaAlias {
    alias_kind: String,
    support_until_epoch: String,
    replacement_kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneSchemaLifecycleGovernance {
    mode: String,
    decision_ref: String,
    owner: String,
    #[serde(default)]
    rollover_cadence_months: Option<u32>,
    #[serde(default)]
    freeze_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage1Parity {
    #[serde(default)]
    profile_kind: String,
    #[serde(default)]
    authority_to_typed_core_route: String,
    #[serde(default)]
    comparison_tuple: ControlPlaneStage1ParityComparisonTuple,
    #[serde(default)]
    failure_classes: ControlPlaneStage1ParityFailureClasses,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage1ParityComparisonTuple {
    #[serde(default)]
    authority_digest_ref: String,
    #[serde(default)]
    typed_core_digest_ref: String,
    #[serde(default)]
    normalizer_id_ref: String,
    #[serde(default)]
    policy_digest_ref: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage1ParityFailureClasses {
    #[serde(default)]
    missing: String,
    #[serde(default)]
    mismatch: String,
    #[serde(default)]
    unbound: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage1Rollback {
    #[serde(default)]
    profile_kind: String,
    #[serde(default)]
    witness_kind: String,
    #[serde(default)]
    from_stage: String,
    #[serde(default)]
    to_stage: String,
    #[serde(default)]
    trigger_failure_classes: Vec<String>,
    #[serde(default)]
    identity_refs: ControlPlaneStage1RollbackIdentityRefs,
    #[serde(default)]
    failure_classes: ControlPlaneStage1RollbackFailureClasses,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage1RollbackIdentityRefs {
    #[serde(default)]
    authority_digest_ref: String,
    #[serde(default)]
    rollback_authority_digest_ref: String,
    #[serde(default)]
    normalizer_id_ref: String,
    #[serde(default)]
    policy_digest_ref: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage1RollbackFailureClasses {
    #[serde(default)]
    precondition: String,
    #[serde(default)]
    identity_drift: String,
    #[serde(default)]
    unbound: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2Authority {
    #[serde(default)]
    profile_kind: String,
    #[serde(default)]
    active_stage: String,
    #[serde(default)]
    typed_authority: ControlPlaneStage2TypedAuthority,
    #[serde(default)]
    compatibility_alias: ControlPlaneStage2CompatibilityAlias,
    #[serde(default)]
    bidir_evidence_route: ControlPlaneStage2BidirEvidenceRoute,
    #[serde(default)]
    kernel_compliance_sentinel: Option<ControlPlaneStage2KernelComplianceSentinel>,
    #[serde(default)]
    failure_classes: ControlPlaneStage2FailureClasses,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2TypedAuthority {
    #[serde(default)]
    kind_ref: String,
    #[serde(default)]
    digest_ref: String,
    #[serde(default)]
    normalizer_id_ref: String,
    #[serde(default)]
    policy_digest_ref: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2CompatibilityAlias {
    #[serde(default)]
    kind_ref: String,
    #[serde(default)]
    digest_ref: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    support_until_epoch: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2FailureClasses {
    #[serde(default)]
    authority_alias_violation: String,
    #[serde(default)]
    alias_window_violation: String,
    #[serde(default)]
    unbound: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2BidirEvidenceRoute {
    #[serde(default)]
    route_kind: String,
    #[serde(default)]
    obligation_field_ref: String,
    #[serde(default)]
    required_obligations: Vec<String>,
    #[serde(default)]
    failure_classes: ControlPlaneStage2BidirEvidenceFailureClasses,
    #[serde(default)]
    fallback: Option<ControlPlaneStage2BidirEvidenceFallback>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2BidirEvidenceFailureClasses {
    #[serde(default)]
    missing: String,
    #[serde(default)]
    drift: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2BidirEvidenceFallback {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    profile_kinds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneStage2KernelComplianceSentinel {
    #[serde(default)]
    required_obligations: Vec<String>,
    #[serde(default)]
    failure_classes: ControlPlaneStage2BidirEvidenceFailureClasses,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceContract {
    pub schema: u32,
    pub contract_kind: String,
    pub contract_id: String,
    pub binding: CoherenceBinding,
    pub obligations: Vec<CoherenceObligationSpec>,
    pub surfaces: CoherenceSurfaces,
    #[serde(default)]
    pub conditional_capability_docs: Vec<ConditionalCapabilityDoc>,
    #[serde(default)]
    pub expected_operation_paths: Vec<String>,
    #[serde(default)]
    pub overlay_docs: Vec<String>,
    #[serde(default)]
    pub required_bidir_obligations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationWitness {
    pub obligation_id: String,
    pub result: String,
    pub failure_classes: Vec<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceWitness {
    pub schema: u32,
    pub witness_kind: String,
    pub contract_kind: String,
    pub contract_id: String,
    pub contract_ref: String,
    pub contract_digest: String,
    pub binding: CoherenceBinding,
    pub result: String,
    pub obligations: Vec<ObligationWitness>,
    pub failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineSite {
    #[serde(default)]
    nodes: Vec<DoctrineNode>,
    #[serde(default)]
    covers: Vec<DoctrineCover>,
    #[serde(default)]
    edges: Vec<DoctrineEdge>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineNode {
    id: String,
    path: String,
    kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineCover {
    over: String,
    #[serde(default)]
    parts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineEdge {
    from: String,
    to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransportManifest {
    schema: u32,
    status: String,
    #[serde(default)]
    vectors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransportExpect {
    schema: u32,
    status: String,
    result: String,
    #[serde(default)]
    expected_failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteManifest {
    schema: u32,
    status: String,
    #[serde(default)]
    vectors: Vec<String>,
    #[serde(default)]
    obligation_vectors: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteCase {
    schema: u32,
    status: String,
    obligation_id: String,
    #[serde(default)]
    semantic_scenario_id: Option<String>,
    #[serde(default)]
    profile: Option<String>,
    artifacts: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteExpect {
    schema: u32,
    status: String,
    result: String,
    #[serde(default)]
    expected_failure_classes: Vec<String>,
}

#[derive(Debug)]
struct ObligationCheck {
    failure_classes: Vec<String>,
    details: Value,
}

type InvarianceRow = (String, String, String, Vec<String>);
type InvarianceGroups = BTreeMap<String, Vec<InvarianceRow>>;

struct InvarianceObservation<'a> {
    vector_id: &'a str,
    semantic_scenario_id: Option<&'a str>,
    profile: Option<&'a str>,
    result: &'a str,
    failure_classes: &'a [String],
}

#[derive(Debug, Default, Clone, Copy)]
struct PolarityCoverage {
    matched_golden_count: usize,
    matched_adversarial_count: usize,
    matched_invariance_count: usize,
    matched_expected_accepted_count: usize,
    matched_expected_rejected_count: usize,
}

impl PolarityCoverage {
    fn record_vector_id(&mut self, vector_id: &str) {
        if vector_id.starts_with("golden/") {
            self.matched_golden_count += 1;
        } else if vector_id.starts_with("adversarial/") {
            self.matched_adversarial_count += 1;
        } else if vector_id.starts_with("invariance/") {
            self.matched_invariance_count += 1;
        }
    }

    fn record_expected_result(&mut self, expected_result: &str) {
        if expected_result == "accepted" {
            self.matched_expected_accepted_count += 1;
        } else if expected_result == "rejected" {
            self.matched_expected_rejected_count += 1;
        }
    }

    fn emit_missing_failures(
        &self,
        failures: &mut Vec<String>,
        failure_prefix: &str,
        enforce: bool,
    ) {
        if !enforce {
            return;
        }
        if self.matched_golden_count == 0 {
            failures.push(format!("{failure_prefix}.missing_golden_vector"));
        }
        if self.matched_adversarial_count == 0 {
            failures.push(format!("{failure_prefix}.missing_adversarial_vector"));
        }
        if self.matched_expected_accepted_count == 0 {
            failures.push(format!("{failure_prefix}.missing_expected_accepted_vector"));
        }
        if self.matched_expected_rejected_count == 0 {
            failures.push(format!("{failure_prefix}.missing_expected_rejected_vector"));
        }
    }

    fn vector_kind_details(&self) -> Value {
        json!({
            "golden": self.matched_golden_count,
            "adversarial": self.matched_adversarial_count,
            "invariance": self.matched_invariance_count,
        })
    }

    fn expected_result_details(&self) -> Value {
        json!({
            "accepted": self.matched_expected_accepted_count,
            "rejected": self.matched_expected_rejected_count,
        })
    }
}

pub fn run_coherence_check(
    repo_root: impl AsRef<Path>,
    contract_path: impl AsRef<Path>,
) -> Result<CoherenceWitness, CoherenceError> {
    let repo_root = repo_root.as_ref().to_path_buf();
    let contract_path = resolve_path(&repo_root, contract_path.as_ref());
    let contract_bytes = read_bytes(&contract_path)?;
    let contract: CoherenceContract =
        serde_json::from_slice(&contract_bytes).map_err(|source| CoherenceError::ParseJson {
            path: display_path(&contract_path),
            source,
        })?;

    let mut obligations: Vec<ObligationWitness> = Vec::new();
    let mut aggregate_failures: BTreeSet<String> = BTreeSet::new();

    let contract_obligation_ids: Vec<String> = contract
        .obligations
        .iter()
        .map(|item| item.id.clone())
        .collect();
    let contract_set_check = validate_contract_obligation_set(&contract_obligation_ids);
    if !contract_set_check.is_empty() {
        let failure_classes = contract_set_check;
        for class_name in &failure_classes {
            aggregate_failures.insert(class_name.clone());
        }
        obligations.push(ObligationWitness {
            obligation_id: "contract_obligation_set".to_string(),
            result: "rejected".to_string(),
            failure_classes,
            details: json!({
                "contractObligations": contract_obligation_ids,
                "requiredObligations": REQUIRED_OBLIGATION_IDS,
            }),
        });
    }

    for obligation_id in REQUIRED_OBLIGATION_IDS {
        let checked = execute_obligation(obligation_id, &repo_root, &contract);
        for class_name in &checked.failure_classes {
            aggregate_failures.insert(class_name.clone());
        }
        obligations.push(ObligationWitness {
            obligation_id: obligation_id.to_string(),
            result: if checked.failure_classes.is_empty() {
                "accepted".to_string()
            } else {
                "rejected".to_string()
            },
            failure_classes: checked.failure_classes,
            details: checked.details,
        });
    }

    let contract_digest = format!("cohctr1_{}", hex_sha256_from_bytes(&contract_bytes));
    let contract_ref = to_repo_relative_or_absolute(&repo_root, &contract_path);
    let failure_classes: Vec<String> = aggregate_failures.into_iter().collect();

    Ok(CoherenceWitness {
        schema: 1,
        witness_kind: "premath.coherence.v1".to_string(),
        contract_kind: contract.contract_kind,
        contract_id: contract.contract_id,
        contract_ref,
        contract_digest,
        binding: contract.binding,
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        obligations,
        failure_classes,
    })
}

fn execute_obligation(
    obligation_id: &str,
    repo_root: &Path,
    contract: &CoherenceContract,
) -> ObligationCheck {
    let result = match obligation_id {
        "scope_noncontradiction" => check_scope_noncontradiction(repo_root, contract),
        "capability_parity" => check_capability_parity(repo_root, contract),
        "gate_chain_parity" => check_gate_chain_parity(repo_root, contract),
        "operation_reachability" => check_operation_reachability(repo_root, contract),
        "overlay_traceability" => check_overlay_traceability(repo_root, contract),
        "transport_functoriality" => check_transport_functoriality(repo_root, contract),
        "span_square_commutation" => check_span_square_commutation(repo_root, contract),
        "coverage_base_change" => check_coverage_base_change(repo_root, contract),
        "coverage_transitivity" => check_coverage_transitivity(repo_root, contract),
        "glue_or_witness_contractibility" => {
            check_glue_or_witness_contractibility(repo_root, contract)
        }
        "cwf_substitution_identity" => check_cwf_substitution_identity(repo_root, contract),
        "cwf_substitution_composition" => check_cwf_substitution_composition(repo_root, contract),
        "cwf_comprehension_beta" => check_cwf_comprehension_beta(repo_root, contract),
        "cwf_comprehension_eta" => check_cwf_comprehension_eta(repo_root, contract),
        _ => Err(CoherenceError::Contract(format!(
            "unknown obligation id: {obligation_id}"
        ))),
    };

    match result {
        Ok(ok) => ok,
        Err(err) => ObligationCheck {
            failure_classes: vec![format!("coherence.{obligation_id}.surface_error")],
            details: json!({ "error": err.to_string() }),
        },
    }
}

fn check_scope_noncontradiction(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    let spec_index_path = resolve_path(repo_root, contract.surfaces.spec_index_path.as_str());
    let spec_index_text = read_text(&spec_index_path)?;
    let section_54 = extract_heading_section(
        &spec_index_text,
        contract.surfaces.spec_index_capability_heading.as_str(),
    )?;
    let section_55 = extract_heading_section(
        &spec_index_text,
        contract.surfaces.spec_index_informative_heading.as_str(),
    )?;
    let spec_index_doc_map = parse_spec_index_capability_doc_map(&section_54)?;

    let mut failures = Vec::new();
    if !section_55.contains(contract.surfaces.informative_clause_needle.as_str()) {
        failures.push("coherence.scope_noncontradiction.informative_clause_missing".to_string());
    }
    for row in &contract.conditional_capability_docs {
        match spec_index_doc_map.get(&row.doc_ref) {
            Some(mapped) if mapped == &row.capability_id => {}
            Some(_) | None => failures
                .push("coherence.scope_noncontradiction.conditional_mapping_mismatch".to_string()),
        }
        if !contains_conditional_normative_clause(
            &section_55,
            row.doc_ref.as_str(),
            row.capability_id.as_str(),
        )? {
            failures
                .push("coherence.scope_noncontradiction.conditional_clause_missing".to_string());
        }
    }

    let capability_registry = load_capability_registry(repo_root, contract)?;
    let conformance_path = resolve_path(repo_root, contract.surfaces.conformance_path.as_str());
    let conformance_text = read_text(&conformance_path)?;
    let conformance_overlay_section = extract_heading_section(&conformance_text, "2.4")?;
    let conformance_profile_claims =
        parse_backticked_profile_overlay_claims(&conformance_overlay_section)?;
    let registry_profile_claims: BTreeSet<String> = capability_registry
        .profile_overlay_claims
        .iter()
        .cloned()
        .collect();
    if capability_registry.profile_overlay_claims.len() != registry_profile_claims.len() {
        failures.push(
            "coherence.scope_noncontradiction.profile_overlay_registry_duplicate".to_string(),
        );
    }
    if registry_profile_claims != conformance_profile_claims {
        failures
            .push("coherence.scope_noncontradiction.profile_overlay_claim_mismatch".to_string());
    }

    let bidir_spec_path = resolve_path(repo_root, contract.surfaces.bidir_spec_path.as_str());
    let bidir_spec_text = read_text(&bidir_spec_path)?;
    let bidir_spec_section = extract_section_between(
        &bidir_spec_text,
        contract.surfaces.bidir_spec_section_start.as_str(),
        contract.surfaces.bidir_spec_section_end.as_str(),
    )?;
    let bidir_spec_obligations = parse_backtick_obligation_tokens(bidir_spec_section)?;
    let obligation_registry_json = obligation_gate_registry_json();
    let obligation_registry_kind = obligation_registry_json
        .get("registryKind")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if obligation_registry_kind != contract.surfaces.obligation_registry_kind {
        failures.push("coherence.scope_noncontradiction.bidir_registry_kind_mismatch".to_string());
    }
    let bidir_checker_obligations: BTreeSet<String> = obligation_gate_registry()
        .into_iter()
        .map(|row| row.obligation_kind.to_string())
        .collect();

    for required in &contract.required_bidir_obligations {
        if !bidir_spec_obligations.contains(required) {
            failures
                .push("coherence.scope_noncontradiction.bidir_spec_missing_obligation".to_string());
        }
        if !bidir_checker_obligations.contains(required) {
            failures.push(
                "coherence.scope_noncontradiction.bidir_checker_missing_obligation".to_string(),
            );
        }
    }

    let coherence_spec_path =
        resolve_path(repo_root, contract.surfaces.coherence_spec_path.as_str());
    let coherence_spec_text = read_text(&coherence_spec_path)?;
    let coherence_spec_obligation_section = extract_section_between(
        &coherence_spec_text,
        contract.surfaces.coherence_spec_obligation_start.as_str(),
        contract.surfaces.coherence_spec_obligation_end.as_str(),
    )?;
    let coherence_spec_obligations =
        parse_backtick_obligation_tokens(coherence_spec_obligation_section)?;
    let required_coherence_obligations: BTreeSet<String> = REQUIRED_OBLIGATION_IDS
        .iter()
        .map(|id| (*id).to_string())
        .collect();
    failures.extend(validate_required_obligation_parity(
        &coherence_spec_obligations,
        &required_coherence_obligations,
    ));

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "conditionalCapabilityDocs": contract.conditional_capability_docs,
            "specIndexCapabilityDocMap": spec_index_doc_map,
            "registryProfileOverlayClaims": registry_profile_claims,
            "conformanceProfileOverlayClaims": conformance_profile_claims,
            "requiredBidirObligations": contract.required_bidir_obligations,
            "bidirSpecObligations": bidir_spec_obligations,
            "bidirCheckerObligations": bidir_checker_obligations,
            "requiredCoherenceObligations": required_coherence_obligations,
            "coherenceSpecObligations": coherence_spec_obligations,
            "obligationRegistryKind": obligation_registry_kind,
        }),
    })
}

fn load_capability_registry(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<CapabilityRegistry, CoherenceError> {
    let capability_registry_path = resolve_path(
        repo_root,
        contract.surfaces.capability_registry_path.as_str(),
    );
    let capability_registry: CapabilityRegistry =
        serde_json::from_slice(&read_bytes(&capability_registry_path)?).map_err(|source| {
            CoherenceError::ParseJson {
                path: display_path(&capability_registry_path),
                source,
            }
        })?;
    if capability_registry.schema != 1 {
        return Err(CoherenceError::Contract(format!(
            "capability registry schema must be 1: {}",
            display_path(&capability_registry_path)
        )));
    }
    if capability_registry.registry_kind != contract.surfaces.capability_registry_kind {
        return Err(CoherenceError::Contract(format!(
            "capability registry kind mismatch at {}: expected {:?}, got {:?}",
            display_path(&capability_registry_path),
            contract.surfaces.capability_registry_kind,
            capability_registry.registry_kind
        )));
    }
    Ok(capability_registry)
}

fn check_capability_parity(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    let capability_registry_path = resolve_path(
        repo_root,
        contract.surfaces.capability_registry_path.as_str(),
    );
    let capability_registry = load_capability_registry(repo_root, contract)?;
    if capability_registry.executable_capabilities.is_empty() {
        return Err(CoherenceError::Contract(format!(
            "capability registry must include at least one capability: {}",
            display_path(&capability_registry_path)
        )));
    }
    let executable_capabilities = dedupe_sorted(capability_registry.executable_capabilities);
    let executable_set: BTreeSet<String> = executable_capabilities.iter().cloned().collect();

    let manifest_root = resolve_path(
        repo_root,
        contract.surfaces.capability_manifest_root.as_str(),
    );
    let manifest_set = parse_manifest_capabilities(&manifest_root)?;

    let readme_text = read_text(&resolve_path(
        repo_root,
        contract.surfaces.readme_path.as_str(),
    ))?;
    let conformance_readme_text = read_text(&resolve_path(
        repo_root,
        contract.surfaces.conformance_readme_path.as_str(),
    ))?;
    let spec_index_text = read_text(&resolve_path(
        repo_root,
        contract.surfaces.spec_index_path.as_str(),
    ))?;
    let section_54 = extract_heading_section(
        &spec_index_text,
        contract.surfaces.spec_index_capability_heading.as_str(),
    )?;

    let readme_set = parse_backticked_capabilities(&readme_text)?;
    let conformance_readme_set = parse_backticked_capabilities(&conformance_readme_text)?;
    let spec_index_set = parse_backticked_capabilities(&section_54)?;

    let mut failures = Vec::new();
    if manifest_set != executable_set {
        failures.push("coherence.capability_parity.manifest_set_mismatch".to_string());
    }
    if readme_set != executable_set {
        failures.push("coherence.capability_parity.readme_set_mismatch".to_string());
    }
    if conformance_readme_set != executable_set {
        failures.push("coherence.capability_parity.conformance_readme_set_mismatch".to_string());
    }
    if spec_index_set != executable_set {
        failures.push("coherence.capability_parity.spec_index_set_mismatch".to_string());
    }

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "expected": executable_capabilities,
            "capabilityRegistryKind": capability_registry.registry_kind,
            "capabilityRegistryPath": to_repo_relative_or_absolute(repo_root, &capability_registry_path),
            "manifest": sorted_vec_from_set(&manifest_set),
            "readme": sorted_vec_from_set(&readme_set),
            "conformanceReadme": sorted_vec_from_set(&conformance_readme_set),
            "specIndex": sorted_vec_from_set(&spec_index_set),
        }),
    })
}

fn is_valid_epoch(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 7
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && matches!(
            (bytes[5], bytes[6]),
            (b'0', b'1'..=b'9') | (b'1', b'0'..=b'2')
        )
}

fn epoch_to_month_index(value: &str) -> Option<i32> {
    if !is_valid_epoch(value) {
        return None;
    }
    let year: i32 = value.get(0..4)?.parse().ok()?;
    let month: i32 = value.get(5..7)?.parse().ok()?;
    Some(year * 12 + month)
}

fn resolve_schema_lifecycle_kind(
    schema_lifecycle: &ControlPlaneSchemaLifecycle,
    family_id: &str,
    kind: &str,
) -> Result<String, String> {
    let family = schema_lifecycle
        .kind_families
        .get(family_id)
        .ok_or_else(|| format!("missing kind family `{family_id}`"))?;
    let canonical_kind = family.canonical_kind.trim();
    if canonical_kind.is_empty() {
        return Err(format!("kind family `{family_id}` has empty canonicalKind"));
    }
    if kind.trim() == canonical_kind {
        return Ok(canonical_kind.to_string());
    }

    let mut seen_aliases: BTreeSet<String> = BTreeSet::new();
    for alias in &family.compatibility_aliases {
        let alias_kind = alias.alias_kind.trim();
        let support_until_epoch = alias.support_until_epoch.trim();
        let replacement_kind = alias.replacement_kind.trim();
        if alias_kind.is_empty()
            || replacement_kind != canonical_kind
            || !is_valid_epoch(support_until_epoch)
            || !seen_aliases.insert(alias_kind.to_string())
        {
            return Err(format!(
                "kind family `{family_id}` has invalid compatibilityAliases rows"
            ));
        }
        if alias_kind == kind.trim() {
            if schema_lifecycle.active_epoch.as_str() > support_until_epoch {
                return Err(format!(
                    "kind `{kind}` for `{family_id}` expired at `{support_until_epoch}` (activeEpoch=`{}`)",
                    schema_lifecycle.active_epoch
                ));
            }
            return Ok(canonical_kind.to_string());
        }
    }

    Err(format!(
        "kind `{kind}` is not supported for kind family `{family_id}` (canonicalKind=`{canonical_kind}`)"
    ))
}

fn resolve_or_record_schema_kind(
    schema_lifecycle: &ControlPlaneSchemaLifecycle,
    family_id: &str,
    field_name: &str,
    value: &str,
    failures: &mut Vec<String>,
    reasons: &mut Vec<String>,
) -> Option<String> {
    if value.trim().is_empty() {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
        reasons.push(format!("{field_name} must be non-empty"));
        return None;
    }
    match resolve_schema_lifecycle_kind(schema_lifecycle, family_id, value) {
        Ok(resolved_kind) => Some(resolved_kind),
        Err(reason) => {
            failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
            reasons.push(format!("{field_name}: {reason}"));
            None
        }
    }
}

fn evaluate_control_plane_schema_lifecycle(
    control_plane_contract: &ControlPlaneProjectionContract,
) -> ObligationCheck {
    let mut failures = Vec::new();
    let mut reasons = Vec::new();
    let mut resolved = json!({});

    let Some(schema_lifecycle) = &control_plane_contract.schema_lifecycle else {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
        return ObligationCheck {
            failure_classes: dedupe_sorted(failures),
            details: json!({
                "present": false,
                "requiredKindFamilies": REQUIRED_SCHEMA_LIFECYCLE_FAMILIES,
                "reasons": ["schemaLifecycle missing"]
            }),
        };
    };

    let active_epoch = schema_lifecycle.active_epoch.trim();
    if !is_valid_epoch(active_epoch) {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
        reasons.push(format!(
            "schemaLifecycle.activeEpoch invalid (expected YYYY-MM, got `{}`)",
            schema_lifecycle.active_epoch
        ));
    }

    let expected_families: BTreeSet<String> = REQUIRED_SCHEMA_LIFECYCLE_FAMILIES
        .iter()
        .map(|id| (*id).to_string())
        .collect();
    let actual_families: BTreeSet<String> = schema_lifecycle
        .kind_families
        .keys()
        .map(|id| id.to_string())
        .collect();
    let missing_families: Vec<String> = expected_families
        .difference(&actual_families)
        .cloned()
        .collect();
    let unknown_families: Vec<String> = actual_families
        .difference(&expected_families)
        .cloned()
        .collect();
    if !missing_families.is_empty() || !unknown_families.is_empty() {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
    }

    let mut alias_support_epochs: BTreeSet<String> = BTreeSet::new();
    for family in schema_lifecycle.kind_families.values() {
        for alias in &family.compatibility_aliases {
            let support_until_epoch = alias.support_until_epoch.trim();
            if !support_until_epoch.is_empty() {
                alias_support_epochs.insert(support_until_epoch.to_string());
            }
        }
    }

    let mut governance_details = json!({
        "present": schema_lifecycle.governance.is_some(),
        "mode": Value::Null,
        "decisionRef": Value::Null,
        "owner": Value::Null,
        "rolloverCadenceMonths": Value::Null,
        "freezeReason": Value::Null,
        "aliasSupportEpochs": alias_support_epochs,
        "rolloverEpoch": Value::Null,
        "aliasRunwayMonths": Value::Null,
    });
    let Some(governance) = &schema_lifecycle.governance else {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
        reasons.push("schemaLifecycle.governance missing".to_string());
        return ObligationCheck {
            failure_classes: dedupe_sorted(failures),
            details: json!({
                "present": true,
                "activeEpoch": schema_lifecycle.active_epoch,
                "requiredKindFamilies": REQUIRED_SCHEMA_LIFECYCLE_FAMILIES,
                "actualKindFamilies": actual_families,
                "missingKindFamilies": missing_families,
                "unknownKindFamilies": unknown_families,
                "resolvedKinds": resolved,
                "governance": governance_details,
                "reasons": dedupe_sorted(reasons),
            }),
        };
    };

    let governance_mode = governance.mode.trim();
    let governance_decision_ref = governance.decision_ref.trim();
    let governance_owner = governance.owner.trim();
    governance_details["mode"] = json!(governance_mode);
    governance_details["decisionRef"] = json!(governance_decision_ref);
    governance_details["owner"] = json!(governance_owner);
    governance_details["rolloverCadenceMonths"] = json!(governance.rollover_cadence_months);
    governance_details["freezeReason"] = json!(governance.freeze_reason.as_deref());

    if governance_decision_ref.is_empty() {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
        reasons.push("schemaLifecycle.governance.decisionRef must be non-empty".to_string());
    }
    if governance_owner.is_empty() {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
        reasons.push("schemaLifecycle.governance.owner must be non-empty".to_string());
    }
    if governance_mode.is_empty() {
        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
        reasons.push("schemaLifecycle.governance.mode must be non-empty".to_string());
    }

    match governance_mode {
        "rollover" => {
            let cadence_months = match governance.rollover_cadence_months {
                Some(value) => value,
                None => {
                    failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                    reasons.push(
                        "schemaLifecycle.governance.rolloverCadenceMonths required when mode=rollover"
                            .to_string(),
                    );
                    0
                }
            };
            if cadence_months == 0 || cadence_months > 12 {
                failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                reasons.push(
                    "schemaLifecycle.governance.rolloverCadenceMonths must be within 1..12"
                        .to_string(),
                );
            }
            if let Some(freeze_reason) = governance.freeze_reason.as_deref()
                && !freeze_reason.trim().is_empty()
            {
                failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                reasons.push(
                    "schemaLifecycle.governance.freezeReason is only allowed when mode=freeze"
                        .to_string(),
                );
            }
            if alias_support_epochs.is_empty() {
                failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                reasons.push(
                    "schemaLifecycle.governance.mode=rollover requires compatibility aliases"
                        .to_string(),
                );
            } else if alias_support_epochs.len() != 1 {
                failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                reasons.push(
                    "schemaLifecycle.governance.mode=rollover requires one shared supportUntilEpoch"
                        .to_string(),
                );
            } else if let Some(rollover_epoch) = alias_support_epochs.iter().next() {
                governance_details["rolloverEpoch"] = json!(rollover_epoch);
                match (
                    epoch_to_month_index(active_epoch),
                    epoch_to_month_index(rollover_epoch),
                ) {
                    (Some(active), Some(rollover)) => {
                        let runway = rollover - active;
                        governance_details["aliasRunwayMonths"] = json!(runway);
                        if runway < 1 {
                            failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                            reasons.push(
                                "schemaLifecycle rollover runway must be positive".to_string(),
                            );
                        }
                        if cadence_months > 0 && runway > cadence_months as i32 {
                            failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                            reasons.push(format!(
                                "schemaLifecycle rollover runway exceeds rolloverCadenceMonths ({runway} > {cadence_months})"
                            ));
                        }
                    }
                    _ => {
                        failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                        reasons.push(
                            "schemaLifecycle rollover runway could not be evaluated".to_string(),
                        );
                    }
                }
            }
        }
        "freeze" => {
            if governance.rollover_cadence_months.is_some() {
                failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                reasons.push(
                    "schemaLifecycle.governance.rolloverCadenceMonths is only allowed when mode=rollover"
                        .to_string(),
                );
            }
            let freeze_reason = governance.freeze_reason.as_deref().unwrap_or("").trim();
            if freeze_reason.is_empty() {
                failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                reasons.push(
                    "schemaLifecycle.governance.freezeReason required when mode=freeze".to_string(),
                );
            }
            if !alias_support_epochs.is_empty() {
                failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
                reasons.push(
                    "schemaLifecycle.governance.mode=freeze requires no compatibility aliases"
                        .to_string(),
                );
            }
        }
        _ => {
            failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
            reasons.push(format!(
                "schemaLifecycle.governance.mode unsupported `{governance_mode}` (expected `rollover` or `freeze`)"
            ));
        }
    }

    if let Some(kind) = resolve_or_record_schema_kind(
        schema_lifecycle,
        "controlPlaneContractKind",
        "contractKind",
        &control_plane_contract.contract_kind,
        &mut failures,
        &mut reasons,
    ) {
        resolved["contractKind"] = json!(kind.clone());
        if kind != "premath.control_plane.contract.v1" {
            failures.push(GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string());
            reasons.push(format!(
                "resolved contractKind must be `premath.control_plane.contract.v1` (actual `{kind}`)"
            ));
        }
    }
    if let Some(kind) = resolve_or_record_schema_kind(
        schema_lifecycle,
        "requiredProjectionPolicy",
        "requiredGateProjection.projectionPolicy",
        &control_plane_contract
            .required_gate_projection
            .projection_policy,
        &mut failures,
        &mut reasons,
    ) {
        resolved["requiredProjectionPolicy"] = json!(kind);
    }
    if let Some(kind) = resolve_or_record_schema_kind(
        schema_lifecycle,
        "requiredWitnessKind",
        "requiredWitness.witnessKind",
        &control_plane_contract.required_witness.witness_kind,
        &mut failures,
        &mut reasons,
    ) {
        resolved["requiredWitnessKind"] = json!(kind);
    }
    if let Some(kind) = resolve_or_record_schema_kind(
        schema_lifecycle,
        "requiredDecisionKind",
        "requiredWitness.decisionKind",
        &control_plane_contract.required_witness.decision_kind,
        &mut failures,
        &mut reasons,
    ) {
        resolved["requiredDecisionKind"] = json!(kind);
    }
    if let Some(kind) = resolve_or_record_schema_kind(
        schema_lifecycle,
        "instructionWitnessKind",
        "instructionWitness.witnessKind",
        &control_plane_contract.instruction_witness.witness_kind,
        &mut failures,
        &mut reasons,
    ) {
        resolved["instructionWitnessKind"] = json!(kind);
    }
    if let Some(kind) = resolve_or_record_schema_kind(
        schema_lifecycle,
        "instructionPolicyKind",
        "instructionWitness.policyKind",
        &control_plane_contract.instruction_witness.policy_kind,
        &mut failures,
        &mut reasons,
    ) {
        resolved["instructionPolicyKind"] = json!(kind);
    }

    ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "present": true,
            "activeEpoch": schema_lifecycle.active_epoch,
            "requiredKindFamilies": REQUIRED_SCHEMA_LIFECYCLE_FAMILIES,
            "actualKindFamilies": actual_families,
            "missingKindFamilies": missing_families,
            "unknownKindFamilies": unknown_families,
            "resolvedKinds": resolved,
            "governance": governance_details,
            "reasons": dedupe_sorted(reasons),
        }),
    }
}

fn check_gate_chain_parity(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    let mise_path = resolve_path(repo_root, contract.surfaces.mise_path.as_str());
    let mise_text = read_text(&mise_path)?;
    let baseline_tasks = parse_baseline_task_ids_from_toml(
        &mise_text,
        contract.surfaces.mise_baseline_task.as_str(),
        &mise_path,
    )?;
    let baseline_set: BTreeSet<String> = baseline_tasks.iter().cloned().collect();

    let ci_closure_text = read_text(&resolve_path(
        repo_root,
        contract.surfaces.ci_closure_path.as_str(),
    ))?;
    let ci_baseline_section = extract_section_between(
        &ci_closure_text,
        contract.surfaces.ci_closure_baseline_start.as_str(),
        contract.surfaces.ci_closure_baseline_end.as_str(),
    )?;
    let ci_baseline_set = parse_backticked_tasks(ci_baseline_section)?;

    let control_plane_contract_path = resolve_path(
        repo_root,
        contract.surfaces.control_plane_contract_path.as_str(),
    );
    let control_plane_contract: ControlPlaneProjectionContract =
        serde_json::from_slice(&read_bytes(&control_plane_contract_path)?).map_err(|source| {
            CoherenceError::ParseJson {
                path: display_path(&control_plane_contract_path),
                source,
            }
        })?;
    if control_plane_contract.schema != 1 {
        return Err(CoherenceError::Contract(format!(
            "control-plane contract schema must be 1: {}",
            display_path(&control_plane_contract_path)
        )));
    }
    let projection_checks = dedupe_sorted(
        control_plane_contract
            .required_gate_projection
            .check_order
            .clone(),
    );
    let projection_set: BTreeSet<String> = projection_checks.iter().cloned().collect();

    let ci_projection_section = extract_section_between(
        &ci_closure_text,
        contract.surfaces.ci_closure_projection_start.as_str(),
        contract.surfaces.ci_closure_projection_end.as_str(),
    )?;
    let ci_projection_set = parse_backticked_tasks(ci_projection_section)?;

    let mut failures = Vec::new();
    if control_plane_contract
        .required_gate_projection
        .projection_policy
        .trim()
        .is_empty()
    {
        failures.push("coherence.gate_chain_parity.projection_policy_invalid".to_string());
    }
    if control_plane_contract
        .required_witness
        .witness_kind
        .trim()
        .is_empty()
        || control_plane_contract
            .required_witness
            .decision_kind
            .trim()
            .is_empty()
    {
        failures.push("coherence.gate_chain_parity.required_witness_shape_invalid".to_string());
    }
    if control_plane_contract
        .instruction_witness
        .witness_kind
        .trim()
        .is_empty()
        || control_plane_contract
            .instruction_witness
            .policy_kind
            .trim()
            .is_empty()
        || control_plane_contract
            .instruction_witness
            .policy_digest_prefix
            .trim()
            .is_empty()
    {
        failures.push("coherence.gate_chain_parity.instruction_witness_shape_invalid".to_string());
    }
    if baseline_set != ci_baseline_set {
        failures.push("coherence.gate_chain_parity.baseline_set_mismatch".to_string());
    }
    if projection_set != ci_projection_set {
        failures.push("coherence.gate_chain_parity.projection_set_mismatch".to_string());
    }

    let schema_lifecycle_check = evaluate_control_plane_schema_lifecycle(&control_plane_contract);
    failures.extend(schema_lifecycle_check.failure_classes.clone());

    let stage1_parity_check = evaluate_control_plane_stage1_parity(&control_plane_contract);
    failures.extend(stage1_parity_check.failure_classes.clone());

    let stage1_rollback_check = evaluate_control_plane_stage1_rollback(&control_plane_contract);
    failures.extend(stage1_rollback_check.failure_classes.clone());

    let stage2_authority_check = evaluate_control_plane_stage2_authority(
        &control_plane_contract,
        &contract.required_bidir_obligations,
    );
    failures.extend(stage2_authority_check.failure_classes.clone());
    let evidence_factorization_check =
        evaluate_control_plane_evidence_factorization(&control_plane_contract);
    failures.extend(evidence_factorization_check.failure_classes.clone());

    let lane_registry_check = evaluate_gate_chain_lane_registry(&control_plane_contract);
    failures.extend(lane_registry_check.failure_classes.clone());
    let worker_lane_check = evaluate_gate_chain_worker_lane_authority(&control_plane_contract);
    failures.extend(worker_lane_check.failure_classes.clone());

    let lane_vectors_check = if contract.surfaces.site_fixture_root_path.trim().is_empty() {
        None
    } else {
        let fixture_root =
            resolve_path(repo_root, contract.surfaces.site_fixture_root_path.as_str());
        if fixture_root.join("manifest.json").exists() {
            let check = check_site_obligation(
                repo_root,
                contract,
                "gate_chain_parity",
                evaluate_site_case_gate_chain_parity,
            )?;
            failures.extend(check.failure_classes.clone());
            Some(check)
        } else {
            None
        }
    };

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "baselineFromMise": baseline_tasks,
            "baselineFromCiClosure": sorted_vec_from_set(&ci_baseline_set),
            "projectionPolicy": control_plane_contract.required_gate_projection.projection_policy,
            "projectionFromControlPlane": projection_checks,
            "projectionFromCiClosure": sorted_vec_from_set(&ci_projection_set),
            "requiredWitnessKind": control_plane_contract.required_witness.witness_kind,
            "requiredDecisionKind": control_plane_contract.required_witness.decision_kind,
            "instructionWitnessKind": control_plane_contract.instruction_witness.witness_kind,
            "instructionPolicyKind": control_plane_contract.instruction_witness.policy_kind,
            "instructionPolicyDigestPrefix": control_plane_contract.instruction_witness.policy_digest_prefix,
            "schemaLifecycle": schema_lifecycle_check.details,
            "stage1Parity": stage1_parity_check.details,
            "stage1Rollback": stage1_rollback_check.details,
            "stage2Authority": stage2_authority_check.details,
            "evidenceFactorization": evidence_factorization_check.details,
            "laneRegistry": lane_registry_check.details,
            "workerLaneAuthority": worker_lane_check.details,
            "laneOwnershipVectors": lane_vectors_check.map(|check| check.details),
        }),
    })
}

fn evaluate_control_plane_stage1_parity(
    control_plane_contract: &ControlPlaneProjectionContract,
) -> ObligationCheck {
    let required_failure_classes = json!({
        "missing": STAGE1_PARITY_CLASS_MISSING,
        "mismatch": STAGE1_PARITY_CLASS_MISMATCH,
        "unbound": STAGE1_PARITY_CLASS_UNBOUND,
    });

    let mut details = json!({
        "present": control_plane_contract.evidence_stage1_parity.is_some(),
        "profileKind": null,
        "authorityToTypedCoreRoute": null,
        "comparisonTuple": null,
        "failureClasses": null,
        "requiredFailureClasses": required_failure_classes,
        "reasons": [],
    });

    let Some(stage1) = &control_plane_contract.evidence_stage1_parity else {
        return ObligationCheck {
            failure_classes: Vec::new(),
            details,
        };
    };

    details["profileKind"] = json!(&stage1.profile_kind);
    details["authorityToTypedCoreRoute"] = json!(&stage1.authority_to_typed_core_route);
    details["comparisonTuple"] = json!(&stage1.comparison_tuple);
    details["failureClasses"] = json!(&stage1.failure_classes);

    let mut failures = Vec::new();
    let mut reasons = Vec::new();

    if stage1.profile_kind.trim().is_empty() {
        failures.push(GATE_CHAIN_STAGE1_PARITY_INVALID_FAILURE.to_string());
        reasons.push("evidenceStage1Parity.profileKind must be non-empty".to_string());
    }

    if stage1.authority_to_typed_core_route.trim().is_empty() {
        failures.push(GATE_CHAIN_STAGE1_PARITY_MISSING_FAILURE.to_string());
        reasons
            .push("evidenceStage1Parity.authorityToTypedCoreRoute must be non-empty".to_string());
    }

    if stage1
        .comparison_tuple
        .authority_digest_ref
        .trim()
        .is_empty()
        || stage1
            .comparison_tuple
            .typed_core_digest_ref
            .trim()
            .is_empty()
    {
        failures.push(GATE_CHAIN_STAGE1_PARITY_MISSING_FAILURE.to_string());
        reasons.push(
            "evidenceStage1Parity.comparisonTuple authority/typed-core refs must be non-empty"
                .to_string(),
        );
    }

    let normalizer_ref = stage1.comparison_tuple.normalizer_id_ref.trim();
    let policy_ref = stage1.comparison_tuple.policy_digest_ref.trim();
    if normalizer_ref.is_empty() || policy_ref.is_empty() {
        failures.push(GATE_CHAIN_STAGE1_PARITY_UNBOUND_FAILURE.to_string());
        reasons.push(
            "evidenceStage1Parity.comparisonTuple normalizer/policy refs must be non-empty"
                .to_string(),
        );
    } else {
        if normalizer_ref != "normalizerId" {
            failures.push(GATE_CHAIN_STAGE1_PARITY_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceStage1Parity.comparisonTuple.normalizerIdRef must be `normalizerId` (got `{normalizer_ref}`)"
            ));
        }
        if policy_ref != "policyDigest" {
            failures.push(GATE_CHAIN_STAGE1_PARITY_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceStage1Parity.comparisonTuple.policyDigestRef must be `policyDigest` (got `{policy_ref}`)"
            ));
        }
    }

    let declared_failure_classes = [
        stage1.failure_classes.missing.trim().to_string(),
        stage1.failure_classes.mismatch.trim().to_string(),
        stage1.failure_classes.unbound.trim().to_string(),
    ];
    let declared_failure_set: BTreeSet<String> = declared_failure_classes
        .iter()
        .filter(|class_id| !class_id.is_empty())
        .cloned()
        .collect();
    if declared_failure_set.len() != 3
        || stage1.failure_classes.missing.trim() != STAGE1_PARITY_CLASS_MISSING
        || stage1.failure_classes.mismatch.trim() != STAGE1_PARITY_CLASS_MISMATCH
        || stage1.failure_classes.unbound.trim() != STAGE1_PARITY_CLASS_UNBOUND
    {
        failures.push(GATE_CHAIN_STAGE1_PARITY_MISMATCH_FAILURE.to_string());
        reasons.push(format!(
            "evidenceStage1Parity.failureClasses must map to canonical classes ({STAGE1_PARITY_CLASS_MISSING}, {STAGE1_PARITY_CLASS_MISMATCH}, {STAGE1_PARITY_CLASS_UNBOUND})"
        ));
    }

    details["reasons"] = json!(dedupe_sorted(reasons));

    ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details,
    }
}

fn evaluate_control_plane_stage1_rollback(
    control_plane_contract: &ControlPlaneProjectionContract,
) -> ObligationCheck {
    let required_trigger_failure_classes = json!([
        STAGE1_PARITY_CLASS_MISSING,
        STAGE1_PARITY_CLASS_MISMATCH,
        STAGE1_PARITY_CLASS_UNBOUND,
    ]);
    let required_failure_classes = json!({
        "precondition": STAGE1_ROLLBACK_CLASS_PRECONDITION,
        "identityDrift": STAGE1_ROLLBACK_CLASS_IDENTITY_DRIFT,
        "unbound": STAGE1_ROLLBACK_CLASS_UNBOUND,
    });

    let mut details = json!({
        "present": control_plane_contract.evidence_stage1_rollback.is_some(),
        "profileKind": null,
        "witnessKind": null,
        "fromStage": null,
        "toStage": null,
        "triggerFailureClasses": null,
        "identityRefs": null,
        "failureClasses": null,
        "requiredTriggerFailureClasses": required_trigger_failure_classes,
        "requiredFailureClasses": required_failure_classes,
        "reasons": [],
    });

    let Some(stage1_rollback) = &control_plane_contract.evidence_stage1_rollback else {
        return ObligationCheck {
            failure_classes: Vec::new(),
            details,
        };
    };

    details["profileKind"] = json!(&stage1_rollback.profile_kind);
    details["witnessKind"] = json!(&stage1_rollback.witness_kind);
    details["fromStage"] = json!(&stage1_rollback.from_stage);
    details["toStage"] = json!(&stage1_rollback.to_stage);
    details["triggerFailureClasses"] = json!(&stage1_rollback.trigger_failure_classes);
    details["identityRefs"] = json!(&stage1_rollback.identity_refs);
    details["failureClasses"] = json!(&stage1_rollback.failure_classes);

    let mut failures = Vec::new();
    let mut reasons = Vec::new();

    if stage1_rollback.profile_kind.trim().is_empty() {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_INVALID_FAILURE.to_string());
        reasons.push("evidenceStage1Rollback.profileKind must be non-empty".to_string());
    }
    if stage1_rollback.witness_kind.trim().is_empty() {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_INVALID_FAILURE.to_string());
        reasons.push("evidenceStage1Rollback.witnessKind must be non-empty".to_string());
    }
    if stage1_rollback.from_stage.trim() != "stage1" {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_INVALID_FAILURE.to_string());
        reasons.push("evidenceStage1Rollback.fromStage must be `stage1`".to_string());
    }
    if stage1_rollback.to_stage.trim() != "stage0" {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_INVALID_FAILURE.to_string());
        reasons.push("evidenceStage1Rollback.toStage must be `stage0`".to_string());
    }

    let trigger_classes = dedupe_sorted(stage1_rollback.trigger_failure_classes.clone());
    if trigger_classes.is_empty() {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_PRECONDITION_FAILURE.to_string());
        reasons.push("evidenceStage1Rollback.triggerFailureClasses must be non-empty".to_string());
    } else {
        for required in [
            STAGE1_PARITY_CLASS_MISSING,
            STAGE1_PARITY_CLASS_MISMATCH,
            STAGE1_PARITY_CLASS_UNBOUND,
        ] {
            if !trigger_classes.iter().any(|class_id| class_id == required) {
                failures.push(GATE_CHAIN_STAGE1_ROLLBACK_PRECONDITION_FAILURE.to_string());
                reasons.push(format!(
                    "evidenceStage1Rollback.triggerFailureClasses must include `{required}`"
                ));
            }
        }
    }

    let authority_ref = stage1_rollback.identity_refs.authority_digest_ref.trim();
    let rollback_ref = stage1_rollback
        .identity_refs
        .rollback_authority_digest_ref
        .trim();
    if authority_ref.is_empty() || rollback_ref.is_empty() {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_PRECONDITION_FAILURE.to_string());
        reasons.push(
            "evidenceStage1Rollback.identityRefs authority/rollback refs must be non-empty"
                .to_string(),
        );
    } else if authority_ref == rollback_ref {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_PRECONDITION_FAILURE.to_string());
        reasons.push(
            "evidenceStage1Rollback.identityRefs authority/rollback refs must differ".to_string(),
        );
    }

    let normalizer_ref = stage1_rollback.identity_refs.normalizer_id_ref.trim();
    let policy_ref = stage1_rollback.identity_refs.policy_digest_ref.trim();
    if normalizer_ref.is_empty() || policy_ref.is_empty() {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_UNBOUND_FAILURE.to_string());
        reasons.push(
            "evidenceStage1Rollback.identityRefs normalizer/policy refs must be non-empty"
                .to_string(),
        );
    } else {
        if normalizer_ref != "normalizerId" {
            failures.push(GATE_CHAIN_STAGE1_ROLLBACK_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceStage1Rollback.identityRefs.normalizerIdRef must be `normalizerId` (got `{normalizer_ref}`)"
            ));
        }
        if policy_ref != "policyDigest" {
            failures.push(GATE_CHAIN_STAGE1_ROLLBACK_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceStage1Rollback.identityRefs.policyDigestRef must be `policyDigest` (got `{policy_ref}`)"
            ));
        }
    }

    let declared_failure_classes = [
        stage1_rollback
            .failure_classes
            .precondition
            .trim()
            .to_string(),
        stage1_rollback
            .failure_classes
            .identity_drift
            .trim()
            .to_string(),
        stage1_rollback.failure_classes.unbound.trim().to_string(),
    ];
    let declared_failure_set: BTreeSet<String> = declared_failure_classes
        .iter()
        .filter(|class_id| !class_id.is_empty())
        .cloned()
        .collect();
    if declared_failure_set.len() != 3
        || stage1_rollback.failure_classes.precondition.trim() != STAGE1_ROLLBACK_CLASS_PRECONDITION
        || stage1_rollback.failure_classes.identity_drift.trim()
            != STAGE1_ROLLBACK_CLASS_IDENTITY_DRIFT
        || stage1_rollback.failure_classes.unbound.trim() != STAGE1_ROLLBACK_CLASS_UNBOUND
    {
        failures.push(GATE_CHAIN_STAGE1_ROLLBACK_MISMATCH_FAILURE.to_string());
        reasons.push(format!(
            "evidenceStage1Rollback.failureClasses must map to canonical classes ({STAGE1_ROLLBACK_CLASS_PRECONDITION}, {STAGE1_ROLLBACK_CLASS_IDENTITY_DRIFT}, {STAGE1_ROLLBACK_CLASS_UNBOUND})"
        ));
    }

    details["reasons"] = json!(dedupe_sorted(reasons));

    ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details,
    }
}

fn schema_lifecycle_rollover_epoch(
    control_plane_contract: &ControlPlaneProjectionContract,
) -> Option<String> {
    let schema_lifecycle = control_plane_contract.schema_lifecycle.as_ref()?;
    let mut support_epochs: BTreeSet<String> = BTreeSet::new();
    for family in schema_lifecycle.kind_families.values() {
        for alias in &family.compatibility_aliases {
            let epoch = alias.support_until_epoch.trim();
            if !epoch.is_empty() {
                support_epochs.insert(epoch.to_string());
            }
        }
    }
    if support_epochs.len() == 1 {
        support_epochs.into_iter().next()
    } else {
        None
    }
}

fn evaluate_control_plane_stage2_authority(
    control_plane_contract: &ControlPlaneProjectionContract,
    required_bidir_obligations_input: &[String],
) -> ObligationCheck {
    let required_failure_classes = json!({
        "authorityAliasViolation": STAGE2_AUTHORITY_CLASS_ALIAS_VIOLATION,
        "aliasWindowViolation": STAGE2_AUTHORITY_CLASS_ALIAS_WINDOW_VIOLATION,
        "unbound": STAGE2_AUTHORITY_CLASS_UNBOUND,
    });
    let required_bidir_failure_classes = json!({
        "missing": STAGE2_KERNEL_CLASS_MISSING,
        "drift": STAGE2_KERNEL_CLASS_DRIFT,
    });
    let canonical_kernel_obligations: Vec<String> = STAGE2_REQUIRED_KERNEL_OBLIGATIONS
        .iter()
        .map(|obligation| (*obligation).to_string())
        .collect();
    let required_bidir_obligations = dedupe_sorted(
        required_bidir_obligations_input
            .iter()
            .map(|obligation| obligation.trim().to_string())
            .filter(|obligation| !obligation.is_empty())
            .collect(),
    );
    let required_bidir_set: BTreeSet<String> = required_bidir_obligations.iter().cloned().collect();
    let canonical_kernel_set: BTreeSet<String> =
        canonical_kernel_obligations.iter().cloned().collect();
    let kernel_registry_obligations: BTreeSet<String> = obligation_gate_registry()
        .into_iter()
        .map(|row| row.obligation_kind.to_string())
        .collect();

    let lifecycle_rollover_epoch = schema_lifecycle_rollover_epoch(control_plane_contract);
    let active_epoch = control_plane_contract
        .schema_lifecycle
        .as_ref()
        .map(|lifecycle| lifecycle.active_epoch.trim().to_string());

    let mut details = json!({
        "present": control_plane_contract.evidence_stage2_authority.is_some(),
        "profileKind": null,
        "activeStage": null,
        "typedAuthority": null,
        "compatibilityAlias": null,
        "bidirEvidenceRoute": null,
        "failureClasses": null,
        "kernelComplianceSentinel": null,
        "lifecycleActiveEpoch": active_epoch,
        "lifecycleRolloverEpoch": lifecycle_rollover_epoch,
        "requiredFailureClasses": required_failure_classes,
        "requiredBidirEvidenceFailureClasses": required_bidir_failure_classes,
        "requiredBidirObligations": required_bidir_obligations,
        "canonicalKernelObligations": canonical_kernel_obligations,
        "kernelRegistryObligations": sorted_vec_from_set(&kernel_registry_obligations),
        "reasons": [],
    });

    let Some(stage2) = &control_plane_contract.evidence_stage2_authority else {
        return ObligationCheck {
            failure_classes: Vec::new(),
            details,
        };
    };

    details["profileKind"] = json!(&stage2.profile_kind);
    details["activeStage"] = json!(&stage2.active_stage);
    details["typedAuthority"] = json!(&stage2.typed_authority);
    details["compatibilityAlias"] = json!(&stage2.compatibility_alias);
    details["bidirEvidenceRoute"] = json!(&stage2.bidir_evidence_route);
    details["failureClasses"] = json!(&stage2.failure_classes);
    if let Some(sentinel) = &stage2.kernel_compliance_sentinel {
        details["kernelComplianceSentinel"] = json!(sentinel);
    }

    let mut failures = Vec::new();
    let mut reasons = Vec::new();

    if stage2.profile_kind.trim().is_empty() {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_INVALID_FAILURE.to_string());
        reasons.push("evidenceStage2Authority.profileKind must be non-empty".to_string());
    }
    if stage2.active_stage.trim() != "stage2" {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_INVALID_FAILURE.to_string());
        reasons.push("evidenceStage2Authority.activeStage must be `stage2`".to_string());
    }

    if stage2.typed_authority.kind_ref.trim().is_empty()
        || stage2.typed_authority.digest_ref.trim().is_empty()
    {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_UNBOUND_FAILURE.to_string());
        reasons.push(
            "evidenceStage2Authority.typedAuthority kind/digest refs must be non-empty".to_string(),
        );
    }

    let typed_normalizer_ref = stage2.typed_authority.normalizer_id_ref.trim();
    let typed_policy_ref = stage2.typed_authority.policy_digest_ref.trim();
    if typed_normalizer_ref.is_empty() || typed_policy_ref.is_empty() {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_UNBOUND_FAILURE.to_string());
        reasons.push(
            "evidenceStage2Authority.typedAuthority normalizer/policy refs must be non-empty"
                .to_string(),
        );
    } else {
        if typed_normalizer_ref != "normalizerId" {
            failures.push(GATE_CHAIN_STAGE2_AUTHORITY_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceStage2Authority.typedAuthority.normalizerIdRef must be `normalizerId` (got `{typed_normalizer_ref}`)"
            ));
        }
        if typed_policy_ref != "policyDigest" {
            failures.push(GATE_CHAIN_STAGE2_AUTHORITY_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceStage2Authority.typedAuthority.policyDigestRef must be `policyDigest` (got `{typed_policy_ref}`)"
            ));
        }
    }

    if stage2.compatibility_alias.kind_ref.trim().is_empty()
        || stage2.compatibility_alias.digest_ref.trim().is_empty()
    {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_VIOLATION_FAILURE.to_string());
        reasons.push(
            "evidenceStage2Authority.compatibilityAlias kind/digest refs must be non-empty"
                .to_string(),
        );
    }
    if stage2.compatibility_alias.role.trim() != STAGE2_AUTHORITY_ALIAS_ROLE {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_VIOLATION_FAILURE.to_string());
        reasons.push(format!(
            "evidenceStage2Authority.compatibilityAlias.role must be `{STAGE2_AUTHORITY_ALIAS_ROLE}`"
        ));
    }
    if stage2
        .compatibility_alias
        .support_until_epoch
        .trim()
        .is_empty()
        || !is_valid_epoch(stage2.compatibility_alias.support_until_epoch.trim())
    {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE.to_string());
        reasons.push(
            "evidenceStage2Authority.compatibilityAlias.supportUntilEpoch must be a valid YYYY-MM epoch"
                .to_string(),
        );
    }
    if stage2.typed_authority.digest_ref.trim() == stage2.compatibility_alias.digest_ref.trim() {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_VIOLATION_FAILURE.to_string());
        reasons.push("evidenceStage2Authority typed/alias digest refs must differ".to_string());
    }

    let alias_support_epoch = stage2.compatibility_alias.support_until_epoch.trim();
    if let Some(rollover_epoch) = lifecycle_rollover_epoch.as_deref() {
        if alias_support_epoch != rollover_epoch {
            failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE.to_string());
            reasons.push(
                "evidenceStage2Authority.compatibilityAlias.supportUntilEpoch must align with schemaLifecycle rolloverEpoch"
                    .to_string(),
            );
        }
    } else {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE.to_string());
        reasons
            .push("evidenceStage2Authority requires one schemaLifecycle rolloverEpoch".to_string());
    }
    if let Some(active_epoch_value) = active_epoch.as_deref() {
        match (
            epoch_to_month_index(active_epoch_value),
            epoch_to_month_index(alias_support_epoch),
        ) {
            (Some(active), Some(support)) if active > support => {
                failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE.to_string());
                reasons.push(format!(
                    "evidenceStage2Authority compatibility alias expired (activeEpoch=`{active_epoch_value}`, supportUntilEpoch=`{alias_support_epoch}`)"
                ));
            }
            (Some(_), Some(_)) => {}
            _ => {
                failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE.to_string());
                reasons.push(
                    "evidenceStage2Authority alias-window comparison could not be evaluated"
                        .to_string(),
                );
            }
        }
    } else {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE.to_string());
        reasons.push("evidenceStage2Authority requires schemaLifecycle.activeEpoch".to_string());
    }

    let declared_failure_classes = [
        stage2
            .failure_classes
            .authority_alias_violation
            .trim()
            .to_string(),
        stage2
            .failure_classes
            .alias_window_violation
            .trim()
            .to_string(),
        stage2.failure_classes.unbound.trim().to_string(),
    ];
    let declared_failure_set: BTreeSet<String> = declared_failure_classes
        .iter()
        .filter(|class_id| !class_id.is_empty())
        .cloned()
        .collect();
    if declared_failure_set.len() != 3
        || stage2.failure_classes.authority_alias_violation.trim()
            != STAGE2_AUTHORITY_CLASS_ALIAS_VIOLATION
        || stage2.failure_classes.alias_window_violation.trim()
            != STAGE2_AUTHORITY_CLASS_ALIAS_WINDOW_VIOLATION
        || stage2.failure_classes.unbound.trim() != STAGE2_AUTHORITY_CLASS_UNBOUND
    {
        failures.push(GATE_CHAIN_STAGE2_AUTHORITY_INVALID_FAILURE.to_string());
        reasons.push(format!(
            "evidenceStage2Authority.failureClasses must map to canonical classes ({STAGE2_AUTHORITY_CLASS_ALIAS_VIOLATION}, {STAGE2_AUTHORITY_CLASS_ALIAS_WINDOW_VIOLATION}, {STAGE2_AUTHORITY_CLASS_UNBOUND})"
        ));
    }

    if stage2.bidir_evidence_route.route_kind.trim() != STAGE2_BIDIR_ROUTE_KIND {
        failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
        reasons.push(format!(
            "evidenceStage2Authority.bidirEvidenceRoute.routeKind must be `{STAGE2_BIDIR_ROUTE_KIND}`"
        ));
    }
    if stage2.bidir_evidence_route.obligation_field_ref.trim() != STAGE2_BIDIR_OBLIGATION_FIELD_REF
    {
        failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
        reasons.push(format!(
            "evidenceStage2Authority.bidirEvidenceRoute.obligationFieldRef must be `{STAGE2_BIDIR_OBLIGATION_FIELD_REF}`"
        ));
    }

    let bidir_required = dedupe_sorted(
        stage2
            .bidir_evidence_route
            .required_obligations
            .iter()
            .map(|obligation| obligation.trim().to_string())
            .filter(|obligation| !obligation.is_empty())
            .collect(),
    );
    let bidir_required_set: BTreeSet<String> = bidir_required.iter().cloned().collect();
    if bidir_required.is_empty() {
        failures.push(GATE_CHAIN_STAGE2_KERNEL_MISSING_FAILURE.to_string());
        reasons.push(
            "evidenceStage2Authority.bidirEvidenceRoute.requiredObligations must be non-empty"
                .to_string(),
        );
    } else {
        for required in &required_bidir_obligations {
            if !bidir_required_set.contains(required) {
                failures.push(GATE_CHAIN_STAGE2_KERNEL_MISSING_FAILURE.to_string());
                reasons.push(format!(
                    "evidenceStage2Authority.bidirEvidenceRoute.requiredObligations missing required BIDIR obligation `{required}`"
                ));
            }
        }
    }
    if bidir_required_set != required_bidir_set {
        failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
        reasons.push(
            "evidenceStage2Authority.bidirEvidenceRoute.requiredObligations must match requiredBidirObligations"
                .to_string(),
        );
    }
    if bidir_required_set != canonical_kernel_set {
        failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
        reasons.push(
            "evidenceStage2Authority.bidirEvidenceRoute.requiredObligations must match canonical Stage 2 kernel obligations"
                .to_string(),
        );
    }
    for obligation in &bidir_required {
        if !kernel_registry_obligations.contains(obligation) {
            failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
            reasons.push(format!(
                "evidenceStage2Authority.bidirEvidenceRoute.requiredObligations contains unknown kernel obligation `{obligation}`"
            ));
        }
    }

    let bidir_missing_class = stage2.bidir_evidence_route.failure_classes.missing.trim();
    let bidir_drift_class = stage2.bidir_evidence_route.failure_classes.drift.trim();
    if bidir_missing_class != STAGE2_KERNEL_CLASS_MISSING
        || bidir_drift_class != STAGE2_KERNEL_CLASS_DRIFT
    {
        failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
        reasons.push(format!(
            "evidenceStage2Authority.bidirEvidenceRoute.failureClasses must map to canonical classes ({STAGE2_KERNEL_CLASS_MISSING}, {STAGE2_KERNEL_CLASS_DRIFT})"
        ));
    }

    if let Some(sentinel) = &stage2.kernel_compliance_sentinel {
        let fallback = stage2.bidir_evidence_route.fallback.as_ref();
        let fallback_mode = fallback.map(|value| value.mode.trim()).unwrap_or_default();
        let fallback_profile_kinds: BTreeSet<String> = fallback
            .map(|value| {
                value
                    .profile_kinds
                    .iter()
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        if fallback_mode != STAGE2_BIDIR_FALLBACK_MODE
            || !fallback_profile_kinds.contains(stage2.profile_kind.trim())
        {
            failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
            reasons.push(
                "evidenceStage2Authority.kernelComplianceSentinel requires bidirEvidenceRoute.fallback.mode=`profile_gated_sentinel` with current profileKind included in fallback.profileKinds"
                    .to_string(),
            );
        }
        let sentinel_required = dedupe_sorted(
            sentinel
                .required_obligations
                .iter()
                .map(|obligation| obligation.trim().to_string())
                .filter(|obligation| !obligation.is_empty())
                .collect(),
        );
        let sentinel_required_set: BTreeSet<String> = sentinel_required.iter().cloned().collect();
        if sentinel_required_set != bidir_required_set {
            failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
            reasons.push(
                "evidenceStage2Authority.kernelComplianceSentinel.requiredObligations must match evidenceStage2Authority.bidirEvidenceRoute.requiredObligations"
                    .to_string(),
            );
        }
        let sentinel_missing_class = sentinel.failure_classes.missing.trim();
        let sentinel_drift_class = sentinel.failure_classes.drift.trim();
        if sentinel_missing_class != bidir_missing_class
            || sentinel_drift_class != bidir_drift_class
        {
            failures.push(GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string());
            reasons.push(
                "evidenceStage2Authority.kernelComplianceSentinel.failureClasses must match evidenceStage2Authority.bidirEvidenceRoute.failureClasses"
                    .to_string(),
            );
        }
    }

    details["reasons"] = json!(dedupe_sorted(reasons));

    ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details,
    }
}

fn evaluate_control_plane_evidence_factorization(
    control_plane_contract: &ControlPlaneProjectionContract,
) -> ObligationCheck {
    let required_failure_classes = json!({
        "missing": EVIDENCE_FACTORIZATION_CLASS_MISSING,
        "ambiguous": EVIDENCE_FACTORIZATION_CLASS_AMBIGUOUS,
        "unbound": EVIDENCE_FACTORIZATION_CLASS_UNBOUND,
    });

    let mut details = json!({
        "present": control_plane_contract.evidence_factorization.is_some(),
        "profileKind": null,
        "routeKind": null,
        "factorizationRoutes": null,
        "binding": null,
        "crossLaneRoutes": null,
        "failureClasses": null,
        "requiredRouteKind": EVIDENCE_FACTORIZATION_ROUTE_KIND,
        "requiredPullbackRoute": REQUIRED_PULLBACK_ROUTE,
        "requiredFailureClasses": required_failure_classes,
        "reasons": [],
    });

    let Some(factorization) = &control_plane_contract.evidence_factorization else {
        return ObligationCheck {
            failure_classes: Vec::new(),
            details,
        };
    };

    details["profileKind"] = json!(&factorization.profile_kind);
    details["routeKind"] = json!(&factorization.route_kind);
    details["factorizationRoutes"] = json!(&factorization.factorization_routes);
    details["binding"] = json!(&factorization.binding);
    details["crossLaneRoutes"] = json!(&factorization.cross_lane_routes);
    details["failureClasses"] = json!(&factorization.failure_classes);

    let mut failures = Vec::new();
    let mut reasons = Vec::new();

    if factorization.profile_kind.trim().is_empty() {
        failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_INVALID_FAILURE.to_string());
        reasons.push("evidenceFactorization.profileKind must be non-empty".to_string());
    }
    if factorization.route_kind.trim() != EVIDENCE_FACTORIZATION_ROUTE_KIND {
        failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_INVALID_FAILURE.to_string());
        reasons.push(format!(
            "evidenceFactorization.routeKind must be `{EVIDENCE_FACTORIZATION_ROUTE_KIND}`"
        ));
    }

    let mut route_count = 0usize;
    let mut unique_routes = BTreeSet::new();
    for route in &factorization.factorization_routes {
        route_count += 1;
        let trimmed = route.trim();
        if trimmed.is_empty() {
            failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_MISSING_FAILURE.to_string());
            reasons.push(
                "evidenceFactorization.factorizationRoutes entries must be non-empty".to_string(),
            );
            continue;
        }
        unique_routes.insert(trimmed.to_string());
    }
    if route_count == 0 || unique_routes.is_empty() {
        failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_MISSING_FAILURE.to_string());
        reasons.push("evidenceFactorization.factorizationRoutes must be non-empty".to_string());
    } else if unique_routes.len() > 1 {
        failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_AMBIGUOUS_FAILURE.to_string());
        reasons.push(
            "evidenceFactorization.factorizationRoutes must provide exactly one canonical route"
                .to_string(),
        );
    }

    let normalizer_ref = factorization.binding.normalizer_id_ref.trim();
    let policy_ref = factorization.binding.policy_digest_ref.trim();
    if normalizer_ref.is_empty() || policy_ref.is_empty() {
        failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_UNBOUND_FAILURE.to_string());
        reasons.push(
            "evidenceFactorization.binding normalizer/policy refs must be non-empty".to_string(),
        );
    } else {
        if normalizer_ref != "normalizerId" {
            failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceFactorization.binding.normalizerIdRef must be `normalizerId` (got `{normalizer_ref}`)"
            ));
        }
        if policy_ref != "policyDigest" {
            failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_UNBOUND_FAILURE.to_string());
            reasons.push(format!(
                "evidenceFactorization.binding.policyDigestRef must be `policyDigest` (got `{policy_ref}`)"
            ));
        }
    }

    match &factorization.cross_lane_routes {
        Some(route) if route.pullback_base_change.trim() == REQUIRED_PULLBACK_ROUTE => {}
        _ => {
            failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_MISSING_FAILURE.to_string());
            reasons.push(format!(
                "evidenceFactorization.crossLaneRoutes.pullbackBaseChange must be `{REQUIRED_PULLBACK_ROUTE}`"
            ));
        }
    }

    if let Some(lane_ownership) = &control_plane_contract.lane_ownership
        && let Some(route) = &lane_ownership.required_cross_lane_witness_route
        && let Some(factorization_route) = &factorization.cross_lane_routes
        && route.pullback_base_change.trim() != factorization_route.pullback_base_change.trim()
    {
        failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_AMBIGUOUS_FAILURE.to_string());
        reasons.push(
            "evidenceFactorization.crossLaneRoutes must match laneOwnership.requiredCrossLaneWitnessRoute"
                .to_string(),
        );
    }

    let actual_failure_classes = (
        factorization.failure_classes.missing.trim(),
        factorization.failure_classes.ambiguous.trim(),
        factorization.failure_classes.unbound.trim(),
    );
    if actual_failure_classes.0 != EVIDENCE_FACTORIZATION_CLASS_MISSING
        || actual_failure_classes.1 != EVIDENCE_FACTORIZATION_CLASS_AMBIGUOUS
        || actual_failure_classes.2 != EVIDENCE_FACTORIZATION_CLASS_UNBOUND
    {
        failures.push(GATE_CHAIN_EVIDENCE_FACTORIZATION_INVALID_FAILURE.to_string());
        reasons.push(format!(
            "evidenceFactorization.failureClasses must map to canonical classes ({EVIDENCE_FACTORIZATION_CLASS_MISSING}, {EVIDENCE_FACTORIZATION_CLASS_AMBIGUOUS}, {EVIDENCE_FACTORIZATION_CLASS_UNBOUND})"
        ));
    }

    details["reasons"] = json!(dedupe_sorted(reasons));

    ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details,
    }
}

fn evaluate_gate_chain_lane_registry(
    control_plane_contract: &ControlPlaneProjectionContract,
) -> ObligationCheck {
    let lane_registry_present = control_plane_contract.evidence_lanes.is_some()
        || control_plane_contract.lane_artifact_kinds.is_some()
        || control_plane_contract.lane_ownership.is_some()
        || control_plane_contract.lane_failure_classes.is_some();

    let expected_checker_core_only: Vec<String> = REQUIRED_OBLIGATION_IDS
        .iter()
        .filter(|id| id.starts_with("cwf_"))
        .map(|id| (*id).to_string())
        .collect();
    let mut lane_details = json!({
        "registryPresent": lane_registry_present,
        "evidenceLanes": null,
        "laneArtifactKinds": null,
        "laneOwnership": null,
        "laneFailureClasses": null,
        "expectedCheckerCoreOnlyObligations": expected_checker_core_only,
        "requiredCrossLaneWitnessRoute": REQUIRED_PULLBACK_ROUTE,
        "requiredLaneFailureClasses": REQUIRED_LANE_FAILURE_CLASSES,
    });

    if !lane_registry_present {
        return ObligationCheck {
            failure_classes: Vec::new(),
            details: lane_details,
        };
    }

    let mut failures = Vec::new();
    let expected_checker_core: BTreeSet<String> = REQUIRED_OBLIGATION_IDS
        .iter()
        .filter(|id| id.starts_with("cwf_"))
        .map(|id| (*id).to_string())
        .collect();

    let Some(evidence_lanes) = &control_plane_contract.evidence_lanes else {
        failures.push("coherence.gate_chain_parity.lane_unknown".to_string());
        lane_details["registryPresent"] = json!(true);
        lane_details["error"] =
            json!("evidenceLanes missing while lane registry fields are present");
        return ObligationCheck {
            failure_classes: dedupe_sorted(failures),
            details: lane_details,
        };
    };

    let lane_ids = vec![
        evidence_lanes.semantic_doctrine.trim().to_string(),
        evidence_lanes.strict_checker.trim().to_string(),
        evidence_lanes.witness_commutation.trim().to_string(),
        evidence_lanes.runtime_transport.trim().to_string(),
    ];
    lane_details["evidenceLanes"] = json!({
        "semanticDoctrine": &evidence_lanes.semantic_doctrine,
        "strictChecker": &evidence_lanes.strict_checker,
        "witnessCommutation": &evidence_lanes.witness_commutation,
        "runtimeTransport": &evidence_lanes.runtime_transport,
    });
    if lane_ids.iter().any(|id| id.is_empty()) {
        failures.push("coherence.gate_chain_parity.lane_unknown".to_string());
    }
    let lane_id_set: BTreeSet<String> = lane_ids.into_iter().collect();
    if lane_id_set.len() != 4 {
        failures.push("coherence.gate_chain_parity.lane_unknown".to_string());
    }

    let lane_artifact_kinds = control_plane_contract
        .lane_artifact_kinds
        .as_ref()
        .cloned()
        .unwrap_or_default();
    lane_details["laneArtifactKinds"] = json!(&lane_artifact_kinds);
    if lane_artifact_kinds.is_empty() {
        failures.push("coherence.gate_chain_parity.lane_kind_unbound".to_string());
    }
    for (lane_id, kinds) in &lane_artifact_kinds {
        if !lane_id_set.contains(lane_id) {
            failures.push("coherence.gate_chain_parity.lane_kind_unbound".to_string());
        }
        let mut seen = BTreeSet::new();
        for kind in kinds {
            if kind.trim().is_empty() || !seen.insert(kind.trim().to_string()) {
                failures.push("coherence.gate_chain_parity.lane_kind_unbound".to_string());
            }
        }
        if kinds.is_empty() {
            failures.push("coherence.gate_chain_parity.lane_kind_unbound".to_string());
        }
    }

    let lane_ownership = control_plane_contract.lane_ownership.clone();
    lane_details["laneOwnership"] = json!(&lane_ownership);
    match lane_ownership {
        Some(ownership) => {
            let checker_core_only: BTreeSet<String> = ownership
                .checker_core_only_obligations
                .iter()
                .map(|obligation| obligation.trim().to_string())
                .collect();
            if checker_core_only.is_empty()
                || checker_core_only
                    .iter()
                    .any(|obligation| obligation.is_empty() || !obligation.starts_with("cwf_"))
                || checker_core_only != expected_checker_core
            {
                failures.push("coherence.gate_chain_parity.lane_ownership_violation".to_string());
            }
            match ownership.required_cross_lane_witness_route {
                Some(route) if route.pullback_base_change.trim() == REQUIRED_PULLBACK_ROUTE => {}
                _ => failures.push("coherence.gate_chain_parity.lane_route_missing".to_string()),
            }
        }
        None => failures.push("coherence.gate_chain_parity.lane_ownership_violation".to_string()),
    }

    let lane_failure_classes = control_plane_contract
        .lane_failure_classes
        .clone()
        .unwrap_or_default();
    lane_details["laneFailureClasses"] = json!(&lane_failure_classes);
    if lane_failure_classes.is_empty() {
        failures.push("coherence.gate_chain_parity.lane_failure_class_mismatch".to_string());
    } else {
        let lane_failure_set: BTreeSet<String> = lane_failure_classes
            .iter()
            .map(|class_id| class_id.trim().to_string())
            .collect();
        if lane_failure_set.len() != lane_failure_classes.len()
            || lane_failure_set.iter().any(|class_id| class_id.is_empty())
        {
            failures.push("coherence.gate_chain_parity.lane_failure_class_mismatch".to_string());
        }
        for required in REQUIRED_LANE_FAILURE_CLASSES {
            if !lane_failure_set.contains(*required) {
                failures
                    .push("coherence.gate_chain_parity.lane_failure_class_mismatch".to_string());
            }
        }
    }

    ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: lane_details,
    }
}

fn evaluate_gate_chain_worker_lane_authority(
    control_plane_contract: &ControlPlaneProjectionContract,
) -> ObligationCheck {
    let lane_registry_present = control_plane_contract.evidence_lanes.is_some()
        || control_plane_contract.lane_artifact_kinds.is_some()
        || control_plane_contract.lane_ownership.is_some()
        || control_plane_contract.lane_failure_classes.is_some();
    let active_epoch = control_plane_contract
        .schema_lifecycle
        .as_ref()
        .map(|lifecycle| lifecycle.active_epoch.trim().to_string());

    let required_allowed_modes: Vec<String> = WORKER_ALLOWED_MUTATION_MODES
        .iter()
        .map(|value| (*value).to_string())
        .collect();
    let mut details = json!({
        "present": control_plane_contract.worker_lane_authority.is_some(),
        "laneRegistryPresent": lane_registry_present,
        "activeEpoch": active_epoch.clone(),
        "requiredDefaultMode": WORKER_MUTATION_DEFAULT_MODE,
        "requiredAllowedModes": required_allowed_modes,
        "requiredMutationRoutes": {
            "issueClaim": WORKER_ROUTE_ISSUE_CLAIM,
            "issueLeaseRenew": WORKER_ROUTE_ISSUE_LEASE_RENEW,
            "issueLeaseRelease": WORKER_ROUTE_ISSUE_LEASE_RELEASE,
            "issueDiscover": WORKER_ROUTE_ISSUE_DISCOVER,
        },
        "requiredFailureClasses": {
            "policyDrift": WORKER_CLASS_POLICY_DRIFT,
            "mutationModeDrift": WORKER_CLASS_MUTATION_MODE_DRIFT,
            "routeUnbound": WORKER_CLASS_ROUTE_UNBOUND,
        },
        "mutationPolicy": null,
        "mutationRoutes": null,
        "failureClasses": null,
        "compatibilityOverrides": null,
    });

    if !lane_registry_present && control_plane_contract.worker_lane_authority.is_none() {
        return ObligationCheck {
            failure_classes: Vec::new(),
            details,
        };
    }

    let mut failures = Vec::new();
    let Some(worker_lane) = &control_plane_contract.worker_lane_authority else {
        failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
        details["reason"] = json!("workerLaneAuthority missing while lane registry is present");
        return ObligationCheck {
            failure_classes: dedupe_sorted(failures),
            details,
        };
    };

    details["mutationPolicy"] = json!(&worker_lane.mutation_policy);
    details["mutationRoutes"] = json!(&worker_lane.mutation_routes);
    details["failureClasses"] = json!(&worker_lane.failure_classes);
    details["compatibilityOverrides"] = json!(&worker_lane.mutation_policy.compatibility_overrides);

    let default_mode = worker_lane.mutation_policy.default_mode.trim();
    if default_mode != WORKER_MUTATION_DEFAULT_MODE {
        failures.push(GATE_CHAIN_WORKER_MUTATION_MODE_DRIFT_FAILURE.to_string());
    }

    let mut allowed_modes = BTreeSet::new();
    for mode in &worker_lane.mutation_policy.allowed_modes {
        let mode_norm = mode.trim();
        if mode_norm.is_empty() || !allowed_modes.insert(mode_norm.to_string()) {
            failures.push(GATE_CHAIN_WORKER_MUTATION_MODE_DRIFT_FAILURE.to_string());
        }
    }
    if allowed_modes.is_empty() || !allowed_modes.contains(WORKER_MUTATION_DEFAULT_MODE) {
        failures.push(GATE_CHAIN_WORKER_MUTATION_MODE_DRIFT_FAILURE.to_string());
    }
    let required_allowed_modes: BTreeSet<String> = WORKER_ALLOWED_MUTATION_MODES
        .iter()
        .map(|mode| (*mode).to_string())
        .collect();
    if allowed_modes != required_allowed_modes {
        failures.push(GATE_CHAIN_WORKER_MUTATION_MODE_DRIFT_FAILURE.to_string());
    }

    let mut seen_override_modes: BTreeSet<String> = BTreeSet::new();
    for override_row in &worker_lane.mutation_policy.compatibility_overrides {
        let mode = override_row.mode.trim();
        let support_until_epoch = override_row.support_until_epoch.trim();
        if mode.is_empty()
            || mode == WORKER_MUTATION_DEFAULT_MODE
            || !allowed_modes.contains(mode)
            || !seen_override_modes.insert(mode.to_string())
        {
            failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
        }
        if !override_row.requires_reason {
            failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
        }
        if !is_valid_epoch(support_until_epoch) {
            failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
        } else if let Some(active_epoch_value) = active_epoch.as_deref() {
            if is_valid_epoch(active_epoch_value) && active_epoch_value > support_until_epoch {
                failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
            }
            if let (Some(active_index), Some(support_index)) = (
                epoch_to_month_index(active_epoch_value),
                epoch_to_month_index(support_until_epoch),
            ) && support_index - active_index > 12
            {
                failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
            }
        }
    }

    let expected_override_modes: BTreeSet<String> = required_allowed_modes
        .iter()
        .filter(|mode| mode.as_str() != WORKER_MUTATION_DEFAULT_MODE)
        .cloned()
        .collect();
    if seen_override_modes != expected_override_modes {
        failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
    }

    if worker_lane.mutation_routes.issue_claim.trim() != WORKER_ROUTE_ISSUE_CLAIM
        || worker_lane.mutation_routes.issue_lease_renew.trim() != WORKER_ROUTE_ISSUE_LEASE_RENEW
        || worker_lane.mutation_routes.issue_lease_release.trim()
            != WORKER_ROUTE_ISSUE_LEASE_RELEASE
        || worker_lane.mutation_routes.issue_discover.trim() != WORKER_ROUTE_ISSUE_DISCOVER
    {
        failures.push(GATE_CHAIN_WORKER_ROUTE_UNBOUND_FAILURE.to_string());
    }

    if worker_lane.failure_classes.policy_drift.trim() != WORKER_CLASS_POLICY_DRIFT {
        failures.push(GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string());
    }
    if worker_lane.failure_classes.mutation_mode_drift.trim() != WORKER_CLASS_MUTATION_MODE_DRIFT {
        failures.push(GATE_CHAIN_WORKER_MUTATION_MODE_DRIFT_FAILURE.to_string());
    }
    if worker_lane.failure_classes.route_unbound.trim() != WORKER_CLASS_ROUTE_UNBOUND {
        failures.push(GATE_CHAIN_WORKER_ROUTE_UNBOUND_FAILURE.to_string());
    }

    ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details,
    }
}

fn evaluate_site_case_gate_chain_parity(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;

    let control_plane_contract_value = artifacts.get("controlPlaneContract").ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts.controlPlaneContract must be present",
            display_path(case_path)
        ))
    })?;
    let control_plane_contract: ControlPlaneProjectionContract =
        serde_json::from_value(control_plane_contract_value.clone()).map_err(|source| {
            CoherenceError::Contract(format!(
                "{}: artifacts.controlPlaneContract invalid: {}",
                display_path(case_path),
                source
            ))
        })?;

    if control_plane_contract.schema != 1 {
        return Err(CoherenceError::Contract(format!(
            "{}: artifacts.controlPlaneContract.schema must be 1",
            display_path(case_path)
        )));
    }
    if control_plane_contract.contract_kind != "premath.control_plane.contract.v1" {
        return Err(CoherenceError::Contract(format!(
            "{}: artifacts.controlPlaneContract.contractKind mismatch: {:?}",
            display_path(case_path),
            control_plane_contract.contract_kind
        )));
    }

    let stage1_parity_check = evaluate_control_plane_stage1_parity(&control_plane_contract);
    let stage1_rollback_check = evaluate_control_plane_stage1_rollback(&control_plane_contract);
    let required_bidir_obligations: Vec<String> = STAGE2_REQUIRED_KERNEL_OBLIGATIONS
        .iter()
        .map(|obligation| (*obligation).to_string())
        .collect();
    let stage2_authority_check = evaluate_control_plane_stage2_authority(
        &control_plane_contract,
        &required_bidir_obligations,
    );
    let evidence_factorization_check =
        evaluate_control_plane_evidence_factorization(&control_plane_contract);
    let lane_registry_check = evaluate_gate_chain_lane_registry(&control_plane_contract);
    let worker_lane_check = evaluate_gate_chain_worker_lane_authority(&control_plane_contract);
    let mut failures = Vec::new();
    failures.extend(stage1_parity_check.failure_classes.clone());
    failures.extend(stage1_rollback_check.failure_classes.clone());
    failures.extend(stage2_authority_check.failure_classes.clone());
    failures.extend(evidence_factorization_check.failure_classes.clone());
    failures.extend(lane_registry_check.failure_classes.clone());
    failures.extend(worker_lane_check.failure_classes.clone());
    let failures = dedupe_sorted(failures);
    let result = if failures.is_empty() {
        "accepted"
    } else {
        "rejected"
    };
    Ok(SiteEvaluation {
        result: result.to_string(),
        failure_classes: failures,
        details: json!({
            "stage1Parity": stage1_parity_check.details,
            "stage1Rollback": stage1_rollback_check.details,
            "stage2Authority": stage2_authority_check.details,
            "evidenceFactorization": evidence_factorization_check.details,
            "laneRegistry": lane_registry_check.details,
            "workerLaneAuthority": worker_lane_check.details,
        }),
    })
}

fn check_operation_reachability(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    let doctrine_site_path = resolve_path(repo_root, contract.surfaces.doctrine_site_path.as_str());
    let doctrine_site: DoctrineSite = serde_json::from_slice(&read_bytes(&doctrine_site_path)?)
        .map_err(|source| CoherenceError::ParseJson {
            path: display_path(&doctrine_site_path),
            source,
        })?;

    let mut operation_path_to_id: BTreeMap<String, String> = BTreeMap::new();
    for node in &doctrine_site.nodes {
        if node.kind == "operation" {
            operation_path_to_id.insert(node.path.clone(), node.id.clone());
        }
    }

    let mut failures = Vec::new();
    let mut operation_ids = Vec::new();
    for path in &contract.expected_operation_paths {
        let disk_path = resolve_path(repo_root, path.as_str());
        if !disk_path.exists() {
            failures.push("coherence.operation_reachability.operation_path_missing".to_string());
        }
        match operation_path_to_id.get(path) {
            Some(operation_id) => operation_ids.push(operation_id.clone()),
            None => {
                failures.push("coherence.operation_reachability.operation_node_missing".to_string())
            }
        }
    }

    let reachable = compute_doctrine_reachability(
        &doctrine_site,
        contract.surfaces.doctrine_root_node_id.as_str(),
    );
    for op_id in &operation_ids {
        if !reachable.contains(op_id) {
            failures.push("coherence.operation_reachability.operation_unreachable".to_string());
        }
    }

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "expectedOperationPaths": contract.expected_operation_paths,
            "operationNodeIds": operation_ids,
            "reachableCount": reachable.len(),
            "rootNodeId": contract.surfaces.doctrine_root_node_id,
        }),
    })
}

fn check_overlay_traceability(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    let spec_index_text = read_text(&resolve_path(
        repo_root,
        contract.surfaces.spec_index_path.as_str(),
    ))?;
    let section_56 = extract_heading_section(
        &spec_index_text,
        contract.surfaces.spec_index_overlay_heading.as_str(),
    )?;
    let profile_readme_text = read_text(&resolve_path(
        repo_root,
        contract.surfaces.profile_readme_path.as_str(),
    ))?;

    let mut failures = Vec::new();
    for overlay_ref in &contract.overlay_docs {
        let overlay_markdown = format!("{overlay_ref}.md");
        let overlay_path = resolve_path(repo_root, format!("specs/premath/{overlay_markdown}"));
        if !overlay_path.exists() {
            failures.push("coherence.overlay_traceability.overlay_file_missing".to_string());
        }
        let overlay_token = format!("`{overlay_ref}`");
        if !section_56.contains(&overlay_token) {
            failures
                .push("coherence.overlay_traceability.overlay_missing_in_spec_index".to_string());
        }
        let overlay_file = overlay_markdown
            .split('/')
            .next_back()
            .unwrap_or(overlay_markdown.as_str());
        if !profile_readme_text.contains(overlay_file) {
            failures.push(
                "coherence.overlay_traceability.overlay_missing_in_profile_readme".to_string(),
            );
        }
    }

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "overlayDocs": contract.overlay_docs,
            "specIndexOverlaySectionFound": !section_56.is_empty(),
        }),
    })
}

fn check_transport_functoriality(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    let fixture_root = resolve_path(
        repo_root,
        contract.surfaces.transport_fixture_root_path.as_str(),
    );
    let manifest_path = fixture_root.join("manifest.json");
    let manifest: TransportManifest = serde_json::from_slice(&read_bytes(&manifest_path)?)
        .map_err(|source| CoherenceError::ParseJson {
            path: display_path(&manifest_path),
            source,
        })?;

    let mut failures = Vec::new();
    if manifest.schema != 1 {
        failures.push("coherence.transport_functoriality.manifest_invalid_schema".to_string());
    }
    if manifest.status != "executable" {
        failures.push("coherence.transport_functoriality.manifest_invalid_status".to_string());
    }
    if manifest.vectors.is_empty() {
        failures.push("coherence.transport_functoriality.manifest_empty".to_string());
    }

    let mut seen_vectors = BTreeSet::new();
    let mut vector_rows: Vec<Value> = Vec::new();
    let mut invariance_groups: InvarianceGroups = BTreeMap::new();
    let mut polarity = PolarityCoverage::default();

    for vector_id in &manifest.vectors {
        if !seen_vectors.insert(vector_id.clone()) {
            failures.push("coherence.transport_functoriality.duplicate_vector_id".to_string());
        }

        let vector_root = fixture_root.join(vector_id);
        let case_path = vector_root.join("case.json");
        let expect_path = vector_root.join("expect.json");

        let case_payload = match read_json_value(&case_path) {
            Ok(payload) => payload,
            Err(err) => {
                failures.push("coherence.transport_functoriality.vector_case_invalid".to_string());
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": err.to_string(),
                }));
                continue;
            }
        };
        polarity.record_vector_id(vector_id);
        let expect_bytes = match read_bytes(&expect_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                failures
                    .push("coherence.transport_functoriality.vector_expect_invalid".to_string());
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": err.to_string(),
                }));
                continue;
            }
        };
        let expect_payload: TransportExpect = match serde_json::from_slice(&expect_bytes) {
            Ok(payload) => payload,
            Err(source) => {
                failures
                    .push("coherence.transport_functoriality.vector_expect_invalid".to_string());
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": CoherenceError::ParseJson {
                        path: display_path(&expect_path),
                        source,
                    }.to_string(),
                }));
                continue;
            }
        };

        let expected_result = expect_payload.result.as_str();
        if expect_payload.schema != 1 {
            failures
                .push("coherence.transport_functoriality.vector_expect_invalid_schema".to_string());
        }
        if expect_payload.status != "executable" {
            failures
                .push("coherence.transport_functoriality.vector_expect_invalid_status".to_string());
        }
        if expected_result != "accepted" && expected_result != "rejected" {
            failures
                .push("coherence.transport_functoriality.vector_expect_invalid_result".to_string());
        } else {
            polarity.record_expected_result(expected_result);
        }
        let expected_failure_classes =
            dedupe_sorted(expect_payload.expected_failure_classes.clone());

        let evaluated = match evaluate_transport_case(&case_payload, &case_path) {
            Ok(ok) => ok,
            Err(err) => {
                failures.push("coherence.transport_functoriality.vector_invalid_shape".to_string());
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": err.to_string(),
                }));
                continue;
            }
        };

        if expected_result == "accepted" || expected_result == "rejected" {
            if evaluated.result != expected_result {
                failures.push("coherence.transport_functoriality.result_mismatch".to_string());
            }
            if !expected_failure_classes.is_empty() {
                let actual_failures = dedupe_sorted(evaluated.failure_classes.clone());
                if expected_failure_classes != actual_failures {
                    failures.push(
                        "coherence.transport_functoriality.failure_class_mismatch".to_string(),
                    );
                }
            }
        }

        if vector_id.starts_with("invariance/") {
            record_invariance_row(
                &mut failures,
                "coherence.transport_functoriality",
                &mut invariance_groups,
                InvarianceObservation {
                    vector_id,
                    semantic_scenario_id: case_payload
                        .get("semanticScenarioId")
                        .and_then(Value::as_str),
                    profile: case_payload.get("profile").and_then(Value::as_str),
                    result: &evaluated.result,
                    failure_classes: &evaluated.failure_classes,
                },
            );
        }

        vector_rows.push(json!({
            "vectorId": vector_id,
            "semanticScenarioId": case_payload.get("semanticScenarioId"),
            "profile": case_payload.get("profile"),
            "expectedResult": expected_result,
            "actualResult": evaluated.result,
            "expectedFailureClasses": expected_failure_classes,
            "actualFailureClasses": evaluated.failure_classes,
            "details": evaluated.details,
        }));
    }

    let invariance_rows = validate_invariance_groups(
        &mut failures,
        "coherence.transport_functoriality",
        &invariance_groups,
    );
    polarity.emit_missing_failures(&mut failures, "coherence.transport_functoriality", true);

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "fixtureRoot": to_repo_relative_or_absolute(repo_root, &fixture_root),
            "manifestVectors": manifest.vectors,
            "matchedVectorKinds": polarity.vector_kind_details(),
            "matchedExpectedResults": polarity.expected_result_details(),
            "invariance": invariance_rows,
            "vectors": vector_rows,
        }),
    })
}

#[derive(Debug)]
struct SiteEvaluation {
    result: String,
    failure_classes: Vec<String>,
    details: Value,
}

fn check_coverage_base_change(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "coverage_base_change",
        evaluate_site_case_coverage_base_change,
    )
}

fn check_span_square_commutation(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "span_square_commutation",
        evaluate_site_case_span_square_commutation,
    )
}

fn check_coverage_transitivity(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "coverage_transitivity",
        evaluate_site_case_coverage_transitivity,
    )
}

fn check_glue_or_witness_contractibility(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "glue_or_witness_contractibility",
        evaluate_site_case_glue_or_witness_contractibility,
    )
}

fn check_cwf_substitution_identity(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "cwf_substitution_identity",
        evaluate_site_case_cwf_substitution_identity,
    )
}

fn check_cwf_substitution_composition(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "cwf_substitution_composition",
        evaluate_site_case_cwf_substitution_composition,
    )
}

fn check_cwf_comprehension_beta(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "cwf_comprehension_beta",
        evaluate_site_case_cwf_comprehension_beta,
    )
}

fn check_cwf_comprehension_eta(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    check_site_obligation(
        repo_root,
        contract,
        "cwf_comprehension_eta",
        evaluate_site_case_cwf_comprehension_eta,
    )
}

fn check_site_obligation(
    repo_root: &Path,
    contract: &CoherenceContract,
    obligation_id: &str,
    evaluator: fn(&Value, &Path) -> Result<SiteEvaluation, CoherenceError>,
) -> Result<ObligationCheck, CoherenceError> {
    let fixture_root = resolve_path(repo_root, contract.surfaces.site_fixture_root_path.as_str());
    let manifest_path = fixture_root.join("manifest.json");
    let manifest: SiteManifest =
        serde_json::from_slice(&read_bytes(&manifest_path)?).map_err(|source| {
            CoherenceError::ParseJson {
                path: display_path(&manifest_path),
                source,
            }
        })?;

    let mut failures = Vec::new();
    if manifest.schema != 1 {
        failures.push(format!("coherence.{obligation_id}.manifest_invalid_schema"));
    }
    if manifest.status != "executable" {
        failures.push(format!("coherence.{obligation_id}.manifest_invalid_status"));
    }
    if manifest.vectors.is_empty() {
        failures.push(format!("coherence.{obligation_id}.manifest_empty"));
    }

    let manifest_vector_set: BTreeSet<String> = manifest.vectors.iter().cloned().collect();
    let scoped_vectors: Vec<String> = if manifest.obligation_vectors.is_empty() {
        failures.push(format!(
            "coherence.{obligation_id}.manifest_missing_obligation_vectors"
        ));
        manifest.vectors.clone()
    } else {
        manifest
            .obligation_vectors
            .get(obligation_id)
            .cloned()
            .unwrap_or_default()
    };
    for vector_id in &scoped_vectors {
        if !manifest_vector_set.contains(vector_id) {
            failures.push(format!(
                "coherence.{obligation_id}.manifest_obligation_vector_not_declared"
            ));
        }
    }

    let mut seen_vectors = BTreeSet::new();
    let mut vector_rows: Vec<Value> = Vec::new();
    let mut matched_count = 0usize;
    let mut polarity = PolarityCoverage::default();
    let mut invariance_groups: InvarianceGroups = BTreeMap::new();
    let invariance_failure_prefix = format!("coherence.{obligation_id}");

    for vector_id in &scoped_vectors {
        if !seen_vectors.insert(vector_id.clone()) {
            failures.push(format!("coherence.{obligation_id}.duplicate_vector_id"));
        }

        let vector_root = fixture_root.join(vector_id);
        let case_path = vector_root.join("case.json");
        let expect_path = vector_root.join("expect.json");

        let case_bytes = match read_bytes(&case_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                failures.push(format!("coherence.{obligation_id}.vector_case_invalid"));
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": err.to_string(),
                }));
                continue;
            }
        };
        let case_payload: SiteCase = match serde_json::from_slice(&case_bytes) {
            Ok(payload) => payload,
            Err(source) => {
                failures.push(format!("coherence.{obligation_id}.vector_case_invalid"));
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": CoherenceError::ParseJson {
                        path: display_path(&case_path),
                        source,
                    }.to_string(),
                }));
                continue;
            }
        };

        if case_payload.obligation_id != obligation_id {
            failures.push(format!(
                "coherence.{obligation_id}.manifest_obligation_vector_mismatch"
            ));
            continue;
        }
        matched_count += 1;
        polarity.record_vector_id(vector_id);

        if case_payload.schema != 1 {
            failures.push(format!(
                "coherence.{obligation_id}.vector_case_invalid_schema"
            ));
        }
        if case_payload.status != "executable" {
            failures.push(format!(
                "coherence.{obligation_id}.vector_case_invalid_status"
            ));
        }

        let expect_bytes = match read_bytes(&expect_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                failures.push(format!("coherence.{obligation_id}.vector_expect_invalid"));
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": err.to_string(),
                }));
                continue;
            }
        };
        let expect_payload: SiteExpect = match serde_json::from_slice(&expect_bytes) {
            Ok(payload) => payload,
            Err(source) => {
                failures.push(format!("coherence.{obligation_id}.vector_expect_invalid"));
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": CoherenceError::ParseJson {
                        path: display_path(&expect_path),
                        source,
                    }.to_string(),
                }));
                continue;
            }
        };

        let expected_result = expect_payload.result.as_str();
        if expect_payload.schema != 1 {
            failures.push(format!(
                "coherence.{obligation_id}.vector_expect_invalid_schema"
            ));
        }
        if expect_payload.status != "executable" {
            failures.push(format!(
                "coherence.{obligation_id}.vector_expect_invalid_status"
            ));
        }
        if expected_result != "accepted" && expected_result != "rejected" {
            failures.push(format!(
                "coherence.{obligation_id}.vector_expect_invalid_result"
            ));
        } else {
            polarity.record_expected_result(expected_result);
        }
        let expected_failure_classes = dedupe_sorted(expect_payload.expected_failure_classes);

        let evaluated = match evaluator(&case_payload.artifacts, &case_path) {
            Ok(ok) => ok,
            Err(err) => {
                failures.push(format!("coherence.{obligation_id}.vector_invalid_shape"));
                vector_rows.push(json!({
                    "vectorId": vector_id,
                    "result": "error",
                    "error": err.to_string(),
                }));
                continue;
            }
        };

        if expected_result == "accepted" || expected_result == "rejected" {
            if evaluated.result != expected_result {
                failures.push(format!("coherence.{obligation_id}.result_mismatch"));
            }
            if !expected_failure_classes.is_empty() {
                let actual_failures = dedupe_sorted(evaluated.failure_classes.clone());
                if expected_failure_classes != actual_failures {
                    failures.push(format!("coherence.{obligation_id}.failure_class_mismatch"));
                }
            }
        }

        if vector_id.starts_with("invariance/") {
            record_invariance_row(
                &mut failures,
                invariance_failure_prefix.as_str(),
                &mut invariance_groups,
                InvarianceObservation {
                    vector_id,
                    semantic_scenario_id: case_payload.semantic_scenario_id.as_deref(),
                    profile: case_payload.profile.as_deref(),
                    result: &evaluated.result,
                    failure_classes: &evaluated.failure_classes,
                },
            );
        }

        vector_rows.push(json!({
            "vectorId": vector_id,
            "semanticScenarioId": case_payload.semantic_scenario_id,
            "profile": case_payload.profile,
            "expectedResult": expected_result,
            "actualResult": evaluated.result,
            "expectedFailureClasses": expected_failure_classes,
            "actualFailureClasses": evaluated.failure_classes,
            "details": evaluated.details,
        }));
    }

    let invariance_rows = validate_invariance_groups(
        &mut failures,
        invariance_failure_prefix.as_str(),
        &invariance_groups,
    );

    if matched_count == 0 {
        failures.push(format!(
            "coherence.{obligation_id}.manifest_missing_vectors"
        ));
    }
    polarity.emit_missing_failures(
        &mut failures,
        invariance_failure_prefix.as_str(),
        matched_count > 0,
    );

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "fixtureRoot": to_repo_relative_or_absolute(repo_root, &fixture_root),
            "manifestVectors": manifest.vectors,
            "manifestObligationVectors": manifest.obligation_vectors,
            "scopedVectors": scoped_vectors,
            "matchedVectors": matched_count,
            "matchedVectorKinds": polarity.vector_kind_details(),
            "matchedExpectedResults": polarity.expected_result_details(),
            "invariance": invariance_rows,
            "vectors": vector_rows,
        }),
    })
}

fn evaluate_site_case_coverage_base_change(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let coverage = require_object_field(artifacts, "coverage", case_path)?;
    let base_cover = require_object_field(coverage, "baseCover", case_path)?;
    let pullback_cover = require_object_field(coverage, "pullbackCover", case_path)?;

    let base_parts = require_string_array_field(
        base_cover,
        "parts",
        case_path,
        "artifacts.coverage.baseCover",
    )?;
    let pullback_parts = require_string_array_field(
        pullback_cover,
        "parts",
        case_path,
        "artifacts.coverage.pullbackCover",
    )?;

    let pullback_of_parts = coverage
        .get("pullbackOfParts")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.coverage.pullbackOfParts must be an array",
                display_path(case_path)
            ))
        })?;

    let mut source_parts: Vec<String> = Vec::new();
    let mut mapped_pullback_parts: Vec<String> = Vec::new();
    for item in pullback_of_parts {
        let row = item.as_object().ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: pullbackOfParts entries must be objects",
                display_path(case_path)
            ))
        })?;
        let source = row
            .get("source")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: pullbackOfParts[].source must be non-empty string",
                    display_path(case_path)
                ))
            })?;
        let pullback = row
            .get("pullback")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: pullbackOfParts[].pullback must be non-empty string",
                    display_path(case_path)
                ))
            })?;
        source_parts.push(source.to_string());
        mapped_pullback_parts.push(pullback.to_string());
    }

    let base_set: BTreeSet<String> = base_parts.iter().cloned().collect();
    let source_set: BTreeSet<String> = source_parts.iter().cloned().collect();
    let pullback_set: BTreeSet<String> = pullback_parts.iter().cloned().collect();
    let mapped_pullback_set: BTreeSet<String> = mapped_pullback_parts.iter().cloned().collect();

    let mut failure_classes = Vec::new();
    if has_duplicates(&base_parts)
        || has_duplicates(&pullback_parts)
        || has_duplicates(&source_parts)
        || has_duplicates(&mapped_pullback_parts)
        || base_set != source_set
        || pullback_set != mapped_pullback_set
    {
        failure_classes.push("coherence.coverage_base_change.violation".to_string());
    }

    Ok(SiteEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "digests": {
                "baseCoverParts": semantic_digest(&json!(base_parts)),
                "pullbackCoverParts": semantic_digest(&json!(pullback_parts)),
                "pullbackMapping": semantic_digest(&json!(pullback_of_parts)),
            },
            "sets": {
                "baseCoverParts": sorted_vec_from_set(&base_set),
                "mappedSources": sorted_vec_from_set(&source_set),
                "pullbackCoverParts": sorted_vec_from_set(&pullback_set),
                "mappedPullbacks": sorted_vec_from_set(&mapped_pullback_set),
            }
        }),
    })
}

fn evaluate_site_case_coverage_transitivity(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let coverage = require_object_field(artifacts, "coverage", case_path)?;
    let outer_cover = require_object_field(coverage, "outerCover", case_path)?;
    let composed_cover = require_object_field(coverage, "composedCover", case_path)?;

    let outer_parts = require_string_array_field(
        outer_cover,
        "parts",
        case_path,
        "artifacts.coverage.outerCover",
    )?;
    let composed_parts = require_string_array_field(
        composed_cover,
        "parts",
        case_path,
        "artifacts.coverage.composedCover",
    )?;

    let refinement_covers = coverage
        .get("refinementCovers")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.coverage.refinementCovers must be an array",
                display_path(case_path)
            ))
        })?;

    let mut coverage_by_outer: BTreeMap<String, usize> = BTreeMap::new();
    let mut refinement_union: BTreeSet<String> = BTreeSet::new();
    let outer_set: BTreeSet<String> = outer_parts.iter().cloned().collect();
    for row in refinement_covers {
        let row_obj = row.as_object().ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: refinementCovers entries must be objects",
                display_path(case_path)
            ))
        })?;
        let over = row_obj
            .get("over")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: refinementCovers[].over must be non-empty string",
                    display_path(case_path)
                ))
            })?
            .to_string();
        let parts = require_string_array_field(row_obj, "parts", case_path, "refinementCovers[]")?;
        *coverage_by_outer.entry(over).or_insert(0) += 1;
        for part in parts {
            refinement_union.insert(part);
        }
    }

    let composed_set: BTreeSet<String> = composed_parts.iter().cloned().collect();
    let covered_outer_set: BTreeSet<String> = coverage_by_outer.keys().cloned().collect();
    let mut failure_classes = Vec::new();
    if has_duplicates(&outer_parts)
        || has_duplicates(&composed_parts)
        || covered_outer_set.iter().any(|k| !outer_set.contains(k))
        || outer_set
            .iter()
            .any(|outer| coverage_by_outer.get(outer).copied().unwrap_or(0) != 1)
        || refinement_union != composed_set
    {
        failure_classes.push("coherence.coverage_transitivity.violation".to_string());
    }

    Ok(SiteEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "digests": {
                "outerCoverParts": semantic_digest(&json!(outer_parts)),
                "refinementCovers": semantic_digest(&json!(refinement_covers)),
                "composedCoverParts": semantic_digest(&json!(composed_parts)),
            },
            "sets": {
                "outerCoverParts": sorted_vec_from_set(&outer_set),
                "coveredOuterParts": sorted_vec_from_set(&covered_outer_set),
                "refinementUnion": sorted_vec_from_set(&refinement_union),
                "composedCoverParts": sorted_vec_from_set(&composed_set),
            },
            "coverageMultiplicity": coverage_by_outer,
        }),
    })
}

fn evaluate_site_case_glue_or_witness_contractibility(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let descent = require_object_field(artifacts, "descent", case_path)?;

    let locals = descent
        .get("locals")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.descent.locals must be an array",
                display_path(case_path)
            ))
        })?;
    let compatibility_witnesses = descent
        .get("compatibilityWitnesses")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.descent.compatibilityWitnesses must be an array",
                display_path(case_path)
            ))
        })?;

    let glue = descent.get("glue").cloned();
    let obstruction = descent.get("obstruction").cloned();
    let has_glue = glue.is_some() && glue != Some(Value::Null);
    let has_obstruction = obstruction.is_some() && obstruction != Some(Value::Null);

    let mut failure_classes = Vec::new();
    if has_glue == has_obstruction {
        failure_classes.push("coherence.glue_or_witness_contractibility.violation".to_string());
    }
    if has_glue && compatibility_witnesses.is_empty() {
        failure_classes.push("coherence.glue_or_witness_contractibility.violation".to_string());
    }
    if has_obstruction {
        let valid_obstruction_class = obstruction
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|row| row.get("class"))
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|v| !v.is_empty());
        if !valid_obstruction_class {
            failure_classes.push("coherence.glue_or_witness_contractibility.violation".to_string());
        }
    }
    if locals.is_empty() {
        failure_classes.push("coherence.glue_or_witness_contractibility.violation".to_string());
    }

    Ok(SiteEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "digests": {
                "locals": semantic_digest(&json!(locals)),
                "compatibilityWitnesses": semantic_digest(&json!(compatibility_witnesses)),
                "glue": semantic_digest(&glue.clone().unwrap_or(Value::Null)),
                "obstruction": semantic_digest(&obstruction.clone().unwrap_or(Value::Null)),
            },
            "shape": {
                "localsCount": locals.len(),
                "compatibilityWitnessCount": compatibility_witnesses.len(),
                "hasGlue": has_glue,
                "hasObstruction": has_obstruction,
            }
        }),
    })
}

fn evaluate_cwf_row_equalities(
    rows: &[Value],
    case_path: &Path,
    field_prefix: &str,
    left_key: &str,
    right_key: &str,
) -> Result<(Vec<String>, Vec<Value>), CoherenceError> {
    let mut mismatch_labels = Vec::new();
    let mut digest_rows = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let row_obj = row.as_object().ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: {field_prefix}[{index}] must be an object",
                display_path(case_path)
            ))
        })?;
        let label = row_obj
            .get("label")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{field_prefix}[{index}]"));
        let left_value = require_value_field(row_obj, left_key, case_path)?;
        let right_value = require_value_field(row_obj, right_key, case_path)?;
        let left_digest = semantic_digest(left_value);
        let right_digest = semantic_digest(right_value);
        if left_digest != right_digest {
            mismatch_labels.push(label.clone());
        }
        digest_rows.push(json!({
            "label": label,
            "leftDigest": left_digest,
            "rightDigest": right_digest,
        }));
    }
    Ok((mismatch_labels, digest_rows))
}

fn evaluate_site_case_cwf_substitution_identity(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let cwf = require_object_field(artifacts, "cwf", case_path)?;
    let substitution = require_object_field(cwf, "substitution", case_path)?;
    let type_rows = substitution
        .get("types")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.cwf.substitution.types must be an array",
                display_path(case_path)
            ))
        })?;
    let term_rows = substitution
        .get("terms")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.cwf.substitution.terms must be an array",
                display_path(case_path)
            ))
        })?;

    let (type_mismatch, type_digests) = evaluate_cwf_row_equalities(
        type_rows,
        case_path,
        "artifacts.cwf.substitution.types",
        "direct",
        "afterIdentity",
    )?;
    let (term_mismatch, term_digests) = evaluate_cwf_row_equalities(
        term_rows,
        case_path,
        "artifacts.cwf.substitution.terms",
        "direct",
        "afterIdentity",
    )?;

    let mut failure_classes = Vec::new();
    if type_rows.is_empty()
        || term_rows.is_empty()
        || !type_mismatch.is_empty()
        || !term_mismatch.is_empty()
    {
        failure_classes.push("coherence.cwf_substitution_identity.violation".to_string());
    }

    Ok(SiteEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "digests": {
                "types": type_digests,
                "terms": term_digests,
            },
            "shape": {
                "typeRowCount": type_rows.len(),
                "termRowCount": term_rows.len(),
                "typeMismatchLabels": type_mismatch,
                "termMismatchLabels": term_mismatch,
            }
        }),
    })
}

fn evaluate_site_case_cwf_substitution_composition(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let cwf = require_object_field(artifacts, "cwf", case_path)?;
    let substitution = require_object_field(cwf, "substitution", case_path)?;
    let type_rows = substitution
        .get("types")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.cwf.substitution.types must be an array",
                display_path(case_path)
            ))
        })?;
    let term_rows = substitution
        .get("terms")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.cwf.substitution.terms must be an array",
                display_path(case_path)
            ))
        })?;

    let (type_mismatch, type_digests) = evaluate_cwf_row_equalities(
        type_rows,
        case_path,
        "artifacts.cwf.substitution.types",
        "afterCompose",
        "afterStepwise",
    )?;
    let (term_mismatch, term_digests) = evaluate_cwf_row_equalities(
        term_rows,
        case_path,
        "artifacts.cwf.substitution.terms",
        "afterCompose",
        "afterStepwise",
    )?;

    let mut failure_classes = Vec::new();
    if type_rows.is_empty()
        || term_rows.is_empty()
        || !type_mismatch.is_empty()
        || !term_mismatch.is_empty()
    {
        failure_classes.push("coherence.cwf_substitution_composition.violation".to_string());
    }

    Ok(SiteEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "digests": {
                "types": type_digests,
                "terms": term_digests,
            },
            "shape": {
                "typeRowCount": type_rows.len(),
                "termRowCount": term_rows.len(),
                "typeMismatchLabels": type_mismatch,
                "termMismatchLabels": term_mismatch,
            }
        }),
    })
}

fn evaluate_site_case_cwf_comprehension_beta(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let cwf = require_object_field(artifacts, "cwf", case_path)?;
    let comprehension = require_object_field(cwf, "comprehension", case_path)?;
    let beta_rows = comprehension
        .get("beta")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.cwf.comprehension.beta must be an array",
                display_path(case_path)
            ))
        })?;

    let (mismatch_labels, digest_rows) = evaluate_cwf_row_equalities(
        beta_rows,
        case_path,
        "artifacts.cwf.comprehension.beta",
        "original",
        "afterBeta",
    )?;

    let mut failure_classes = Vec::new();
    if beta_rows.is_empty() || !mismatch_labels.is_empty() {
        failure_classes.push("coherence.cwf_comprehension_beta.violation".to_string());
    }

    Ok(SiteEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "digests": {
                "beta": digest_rows,
            },
            "shape": {
                "betaRowCount": beta_rows.len(),
                "mismatchLabels": mismatch_labels,
            }
        }),
    })
}

fn evaluate_site_case_cwf_comprehension_eta(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let cwf = require_object_field(artifacts, "cwf", case_path)?;
    let comprehension = require_object_field(cwf, "comprehension", case_path)?;
    let eta_rows = comprehension
        .get("eta")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.cwf.comprehension.eta must be an array",
                display_path(case_path)
            ))
        })?;

    let (mismatch_labels, digest_rows) = evaluate_cwf_row_equalities(
        eta_rows,
        case_path,
        "artifacts.cwf.comprehension.eta",
        "original",
        "afterEta",
    )?;

    let mut failure_classes = Vec::new();
    if eta_rows.is_empty() || !mismatch_labels.is_empty() {
        failure_classes.push("coherence.cwf_comprehension_eta.violation".to_string());
    }

    Ok(SiteEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "digests": {
                "eta": digest_rows,
            },
            "shape": {
                "etaRowCount": eta_rows.len(),
                "mismatchLabels": mismatch_labels,
            }
        }),
    })
}

fn evaluate_site_case_span_square_commutation(
    artifacts_payload: &Value,
    case_path: &Path,
) -> Result<SiteEvaluation, CoherenceError> {
    let artifacts = artifacts_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts must be an object",
            display_path(case_path)
        ))
    })?;
    let span_square = require_object_field(artifacts, "spanSquare", case_path)?;
    let spans = span_square
        .get("spans")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.spanSquare.spans must be an array",
                display_path(case_path)
            ))
        })?;
    let squares = span_square
        .get("squares")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.spanSquare.squares must be an array",
                display_path(case_path)
            ))
        })?;

    let mut failures = Vec::new();
    if spans.is_empty() || squares.is_empty() {
        failures.push("coherence.span_square_commutation.violation".to_string());
    }

    let mut span_digests: BTreeMap<String, String> = BTreeMap::new();
    let mut span_rows = Vec::new();
    for (index, span) in spans.iter().enumerate() {
        let span_obj = span.as_object().ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.spanSquare.spans[{index}] must be an object",
                display_path(case_path)
            ))
        })?;
        let span_id = require_non_empty_string_field(span_obj, "id", case_path)?;
        let span_kind = require_non_empty_string_field(span_obj, "kind", case_path)?;
        let left = require_value_field(span_obj, "left", case_path)?;
        let apex = require_value_field(span_obj, "apex", case_path)?;
        let right = require_value_field(span_obj, "right", case_path)?;
        let span_digest = semantic_digest(&json!({
            "kind": span_kind,
            "left": left,
            "apex": apex,
            "right": right,
        }));
        if span_digests
            .insert(span_id.clone(), span_digest.clone())
            .is_some()
        {
            failures.push("coherence.span_square_commutation.violation".to_string());
        }
        span_rows.push(json!({
            "id": span_id,
            "kind": span_kind,
            "digest": span_digest,
        }));
    }

    let mut square_ids = BTreeSet::new();
    let mut square_digests: BTreeMap<String, String> = BTreeMap::new();
    let mut square_rows = Vec::new();
    for (index, square) in squares.iter().enumerate() {
        let square_obj = square.as_object().ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.spanSquare.squares[{index}] must be an object",
                display_path(case_path)
            ))
        })?;
        let square_id = require_non_empty_string_field(square_obj, "id", case_path)?;
        if !square_ids.insert(square_id.clone()) {
            failures.push("coherence.span_square_commutation.violation".to_string());
        }
        let top = require_non_empty_string_field(square_obj, "top", case_path)?;
        let bottom = require_non_empty_string_field(square_obj, "bottom", case_path)?;
        let left = require_non_empty_string_field(square_obj, "left", case_path)?;
        let right = require_non_empty_string_field(square_obj, "right", case_path)?;
        let result = require_non_empty_string_field(square_obj, "result", case_path)?;
        let square_failure_classes = dedupe_sorted(require_string_array_field(
            square_obj,
            "failureClasses",
            case_path,
            "artifacts.spanSquare.squares[]",
        )?);
        let digest = require_non_empty_string_field(square_obj, "digest", case_path)?;
        let expected_digest = square_witness_digest(
            top.as_str(),
            bottom.as_str(),
            left.as_str(),
            right.as_str(),
            result.as_str(),
            &square_failure_classes,
        );
        square_digests.insert(square_id.clone(), expected_digest.clone());
        if digest != expected_digest {
            failures.push("coherence.span_square_commutation.violation".to_string());
        }

        let top_digest = span_digests.get(&top).cloned();
        let bottom_digest = span_digests.get(&bottom).cloned();
        let left_digest = span_digests.get(&left).cloned();
        let right_digest = span_digests.get(&right).cloned();
        if top_digest.is_none()
            || bottom_digest.is_none()
            || left_digest.is_none()
            || right_digest.is_none()
        {
            failures.push("coherence.span_square_commutation.violation".to_string());
        }
        if result != "accepted" && result != "rejected" {
            failures.push("coherence.span_square_commutation.violation".to_string());
        } else if result == "accepted" {
            if !square_failure_classes.is_empty()
                || top_digest.as_deref().unwrap_or_default()
                    != bottom_digest.as_deref().unwrap_or_default()
            {
                failures.push("coherence.span_square_commutation.violation".to_string());
            }
        } else if square_failure_classes.is_empty() {
            failures.push("coherence.span_square_commutation.violation".to_string());
        }

        square_rows.push(json!({
            "id": square_id,
            "result": result,
            "top": {"id": top, "digest": top_digest},
            "bottom": {"id": bottom, "digest": bottom_digest},
            "left": {"id": left, "digest": left_digest},
            "right": {"id": right, "digest": right_digest},
            "failureClasses": square_failure_classes,
            "providedDigest": digest,
            "expectedDigest": expected_digest,
        }));
    }

    let mut composition_rows = Vec::new();
    let mut composition_summary = json!({
        "present": false
    });
    if let Some(composition_value) = span_square.get("compositionLaws") {
        let composition = composition_value.as_object().ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.spanSquare.compositionLaws must be an object",
                display_path(case_path)
            ))
        })?;
        let identity_span_ids = dedupe_sorted(optional_string_array_field(
            composition,
            "identitySpanIds",
            case_path,
            "artifacts.spanSquare.compositionLaws",
        )?);
        let identity_square_ids = dedupe_sorted(optional_string_array_field(
            composition,
            "identitySquareIds",
            case_path,
            "artifacts.spanSquare.compositionLaws",
        )?);
        for span_id in &identity_span_ids {
            if !span_digests.contains_key(span_id) {
                failures.push("coherence.span_square_commutation.violation".to_string());
            }
        }
        for square_id in &identity_square_ids {
            if !square_digests.contains_key(square_id) {
                failures.push("coherence.span_square_commutation.violation".to_string());
            }
        }

        let law_rows = composition
            .get("laws")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: artifacts.spanSquare.compositionLaws.laws must be an array",
                    display_path(case_path)
                ))
            })?;
        if law_rows.is_empty() {
            failures.push("coherence.span_square_commutation.violation".to_string());
        }
        let mut law_ids = BTreeSet::new();
        let mut accepted_laws = BTreeSet::new();
        let mut used_square_modes = SquareCompositionModes::default();
        let identity_span_set: BTreeSet<String> = identity_span_ids.iter().cloned().collect();
        let identity_square_set: BTreeSet<String> = identity_square_ids.iter().cloned().collect();
        for (index, law_row) in law_rows.iter().enumerate() {
            let law_obj = law_row.as_object().ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: artifacts.spanSquare.compositionLaws.laws[{index}] must be an object",
                    display_path(case_path)
                ))
            })?;
            let law_id = require_non_empty_string_field(law_obj, "id", case_path)?;
            if !law_ids.insert(law_id.clone()) {
                failures.push("coherence.span_square_commutation.violation".to_string());
            }
            let kind = require_non_empty_string_field(law_obj, "kind", case_path)?;
            let law = require_non_empty_string_field(law_obj, "law", case_path)?;
            let left_expr = require_value_field(law_obj, "left", case_path)?;
            let right_expr = require_value_field(law_obj, "right", case_path)?;
            let result = require_non_empty_string_field(law_obj, "result", case_path)?;
            let law_failure_classes = dedupe_sorted(require_string_array_field(
                law_obj,
                "failureClasses",
                case_path,
                "artifacts.spanSquare.compositionLaws.laws[]",
            )?);
            let digest = require_non_empty_string_field(law_obj, "digest", case_path)?;
            let expected_digest = composition_law_digest(
                kind.as_str(),
                law.as_str(),
                left_expr,
                right_expr,
                result.as_str(),
                &law_failure_classes,
            );
            if digest != expected_digest {
                failures.push("coherence.span_square_commutation.violation".to_string());
            }

            let mut law_eval_error: Option<String> = None;
            let mut normalized_left = Value::Null;
            let mut normalized_right = Value::Null;
            let mut normalized_equal = false;
            if kind == "span" {
                if law != "span_identity" && law != "span_associativity" {
                    failures.push("coherence.span_square_commutation.violation".to_string());
                    law_eval_error = Some(format!("unsupported span law {law:?}"));
                } else {
                    match evaluate_span_expression(left_expr, &span_digests, &identity_span_set) {
                        Ok(tokens) => {
                            normalized_left = json!(tokens);
                        }
                        Err(message) => {
                            failures
                                .push("coherence.span_square_commutation.violation".to_string());
                            law_eval_error = Some(message);
                        }
                    }
                    if law_eval_error.is_none() {
                        match evaluate_span_expression(
                            right_expr,
                            &span_digests,
                            &identity_span_set,
                        ) {
                            Ok(tokens) => {
                                normalized_right = json!(tokens);
                            }
                            Err(message) => {
                                failures.push(
                                    "coherence.span_square_commutation.violation".to_string(),
                                );
                                law_eval_error = Some(message);
                            }
                        }
                    }
                }
            } else if kind == "square" {
                if !matches!(
                    law.as_str(),
                    "square_identity"
                        | "square_associativity_horizontal"
                        | "square_associativity_vertical"
                        | "square_hv_compatibility"
                        | "square_interchange"
                ) {
                    failures.push("coherence.span_square_commutation.violation".to_string());
                    law_eval_error = Some(format!("unsupported square law {law:?}"));
                } else {
                    match evaluate_square_expression(
                        left_expr,
                        &square_digests,
                        &identity_square_set,
                        &mut used_square_modes,
                    ) {
                        Ok(value) => {
                            normalized_left = square_expression_to_json(&value);
                        }
                        Err(message) => {
                            failures
                                .push("coherence.span_square_commutation.violation".to_string());
                            law_eval_error = Some(message);
                        }
                    }
                    if law_eval_error.is_none() {
                        match evaluate_square_expression(
                            right_expr,
                            &square_digests,
                            &identity_square_set,
                            &mut used_square_modes,
                        ) {
                            Ok(value) => {
                                normalized_right = square_expression_to_json(&value);
                            }
                            Err(message) => {
                                failures.push(
                                    "coherence.span_square_commutation.violation".to_string(),
                                );
                                law_eval_error = Some(message);
                            }
                        }
                    }
                }
            } else {
                failures.push("coherence.span_square_commutation.violation".to_string());
                law_eval_error = Some(format!("unsupported law kind {kind:?}"));
            }

            if law_eval_error.is_none() {
                normalized_equal = normalized_left == normalized_right;
            }

            if result != "accepted" && result != "rejected" {
                failures.push("coherence.span_square_commutation.violation".to_string());
            } else if result == "accepted" {
                if !law_failure_classes.is_empty() || !normalized_equal || law_eval_error.is_some()
                {
                    failures.push("coherence.span_square_commutation.violation".to_string());
                } else {
                    accepted_laws.insert(law.clone());
                }
            } else {
                if law_failure_classes.is_empty() {
                    failures.push("coherence.span_square_commutation.violation".to_string());
                }
                if law_eval_error.is_none() && normalized_equal {
                    failures.push("coherence.span_square_commutation.violation".to_string());
                }
            }

            composition_rows.push(json!({
                "id": law_id,
                "kind": kind,
                "law": law,
                "result": result,
                "normalizedLeft": normalized_left,
                "normalizedRight": normalized_right,
                "normalizedEqual": normalized_equal,
                "failureClasses": law_failure_classes,
                "providedDigest": digest,
                "expectedDigest": expected_digest,
                "evaluationError": law_eval_error,
            }));
        }

        for required_law in [
            "span_identity",
            "span_associativity",
            "square_identity",
            "square_associativity_horizontal",
            "square_associativity_vertical",
            "square_hv_compatibility",
            "square_interchange",
        ] {
            if !accepted_laws.contains(required_law) {
                failures.push("coherence.span_square_commutation.violation".to_string());
            }
        }
        if !used_square_modes.horizontal || !used_square_modes.vertical {
            failures.push("coherence.span_square_commutation.violation".to_string());
        }
        composition_summary = json!({
            "present": true,
            "lawCount": law_rows.len(),
            "acceptedLaws": accepted_laws.into_iter().collect::<Vec<String>>(),
            "identitySpanIds": identity_span_ids,
            "identitySquareIds": identity_square_ids,
            "usedSquareModes": {
                "horizontal": used_square_modes.horizontal,
                "vertical": used_square_modes.vertical,
            }
        });
    }

    Ok(SiteEvaluation {
        result: if failures.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "shape": {
                "spanCount": spans.len(),
                "squareCount": squares.len(),
            },
            "spans": span_rows,
            "squares": square_rows,
            "compositionLaws": composition_rows,
            "compositionSummary": composition_summary,
        }),
    })
}

#[derive(Debug)]
struct TransportEvaluation {
    result: String,
    failure_classes: Vec<String>,
    details: Value,
}

fn evaluate_transport_case(
    case_payload: &Value,
    case_path: &Path,
) -> Result<TransportEvaluation, CoherenceError> {
    let root = case_payload.as_object().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: root must be an object",
            display_path(case_path)
        ))
    })?;
    let artifacts = require_object_field(root, "artifacts", case_path)?;
    let binding = require_object_field(artifacts, "binding", case_path)?;
    let base = require_object_field(artifacts, "base", case_path)?;
    let fibre = require_object_field(artifacts, "fibre", case_path)?;
    let naturality = require_object_field(artifacts, "naturality", case_path)?;

    let normalizer_id = require_non_empty_string_field(binding, "normalizerId", case_path)?;
    let policy_digest = require_non_empty_string_field(binding, "policyDigest", case_path)?;

    let base_identity = require_value_field(base, "identity", case_path)?;
    let base_f = require_value_field(base, "f", case_path)?;
    let base_g = require_value_field(base, "g", case_path)?;
    let base_g_after_f = require_value_field(base, "gAfterF", case_path)?;

    let fibre_identity = require_value_field(fibre, "identity", case_path)?;
    let fibre_f_identity = require_value_field(fibre, "FIdentity", case_path)?;
    let fibre_f_f = require_value_field(fibre, "FF", case_path)?;
    let fibre_f_g = require_value_field(fibre, "FG", case_path)?;
    let fibre_f_g_after_f = require_value_field(fibre, "FGAfterF", case_path)?;
    let fibre_f_g_after_f_f = require_value_field(fibre, "FGAfterFF", case_path)?;

    let naturality_left = require_value_field(naturality, "left", case_path)?;
    let naturality_right = require_value_field(naturality, "right", case_path)?;

    let base_identity_digest = semantic_digest(base_identity);
    let base_f_digest = semantic_digest(base_f);
    let base_g_digest = semantic_digest(base_g);
    let base_g_after_f_digest = semantic_digest(base_g_after_f);

    let fibre_identity_digest = semantic_digest(fibre_identity);
    let fibre_f_identity_digest = semantic_digest(fibre_f_identity);
    let fibre_f_f_digest = semantic_digest(fibre_f_f);
    let fibre_f_g_digest = semantic_digest(fibre_f_g);
    let fibre_f_g_after_f_digest = semantic_digest(fibre_f_g_after_f);
    let fibre_f_g_after_f_f_digest = semantic_digest(fibre_f_g_after_f_f);

    let naturality_left_digest = semantic_digest(naturality_left);
    let naturality_right_digest = semantic_digest(naturality_right);

    let mut failure_classes = Vec::new();
    if fibre_identity_digest != fibre_f_identity_digest {
        failure_classes.push("coherence.transport_functoriality.identity_violation".to_string());
    }
    if fibre_f_g_after_f_digest != fibre_f_g_after_f_f_digest {
        failure_classes.push("coherence.transport_functoriality.composition_violation".to_string());
    }
    if naturality_left_digest != naturality_right_digest {
        failure_classes.push("coherence.transport_functoriality.naturality_violation".to_string());
    }

    Ok(TransportEvaluation {
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: dedupe_sorted(failure_classes),
        details: json!({
            "binding": {
                "normalizerId": normalizer_id,
                "policyDigest": policy_digest,
            },
            "digests": {
                "base": {
                    "identity": base_identity_digest,
                    "f": base_f_digest,
                    "g": base_g_digest,
                    "gAfterF": base_g_after_f_digest,
                },
                "fibre": {
                    "identity": fibre_identity_digest,
                    "FIdentity": fibre_f_identity_digest,
                    "FF": fibre_f_f_digest,
                    "FG": fibre_f_g_digest,
                    "FGAfterF": fibre_f_g_after_f_digest,
                    "FGAfterFF": fibre_f_g_after_f_f_digest,
                },
                "naturality": {
                    "left": naturality_left_digest,
                    "right": naturality_right_digest,
                }
            }
        }),
    })
}

fn normalize_semantics(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut sorted = Map::new();
            for key in keys {
                if let Some(item) = map.get(key) {
                    sorted.insert(key.clone(), normalize_semantics(item));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(items) => {
            let mut by_key: BTreeMap<String, Value> = BTreeMap::new();
            for item in items {
                let normalized = normalize_semantics(item);
                let key = serde_json::to_string(&normalized).expect("normalize semantics");
                by_key.insert(key, normalized);
            }
            Value::Array(by_key.into_values().collect())
        }
        _ => value.clone(),
    }
}

fn semantic_digest(value: &Value) -> String {
    let normalized = normalize_semantics(value);
    let canonical = serde_json::to_string(&normalized).expect("semantic digest serialization");
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("sem1_{:x}", hasher.finalize())
}

fn square_witness_digest(
    top: &str,
    bottom: &str,
    left: &str,
    right: &str,
    result: &str,
    failure_classes: &[String],
) -> String {
    let core = json!({
        "top": top,
        "bottom": bottom,
        "left": left,
        "right": right,
        "result": result,
        "failureClasses": failure_classes,
    });
    let canonical = serde_json::to_string(&core).expect("square witness digest serialization");
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("sqw1_{:x}", hasher.finalize())
}

fn composition_law_digest(
    kind: &str,
    law: &str,
    left: &Value,
    right: &Value,
    result: &str,
    failure_classes: &[String],
) -> String {
    let core = normalize_semantics(&json!({
        "kind": kind,
        "law": law,
        "left": left,
        "right": right,
        "result": result,
        "failureClasses": failure_classes,
    }));
    let canonical = serde_json::to_string(&core).expect("composition law digest serialization");
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("sqlw1_{:x}", hasher.finalize())
}

#[derive(Debug, Default)]
struct SquareCompositionModes {
    horizontal: bool,
    vertical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SquareExpressionValue {
    Identity,
    Grid(Vec<Vec<String>>),
}

fn evaluate_span_expression(
    expression: &Value,
    span_digests: &BTreeMap<String, String>,
    identity_span_ids: &BTreeSet<String>,
) -> Result<Vec<String>, String> {
    let expression_obj = expression
        .as_object()
        .ok_or_else(|| "span expression must be an object".to_string())?;
    if let Some(span_ref) = expression_obj.get("span") {
        let span_id = span_ref
            .as_str()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| "span expression field \"span\" must be non-empty".to_string())?;
        let digest = span_digests
            .get(span_id)
            .ok_or_else(|| format!("span expression references unknown span {span_id:?}"))?;
        if identity_span_ids.contains(span_id) {
            Ok(Vec::new())
        } else {
            Ok(vec![digest.clone()])
        }
    } else if let Some(compose_ref) = expression_obj.get("compose") {
        let compose_obj = compose_ref
            .as_object()
            .ok_or_else(|| "span expression compose must be an object".to_string())?;
        let left = compose_obj
            .get("left")
            .ok_or_else(|| "span expression compose missing left".to_string())?;
        let right = compose_obj
            .get("right")
            .ok_or_else(|| "span expression compose missing right".to_string())?;
        let mut left_tokens = evaluate_span_expression(left, span_digests, identity_span_ids)?;
        let right_tokens = evaluate_span_expression(right, span_digests, identity_span_ids)?;
        left_tokens.extend(right_tokens);
        Ok(left_tokens)
    } else {
        Err("span expression must include either \"span\" or \"compose\"".to_string())
    }
}

fn evaluate_square_expression(
    expression: &Value,
    square_digests: &BTreeMap<String, String>,
    identity_square_ids: &BTreeSet<String>,
    used_modes: &mut SquareCompositionModes,
) -> Result<SquareExpressionValue, String> {
    let expression_obj = expression
        .as_object()
        .ok_or_else(|| "square expression must be an object".to_string())?;
    if let Some(square_ref) = expression_obj.get("square") {
        let square_id = square_ref
            .as_str()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| "square expression field \"square\" must be non-empty".to_string())?;
        if identity_square_ids.contains(square_id) {
            return Ok(SquareExpressionValue::Identity);
        }
        let digest = square_digests
            .get(square_id)
            .ok_or_else(|| format!("square expression references unknown square {square_id:?}"))?;
        return Ok(SquareExpressionValue::Grid(vec![vec![digest.clone()]]));
    }
    let compose_ref = expression_obj.get("compose").ok_or_else(|| {
        "square expression must include either \"square\" or \"compose\"".to_string()
    })?;
    let compose_obj = compose_ref
        .as_object()
        .ok_or_else(|| "square expression compose must be an object".to_string())?;
    let mode = compose_obj
        .get("mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| "square expression compose.mode must be non-empty".to_string())?;
    let left_expr = compose_obj
        .get("left")
        .ok_or_else(|| "square expression compose missing left".to_string())?;
    let right_expr = compose_obj
        .get("right")
        .ok_or_else(|| "square expression compose missing right".to_string())?;
    let left =
        evaluate_square_expression(left_expr, square_digests, identity_square_ids, used_modes)?;
    let right =
        evaluate_square_expression(right_expr, square_digests, identity_square_ids, used_modes)?;
    if mode == "horizontal" {
        used_modes.horizontal = true;
        compose_square_horizontal(left, right)
    } else if mode == "vertical" {
        used_modes.vertical = true;
        compose_square_vertical(left, right)
    } else {
        Err(format!(
            "square expression compose.mode must be \"horizontal\" or \"vertical\", got {mode:?}"
        ))
    }
}

fn compose_square_horizontal(
    left: SquareExpressionValue,
    right: SquareExpressionValue,
) -> Result<SquareExpressionValue, String> {
    match (left, right) {
        (SquareExpressionValue::Identity, value) | (value, SquareExpressionValue::Identity) => {
            Ok(value)
        }
        (SquareExpressionValue::Grid(mut left_grid), SquareExpressionValue::Grid(right_grid)) => {
            let left_dims = square_grid_dimensions(&left_grid)
                .ok_or_else(|| "square expression left grid is not rectangular".to_string())?;
            let right_dims = square_grid_dimensions(&right_grid)
                .ok_or_else(|| "square expression right grid is not rectangular".to_string())?;
            if left_dims.0 != right_dims.0 {
                return Err(format!(
                    "square horizontal composition row mismatch: left rows={}, right rows={}",
                    left_dims.0, right_dims.0
                ));
            }
            for (left_row, right_row) in left_grid.iter_mut().zip(right_grid.iter()) {
                left_row.extend(right_row.iter().cloned());
            }
            Ok(SquareExpressionValue::Grid(left_grid))
        }
    }
}

fn compose_square_vertical(
    left: SquareExpressionValue,
    right: SquareExpressionValue,
) -> Result<SquareExpressionValue, String> {
    match (left, right) {
        (SquareExpressionValue::Identity, value) | (value, SquareExpressionValue::Identity) => {
            Ok(value)
        }
        (SquareExpressionValue::Grid(mut left_grid), SquareExpressionValue::Grid(right_grid)) => {
            let left_dims = square_grid_dimensions(&left_grid)
                .ok_or_else(|| "square expression left grid is not rectangular".to_string())?;
            let right_dims = square_grid_dimensions(&right_grid)
                .ok_or_else(|| "square expression right grid is not rectangular".to_string())?;
            if left_dims.1 != right_dims.1 {
                return Err(format!(
                    "square vertical composition column mismatch: left cols={}, right cols={}",
                    left_dims.1, right_dims.1
                ));
            }
            left_grid.extend(right_grid);
            Ok(SquareExpressionValue::Grid(left_grid))
        }
    }
}

fn square_grid_dimensions(grid: &[Vec<String>]) -> Option<(usize, usize)> {
    let row_count = grid.len();
    let col_count = grid.first().map(|row| row.len()).unwrap_or(0);
    for row in grid {
        if row.len() != col_count {
            return None;
        }
    }
    Some((row_count, col_count))
}

fn square_expression_to_json(value: &SquareExpressionValue) -> Value {
    match value {
        SquareExpressionValue::Identity => json!({"identity": true, "grid": []}),
        SquareExpressionValue::Grid(grid) => json!({"identity": false, "grid": grid}),
    }
}

fn require_object_field<'a>(
    parent: &'a Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<&'a Map<String, Value>, CoherenceError> {
    parent.get(key).and_then(Value::as_object).ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: artifacts.{key} must be an object",
            display_path(path)
        ))
    })
}

fn require_value_field<'a>(
    parent: &'a Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<&'a Value, CoherenceError> {
    parent.get(key).ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: missing artifacts field {key:?}",
            display_path(path)
        ))
    })
}

fn require_non_empty_string_field(
    parent: &Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<String, CoherenceError> {
    let value = parent
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "{}: artifacts.binding.{key} must be a non-empty string",
                display_path(path)
            ))
        })?;
    Ok(value.to_string())
}

fn require_string_array_field(
    parent: &Map<String, Value>,
    key: &str,
    path: &Path,
    field_prefix: &str,
) -> Result<Vec<String>, CoherenceError> {
    let values = parent.get(key).and_then(Value::as_array).ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: {field_prefix}.{key} must be an array of non-empty strings",
            display_path(path)
        ))
    })?;
    let mut out = Vec::new();
    for item in values {
        let value = item
            .as_str()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: {field_prefix}.{key} must contain non-empty strings",
                    display_path(path)
                ))
            })?;
        out.push(value.to_string());
    }
    Ok(out)
}

fn optional_string_array_field(
    parent: &Map<String, Value>,
    key: &str,
    path: &Path,
    field_prefix: &str,
) -> Result<Vec<String>, CoherenceError> {
    let Some(raw_values) = parent.get(key) else {
        return Ok(Vec::new());
    };
    let values = raw_values.as_array().ok_or_else(|| {
        CoherenceError::Contract(format!(
            "{}: {field_prefix}.{key} must be an array of non-empty strings",
            display_path(path)
        ))
    })?;
    let mut out = Vec::new();
    for item in values {
        let value = item
            .as_str()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: {field_prefix}.{key} must contain non-empty strings",
                    display_path(path)
                ))
            })?;
        out.push(value.to_string());
    }
    Ok(out)
}

fn validate_contract_obligation_set(contract_ids: &[String]) -> Vec<String> {
    let mut failures = Vec::new();
    let allowed: BTreeSet<String> = REQUIRED_OBLIGATION_IDS
        .iter()
        .map(|v| (*v).to_string())
        .collect();
    let mut seen = BTreeSet::new();
    for obligation_id in contract_ids {
        if !allowed.contains(obligation_id) {
            failures.push("coherence.contract.unknown_obligation".to_string());
            continue;
        }
        if !seen.insert(obligation_id.clone()) {
            failures.push("coherence.contract.duplicate_obligation".to_string());
        }
    }
    for required in REQUIRED_OBLIGATION_IDS {
        if !seen.contains(*required) {
            failures.push("coherence.contract.missing_required_obligation".to_string());
        }
    }
    dedupe_sorted(failures)
}

fn validate_required_obligation_parity(
    declared: &BTreeSet<String>,
    required: &BTreeSet<String>,
) -> Vec<String> {
    let mut failures = Vec::new();
    for obligation_id in required {
        if !declared.contains(obligation_id) {
            failures.push(
                "coherence.scope_noncontradiction.coherence_spec_missing_obligation".to_string(),
            );
        }
    }
    for obligation_id in declared {
        if !required.contains(obligation_id) {
            failures.push(
                "coherence.scope_noncontradiction.coherence_spec_unknown_obligation".to_string(),
            );
        }
    }
    dedupe_sorted(failures)
}

fn compute_doctrine_reachability(site: &DoctrineSite, root: &str) -> BTreeSet<String> {
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in &site.edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }
    for cover in &site.covers {
        for part in &cover.parts {
            adjacency
                .entry(cover.over.clone())
                .or_default()
                .push(part.clone());
        }
    }

    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root.to_string());
    visited.insert(root.to_string());

    while let Some(current) = queue.pop_front() {
        if let Some(nexts) = adjacency.get(&current) {
            for next in nexts {
                if visited.insert(next.clone()) {
                    queue.push_back(next.clone());
                }
            }
        }
    }
    visited
}

fn parse_manifest_capabilities(root: &Path) -> Result<BTreeSet<String>, CoherenceError> {
    let mut out = BTreeSet::new();
    let entries = fs::read_dir(root).map_err(|source| CoherenceError::ReadFile {
        path: display_path(root),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| CoherenceError::ReadFile {
            path: display_path(root),
            source,
        })?;
        let file_type = entry
            .file_type()
            .map_err(|source| CoherenceError::ReadFile {
                path: display_path(&entry.path()),
                source,
            })?;
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("capabilities.") {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        let payload: Value =
            serde_json::from_slice(&read_bytes(&manifest_path)?).map_err(|source| {
                CoherenceError::ParseJson {
                    path: display_path(&manifest_path),
                    source,
                }
            })?;
        let capability = payload
            .get("capabilityId")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "{}: capabilityId must be non-empty string",
                    display_path(&manifest_path)
                ))
            })?;
        if capability.is_empty() {
            return Err(CoherenceError::Contract(format!(
                "{}: capabilityId must be non-empty string",
                display_path(&manifest_path)
            )));
        }
        out.insert(capability.to_string());
    }
    if out.is_empty() {
        return Err(CoherenceError::Contract(format!(
            "no capability manifests found under {}",
            display_path(root)
        )));
    }
    Ok(out)
}

fn parse_backticked_capabilities(text: &str) -> Result<BTreeSet<String>, CoherenceError> {
    let re = compile_regex(r"`(capabilities\.[a-z0-9_]+)`")?;
    Ok(re
        .captures_iter(text)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

fn parse_backticked_profile_overlay_claims(text: &str) -> Result<BTreeSet<String>, CoherenceError> {
    let re = compile_regex(r"`(profile\.[a-z0-9_.]+)`")?;
    Ok(re
        .captures_iter(text)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

fn parse_backticked_tasks(text: &str) -> Result<BTreeSet<String>, CoherenceError> {
    let re = compile_regex(r"`([a-z][a-z0-9-]*)`")?;
    Ok(re
        .captures_iter(text)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

fn parse_backtick_obligation_tokens(text: &str) -> Result<BTreeSet<String>, CoherenceError> {
    let re = compile_regex(r"`([a-z_]+)`")?;
    Ok(re
        .captures_iter(text)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

fn parse_baseline_task_ids_from_toml(
    toml_text: &str,
    task_name: &str,
    path: &Path,
) -> Result<Vec<String>, CoherenceError> {
    let parsed: toml::Value = toml_text
        .parse()
        .map_err(|source| CoherenceError::ParseToml {
            path: display_path(path),
            source,
        })?;
    let tasks = parsed
        .get("tasks")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| CoherenceError::Contract("missing [tasks] table".to_string()))?;
    let task = tasks
        .get(task_name)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| CoherenceError::Contract(format!("missing [tasks.{task_name}] table")))?;
    let run = task
        .get("run")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            CoherenceError::Contract(format!(
                "[tasks.{task_name}] must have run = [\"...\"] list"
            ))
        })?;
    let command_re = compile_regex(r"^mise run ([a-z][a-z0-9-]*)$")?;
    let mut out = Vec::new();
    for item in run {
        let command = item.as_str().ok_or_else(|| {
            CoherenceError::Contract(format!("[tasks.{task_name}] run entries must be strings"))
        })?;
        let captured = command_re
            .captures(command)
            .and_then(|caps| caps.get(1))
            .ok_or_else(|| {
                CoherenceError::Contract(format!(
                    "[tasks.{task_name}] unsupported command shape: {command:?}"
                ))
            })?;
        out.push(captured.as_str().to_string());
    }
    Ok(out)
}

fn parse_spec_index_capability_doc_map(
    section_54: &str,
) -> Result<BTreeMap<String, String>, CoherenceError> {
    let pattern = compile_regex(r"- `([^`]+)`\s+\(for `([^`]+)`\)")?;
    let mut out = BTreeMap::new();
    for captures in pattern.captures_iter(section_54) {
        let doc_ref = captures.get(1).map(|m| m.as_str()).ok_or_else(|| {
            CoherenceError::Contract(
                "malformed capture for spec-index doc map (doc_ref)".to_string(),
            )
        })?;
        let capability = captures.get(2).map(|m| m.as_str()).ok_or_else(|| {
            CoherenceError::Contract(
                "malformed capture for spec-index doc map (capability)".to_string(),
            )
        })?;
        out.insert(doc_ref.to_string(), capability.to_string());
    }
    Ok(out)
}

fn contains_conditional_normative_clause(
    section_55: &str,
    doc_ref: &str,
    capability_id: &str,
) -> Result<bool, CoherenceError> {
    let pattern = compile_regex(&format!(
        r#"(?is)`{}`.*?normative\s+only\s+when\s+`{}`\s+is\s+claimed"#,
        regex::escape(doc_ref),
        regex::escape(capability_id)
    ))?;
    Ok(pattern.is_match(section_55))
}

fn extract_section_between<'a>(
    text: &'a str,
    start_marker: &str,
    end_marker: &str,
) -> Result<&'a str, CoherenceError> {
    let start = text.find(start_marker).ok_or_else(|| {
        CoherenceError::Contract(format!("missing start marker: {start_marker:?}"))
    })? + start_marker.len();
    let end = text[start..].find(end_marker).ok_or_else(|| {
        CoherenceError::Contract(format!(
            "missing end marker {end_marker:?} after {start_marker:?}"
        ))
    })? + start;
    Ok(&text[start..end])
}

fn extract_heading_section(text: &str, heading_prefix: &str) -> Result<String, CoherenceError> {
    let heading_re = compile_regex(&format!(r"(?m)^### {}\b.*$", regex::escape(heading_prefix)))?;
    let heading_match = heading_re
        .find(text)
        .ok_or_else(|| CoherenceError::Contract(format!("missing heading: {heading_prefix:?}")))?;
    let start = heading_match.end();
    let tail = &text[start..];
    let next_heading_re = compile_regex(r"(?m)^### ")?;
    if let Some(next) = next_heading_re.find(tail) {
        Ok(tail[..next.start()].to_string())
    } else {
        Ok(tail.to_string())
    }
}

fn read_text(path: &Path) -> Result<String, CoherenceError> {
    fs::read_to_string(path).map_err(|source| CoherenceError::ReadFile {
        path: display_path(path),
        source,
    })
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, CoherenceError> {
    fs::read(path).map_err(|source| CoherenceError::ReadFile {
        path: display_path(path),
        source,
    })
}

fn read_json_value(path: &Path) -> Result<Value, CoherenceError> {
    serde_json::from_slice(&read_bytes(path)?).map_err(|source| CoherenceError::ParseJson {
        path: display_path(path),
        source,
    })
}

fn compile_regex(pattern: &str) -> Result<Regex, CoherenceError> {
    Regex::new(pattern).map_err(|source| {
        CoherenceError::Contract(format!("invalid regex pattern {pattern:?}: {source}"))
    })
}

fn resolve_path(root: &Path, path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn to_repo_relative_or_absolute(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().to_string(),
        Err(_) => display_path(path),
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn dedupe_sorted(values: Vec<String>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for value in values {
        set.insert(value);
    }
    set.into_iter().collect()
}

fn non_empty_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

fn record_invariance_row(
    failures: &mut Vec<String>,
    failure_prefix: &str,
    invariance_groups: &mut InvarianceGroups,
    observation: InvarianceObservation<'_>,
) {
    let semantic_scenario_id = non_empty_trimmed(observation.semantic_scenario_id);
    let profile = non_empty_trimmed(observation.profile);

    if semantic_scenario_id.is_none() {
        failures.push(format!(
            "{failure_prefix}.invariance_missing_semantic_scenario"
        ));
    }
    if profile.is_none() {
        failures.push(format!("{failure_prefix}.invariance_missing_profile"));
    }

    if let (Some(scenario_id), Some(profile)) = (semantic_scenario_id, profile) {
        invariance_groups.entry(scenario_id).or_default().push((
            observation.vector_id.to_string(),
            profile,
            observation.result.to_string(),
            dedupe_sorted(observation.failure_classes.to_vec()),
        ));
    }
}

fn validate_invariance_groups(
    failures: &mut Vec<String>,
    failure_prefix: &str,
    invariance_groups: &InvarianceGroups,
) -> Vec<Value> {
    let mut invariance_rows: Vec<Value> = Vec::new();
    for (scenario_id, rows) in invariance_groups {
        if rows.len() != 2 {
            failures.push(format!("{failure_prefix}.invariance_pair_count_mismatch"));
        } else {
            let profile_set: BTreeSet<String> = rows.iter().map(|row| row.1.clone()).collect();
            if profile_set.len() < 2 {
                failures.push(format!("{failure_prefix}.invariance_profile_not_distinct"));
            }
            let result_set: BTreeSet<String> = rows.iter().map(|row| row.2.clone()).collect();
            if result_set.len() != 1 {
                failures.push(format!("{failure_prefix}.invariance_result_mismatch"));
            }
            let failure_class_set: BTreeSet<Vec<String>> =
                rows.iter().map(|row| row.3.clone()).collect();
            if failure_class_set.len() != 1 {
                failures.push(format!(
                    "{failure_prefix}.invariance_failure_class_mismatch"
                ));
            }
        }
        invariance_rows.push(json!({
            "semanticScenarioId": scenario_id,
            "rowCount": rows.len(),
            "rows": rows
                .iter()
                .map(|(vector_id, profile, result, failure_classes)| json!({
                    "vectorId": vector_id,
                    "profile": profile,
                    "result": result,
                    "failureClasses": failure_classes,
                }))
                .collect::<Vec<Value>>(),
        }));
    }
    invariance_rows
}

fn sorted_vec_from_set(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
}

fn has_duplicates(values: &[String]) -> bool {
    let set: BTreeSet<String> = values.iter().cloned().collect();
    set.len() != values.len()
}

fn hex_sha256_from_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let mut path = std::env::temp_dir();
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be monotonic after unix epoch")
                .as_nanos();
            path.push(format!(
                "premath-coherence-{prefix}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temp test directory should be creatable");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_json_file(path: &Path, payload: &Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be creatable");
        }
        let bytes = serde_json::to_vec_pretty(payload).expect("json should serialize");
        fs::write(path, bytes).expect("json fixture should be writable");
    }

    fn write_text_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be creatable");
        }
        fs::write(path, content).expect("text fixture should be writable");
    }

    fn write_gate_chain_mise(path: &Path) {
        write_text_file(
            path,
            r#"[tasks.baseline]
run = [
  "mise run baseline",
  "mise run build",
  "mise run test",
]
"#,
        );
    }

    fn write_gate_chain_ci_closure(path: &Path) {
        write_text_file(
            path,
            r#"Current full baseline gate (`mise run baseline`) includes:
- `baseline`
- `build`
- `test`
Local command:

Current deterministic projected check IDs include:
- `baseline`
- `build`
- `test`
## 5. Variants and capability projection
"#,
        );
    }

    fn base_control_plane_contract_payload() -> Value {
        json!({
            "schema": 1,
            "contractKind": "premath.control_plane.contract.v1",
            "schemaLifecycle": {
                "activeEpoch": "2026-02",
                "governance": {
                    "mode": "rollover",
                    "decisionRef": "decision-0105",
                    "owner": "premath-core",
                    "rolloverCadenceMonths": 6
                },
                "kindFamilies": {
                    "controlPlaneContractKind": {
                        "canonicalKind": "premath.control_plane.contract.v1",
                        "compatibilityAliases": [
                            {
                                "aliasKind": "premath.control_plane.contract.v0",
                                "supportUntilEpoch": "2026-06",
                                "replacementKind": "premath.control_plane.contract.v1"
                            }
                        ]
                    },
                    "requiredWitnessKind": {
                        "canonicalKind": "ci.required.v1",
                        "compatibilityAliases": [
                            {
                                "aliasKind": "ci.required.v0",
                                "supportUntilEpoch": "2026-06",
                                "replacementKind": "ci.required.v1"
                            }
                        ]
                    },
                    "requiredDecisionKind": {
                        "canonicalKind": "ci.required.decision.v1",
                        "compatibilityAliases": [
                            {
                                "aliasKind": "ci.required.decision.v0",
                                "supportUntilEpoch": "2026-06",
                                "replacementKind": "ci.required.decision.v1"
                            }
                        ]
                    },
                    "instructionWitnessKind": {
                        "canonicalKind": "ci.instruction.v1",
                        "compatibilityAliases": [
                            {
                                "aliasKind": "ci.instruction.v0",
                                "supportUntilEpoch": "2026-06",
                                "replacementKind": "ci.instruction.v1"
                            }
                        ]
                    },
                    "instructionPolicyKind": {
                        "canonicalKind": "ci.instruction.policy.v1",
                        "compatibilityAliases": [
                            {
                                "aliasKind": "ci.instruction.policy.v0",
                                "supportUntilEpoch": "2026-06",
                                "replacementKind": "ci.instruction.policy.v1"
                            }
                        ]
                    },
                    "requiredProjectionPolicy": {
                        "canonicalKind": "ci-topos-v0",
                        "compatibilityAliases": [
                            {
                                "aliasKind": "ci-topos-v0-preview",
                                "supportUntilEpoch": "2026-06",
                                "replacementKind": "ci-topos-v0"
                            }
                        ]
                    },
                    "requiredDeltaKind": {
                        "canonicalKind": "ci.required.delta.v1",
                        "compatibilityAliases": [
                            {
                                "aliasKind": "ci.delta.v1",
                                "supportUntilEpoch": "2026-06",
                                "replacementKind": "ci.required.delta.v1"
                            }
                        ]
                    }
                }
            },
            "requiredGateProjection": {
                "projectionPolicy": "ci-topos-v0",
                "checkOrder": ["baseline", "build", "test"]
            },
            "requiredWitness": {
                "witnessKind": "ci.required.v1",
                "decisionKind": "ci.required.decision.v1"
            },
            "instructionWitness": {
                "witnessKind": "ci.instruction.v1",
                "policyKind": "ci.instruction.policy.v1",
                "policyDigestPrefix": "pol1_"
            },
            "evidenceStage1Parity": {
                "profileKind": "ev.stage1.core.v1",
                "authorityToTypedCoreRoute": "authority_to_typed_core_projection",
                "comparisonTuple": {
                    "authorityDigestRef": "authorityPayloadDigest",
                    "typedCoreDigestRef": "typedCoreProjectionDigest",
                    "normalizerIdRef": "normalizerId",
                    "policyDigestRef": "policyDigest"
                },
                "failureClasses": {
                    "missing": "unification.evidence_stage1.parity.missing",
                    "mismatch": "unification.evidence_stage1.parity.mismatch",
                    "unbound": "unification.evidence_stage1.parity.unbound"
                }
            },
            "evidenceStage1Rollback": {
                "profileKind": "ev.stage1.rollback.v1",
                "witnessKind": "ev.stage1.rollback.witness.v1",
                "fromStage": "stage1",
                "toStage": "stage0",
                "triggerFailureClasses": [
                    "unification.evidence_stage1.parity.missing",
                    "unification.evidence_stage1.parity.mismatch",
                    "unification.evidence_stage1.parity.unbound"
                ],
                "identityRefs": {
                    "authorityDigestRef": "authorityPayloadDigest",
                    "rollbackAuthorityDigestRef": "rollbackAuthorityPayloadDigest",
                    "normalizerIdRef": "normalizerId",
                    "policyDigestRef": "policyDigest"
                },
                "failureClasses": {
                    "precondition": "unification.evidence_stage1.rollback.precondition",
                    "identityDrift": "unification.evidence_stage1.rollback.identity_drift",
                    "unbound": "unification.evidence_stage1.rollback.unbound"
                }
            },
            "evidenceStage2Authority": {
                "profileKind": "ev.stage2.authority.v1",
                "activeStage": "stage2",
                "typedAuthority": {
                    "kindRef": "ev.stage1.core.v1",
                    "digestRef": "typedCoreProjectionDigest",
                    "normalizerIdRef": "normalizerId",
                    "policyDigestRef": "policyDigest"
                },
                "compatibilityAlias": {
                    "kindRef": "ev.legacy.payload.v1",
                    "digestRef": "authorityPayloadDigest",
                    "role": "projection_only",
                    "supportUntilEpoch": "2026-06"
                },
                "bidirEvidenceRoute": {
                    "routeKind": "direct_checker_discharge",
                    "obligationFieldRef": "bidirCheckerObligations",
                    "requiredObligations": [
                        "stability",
                        "locality",
                        "descent_exists",
                        "descent_contractible",
                        "adjoint_triple",
                        "ext_gap",
                        "ext_ambiguous"
                    ],
                    "failureClasses": {
                        "missing": "unification.evidence_stage2.kernel_compliance_missing",
                        "drift": "unification.evidence_stage2.kernel_compliance_drift"
                    }
                },
                "failureClasses": {
                    "authorityAliasViolation": "unification.evidence_stage2.authority_alias_violation",
                    "aliasWindowViolation": "unification.evidence_stage2.alias_window_violation",
                    "unbound": "unification.evidence_stage2.unbound"
                }
            },
            "evidenceFactorization": {
                "profileKind": "ev.factorization.v1",
                "routeKind": "eta_F_to_Ev",
                "factorizationRoutes": [
                    "eta.control_plane_to_ev"
                ],
                "binding": {
                    "normalizerIdRef": "normalizerId",
                    "policyDigestRef": "policyDigest"
                },
                "crossLaneRoutes": {
                    "pullbackBaseChange": "span_square_commutation"
                },
                "failureClasses": {
                    "missing": "unification.evidence_factorization.missing",
                    "ambiguous": "unification.evidence_factorization.ambiguous",
                    "unbound": "unification.evidence_factorization.unbound"
                }
            },
            "evidenceLanes": {
                "semanticDoctrine": "semantic_doctrine",
                "strictChecker": "strict_checker",
                "witnessCommutation": "witness_commutation",
                "runtimeTransport": "runtime_transport"
            },
            "laneArtifactKinds": {
                "semantic_doctrine": ["kernel_obligation"],
                "strict_checker": ["coherence_obligation"],
                "witness_commutation": ["square_witness"],
                "runtime_transport": ["squeak_site_witness"]
            },
            "laneOwnership": {
                "checkerCoreOnlyObligations": [
                    "cwf_substitution_identity",
                    "cwf_substitution_composition",
                    "cwf_comprehension_beta",
                    "cwf_comprehension_eta"
                ],
                "requiredCrossLaneWitnessRoute": {
                    "pullbackBaseChange": "span_square_commutation"
                }
            },
            "laneFailureClasses": [
                "lane_unknown",
                "lane_kind_unbound",
                "lane_ownership_violation",
                "lane_route_missing"
            ],
            "workerLaneAuthority": {
                "mutationPolicy": {
                    "defaultMode": "instruction-linked",
                    "allowedModes": [
                        "instruction-linked",
                        "human-override"
                    ],
                    "compatibilityOverrides": [
                        {
                            "mode": "human-override",
                            "supportUntilEpoch": "2026-06",
                            "requiresReason": true
                        }
                    ]
                },
                "mutationRoutes": {
                    "issueClaim": "capabilities.change_morphisms.issue_claim",
                    "issueLeaseRenew": "capabilities.change_morphisms.issue_lease_renew",
                    "issueLeaseRelease": "capabilities.change_morphisms.issue_lease_release",
                    "issueDiscover": "capabilities.change_morphisms.issue_discover"
                },
                "failureClasses": {
                    "policyDrift": "worker_lane_policy_drift",
                    "mutationModeDrift": "worker_lane_mutation_mode_drift",
                    "routeUnbound": "worker_lane_route_unbound"
                }
            }
        })
    }

    fn test_contract_for_gate_chain(control_plane_contract_path: &str) -> CoherenceContract {
        let mut contract = test_contract_with_fixture_roots("", "");
        contract.surfaces.mise_path = ".mise.toml".to_string();
        contract.surfaces.mise_baseline_task = "baseline".to_string();
        contract.surfaces.ci_closure_path = "docs/design/CI-CLOSURE.md".to_string();
        contract.surfaces.ci_closure_baseline_start =
            "Current full baseline gate (`mise run baseline`) includes:".to_string();
        contract.surfaces.ci_closure_baseline_end = "Local command:".to_string();
        contract.surfaces.ci_closure_projection_start =
            "Current deterministic projected check IDs include:".to_string();
        contract.surfaces.ci_closure_projection_end =
            "## 5. Variants and capability projection".to_string();
        contract.surfaces.control_plane_contract_path = control_plane_contract_path.to_string();
        contract
    }

    fn write_transport_manifest(fixture_root: &Path, vectors: &[&str]) {
        write_json_file(
            &fixture_root.join("manifest.json"),
            &json!({
                "schema": 1,
                "status": "executable",
                "vectors": vectors,
            }),
        );
    }

    fn write_transport_vector_with_metadata(
        fixture_root: &Path,
        vector_id: &str,
        expected_result: &str,
        semantic_scenario_id: Option<&str>,
        profile: Option<&str>,
    ) {
        let (f_identity_arrow, expected_failure_classes) = if expected_result == "accepted" {
            ("id_fx", json!([]))
        } else if expected_result == "rejected" {
            (
                "id_fx_bad",
                json!(["coherence.transport_functoriality.identity_violation"]),
            )
        } else {
            panic!("unsupported expected_result in write_transport_vector: {expected_result}");
        };

        let vector_root = fixture_root.join(vector_id);
        let mut case_payload = serde_json::Map::new();
        case_payload.insert("schema".to_string(), json!(1));
        case_payload.insert("status".to_string(), json!("executable"));
        case_payload.insert("vectorId".to_string(), json!(vector_id));
        case_payload.insert(
            "artifacts".to_string(),
            json!({
                "binding": {
                    "normalizerId": "normalizer.coherence.v1",
                    "policyDigest": "policy.coherence.v1",
                },
                "base": {
                    "identity": {"arrow": "id_x"},
                    "f": {"arrow": "f"},
                    "g": {"arrow": "g"},
                    "gAfterF": {"arrow": "g_after_f"},
                },
                "fibre": {
                    "identity": {"arrow": "id_fx"},
                    "FIdentity": {"arrow": f_identity_arrow},
                    "FF": {"arrow": "f_f"},
                    "FG": {"arrow": "f_g"},
                    "FGAfterF": {"arrow": "f_g_after_f"},
                    "FGAfterFF": {"arrow": "f_g_after_f"},
                },
                "naturality": {
                    "left": {"square": {"bottom": "g_f"}},
                    "right": {"square": {"bottom": "g_f"}},
                },
            }),
        );
        if let Some(value) = semantic_scenario_id {
            case_payload.insert("semanticScenarioId".to_string(), json!(value));
        }
        if let Some(value) = profile {
            case_payload.insert("profile".to_string(), json!(value));
        }
        write_json_file(&vector_root.join("case.json"), &Value::Object(case_payload));
        write_json_file(
            &vector_root.join("expect.json"),
            &json!({
                "schema": 1,
                "status": "executable",
                "result": expected_result,
                "expectedFailureClasses": expected_failure_classes,
            }),
        );
    }

    fn write_transport_vector(fixture_root: &Path, vector_id: &str, expected_result: &str) {
        write_transport_vector_with_metadata(fixture_root, vector_id, expected_result, None, None);
    }

    fn write_site_manifest(fixture_root: &Path, vectors: &[&str], obligation_vectors: &[&str]) {
        write_json_file(
            &fixture_root.join("manifest.json"),
            &json!({
                "schema": 1,
                "status": "executable",
                "vectors": vectors,
                "obligationVectors": {
                    "span_square_commutation": obligation_vectors
                }
            }),
        );
    }

    fn span_square_spans() -> Value {
        json!([
            {
                "id": "top",
                "kind": "pipeline",
                "left": {"ctx": "Gamma", "input": "x"},
                "apex": {"run": "r"},
                "right": {"out": "y"}
            },
            {
                "id": "bottom",
                "kind": "pipeline",
                "left": {"ctx": "Gamma", "input": "x"},
                "apex": {"run": "r"},
                "right": {"out": "y"}
            },
            {
                "id": "left",
                "kind": "base_change",
                "left": {"ctx": "Delta", "input": "x"},
                "apex": {"map": "rho"},
                "right": {"ctx": "Gamma", "input": "x"}
            },
            {
                "id": "right",
                "kind": "base_change",
                "left": {"out": "y"},
                "apex": {"map": "rho"},
                "right": {"out": "y"}
            }
        ])
    }

    fn valid_span_square_artifacts_for_result(expected_result: &str) -> Value {
        let (square_result, square_failures, square_digest) = if expected_result == "rejected" {
            (
                "accepted",
                Vec::<String>::new(),
                "sqw1_digest_mismatch_for_reject_fixture".to_string(),
            )
        } else {
            let failures = Vec::<String>::new();
            (
                "accepted",
                failures.clone(),
                square_witness_digest("top", "bottom", "left", "right", "accepted", &failures),
            )
        };
        json!({
            "spanSquare": {
                "spans": span_square_spans(),
                "squares": [
                    {
                        "id": "sq_ok",
                        "top": "top",
                        "bottom": "bottom",
                        "left": "left",
                        "right": "right",
                        "result": square_result,
                        "failureClasses": square_failures,
                        "digest": square_digest
                    }
                ]
            }
        })
    }

    fn write_site_vector_with_metadata(
        fixture_root: &Path,
        vector_id: &str,
        obligation_id: &str,
        expected_result: &str,
        semantic_scenario_id: Option<&str>,
        profile: Option<&str>,
    ) {
        let (artifacts, expected_failure_classes) = if expected_result == "accepted" {
            (
                valid_span_square_artifacts_for_result("accepted"),
                json!([]),
            )
        } else if expected_result == "rejected" {
            (
                valid_span_square_artifacts_for_result("rejected"),
                json!(["coherence.span_square_commutation.violation"]),
            )
        } else {
            panic!("unsupported expected_result in write_site_vector: {expected_result}");
        };
        let vector_root = fixture_root.join(vector_id);
        let mut case_payload = serde_json::Map::new();
        case_payload.insert("schema".to_string(), json!(1));
        case_payload.insert("status".to_string(), json!("executable"));
        case_payload.insert("obligationId".to_string(), json!(obligation_id));
        case_payload.insert("artifacts".to_string(), artifacts);
        if let Some(value) = semantic_scenario_id {
            case_payload.insert("semanticScenarioId".to_string(), json!(value));
        }
        if let Some(value) = profile {
            case_payload.insert("profile".to_string(), json!(value));
        }
        write_json_file(&vector_root.join("case.json"), &Value::Object(case_payload));
        write_json_file(
            &vector_root.join("expect.json"),
            &json!({
                "schema": 1,
                "status": "executable",
                "result": expected_result,
                "expectedFailureClasses": expected_failure_classes,
            }),
        );
    }

    fn write_site_vector(
        fixture_root: &Path,
        vector_id: &str,
        obligation_id: &str,
        expected_result: &str,
    ) {
        write_site_vector_with_metadata(
            fixture_root,
            vector_id,
            obligation_id,
            expected_result,
            None,
            None,
        );
    }

    fn test_contract_with_fixture_roots(
        transport_fixture_root_path: &str,
        site_fixture_root_path: &str,
    ) -> CoherenceContract {
        CoherenceContract {
            schema: 1,
            contract_kind: "premath.coherence.contract.v1".to_string(),
            contract_id: "coherence.test.v1".to_string(),
            binding: CoherenceBinding {
                normalizer_id: "normalizer.coherence.v1".to_string(),
                policy_digest: "policy.coherence.v1".to_string(),
            },
            obligations: Vec::new(),
            surfaces: CoherenceSurfaces {
                capability_registry_path: String::new(),
                capability_registry_kind: String::new(),
                conformance_path: String::new(),
                capability_manifest_root: String::new(),
                readme_path: String::new(),
                conformance_readme_path: String::new(),
                spec_index_path: String::new(),
                spec_index_capability_heading: String::new(),
                spec_index_informative_heading: String::new(),
                spec_index_overlay_heading: String::new(),
                ci_closure_path: String::new(),
                ci_closure_baseline_start: String::new(),
                ci_closure_baseline_end: String::new(),
                ci_closure_projection_start: String::new(),
                ci_closure_projection_end: String::new(),
                mise_path: String::new(),
                mise_baseline_task: String::new(),
                control_plane_contract_path: String::new(),
                doctrine_site_path: String::new(),
                doctrine_root_node_id: String::new(),
                profile_readme_path: String::new(),
                bidir_spec_path: String::new(),
                bidir_spec_section_start: String::new(),
                bidir_spec_section_end: String::new(),
                coherence_spec_path: String::new(),
                coherence_spec_obligation_start: String::new(),
                coherence_spec_obligation_end: String::new(),
                obligation_registry_kind: String::new(),
                informative_clause_needle: String::new(),
                transport_fixture_root_path: transport_fixture_root_path.to_string(),
                site_fixture_root_path: site_fixture_root_path.to_string(),
            },
            conditional_capability_docs: Vec::new(),
            expected_operation_paths: Vec::new(),
            overlay_docs: Vec::new(),
            required_bidir_obligations: vec![
                "stability".to_string(),
                "locality".to_string(),
                "descent_exists".to_string(),
                "descent_contractible".to_string(),
                "adjoint_triple".to_string(),
                "ext_gap".to_string(),
                "ext_ambiguous".to_string(),
            ],
        }
    }

    fn test_contract_with_transport_fixture_root(
        transport_fixture_root_path: &str,
    ) -> CoherenceContract {
        test_contract_with_fixture_roots(transport_fixture_root_path, "")
    }

    fn test_contract_with_site_fixture_root(site_fixture_root_path: &str) -> CoherenceContract {
        test_contract_with_fixture_roots("", site_fixture_root_path)
    }

    #[test]
    fn extract_section_between_returns_body() {
        let text = "prefix START body END suffix";
        let section =
            extract_section_between(text, "START", "END").expect("section extraction should work");
        assert_eq!(section.trim(), "body");
    }

    #[test]
    fn check_gate_chain_parity_accepts_valid_lane_registry() {
        let temp = TempDirGuard::new("gate-chain-lane-registry-valid");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &base_control_plane_contract_payload(),
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn check_gate_chain_parity_rejects_missing_schema_lifecycle() {
        let temp = TempDirGuard::new("gate-chain-schema-lifecycle-missing");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload
            .as_object_mut()
            .expect("payload should be object")
            .remove("schemaLifecycle");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_expired_schema_alias() {
        let temp = TempDirGuard::new("gate-chain-schema-lifecycle-expired-alias");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["requiredWitness"]["witnessKind"] = json!("ci.required.v0");
        payload["schemaLifecycle"]["activeEpoch"] = json!("2026-07");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_freeze_with_active_aliases() {
        let temp = TempDirGuard::new("gate-chain-schema-lifecycle-freeze-with-aliases");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["schemaLifecycle"]["governance"] = json!({
            "mode": "freeze",
            "decisionRef": "decision-0105",
            "owner": "premath-core",
            "freezeReason": "release-freeze"
        });
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_SCHEMA_LIFECYCLE_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_accepts_freeze_without_aliases() {
        let temp = TempDirGuard::new("gate-chain-schema-lifecycle-freeze-no-aliases");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["schemaLifecycle"]["governance"] = json!({
            "mode": "freeze",
            "decisionRef": "decision-0105",
            "owner": "premath-core",
            "freezeReason": "release-freeze"
        });
        if let Some(kind_families) = payload["schemaLifecycle"]["kindFamilies"].as_object_mut() {
            for family in kind_families.values_mut() {
                family["compatibilityAliases"] = json!([]);
            }
        }
        payload
            .as_object_mut()
            .expect("payload should be object")
            .remove("evidenceStage2Authority");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn check_gate_chain_parity_rejects_duplicate_lane_ids() {
        let temp = TempDirGuard::new("gate-chain-lane-registry-duplicate-ids");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceLanes"]["runtimeTransport"] = json!("strict_checker");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.gate_chain_parity.lane_unknown".to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_unknown_lane_artifact_kind_mapping() {
        let temp = TempDirGuard::new("gate-chain-lane-registry-unknown-lane-kind");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["laneArtifactKinds"]["unknown_lane"] = json!(["opaque_kind"]);
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.gate_chain_parity.lane_kind_unbound".to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_missing_cross_lane_route() {
        let temp = TempDirGuard::new("gate-chain-lane-registry-missing-route");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["laneOwnership"]["requiredCrossLaneWitnessRoute"] = Value::Null;
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.gate_chain_parity.lane_route_missing".to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_checker_core_ownership_violation() {
        let temp = TempDirGuard::new("gate-chain-lane-registry-ownership-violation");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["laneOwnership"]["checkerCoreOnlyObligations"] =
            json!(["cwf_substitution_identity", "span_square_commutation"]);
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.gate_chain_parity.lane_ownership_violation".to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_worker_lane_default_mode_drift() {
        let temp = TempDirGuard::new("gate-chain-worker-lane-default-mode-drift");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["workerLaneAuthority"]["mutationPolicy"]["defaultMode"] = json!("human-override");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_WORKER_MUTATION_MODE_DRIFT_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_worker_lane_route_drift() {
        let temp = TempDirGuard::new("gate-chain-worker-lane-route-drift");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["workerLaneAuthority"]["mutationRoutes"]["issueDiscover"] = json!("issue_discover");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_WORKER_ROUTE_UNBOUND_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_worker_lane_policy_drift() {
        let temp = TempDirGuard::new("gate-chain-worker-lane-policy-drift");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["workerLaneAuthority"]["mutationPolicy"]["compatibilityOverrides"][0]["supportUntilEpoch"] =
            json!("2026-01");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_WORKER_POLICY_DRIFT_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_evidence_factorization_missing_route() {
        let temp = TempDirGuard::new("gate-chain-evidence-factorization-missing-route");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceFactorization"]["factorizationRoutes"] = json!([]);
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_EVIDENCE_FACTORIZATION_MISSING_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_evidence_factorization_ambiguous_routes() {
        let temp = TempDirGuard::new("gate-chain-evidence-factorization-ambiguous-routes");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceFactorization"]["factorizationRoutes"] =
            json!(["eta.route_a", "eta.route_b"]);
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_EVIDENCE_FACTORIZATION_AMBIGUOUS_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_evidence_factorization_unbound_binding() {
        let temp = TempDirGuard::new("gate-chain-evidence-factorization-unbound-binding");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceFactorization"]["binding"]["policyDigestRef"] = json!("policy");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_EVIDENCE_FACTORIZATION_UNBOUND_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage1_missing_route() {
        let temp = TempDirGuard::new("gate-chain-stage1-missing-route");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage1Parity"]["authorityToTypedCoreRoute"] = json!("");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE1_PARITY_MISSING_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage1_unbound_binding_tuple() {
        let temp = TempDirGuard::new("gate-chain-stage1-unbound-binding");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage1Parity"]["comparisonTuple"]["normalizerIdRef"] = json!("normalizer");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE1_PARITY_UNBOUND_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage1_failure_class_mismatch() {
        let temp = TempDirGuard::new("gate-chain-stage1-failure-class-mismatch");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage1Parity"]["failureClasses"]["mismatch"] = json!("ev.parity.mismatch");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE1_PARITY_MISMATCH_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage1_missing_profile_kind() {
        let temp = TempDirGuard::new("gate-chain-stage1-missing-profile-kind");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage1Parity"]["profileKind"] = json!("");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE1_PARITY_INVALID_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage1_rollback_missing_trigger_classes() {
        let temp = TempDirGuard::new("gate-chain-stage1-rollback-missing-triggers");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage1Rollback"]["triggerFailureClasses"] =
            json!(["unification.evidence_stage1.parity.missing"]);
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE1_ROLLBACK_PRECONDITION_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage1_rollback_unbound_binding_tuple() {
        let temp = TempDirGuard::new("gate-chain-stage1-rollback-unbound-binding");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage1Rollback"]["identityRefs"]["policyDigestRef"] = json!("policy");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE1_ROLLBACK_UNBOUND_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage1_rollback_failure_class_mismatch() {
        let temp = TempDirGuard::new("gate-chain-stage1-rollback-class-mismatch");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage1Rollback"]["failureClasses"]["identityDrift"] =
            json!("ev.rollback.identity");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE1_ROLLBACK_MISMATCH_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage2_alias_role_mismatch() {
        let temp = TempDirGuard::new("gate-chain-stage2-alias-role-mismatch");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage2Authority"]["compatibilityAlias"]["role"] = json!("authority");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_VIOLATION_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage2_alias_window_mismatch() {
        let temp = TempDirGuard::new("gate-chain-stage2-alias-window-mismatch");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage2Authority"]["compatibilityAlias"]["supportUntilEpoch"] =
            json!("2026-07");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE2_AUTHORITY_ALIAS_WINDOW_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage2_unbound_binding_tuple() {
        let temp = TempDirGuard::new("gate-chain-stage2-unbound-binding");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage2Authority"]["typedAuthority"]["policyDigestRef"] = json!("policy");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE2_AUTHORITY_UNBOUND_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage2_failure_class_mismatch() {
        let temp = TempDirGuard::new("gate-chain-stage2-failure-class-mismatch");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage2Authority"]["failureClasses"]["unbound"] =
            json!("unification.evidence_stage2.not_bound");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE2_AUTHORITY_INVALID_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage2_bidir_route_obligation_mismatch() {
        let temp = TempDirGuard::new("gate-chain-stage2-bidir-route-obligation-mismatch");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage2Authority"]["bidirEvidenceRoute"]["requiredObligations"] =
            json!(["stability"]);
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE2_KERNEL_MISSING_FAILURE.to_string())
        );
    }

    #[test]
    fn check_gate_chain_parity_rejects_stage2_bidir_route_failure_class_mismatch() {
        let temp = TempDirGuard::new("gate-chain-stage2-bidir-route-class-mismatch");
        write_gate_chain_mise(&temp.path().join(".mise.toml"));
        write_gate_chain_ci_closure(&temp.path().join("docs/design/CI-CLOSURE.md"));
        let mut payload = base_control_plane_contract_payload();
        payload["evidenceStage2Authority"]["bidirEvidenceRoute"]["failureClasses"]["drift"] =
            json!("unification.evidence_stage2.kernel_drift");
        write_json_file(
            &temp
                .path()
                .join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json"),
            &payload,
        );
        let contract =
            test_contract_for_gate_chain("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

        let evaluated =
            check_gate_chain_parity(temp.path(), &contract).expect("gate parity should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&GATE_CHAIN_STAGE2_KERNEL_DRIFT_FAILURE.to_string())
        );
    }

    #[test]
    fn semantic_digest_is_order_invariant_for_transport_payloads() {
        let a = json!({
            "terms": [{"sym": "v"}, {"sym": "u"}, {"sym": "u"}],
            "arrow": "id_fx",
        });
        let b = json!({
            "arrow": "id_fx",
            "terms": [{"sym": "u"}, {"sym": "v"}],
        });
        assert_eq!(semantic_digest(&a), semantic_digest(&b));
    }

    #[test]
    fn evaluate_transport_case_detects_identity_violation() {
        let case = json!({
            "artifacts": {
                "binding": {
                    "normalizerId": "normalizer.coherence.v1",
                    "policyDigest": "policy.coherence.v1",
                },
                "base": {
                    "identity": {"arrow": "id_x"},
                    "f": {"arrow": "f"},
                    "g": {"arrow": "g"},
                    "gAfterF": {"arrow": "g_after_f"},
                },
                "fibre": {
                    "identity": {"arrow": "id_fx"},
                    "FIdentity": {"arrow": "id_fx_bad"},
                    "FF": {"arrow": "f_f"},
                    "FG": {"arrow": "f_g"},
                    "FGAfterF": {"arrow": "f_g_after_f"},
                    "FGAfterFF": {"arrow": "f_g_after_f"},
                },
                "naturality": {
                    "left": {"square": {"bottom": "g_f"}},
                    "right": {"square": {"bottom": "g_f"}},
                },
            }
        });
        let evaluated = evaluate_transport_case(&case, Path::new("transport-case.json"))
            .expect("transport case should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.transport_functoriality.identity_violation".to_string())
        );
    }

    #[test]
    fn check_transport_functoriality_requires_golden_polarity_vector() {
        let temp = TempDirGuard::new("transport-missing-golden");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(&fixture_root, &["adversarial/only_reject"]);
        write_transport_vector(&fixture_root, "adversarial/only_reject", "rejected");
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.transport_functoriality.missing_golden_vector".to_string())
        );
    }

    #[test]
    fn check_transport_functoriality_requires_adversarial_polarity_vector() {
        let temp = TempDirGuard::new("transport-missing-adversarial");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(&fixture_root, &["golden/only_accept"]);
        write_transport_vector(&fixture_root, "golden/only_accept", "accepted");
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(
            evaluated.failure_classes.contains(
                &"coherence.transport_functoriality.missing_adversarial_vector".to_string()
            )
        );
    }

    #[test]
    fn check_transport_functoriality_requires_expected_accept_result_vector() {
        let temp = TempDirGuard::new("transport-missing-expected-accept");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(
            &fixture_root,
            &["golden/reject_vector", "adversarial/reject_vector"],
        );
        write_transport_vector(&fixture_root, "golden/reject_vector", "rejected");
        write_transport_vector(&fixture_root, "adversarial/reject_vector", "rejected");
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(evaluated.failure_classes.contains(
            &"coherence.transport_functoriality.missing_expected_accepted_vector".to_string()
        ));
    }

    #[test]
    fn check_transport_functoriality_requires_expected_reject_result_vector() {
        let temp = TempDirGuard::new("transport-missing-expected-reject");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(
            &fixture_root,
            &["golden/accept_vector", "adversarial/accept_vector"],
        );
        write_transport_vector(&fixture_root, "golden/accept_vector", "accepted");
        write_transport_vector(&fixture_root, "adversarial/accept_vector", "accepted");
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(evaluated.failure_classes.contains(
            &"coherence.transport_functoriality.missing_expected_rejected_vector".to_string()
        ));
    }

    #[test]
    fn check_transport_functoriality_accepts_when_both_polarities_present() {
        let temp = TempDirGuard::new("transport-both-polarities");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(
            &fixture_root,
            &["golden/accept_vector", "adversarial/reject_vector"],
        );
        write_transport_vector(&fixture_root, "golden/accept_vector", "accepted");
        write_transport_vector(&fixture_root, "adversarial/reject_vector", "rejected");
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn check_transport_functoriality_requires_invariance_pair_count() {
        let temp = TempDirGuard::new("transport-invariance-pair-count");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(
            &fixture_root,
            &[
                "golden/functorial_transport_accept",
                "adversarial/identity_violation_reject",
                "invariance/permuted_payload_local_accept",
            ],
        );
        write_transport_vector(
            &fixture_root,
            "golden/functorial_transport_accept",
            "accepted",
        );
        write_transport_vector(
            &fixture_root,
            "adversarial/identity_violation_reject",
            "rejected",
        );
        write_transport_vector_with_metadata(
            &fixture_root,
            "invariance/permuted_payload_local_accept",
            "accepted",
            Some("transport_functoriality_invariance_pair"),
            Some("local"),
        );
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(evaluated.failure_classes.contains(
            &"coherence.transport_functoriality.invariance_pair_count_mismatch".to_string()
        ));
    }

    #[test]
    fn check_transport_functoriality_requires_invariance_pair_result_match() {
        let temp = TempDirGuard::new("transport-invariance-result-mismatch");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(
            &fixture_root,
            &[
                "golden/functorial_transport_accept",
                "adversarial/identity_violation_reject",
                "invariance/permuted_payload_local_accept",
                "invariance/permuted_payload_external_reject",
            ],
        );
        write_transport_vector(
            &fixture_root,
            "golden/functorial_transport_accept",
            "accepted",
        );
        write_transport_vector(
            &fixture_root,
            "adversarial/identity_violation_reject",
            "rejected",
        );
        write_transport_vector_with_metadata(
            &fixture_root,
            "invariance/permuted_payload_local_accept",
            "accepted",
            Some("transport_functoriality_invariance_pair"),
            Some("local"),
        );
        write_transport_vector_with_metadata(
            &fixture_root,
            "invariance/permuted_payload_external_reject",
            "rejected",
            Some("transport_functoriality_invariance_pair"),
            Some("external"),
        );
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(
            evaluated.failure_classes.contains(
                &"coherence.transport_functoriality.invariance_result_mismatch".to_string()
            )
        );
    }

    #[test]
    fn check_transport_functoriality_accepts_with_invariance_pair() {
        let temp = TempDirGuard::new("transport-invariance-pair-pass");
        let fixture_root = temp.path().join("fixtures");
        write_transport_manifest(
            &fixture_root,
            &[
                "golden/functorial_transport_accept",
                "adversarial/identity_violation_reject",
                "invariance/permuted_payload_local_accept",
                "invariance/permuted_payload_external_accept",
            ],
        );
        write_transport_vector(
            &fixture_root,
            "golden/functorial_transport_accept",
            "accepted",
        );
        write_transport_vector(
            &fixture_root,
            "adversarial/identity_violation_reject",
            "rejected",
        );
        write_transport_vector_with_metadata(
            &fixture_root,
            "invariance/permuted_payload_local_accept",
            "accepted",
            Some("transport_functoriality_invariance_pair"),
            Some("local"),
        );
        write_transport_vector_with_metadata(
            &fixture_root,
            "invariance/permuted_payload_external_accept",
            "accepted",
            Some("transport_functoriality_invariance_pair"),
            Some("external"),
        );
        let contract = test_contract_with_transport_fixture_root("fixtures");

        let evaluated = check_transport_functoriality(temp.path(), &contract)
            .expect("transport should evaluate");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn evaluate_site_case_coverage_base_change_detects_violation() {
        let case = json!({
            "coverage": {
                "baseCover": {"parts": ["U1", "U2"]},
                "pullbackCover": {"parts": ["U1_pb", "WRONG_pb"]},
                "pullbackOfParts": [
                    {"source": "U1", "pullback": "U1_pb"},
                    {"source": "U2", "pullback": "U2_pb"}
                ]
            }
        });
        let evaluated =
            evaluate_site_case_coverage_base_change(&case, Path::new("site-case-base-change.json"))
                .expect("site base-change case should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.coverage_base_change.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_coverage_transitivity_detects_violation() {
        let case = json!({
            "coverage": {
                "outerCover": {"parts": ["U1", "U2"]},
                "refinementCovers": [
                    {"over": "U1", "parts": ["U11"]},
                    {"over": "U3", "parts": ["U31"]}
                ],
                "composedCover": {"parts": ["U11"]}
            }
        });
        let evaluated = evaluate_site_case_coverage_transitivity(
            &case,
            Path::new("site-case-transitivity.json"),
        )
        .expect("site transitivity case should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.coverage_transitivity.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_glue_or_witness_detects_missing_both() {
        let case = json!({
            "descent": {
                "locals": [{"id": "s1"}, {"id": "s2"}],
                "compatibilityWitnesses": []
            }
        });
        let evaluated = evaluate_site_case_glue_or_witness_contractibility(
            &case,
            Path::new("site-case-glue-or-witness.json"),
        )
        .expect("site glue-or-witness case should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.glue_or_witness_contractibility.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_cwf_substitution_identity_detects_violation() {
        let case = json!({
            "cwf": {
                "substitution": {
                    "types": [
                        {"label": "A", "direct": {"type": "A"}, "afterIdentity": {"type": "A_bad"}}
                    ],
                    "terms": [
                        {"label": "t", "direct": {"term": "t"}, "afterIdentity": {"term": "t"}}
                    ]
                }
            }
        });
        let evaluated = evaluate_site_case_cwf_substitution_identity(
            &case,
            Path::new("site-case-cwf-substitution-identity.json"),
        )
        .expect("cwf substitution identity should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.cwf_substitution_identity.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_cwf_substitution_composition_detects_violation() {
        let case = json!({
            "cwf": {
                "substitution": {
                    "types": [
                        {"label": "A", "afterCompose": {"type": "A_fg"}, "afterStepwise": {"type": "A_fg"}}
                    ],
                    "terms": [
                        {"label": "t", "afterCompose": {"term": "t_fg"}, "afterStepwise": {"term": "t_bad"}}
                    ]
                }
            }
        });
        let evaluated = evaluate_site_case_cwf_substitution_composition(
            &case,
            Path::new("site-case-cwf-substitution-composition.json"),
        )
        .expect("cwf substitution composition should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.cwf_substitution_composition.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_cwf_comprehension_beta_detects_violation() {
        let case = json!({
            "cwf": {
                "comprehension": {
                    "beta": [
                        {"label": "a", "original": {"term": "a"}, "afterBeta": {"term": "a_bad"}}
                    ]
                }
            }
        });
        let evaluated = evaluate_site_case_cwf_comprehension_beta(
            &case,
            Path::new("site-case-cwf-comprehension-beta.json"),
        )
        .expect("cwf comprehension beta should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.cwf_comprehension_beta.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_cwf_comprehension_eta_detects_violation() {
        let case = json!({
            "cwf": {
                "comprehension": {
                    "eta": [
                        {"label": "sigma", "original": {"subst": "sigma"}, "afterEta": {"subst": "sigma_bad"}}
                    ]
                }
            }
        });
        let evaluated = evaluate_site_case_cwf_comprehension_eta(
            &case,
            Path::new("site-case-cwf-comprehension-eta.json"),
        )
        .expect("cwf comprehension eta should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.cwf_comprehension_eta.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_span_square_commutation_detects_violation() {
        let failure_classes: Vec<String> = Vec::new();
        let case = json!({
            "spanSquare": {
                "spans": [
                    {
                        "id": "top",
                        "kind": "pipeline",
                        "left": {"ctx": "Gamma"},
                        "apex": {"run": "a"},
                        "right": {"out": "x"}
                    },
                    {
                        "id": "bottom",
                        "kind": "pipeline",
                        "left": {"ctx": "Gamma"},
                        "apex": {"run": "b"},
                        "right": {"out": "y"}
                    },
                    {
                        "id": "left",
                        "kind": "base_change",
                        "left": {"ctx": "Delta"},
                        "apex": {"reindex": "in"},
                        "right": {"ctx": "Gamma"}
                    },
                    {
                        "id": "right",
                        "kind": "base_change",
                        "left": {"out": "x"},
                        "apex": {"reindex": "out"},
                        "right": {"out": "y"}
                    }
                ],
                "squares": [
                    {
                        "id": "sq1",
                        "top": "top",
                        "bottom": "bottom",
                        "left": "left",
                        "right": "right",
                        "result": "accepted",
                        "failureClasses": failure_classes,
                        "digest": square_witness_digest("top", "bottom", "left", "right", "accepted", &Vec::new())
                    }
                ]
            }
        });
        let evaluated = evaluate_site_case_span_square_commutation(
            &case,
            Path::new("site-case-span-square-commutation.json"),
        )
        .expect("span/square commutation case should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.span_square_commutation.violation".to_string())
        );
    }

    #[test]
    fn evaluate_site_case_span_square_commutation_accepts_composition_laws() {
        let square_failures: Vec<String> = Vec::new();
        let span_identity_left =
            json!({"compose": {"left": {"span": "span_id"}, "right": {"span": "run_on_base"}}});
        let span_identity_right = json!({"span": "run_on_base"});
        let span_assoc_left = json!({
            "compose": {
                "left": {"compose": {"left": {"span": "run_on_base"}, "right": {"span": "reindex_input"}}},
                "right": {"span": "reindex_output"}
            }
        });
        let span_assoc_right = json!({
            "compose": {
                "left": {"span": "run_on_base"},
                "right": {"compose": {"left": {"span": "reindex_input"}, "right": {"span": "reindex_output"}}}
            }
        });
        let square_identity_left = json!({
            "compose": {
                "mode": "horizontal",
                "left": {"square": "sq_id"},
                "right": {"square": "sq_accept"}
            }
        });
        let square_identity_right = json!({"square": "sq_accept"});
        let square_assoc_horizontal_left = json!({
            "compose": {
                "mode": "horizontal",
                "left": {
                    "compose": {
                        "mode": "horizontal",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                },
                "right": {"square": "sq_accept"}
            }
        });
        let square_assoc_horizontal_right = json!({
            "compose": {
                "mode": "horizontal",
                "left": {"square": "sq_accept"},
                "right": {
                    "compose": {
                        "mode": "horizontal",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                }
            }
        });
        let square_assoc_vertical_left = json!({
            "compose": {
                "mode": "vertical",
                "left": {
                    "compose": {
                        "mode": "vertical",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                },
                "right": {"square": "sq_accept"}
            }
        });
        let square_assoc_vertical_right = json!({
            "compose": {
                "mode": "vertical",
                "left": {"square": "sq_accept"},
                "right": {
                    "compose": {
                        "mode": "vertical",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                }
            }
        });
        let square_hv_left = json!({
            "compose": {
                "mode": "horizontal",
                "left": {"square": "sq_id"},
                "right": {
                    "compose": {
                        "mode": "vertical",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                }
            }
        });
        let square_hv_right = json!({
            "compose": {
                "mode": "vertical",
                "left": {"square": "sq_accept"},
                "right": {"square": "sq_accept"}
            }
        });
        let square_interchange_left = json!({
            "compose": {
                "mode": "vertical",
                "left": {
                    "compose": {
                        "mode": "horizontal",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                },
                "right": {
                    "compose": {
                        "mode": "horizontal",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                }
            }
        });
        let square_interchange_right = json!({
            "compose": {
                "mode": "horizontal",
                "left": {
                    "compose": {
                        "mode": "vertical",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                },
                "right": {
                    "compose": {
                        "mode": "vertical",
                        "left": {"square": "sq_accept"},
                        "right": {"square": "sq_accept"}
                    }
                }
            }
        });
        let case = json!({
            "spanSquare": {
                "spans": [
                    {
                        "id": "span_id",
                        "kind": "identity",
                        "left": {"ctx": "Gamma"},
                        "apex": {"id": true},
                        "right": {"ctx": "Gamma"}
                    },
                    {
                        "id": "run_on_base",
                        "kind": "pipeline",
                        "left": {"ctx": "Gamma"},
                        "apex": {"run": "base"},
                        "right": {"out": "y"}
                    },
                    {
                        "id": "run_after_reindex",
                        "kind": "pipeline",
                        "left": {"ctx": "Gamma"},
                        "apex": {"run": "base"},
                        "right": {"out": "y"}
                    },
                    {
                        "id": "reindex_input",
                        "kind": "base_change",
                        "left": {"ctx": "Delta"},
                        "apex": {"map": "rho"},
                        "right": {"ctx": "Gamma"}
                    },
                    {
                        "id": "reindex_output",
                        "kind": "base_change",
                        "left": {"out": "y"},
                        "apex": {"map": "rho"},
                        "right": {"out": "y"}
                    }
                ],
                "squares": [
                    {
                        "id": "sq_accept",
                        "top": "run_on_base",
                        "bottom": "run_after_reindex",
                        "left": "reindex_input",
                        "right": "reindex_output",
                        "result": "accepted",
                        "failureClasses": square_failures,
                        "digest": square_witness_digest("run_on_base", "run_after_reindex", "reindex_input", "reindex_output", "accepted", &Vec::new())
                    },
                    {
                        "id": "sq_id",
                        "top": "run_on_base",
                        "bottom": "run_after_reindex",
                        "left": "reindex_input",
                        "right": "reindex_output",
                        "result": "accepted",
                        "failureClasses": [],
                        "digest": square_witness_digest("run_on_base", "run_after_reindex", "reindex_input", "reindex_output", "accepted", &Vec::new())
                    }
                ],
                "compositionLaws": {
                    "identitySpanIds": ["span_id"],
                    "identitySquareIds": ["sq_id"],
                    "laws": [
                        {
                            "id": "law_span_identity",
                            "kind": "span",
                            "law": "span_identity",
                            "left": span_identity_left,
                            "right": span_identity_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("span", "span_identity", &span_identity_left, &span_identity_right, "accepted", &Vec::new())
                        },
                        {
                            "id": "law_span_assoc",
                            "kind": "span",
                            "law": "span_associativity",
                            "left": span_assoc_left,
                            "right": span_assoc_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("span", "span_associativity", &span_assoc_left, &span_assoc_right, "accepted", &Vec::new())
                        },
                        {
                            "id": "law_sq_identity",
                            "kind": "square",
                            "law": "square_identity",
                            "left": square_identity_left,
                            "right": square_identity_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("square", "square_identity", &square_identity_left, &square_identity_right, "accepted", &Vec::new())
                        },
                        {
                            "id": "law_sq_assoc_h",
                            "kind": "square",
                            "law": "square_associativity_horizontal",
                            "left": square_assoc_horizontal_left,
                            "right": square_assoc_horizontal_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("square", "square_associativity_horizontal", &square_assoc_horizontal_left, &square_assoc_horizontal_right, "accepted", &Vec::new())
                        },
                        {
                            "id": "law_sq_assoc_v",
                            "kind": "square",
                            "law": "square_associativity_vertical",
                            "left": square_assoc_vertical_left,
                            "right": square_assoc_vertical_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("square", "square_associativity_vertical", &square_assoc_vertical_left, &square_assoc_vertical_right, "accepted", &Vec::new())
                        },
                        {
                            "id": "law_sq_hv",
                            "kind": "square",
                            "law": "square_hv_compatibility",
                            "left": square_hv_left,
                            "right": square_hv_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("square", "square_hv_compatibility", &square_hv_left, &square_hv_right, "accepted", &Vec::new())
                        },
                        {
                            "id": "law_sq_interchange",
                            "kind": "square",
                            "law": "square_interchange",
                            "left": square_interchange_left,
                            "right": square_interchange_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("square", "square_interchange", &square_interchange_left, &square_interchange_right, "accepted", &Vec::new())
                        }
                    ]
                }
            }
        });
        let evaluated = evaluate_site_case_span_square_commutation(
            &case,
            Path::new("site-case-span-square-commutation-composition-accept.json"),
        )
        .expect("span/square commutation composition case should evaluate");
        assert_eq!(evaluated.result, "accepted");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn evaluate_site_case_span_square_commutation_rejects_missing_composition_law_coverage() {
        let span_identity_left =
            json!({"compose": {"left": {"span": "span_id"}, "right": {"span": "run_on_base"}}});
        let span_identity_right = json!({"span": "run_on_base"});
        let case = json!({
            "spanSquare": {
                "spans": [
                    {
                        "id": "span_id",
                        "kind": "identity",
                        "left": {"ctx": "Gamma"},
                        "apex": {"id": true},
                        "right": {"ctx": "Gamma"}
                    },
                    {
                        "id": "run_on_base",
                        "kind": "pipeline",
                        "left": {"ctx": "Gamma"},
                        "apex": {"run": "base"},
                        "right": {"out": "y"}
                    },
                    {
                        "id": "run_after_reindex",
                        "kind": "pipeline",
                        "left": {"ctx": "Gamma"},
                        "apex": {"run": "base"},
                        "right": {"out": "y"}
                    },
                    {
                        "id": "reindex_input",
                        "kind": "base_change",
                        "left": {"ctx": "Delta"},
                        "apex": {"map": "rho"},
                        "right": {"ctx": "Gamma"}
                    },
                    {
                        "id": "reindex_output",
                        "kind": "base_change",
                        "left": {"out": "y"},
                        "apex": {"map": "rho"},
                        "right": {"out": "y"}
                    }
                ],
                "squares": [
                    {
                        "id": "sq_accept",
                        "top": "run_on_base",
                        "bottom": "run_after_reindex",
                        "left": "reindex_input",
                        "right": "reindex_output",
                        "result": "accepted",
                        "failureClasses": [],
                        "digest": square_witness_digest("run_on_base", "run_after_reindex", "reindex_input", "reindex_output", "accepted", &Vec::new())
                    }
                ],
                "compositionLaws": {
                    "identitySpanIds": ["span_id"],
                    "identitySquareIds": [],
                    "laws": [
                        {
                            "id": "law_span_identity",
                            "kind": "span",
                            "law": "span_identity",
                            "left": span_identity_left,
                            "right": span_identity_right,
                            "result": "accepted",
                            "failureClasses": [],
                            "digest": composition_law_digest("span", "span_identity", &span_identity_left, &span_identity_right, "accepted", &Vec::new())
                        }
                    ]
                }
            }
        });
        let evaluated = evaluate_site_case_span_square_commutation(
            &case,
            Path::new("site-case-span-square-commutation-composition-missing-coverage.json"),
        )
        .expect("span/square commutation composition case should evaluate");
        assert_eq!(evaluated.result, "rejected");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.span_square_commutation.violation".to_string())
        );
    }

    #[test]
    fn check_site_obligation_requires_golden_polarity_vector() {
        let temp = TempDirGuard::new("site-obligation-missing-golden");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &["adversarial/only_vector"],
            &["adversarial/only_vector"],
        );
        write_site_vector(
            &fixture_root,
            "adversarial/only_vector",
            "span_square_commutation",
            "accepted",
        );
        let contract = test_contract_with_site_fixture_root("fixtures");

        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(
            evaluated
                .failure_classes
                .contains(&"coherence.span_square_commutation.missing_golden_vector".to_string())
        );
    }

    #[test]
    fn check_site_obligation_requires_adversarial_polarity_vector() {
        let temp = TempDirGuard::new("site-obligation-missing-adversarial");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &["golden/only_vector"],
            &["golden/only_vector"],
        );
        write_site_vector(
            &fixture_root,
            "golden/only_vector",
            "span_square_commutation",
            "accepted",
        );
        let contract = test_contract_with_site_fixture_root("fixtures");

        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(
            evaluated.failure_classes.contains(
                &"coherence.span_square_commutation.missing_adversarial_vector".to_string()
            )
        );
    }

    #[test]
    fn check_site_obligation_accepts_when_both_polarities_present() {
        let temp = TempDirGuard::new("site-obligation-both-polarities");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &["golden/ok_vector", "adversarial/ok_vector"],
            &["golden/ok_vector", "adversarial/ok_vector"],
        );
        write_site_vector(
            &fixture_root,
            "golden/ok_vector",
            "span_square_commutation",
            "accepted",
        );
        write_site_vector(
            &fixture_root,
            "adversarial/ok_vector",
            "span_square_commutation",
            "rejected",
        );
        let contract = test_contract_with_site_fixture_root("fixtures");

        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn check_site_obligation_requires_expected_accept_result_vector() {
        let temp = TempDirGuard::new("site-obligation-missing-expected-accept");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &["golden/reject_vector", "adversarial/reject_vector"],
            &["golden/reject_vector", "adversarial/reject_vector"],
        );
        write_site_vector(
            &fixture_root,
            "golden/reject_vector",
            "span_square_commutation",
            "rejected",
        );
        write_site_vector(
            &fixture_root,
            "adversarial/reject_vector",
            "span_square_commutation",
            "rejected",
        );
        let contract = test_contract_with_site_fixture_root("fixtures");

        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(evaluated.failure_classes.contains(
            &"coherence.span_square_commutation.missing_expected_accepted_vector".to_string()
        ));
    }

    #[test]
    fn check_site_obligation_requires_expected_reject_result_vector() {
        let temp = TempDirGuard::new("site-obligation-missing-expected-reject");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &["golden/accept_vector", "adversarial/accept_vector"],
            &["golden/accept_vector", "adversarial/accept_vector"],
        );
        write_site_vector(
            &fixture_root,
            "golden/accept_vector",
            "span_square_commutation",
            "accepted",
        );
        write_site_vector(
            &fixture_root,
            "adversarial/accept_vector",
            "span_square_commutation",
            "accepted",
        );
        let contract = test_contract_with_site_fixture_root("fixtures");

        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(evaluated.failure_classes.contains(
            &"coherence.span_square_commutation.missing_expected_rejected_vector".to_string()
        ));
    }

    #[test]
    fn check_site_obligation_ignores_unscoped_malformed_vectors() {
        let temp = TempDirGuard::new("site-obligation-scope-isolation");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &[
                "golden/ok_vector",
                "adversarial/ok_vector",
                "golden/unscoped_bad_vector",
            ],
            &["golden/ok_vector", "adversarial/ok_vector"],
        );
        write_site_vector(
            &fixture_root,
            "golden/ok_vector",
            "span_square_commutation",
            "accepted",
        );
        write_site_vector(
            &fixture_root,
            "adversarial/ok_vector",
            "span_square_commutation",
            "rejected",
        );
        let bad_vector_root = fixture_root.join("golden/unscoped_bad_vector");
        fs::create_dir_all(&bad_vector_root).expect("bad vector root should be creatable");
        fs::write(bad_vector_root.join("case.json"), b"{not-json")
            .expect("bad vector case should be writable");
        fs::write(bad_vector_root.join("expect.json"), b"{not-json")
            .expect("bad vector expect should be writable");

        let contract = test_contract_with_site_fixture_root("fixtures");
        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn check_site_obligation_requires_invariance_pair_count() {
        let temp = TempDirGuard::new("site-obligation-invariance-pair-count");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &[
                "golden/ok_vector",
                "adversarial/reject_vector",
                "invariance/only_local_accept",
            ],
            &[
                "golden/ok_vector",
                "adversarial/reject_vector",
                "invariance/only_local_accept",
            ],
        );
        write_site_vector(
            &fixture_root,
            "golden/ok_vector",
            "span_square_commutation",
            "accepted",
        );
        write_site_vector(
            &fixture_root,
            "adversarial/reject_vector",
            "span_square_commutation",
            "rejected",
        );
        write_site_vector_with_metadata(
            &fixture_root,
            "invariance/only_local_accept",
            "span_square_commutation",
            "accepted",
            Some("span_square_equiv"),
            Some("local"),
        );

        let contract = test_contract_with_site_fixture_root("fixtures");
        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(evaluated.failure_classes.contains(
            &"coherence.span_square_commutation.invariance_pair_count_mismatch".to_string()
        ));
    }

    #[test]
    fn check_site_obligation_requires_invariance_pair_result_match() {
        let temp = TempDirGuard::new("site-obligation-invariance-result-mismatch");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &[
                "golden/ok_vector",
                "adversarial/reject_vector",
                "invariance/local_accept",
                "invariance/external_reject",
            ],
            &[
                "golden/ok_vector",
                "adversarial/reject_vector",
                "invariance/local_accept",
                "invariance/external_reject",
            ],
        );
        write_site_vector(
            &fixture_root,
            "golden/ok_vector",
            "span_square_commutation",
            "accepted",
        );
        write_site_vector(
            &fixture_root,
            "adversarial/reject_vector",
            "span_square_commutation",
            "rejected",
        );
        write_site_vector_with_metadata(
            &fixture_root,
            "invariance/local_accept",
            "span_square_commutation",
            "accepted",
            Some("span_square_equiv"),
            Some("local"),
        );
        write_site_vector_with_metadata(
            &fixture_root,
            "invariance/external_reject",
            "span_square_commutation",
            "rejected",
            Some("span_square_equiv"),
            Some("external"),
        );

        let contract = test_contract_with_site_fixture_root("fixtures");
        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(
            evaluated.failure_classes.contains(
                &"coherence.span_square_commutation.invariance_result_mismatch".to_string()
            )
        );
    }

    #[test]
    fn check_site_obligation_accepts_with_invariance_pair() {
        let temp = TempDirGuard::new("site-obligation-invariance-pair-pass");
        let fixture_root = temp.path().join("fixtures");
        write_site_manifest(
            &fixture_root,
            &[
                "golden/ok_vector",
                "adversarial/reject_vector",
                "invariance/local_accept",
                "invariance/external_accept",
            ],
            &[
                "golden/ok_vector",
                "adversarial/reject_vector",
                "invariance/local_accept",
                "invariance/external_accept",
            ],
        );
        write_site_vector(
            &fixture_root,
            "golden/ok_vector",
            "span_square_commutation",
            "accepted",
        );
        write_site_vector(
            &fixture_root,
            "adversarial/reject_vector",
            "span_square_commutation",
            "rejected",
        );
        write_site_vector_with_metadata(
            &fixture_root,
            "invariance/local_accept",
            "span_square_commutation",
            "accepted",
            Some("span_square_equiv"),
            Some("local"),
        );
        write_site_vector_with_metadata(
            &fixture_root,
            "invariance/external_accept",
            "span_square_commutation",
            "accepted",
            Some("span_square_equiv"),
            Some("external"),
        );

        let contract = test_contract_with_site_fixture_root("fixtures");
        let evaluated = check_site_obligation(
            temp.path(),
            &contract,
            "span_square_commutation",
            evaluate_site_case_span_square_commutation,
        )
        .expect("site obligation should evaluate");
        assert!(evaluated.failure_classes.is_empty());
    }

    #[test]
    fn validate_required_obligation_parity_reports_missing_and_unknown() {
        let declared: BTreeSet<String> = ["scope_noncontradiction", "unknown_obligation"]
            .iter()
            .map(|value| (*value).to_string())
            .collect();
        let required: BTreeSet<String> = ["scope_noncontradiction", "capability_parity"]
            .iter()
            .map(|value| (*value).to_string())
            .collect();

        let failures = validate_required_obligation_parity(&declared, &required);

        assert!(failures.contains(
            &"coherence.scope_noncontradiction.coherence_spec_missing_obligation".to_string()
        ));
        assert!(failures.contains(
            &"coherence.scope_noncontradiction.coherence_spec_unknown_obligation".to_string()
        ));
    }
}
