//! Typed coherence-contract checker surface.
//!
//! This crate evaluates a machine contract artifact against repository surfaces
//! and emits deterministic witnesses.

mod instruction;
mod proposal;

pub use instruction::{
    InstructionError, InstructionTypingPolicy, ValidatedInstructionEnvelope,
    ValidatedInstructionProposal, validate_instruction_envelope_payload,
};
pub use proposal::{
    CanonicalProposal, ProposalBinding, ProposalDischarge, ProposalError, ProposalObligation,
    ProposalStep, ProposalTargetJudgment, ValidatedProposal, compile_proposal_obligations,
    compute_proposal_digest, compute_proposal_kcir_ref, discharge_proposal_obligations,
    validate_proposal_payload,
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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
];

const CAP_ASSIGN_PATTERN: &str =
    r#"(?m)^(CAPABILITY_[A-Z0-9_]+)\s*=\s*"(capabilities\.[a-z0-9_]+)"$"#;
const CHECK_ASSIGN_PATTERN: &str = r#"(?m)^(CHECK_[A-Z0-9_]+)\s*=\s*"([a-z0-9-]+)"$"#;

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
    pub capability_source_path: String,
    pub capability_tuple: String,
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
    pub projection_source_path: String,
    pub projection_tuple: String,
    pub doctrine_site_path: String,
    pub doctrine_root_node_id: String,
    pub profile_readme_path: String,
    pub bidir_spec_path: String,
    pub bidir_spec_section_start: String,
    pub bidir_spec_section_end: String,
    pub bidir_checker_path: String,
    pub bidir_checker_map_name: String,
    pub informative_clause_needle: String,
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

#[derive(Debug)]
struct ObligationCheck {
    failure_classes: Vec<String>,
    details: Value,
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
    let bidir_checker_path = resolve_path(repo_root, contract.surfaces.bidir_checker_path.as_str());
    let bidir_spec_text = read_text(&bidir_spec_path)?;
    let bidir_spec_section = extract_section_between(
        &bidir_spec_text,
        contract.surfaces.bidir_spec_section_start.as_str(),
        contract.surfaces.bidir_spec_section_end.as_str(),
    )?;
    let bidir_spec_obligations = parse_backtick_obligation_tokens(bidir_spec_section)?;
    let bidir_checker_text = read_text(&bidir_checker_path)?;
    let bidir_checker_obligations = parse_checker_obligation_map_keys(
        &bidir_checker_text,
        contract.surfaces.bidir_checker_map_name.as_str(),
    )?;

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

    Ok(ObligationCheck {
        failure_classes: dedupe_sorted(failures),
        details: json!({
            "conditionalCapabilityDocs": contract.conditional_capability_docs,
            "specIndexCapabilityDocMap": spec_index_doc_map,
            "requiredBidirObligations": contract.required_bidir_obligations,
            "bidirSpecObligations": bidir_spec_obligations,
            "bidirCheckerObligations": bidir_checker_obligations,
        }),
    })
}

fn check_capability_parity(
    repo_root: &Path,
    contract: &CoherenceContract,
) -> Result<ObligationCheck, CoherenceError> {
    let capability_source_path =
        resolve_path(repo_root, contract.surfaces.capability_source_path.as_str());
    let capability_source_text = read_text(&capability_source_path)?;
    let executable_capabilities = parse_symbol_tuple_values(
        &capability_source_text,
        CAP_ASSIGN_PATTERN,
        contract.surfaces.capability_tuple.as_str(),
    )?;
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

    let projection_source_text = read_text(&resolve_path(
        repo_root,
        contract.surfaces.projection_source_path.as_str(),
    ))?;
    let projection_checks = parse_symbol_tuple_values(
        &projection_source_text,
        CHECK_ASSIGN_PATTERN,
        contract.surfaces.projection_tuple.as_str(),
    )?;
    let projection_set: BTreeSet<String> = projection_checks.iter().cloned().collect();

    let ci_projection_section = extract_section_between(
        &ci_closure_text,
        contract.surfaces.ci_closure_projection_start.as_str(),
        contract.surfaces.ci_closure_projection_end.as_str(),
    )?;
    let ci_projection_set = parse_backticked_tasks(ci_projection_section)?;

    let mut failures = Vec::new();
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
            "projectionFromSource": projection_checks,
            "projectionFromCiClosure": sorted_vec_from_set(&ci_projection_set),
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

fn parse_checker_obligation_map_keys(
    text: &str,
    map_name: &str,
) -> Result<BTreeSet<String>, CoherenceError> {
    let py_map_re = compile_regex(&format!(
        r#"(?s){}\s*=\s*\{{(.*?)\}}"#,
        regex::escape(map_name)
    ))?;
    if let Some(body) = py_map_re
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    {
        let key_re = compile_regex(r#""([a-z_]+)"\s*:"#)?;
        return Ok(key_re
            .captures_iter(body.as_str())
            .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .collect());
    }

    let rs_tuple_re = compile_regex(&format!(
        r#"(?s){}\s*:[^=]*=\s*&\s*\[(.*?)\]"#,
        regex::escape(map_name)
    ))?;
    if let Some(body) = rs_tuple_re
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    {
        let key_re = compile_regex(r#"\(\s*"([a-z_]+)"\s*,"#)?;
        return Ok(key_re
            .captures_iter(body.as_str())
            .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .collect());
    }

    Err(CoherenceError::Contract(format!(
        "missing map assignment: {map_name}"
    )))
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

fn parse_symbol_tuple_values(
    text: &str,
    assign_pattern: &str,
    tuple_name: &str,
) -> Result<Vec<String>, CoherenceError> {
    let assign_re = compile_regex(assign_pattern)?;
    let mut symbol_map: BTreeMap<String, String> = BTreeMap::new();
    for captures in assign_re.captures_iter(text) {
        let symbol = captures.get(1).map(|m| m.as_str()).ok_or_else(|| {
            CoherenceError::Contract(format!(
                "malformed assignment capture for tuple {tuple_name}"
            ))
        })?;
        let value = captures.get(2).map(|m| m.as_str()).ok_or_else(|| {
            CoherenceError::Contract(format!(
                "malformed assignment capture for tuple {tuple_name}"
            ))
        })?;
        symbol_map.insert(symbol.to_string(), value.to_string());
    }
    if symbol_map.is_empty() {
        return Err(CoherenceError::Contract(format!(
            "no assignment symbols found for tuple {tuple_name}"
        )));
    }

    let tuple_re = compile_regex(&format!(
        r#"(?s){}\s*[^\n]*=\s*\((.*?)\)"#,
        regex::escape(tuple_name)
    ))?;
    let tuple_body = tuple_re
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .ok_or_else(|| {
            CoherenceError::Contract(format!("missing tuple definition: {tuple_name}"))
        })?;

    let symbol_re = compile_regex(r"\b([A-Z][A-Z0-9_]+)\b")?;
    let mut ordered_symbols = Vec::new();
    for captures in symbol_re.captures_iter(tuple_body.as_str()) {
        let symbol = captures.get(1).map(|m| m.as_str()).ok_or_else(|| {
            CoherenceError::Contract(format!("malformed tuple symbol in {tuple_name}"))
        })?;
        if symbol_map.contains_key(symbol) && !ordered_symbols.iter().any(|s: &String| s == symbol)
        {
            ordered_symbols.push(symbol.to_string());
        }
    }
    if ordered_symbols.is_empty() {
        return Err(CoherenceError::Contract(format!(
            "tuple {tuple_name} does not reference known symbols"
        )));
    }

    let mut out = Vec::new();
    for symbol in ordered_symbols {
        let value = symbol_map.get(&symbol).ok_or_else(|| {
            CoherenceError::Contract(format!(
                "tuple {tuple_name} references unknown symbol: {symbol}"
            ))
        })?;
        out.push(value.clone());
    }
    Ok(out)
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

fn sorted_vec_from_set(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
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

    #[test]
    fn parse_symbol_tuple_values_extracts_ordered_values() {
        let text = r#"
CAPABILITY_A = "capabilities.alpha"
CAPABILITY_B = "capabilities.beta"
DEFAULT_EXECUTABLE_CAPABILITIES = (
    CAPABILITY_A,
    CAPABILITY_B,
)
"#;
        let values =
            parse_symbol_tuple_values(text, CAP_ASSIGN_PATTERN, "DEFAULT_EXECUTABLE_CAPABILITIES")
                .expect("tuple parse should succeed");
        assert_eq!(
            values,
            vec![
                "capabilities.alpha".to_string(),
                "capabilities.beta".to_string()
            ]
        );
    }

    #[test]
    fn extract_section_between_returns_body() {
        let text = "prefix START body END suffix";
        let section =
            extract_section_between(text, "START", "END").expect("section extraction should work");
        assert_eq!(section.trim(), "body");
    }

    #[test]
    fn parse_checker_obligation_map_keys_extracts_key_set() {
        let text = r#"
OBLIGATION_TO_GATE_FAILURE = {
    "stability": "stability_failure",
    "locality": "locality_failure",
}
"#;
        let keys = parse_checker_obligation_map_keys(text, "OBLIGATION_TO_GATE_FAILURE")
            .expect("map extraction should succeed");
        assert!(keys.contains("stability"));
        assert!(keys.contains("locality"));
    }

    #[test]
    fn parse_checker_obligation_map_keys_extracts_rust_tuple_set() {
        let text = r#"
const OBLIGATION_TO_GATE_FAILURE: &[(&str, &str)] = &[
    ("stability", "stability_failure"),
    ("locality", "locality_failure"),
];
"#;
        let keys = parse_checker_obligation_map_keys(text, "OBLIGATION_TO_GATE_FAILURE")
            .expect("map extraction should succeed");
        assert!(keys.contains("stability"));
        assert!(keys.contains("locality"));
    }
}
