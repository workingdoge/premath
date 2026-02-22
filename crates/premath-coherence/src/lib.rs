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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityRegistry {
    schema: u32,
    registry_kind: String,
    executable_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneProjectionContract {
    schema: u32,
    contract_kind: String,
    required_gate_projection: RequiredGateProjection,
    required_witness: ControlPlaneRequiredWitness,
    instruction_witness: ControlPlaneInstructionWitness,
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
            "requiredBidirObligations": contract.required_bidir_obligations,
            "bidirSpecObligations": bidir_spec_obligations,
            "bidirCheckerObligations": bidir_checker_obligations,
            "requiredCoherenceObligations": required_coherence_obligations,
            "coherenceSpecObligations": coherence_spec_obligations,
            "obligationRegistryKind": obligation_registry_kind,
        }),
    })
}

fn check_capability_parity(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
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
    if control_plane_contract.contract_kind != "premath.control_plane.contract.v1" {
        return Err(CoherenceError::Contract(format!(
            "control-plane contract kind mismatch at {}: {:?}",
            display_path(&control_plane_contract_path),
            control_plane_contract.contract_kind
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
    let mut matched_golden_count = 0usize;
    let mut matched_adversarial_count = 0usize;
    let mut matched_invariance_count = 0usize;
    let mut matched_expected_accepted_count = 0usize;
    let mut matched_expected_rejected_count = 0usize;

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
        if vector_id.starts_with("golden/") {
            matched_golden_count += 1;
        } else if vector_id.starts_with("adversarial/") {
            matched_adversarial_count += 1;
        } else if vector_id.starts_with("invariance/") {
            matched_invariance_count += 1;
        }
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
        } else if expected_result == "accepted" {
            matched_expected_accepted_count += 1;
        } else {
            matched_expected_rejected_count += 1;
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
    if matched_golden_count == 0 {
        failures.push("coherence.transport_functoriality.missing_golden_vector".to_string());
    }
    if matched_adversarial_count == 0 {
        failures.push("coherence.transport_functoriality.missing_adversarial_vector".to_string());
    }
    if matched_expected_accepted_count == 0 {
        failures
            .push("coherence.transport_functoriality.missing_expected_accepted_vector".to_string());
    }
    if matched_expected_rejected_count == 0 {
        failures
            .push("coherence.transport_functoriality.missing_expected_rejected_vector".to_string());
    }

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "fixtureRoot": to_repo_relative_or_absolute(repo_root, &fixture_root),
            "manifestVectors": manifest.vectors,
            "matchedVectorKinds": {
                "golden": matched_golden_count,
                "adversarial": matched_adversarial_count,
                "invariance": matched_invariance_count,
            },
            "matchedExpectedResults": {
                "accepted": matched_expected_accepted_count,
                "rejected": matched_expected_rejected_count,
            },
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
    let mut matched_golden_count = 0usize;
    let mut matched_adversarial_count = 0usize;
    let mut matched_invariance_count = 0usize;
    let mut matched_expected_accepted_count = 0usize;
    let mut matched_expected_rejected_count = 0usize;
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
        if vector_id.starts_with("golden/") {
            matched_golden_count += 1;
        } else if vector_id.starts_with("adversarial/") {
            matched_adversarial_count += 1;
        } else if vector_id.starts_with("invariance/") {
            matched_invariance_count += 1;
        }

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
        } else if expected_result == "accepted" {
            matched_expected_accepted_count += 1;
        } else {
            matched_expected_rejected_count += 1;
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
    } else {
        if matched_golden_count == 0 {
            failures.push(format!("coherence.{obligation_id}.missing_golden_vector"));
        }
        if matched_adversarial_count == 0 {
            failures.push(format!(
                "coherence.{obligation_id}.missing_adversarial_vector"
            ));
        }
        if matched_expected_accepted_count == 0 {
            failures.push(format!(
                "coherence.{obligation_id}.missing_expected_accepted_vector"
            ));
        }
        if matched_expected_rejected_count == 0 {
            failures.push(format!(
                "coherence.{obligation_id}.missing_expected_rejected_vector"
            ));
        }
    }

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "fixtureRoot": to_repo_relative_or_absolute(repo_root, &fixture_root),
            "manifestVectors": manifest.vectors,
            "manifestObligationVectors": manifest.obligation_vectors,
            "scopedVectors": scoped_vectors,
            "matchedVectors": matched_count,
            "matchedVectorKinds": {
                "golden": matched_golden_count,
                "adversarial": matched_adversarial_count,
                "invariance": matched_invariance_count,
            },
            "matchedExpectedResults": {
                "accepted": matched_expected_accepted_count,
                "rejected": matched_expected_rejected_count,
            },
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
            required_bidir_obligations: Vec::new(),
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
