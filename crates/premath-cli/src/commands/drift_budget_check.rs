use premath_coherence::run_coherence_check;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA: u32 = 1;
const CHECK_KIND: &str = "ci.drift_budget.v1";

const DRIFT_CLASS_SPEC_INDEX: &str = "spec_index_capability_map_drift";
const DRIFT_CLASS_PROFILE_OVERLAYS: &str = "profile_overlay_claim_drift";
const DRIFT_CLASS_LANE_BINDINGS: &str = "control_plane_lane_binding_drift";
const DRIFT_CLASS_KCIR_MAPPINGS: &str = "control_plane_kcir_mapping_drift";
const DRIFT_CLASS_RUNTIME_ROUTE_BINDINGS: &str = "runtime_route_binding_drift";
const DRIFT_CLASS_REQUIRED_OBLIGATIONS: &str = "coherence_required_obligation_drift";
const DRIFT_CLASS_SIGPI_NOTATION: &str = "sigpi_notation_drift";
const DRIFT_CLASS_CACHE_CLOSURE: &str = "coherence_cache_input_closure_drift";
const DRIFT_CLASS_TOPOLOGY_BUDGET: &str = "topology_budget_drift";
const WARN_CLASS_TOPOLOGY_BUDGET: &str = "topology_budget_watch";

const TOPOLOGY_BUDGET_SCHEMA: i64 = 1;
const TOPOLOGY_BUDGET_KIND: &str = "premath.topology_budget.v1";

const SIGPI_NORMATIVE_DOCS: [&str; 3] = [
    "specs/premath/draft/SPEC-INDEX.md",
    "specs/premath/draft/UNIFICATION-DOCTRINE.md",
    "specs/premath/profile/ADJOINTS-AND-SITES.md",
];

const CACHE_CLOSURE_REQUIRED_PATHS: [&str; 5] = [
    "specs/premath/draft/COHERENCE-CONTRACT.json",
    "specs/premath/draft/CONTROL-PLANE-CONTRACT.json",
    "tools/ci/control_plane_contract.py",
    "crates/premath-coherence/src",
    "crates/premath-cli/src/commands/coherence_check.rs",
];

const REQUIRED_KCIR_MAPPING_ROWS: [&str; 2] = ["instructionEnvelope", "requiredDecisionInput"];
const REQUIRED_KCIR_MAPPING_ROW_FIELDS: [&str; 3] = ["sourceKind", "targetDomain", "targetKind"];

#[derive(Clone, Debug)]
struct CapabilityRegistryContract {
    executable_capabilities: Vec<String>,
    profile_overlay_claims: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct DoctrineOperationRow {
    path: String,
    morphisms: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct TopologyThreshold {
    warn_above: Option<i64>,
    fail_above: Option<i64>,
    warn_below: Option<i64>,
    fail_below: Option<i64>,
}

#[derive(Clone, Debug, Default)]
struct TopologyBudgetContract {
    metrics: BTreeMap<String, TopologyThreshold>,
    deprecated_design_fragments: Vec<String>,
    doctrine_site_authority_inputs: Vec<String>,
    doctrine_site_generated_views: Vec<String>,
}

fn resolve_repo_root(input: &str) -> PathBuf {
    let path = PathBuf::from(input.trim());
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn resolve_rel_path(root: &Path, input: &str) -> PathBuf {
    let path = PathBuf::from(input.trim());
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn load_json_object(path: &Path) -> Result<Map<String, Value>, String> {
    let raw =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value = serde_json::from_slice::<Value>(&raw)
        .map_err(|error| format!("invalid JSON at {}: {error}", path.display()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{}: root must be an object", path.display()))
}

fn as_sorted_strings(value: &Value) -> Vec<String> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    let mut out = items
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn normalize_lane_artifact_kinds(value: &Value) -> BTreeMap<String, Vec<String>> {
    let Some(obj) = value.as_object() else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for (lane_id, raw) in obj {
        let key = lane_id.trim();
        if key.is_empty() {
            continue;
        }
        out.insert(key.to_string(), as_sorted_strings(raw));
    }
    out
}

fn normalize_runtime_route_bindings(value: &Value) -> BTreeMap<String, Map<String, Value>> {
    let Some(obj) = value.as_object() else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for (route_id, raw) in obj {
        let route_id = route_id.trim();
        if route_id.is_empty() {
            continue;
        }
        let Some(route_obj) = raw.as_object() else {
            continue;
        };
        let operation_id = route_obj
            .get("operationId")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if operation_id.is_empty() {
            continue;
        }
        let mut row = Map::new();
        row.insert(
            "operationId".to_string(),
            Value::String(operation_id.to_string()),
        );
        row.insert(
            "requiredMorphisms".to_string(),
            Value::Array(
                as_sorted_strings(route_obj.get("requiredMorphisms").unwrap_or(&Value::Null))
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
        out.insert(route_id.to_string(), row);
    }
    out
}

fn normalize_kcir_mapping_row(value: &Value) -> Map<String, Value> {
    let Some(obj) = value.as_object() else {
        return Map::new();
    };
    let mut row = Map::new();
    row.insert(
        "sourceKind".to_string(),
        Value::String(
            obj.get("sourceKind")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .to_string(),
        ),
    );
    row.insert(
        "targetDomain".to_string(),
        Value::String(
            obj.get("targetDomain")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .to_string(),
        ),
    );
    row.insert(
        "targetKind".to_string(),
        Value::String(
            obj.get("targetKind")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .to_string(),
        ),
    );
    row.insert(
        "identityFields".to_string(),
        Value::Array(
            as_sorted_strings(obj.get("identityFields").unwrap_or(&Value::Null))
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    row
}

fn normalize_kcir_mapping_table(value: &Value) -> BTreeMap<String, Map<String, Value>> {
    let Some(obj) = value.as_object() else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for (row_id, raw) in obj {
        let row_id = row_id.trim();
        if row_id.is_empty() {
            continue;
        }
        out.insert(row_id.to_string(), normalize_kcir_mapping_row(raw));
    }
    out
}

fn normalize_kcir_legacy_policy(value: &Value) -> Map<String, Value> {
    let Some(obj) = value.as_object() else {
        return Map::new();
    };
    let mut out = Map::new();
    for field in ["mode", "authorityMode", "supportUntilEpoch", "failureClass"] {
        out.insert(
            field.to_string(),
            Value::String(
                obj.get(field)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("")
                    .to_string(),
            ),
        );
    }
    out
}

fn extract_heading_section(text: &str, heading_prefix: &str) -> Result<String, String> {
    let heading = format!("### {heading_prefix}");
    let lines = text.lines().collect::<Vec<_>>();
    let mut start: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with(&heading) {
            start = Some(idx + 1);
            break;
        }
    }
    let Some(start_idx) = start else {
        return Err(format!("missing heading: {heading_prefix:?}"));
    };
    let mut out = Vec::new();
    for line in lines.iter().skip(start_idx) {
        if line.trim_start().starts_with("### ") {
            break;
        }
        out.push(*line);
    }
    Ok(out.join("\n"))
}

fn parse_spec_index_capability_doc_map(
    spec_index_path: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(spec_index_path)
        .map_err(|error| format!("failed to read {}: {error}", spec_index_path.display()))?;
    let section = extract_heading_section(&text, "5.4")?;
    let map_re = Regex::new(r"- `([^`]+)`\s+\(for `([^`]+)`\)")
        .map_err(|error| format!("failed to compile SPEC-INDEX map regex: {error}"))?;
    let mut out = BTreeMap::new();
    for capture in map_re.captures_iter(&section) {
        let doc_ref = capture
            .get(1)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let capability_id = capture
            .get(2)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        if !doc_ref.is_empty() && !capability_id.is_empty() {
            out.insert(doc_ref, capability_id);
        }
    }
    if out.is_empty() {
        return Err(format!(
            "{}: §5.4 capability doc map is empty",
            spec_index_path.display()
        ));
    }
    Ok(out)
}

fn parse_capability_registry(path: &Path) -> Result<CapabilityRegistryContract, String> {
    let payload = load_json_object(path)?;

    let executable_capabilities_raw = payload
        .get("executableCapabilities")
        .ok_or_else(|| format!("{}: missing executableCapabilities", path.display()))?;
    let executable_capabilities = as_sorted_strings(executable_capabilities_raw);
    if executable_capabilities.is_empty() {
        return Err(format!(
            "{}: executableCapabilities must be a non-empty list",
            path.display()
        ));
    }

    let profile_overlay_claims = as_sorted_strings(
        payload
            .get("profileOverlayClaims")
            .unwrap_or(&Value::Array(vec![])),
    );

    Ok(CapabilityRegistryContract {
        executable_capabilities,
        profile_overlay_claims,
    })
}

fn parse_conformance_profile_overlay_claims(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let section = extract_heading_section(&text, "2.4")?;
    let claim_re = Regex::new(r"`(profile\.[a-z0-9_.]+)`")
        .map_err(|error| format!("failed to compile CONFORMANCE profile-claim regex: {error}"))?;
    let mut out = claim_re
        .captures_iter(&section)
        .filter_map(|capture| capture.get(1).map(|m| m.as_str().trim().to_string()))
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    Ok(out)
}

fn parse_conditional_capability_docs(
    coherence_contract: &Map<String, Value>,
) -> Result<BTreeMap<String, String>, String> {
    let docs = coherence_contract
        .get("conditionalCapabilityDocs")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "coherence contract conditionalCapabilityDocs must be a non-empty list".to_string()
        })?;
    if docs.is_empty() {
        return Err(
            "coherence contract conditionalCapabilityDocs must be a non-empty list".to_string(),
        );
    }
    let mut out = BTreeMap::new();
    for (idx, row) in docs.iter().enumerate() {
        let Some(obj) = row.as_object() else {
            return Err(format!(
                "conditionalCapabilityDocs[{idx}] must be an object"
            ));
        };
        let doc_ref = obj
            .get("docRef")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let capability_id = obj
            .get("capabilityId")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if doc_ref.is_empty() {
            return Err(format!(
                "conditionalCapabilityDocs[{idx}].docRef must be non-empty"
            ));
        }
        if capability_id.is_empty() {
            return Err(format!(
                "conditionalCapabilityDocs[{idx}].capabilityId must be non-empty"
            ));
        }
        out.insert(doc_ref.to_string(), capability_id.to_string());
    }
    Ok(out)
}

fn parse_required_obligation_ids(
    coherence_contract: &Map<String, Value>,
) -> Result<Vec<String>, String> {
    let obligations = coherence_contract
        .get("obligations")
        .and_then(Value::as_array)
        .ok_or_else(|| "coherence contract obligations must be a non-empty list".to_string())?;
    if obligations.is_empty() {
        return Err("coherence contract obligations must be a non-empty list".to_string());
    }
    let mut out = Vec::new();
    for (idx, row) in obligations.iter().enumerate() {
        let Some(obj) = row.as_object() else {
            return Err(format!("obligations[{idx}] must be an object"));
        };
        let obligation_id = obj
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if obligation_id.is_empty() {
            return Err(format!("obligations[{idx}].id must be a non-empty string"));
        }
        out.push(obligation_id.to_string());
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn parse_required_bidir_obligations(
    coherence_contract: &Map<String, Value>,
) -> Result<Vec<String>, String> {
    let values = coherence_contract
        .get("requiredBidirObligations")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "coherence contract requiredBidirObligations must be non-empty".to_string()
        })?;
    if values.is_empty() {
        return Err("coherence contract requiredBidirObligations must be non-empty".to_string());
    }
    let mut out = Vec::new();
    for (idx, value) in values.iter().enumerate() {
        let obligation = value.as_str().map(str::trim).unwrap_or("");
        if obligation.is_empty() {
            return Err(format!(
                "requiredBidirObligations[{idx}] must be a non-empty string"
            ));
        }
        out.push(obligation.to_string());
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn parse_doctrine_operation_registry(
    path: &Path,
) -> Result<BTreeMap<String, DoctrineOperationRow>, String> {
    let payload = load_json_object(path)?;
    let operations = payload
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{}: operations must be a non-empty list", path.display()))?;
    if operations.is_empty() {
        return Err(format!(
            "{}: operations must be a non-empty list",
            path.display()
        ));
    }
    let mut out = BTreeMap::new();
    for (idx, row) in operations.iter().enumerate() {
        let Some(obj) = row.as_object() else {
            return Err(format!(
                "{}: operations[{idx}] must be an object",
                path.display()
            ));
        };
        let operation_id = obj
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if operation_id.is_empty() {
            return Err(format!(
                "{}: operations[{idx}].id must be non-empty",
                path.display()
            ));
        }
        if out.contains_key(operation_id) {
            return Err(format!(
                "{}: duplicate operation id {operation_id:?}",
                path.display()
            ));
        }
        let path_value = obj
            .get("path")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let morphisms = as_sorted_strings(obj.get("morphisms").unwrap_or(&Value::Null));
        out.insert(
            operation_id.to_string(),
            DoctrineOperationRow {
                path: path_value,
                morphisms,
            },
        );
    }
    Ok(out)
}

fn obligation_details(
    coherence_witness: &Map<String, Value>,
    obligation_id: &str,
) -> Result<Map<String, Value>, String> {
    let obligations = coherence_witness
        .get("obligations")
        .and_then(Value::as_array)
        .ok_or_else(|| "coherence witness obligations must be a list".to_string())?;
    for row in obligations {
        let Some(obj) = row.as_object() else {
            continue;
        };
        if obj
            .get("obligationId")
            .and_then(Value::as_str)
            .is_some_and(|value| value == obligation_id)
        {
            let details = obj
                .get("details")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "coherence witness obligation {obligation_id} details must be an object"
                    )
                })?;
            return Ok(details);
        }
    }
    Err(format!(
        "coherence witness missing obligation details for {obligation_id:?}"
    ))
}

fn check_spec_index_capability_map(
    spec_map: &BTreeMap<String, String>,
    executable_capabilities: &[String],
    conditional_docs_map: &BTreeMap<String, String>,
) -> (bool, Value) {
    let mut reasons = Vec::new();
    let executable_set = executable_capabilities
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let unknown_caps = spec_map
        .values()
        .filter(|capability| !executable_set.contains(*capability))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if !unknown_caps.is_empty() {
        reasons.push(
            "spec-index references capabilities not present in CAPABILITY-REGISTRY".to_string(),
        );
    }

    let mut conditional_mismatches = Vec::new();
    let mut missing_conditional_docs = Vec::new();
    for (doc_ref, capability_id) in conditional_docs_map {
        let Some(mapped) = spec_map.get(doc_ref) else {
            missing_conditional_docs.push(doc_ref.clone());
            continue;
        };
        if mapped != capability_id {
            conditional_mismatches.push(json!({
                "docRef": doc_ref,
                "expected": capability_id,
                "actual": mapped
            }));
        }
    }
    if !missing_conditional_docs.is_empty() || !conditional_mismatches.is_empty() {
        reasons.push(
            "SPEC-INDEX §5.4 conditional capability docs diverge from COHERENCE-CONTRACT"
                .to_string(),
        );
    }

    let details = json!({
        "reasons": reasons,
        "specIndexCapabilityDocMap": spec_map,
        "conditionalCapabilityDocs": conditional_docs_map,
        "unknownCapabilities": unknown_caps,
        "missingConditionalDocs": missing_conditional_docs,
        "conditionalMismatches": conditional_mismatches
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    (failed, details)
}

fn check_profile_overlay_claims(
    registry_overlay_claims: &[String],
    conformance_overlay_claims: &[String],
) -> (bool, Value) {
    let mut reasons = Vec::new();
    let registry_set = registry_overlay_claims
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let conformance_set = conformance_overlay_claims
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing_in_conformance = registry_set
        .difference(&conformance_set)
        .cloned()
        .collect::<Vec<_>>();
    let missing_in_registry = conformance_set
        .difference(&registry_set)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_in_conformance.is_empty() || !missing_in_registry.is_empty() {
        reasons.push(
            "CONFORMANCE §2.4 profile-overlay claims diverge from CAPABILITY-REGISTRY".to_string(),
        );
    }

    let details = json!({
        "reasons": reasons,
        "registryProfileOverlayClaims": registry_set.into_iter().collect::<Vec<_>>(),
        "conformanceProfileOverlayClaims": conformance_set.into_iter().collect::<Vec<_>>(),
        "missingInConformance": missing_in_conformance,
        "missingInRegistry": missing_in_registry
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    (failed, details)
}

fn check_control_plane_lane_bindings(
    control_plane_contract: &Map<String, Value>,
    gate_chain_details: &Map<String, Value>,
) -> (bool, Value) {
    let mut reasons = Vec::new();

    let lane_registry = gate_chain_details
        .get("laneRegistry")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| {
            reasons.push(
                "coherence witness missing gate_chain_parity laneRegistry details".to_string(),
            );
            Map::new()
        });

    let contract_evidence_lanes = control_plane_contract
        .get("evidenceLanes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let contract_lane_artifact_kinds = normalize_lane_artifact_kinds(
        control_plane_contract
            .get("laneArtifactKinds")
            .unwrap_or(&Value::Null),
    );
    let contract_lane_ownership = control_plane_contract
        .get("laneOwnership")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let contract_checker_core = as_sorted_strings(
        contract_lane_ownership
            .get("checkerCoreOnlyObligations")
            .unwrap_or(&Value::Null),
    );
    let contract_required_route = match contract_lane_ownership.get("requiredCrossLaneWitnessRoute")
    {
        Some(Value::String(value)) => value.trim().to_string(),
        Some(Value::Object(obj)) => obj
            .get("pullbackBaseChange")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    };
    let contract_lane_failure_classes = as_sorted_strings(
        control_plane_contract
            .get("laneFailureClasses")
            .unwrap_or(&Value::Null),
    );

    let contract_stage1_parity_failure_classes = control_plane_contract
        .get("evidenceStage1Parity")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("failureClasses"))
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();

    let contract_stage1_rollback_trigger_classes = as_sorted_strings(
        control_plane_contract
            .get("evidenceStage1Rollback")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("triggerFailureClasses"))
            .unwrap_or(&Value::Null),
    );
    let contract_stage1_rollback_failure_classes = control_plane_contract
        .get("evidenceStage1Rollback")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("failureClasses"))
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();

    let contract_stage2_failure_classes = control_plane_contract
        .get("evidenceStage2Authority")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("failureClasses"))
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();
    let contract_stage2_bidir_required_obligations = as_sorted_strings(
        control_plane_contract
            .get("evidenceStage2Authority")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("bidirEvidenceRoute"))
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("requiredObligations"))
            .unwrap_or(&Value::Null),
    );
    let contract_stage2_bidir_failure_classes = control_plane_contract
        .get("evidenceStage2Authority")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("bidirEvidenceRoute"))
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("failureClasses"))
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();

    let checker_expected_core = as_sorted_strings(
        lane_registry
            .get("expectedCheckerCoreOnlyObligations")
            .unwrap_or(&Value::Null),
    );
    let checker_required_route = lane_registry
        .get("requiredCrossLaneWitnessRoute")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let checker_required_failures = as_sorted_strings(
        lane_registry
            .get("requiredLaneFailureClasses")
            .unwrap_or(&Value::Null),
    );

    let checker_stage1_parity_required_classes = gate_chain_details
        .get("stage1Parity")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("requiredFailureClasses"))
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();
    if gate_chain_details
        .get("stage1Parity")
        .and_then(Value::as_object)
        .is_none()
    {
        reasons
            .push("coherence witness missing gate_chain_parity stage1Parity details".to_string());
    }

    let checker_stage1_rollback = gate_chain_details
        .get("stage1Rollback")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| {
            reasons.push(
                "coherence witness missing gate_chain_parity stage1Rollback details".to_string(),
            );
            Map::new()
        });
    let checker_stage1_rollback_required_trigger_classes = as_sorted_strings(
        checker_stage1_rollback
            .get("requiredTriggerFailureClasses")
            .unwrap_or(&Value::Null),
    );
    let checker_stage1_rollback_required_classes = checker_stage1_rollback
        .get("requiredFailureClasses")
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();

    let checker_stage2_authority = gate_chain_details
        .get("stage2Authority")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if control_plane_contract.contains_key("evidenceStage2Authority")
        && gate_chain_details
            .get("stage2Authority")
            .and_then(Value::as_object)
            .is_none()
    {
        reasons.push(
            "coherence witness missing gate_chain_parity stage2Authority details".to_string(),
        );
    }

    let checker_stage2_required_classes = checker_stage2_authority
        .get("requiredFailureClasses")
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();

    let checker_stage2_bidir_required_obligations = checker_stage2_authority
        .get("bidirEvidenceRoute")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("requiredObligations"))
        .map(as_sorted_strings)
        .unwrap_or_default();

    let checker_stage2_bidir_required_classes = checker_stage2_authority
        .get("requiredBidirEvidenceFailureClasses")
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_else(|| {
            checker_stage2_authority
                .get("requiredKernelComplianceFailureClasses")
                .and_then(Value::as_object)
                .map(|obj| {
                    let values = obj.values().cloned().collect::<Vec<_>>();
                    as_sorted_strings(&Value::Array(values))
                })
                .unwrap_or_default()
        });

    let checker_lane_values = lane_registry
        .get("evidenceLanes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if !checker_lane_values.is_empty() && checker_lane_values != contract_evidence_lanes {
        reasons.push(
            "coherence checker lane IDs differ from CONTROL-PLANE-CONTRACT evidenceLanes"
                .to_string(),
        );
    }

    let checker_kinds = normalize_lane_artifact_kinds(
        lane_registry
            .get("laneArtifactKinds")
            .unwrap_or(&Value::Null),
    );
    if !checker_kinds.is_empty() && checker_kinds != contract_lane_artifact_kinds {
        reasons.push(
            "coherence checker laneArtifactKinds differ from CONTROL-PLANE-CONTRACT".to_string(),
        );
    }

    if !checker_expected_core.is_empty() && checker_expected_core != contract_checker_core {
        reasons.push(
            "checker expected checker-core-only obligations differ from CONTROL-PLANE-CONTRACT laneOwnership".to_string(),
        );
    }
    if !checker_required_route.is_empty() && checker_required_route != contract_required_route {
        reasons.push(
            "checker required cross-lane witness route differs from CONTROL-PLANE-CONTRACT laneOwnership".to_string(),
        );
    }
    if !checker_required_failures.is_empty()
        && !checker_required_failures
            .iter()
            .all(|class_name| contract_lane_failure_classes.contains(class_name))
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT laneFailureClasses missing checker-required failure classes"
                .to_string(),
        );
    }
    if !checker_stage1_parity_required_classes.is_empty()
        && checker_stage1_parity_required_classes != contract_stage1_parity_failure_classes
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT evidenceStage1Parity.failureClasses differ from checker-required classes".to_string(),
        );
    }
    if !checker_stage1_rollback_required_classes.is_empty()
        && checker_stage1_rollback_required_classes != contract_stage1_rollback_failure_classes
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT evidenceStage1Rollback.failureClasses differ from checker-required classes".to_string(),
        );
    }
    if !checker_stage1_rollback_required_trigger_classes.is_empty()
        && !checker_stage1_rollback_required_trigger_classes
            .iter()
            .all(|class_name| contract_stage1_rollback_trigger_classes.contains(class_name))
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT evidenceStage1Rollback.triggerFailureClasses missing checker-required trigger classes".to_string(),
        );
    }
    if !checker_stage2_required_classes.is_empty()
        && checker_stage2_required_classes != contract_stage2_failure_classes
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT evidenceStage2Authority.failureClasses differ from checker-required classes".to_string(),
        );
    }
    if !checker_stage2_bidir_required_obligations.is_empty()
        && checker_stage2_bidir_required_obligations != contract_stage2_bidir_required_obligations
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT evidenceStage2Authority.bidirEvidenceRoute.requiredObligations differ from checker-observed values".to_string(),
        );
    }
    if !checker_stage2_bidir_required_classes.is_empty()
        && checker_stage2_bidir_required_classes != contract_stage2_bidir_failure_classes
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT evidenceStage2Authority.bidirEvidenceRoute.failureClasses differ from checker-required classes".to_string(),
        );
    }

    let details = json!({
        "reasons": reasons,
        "contract": {
            "evidenceLanes": contract_evidence_lanes,
            "laneArtifactKinds": contract_lane_artifact_kinds,
            "checkerCoreOnlyObligations": contract_checker_core,
            "requiredCrossLaneWitnessRoute": contract_required_route,
            "laneFailureClasses": contract_lane_failure_classes,
            "stage1": {
                "parityFailureClasses": contract_stage1_parity_failure_classes,
                "rollbackTriggerFailureClasses": contract_stage1_rollback_trigger_classes,
                "rollbackFailureClasses": contract_stage1_rollback_failure_classes
            },
            "stage2": {
                "authorityFailureClasses": contract_stage2_failure_classes,
                "bidirRequiredObligations": contract_stage2_bidir_required_obligations,
                "bidirFailureClasses": contract_stage2_bidir_failure_classes
            }
        },
        "checker": {
            "evidenceLanes": checker_lane_values,
            "laneArtifactKinds": checker_kinds,
            "expectedCheckerCoreOnlyObligations": checker_expected_core,
            "requiredCrossLaneWitnessRoute": checker_required_route,
            "requiredLaneFailureClasses": checker_required_failures,
            "stage1": {
                "parityRequiredFailureClasses": checker_stage1_parity_required_classes,
                "rollbackRequiredTriggerFailureClasses": checker_stage1_rollback_required_trigger_classes,
                "rollbackRequiredFailureClasses": checker_stage1_rollback_required_classes
            },
            "stage2": {
                "authorityRequiredFailureClasses": checker_stage2_required_classes,
                "bidirRequiredObligations": checker_stage2_bidir_required_obligations,
                "bidirRequiredFailureClasses": checker_stage2_bidir_required_classes
            }
        }
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    (failed, details)
}

fn check_runtime_route_bindings(
    control_plane_contract: &Map<String, Value>,
    doctrine_operations: &BTreeMap<String, DoctrineOperationRow>,
) -> (bool, Value) {
    let mut reasons = Vec::new();

    let contract_runtime_routes = normalize_runtime_route_bindings(
        control_plane_contract
            .get("runtimeRouteBindings")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("requiredOperationRoutes"))
            .unwrap_or(&Value::Null),
    );
    if contract_runtime_routes.is_empty() {
        reasons.push(
            "CONTROL-PLANE-CONTRACT missing runtimeRouteBindings.requiredOperationRoutes"
                .to_string(),
        );
    }

    let contract_runtime_failure_classes = control_plane_contract
        .get("runtimeRouteBindings")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("failureClasses"))
        .and_then(Value::as_object)
        .map(|obj| {
            let values = obj.values().cloned().collect::<Vec<_>>();
            as_sorted_strings(&Value::Array(values))
        })
        .unwrap_or_default();
    if contract_runtime_failure_classes.is_empty() {
        reasons
            .push("CONTROL-PLANE-CONTRACT missing runtimeRouteBindings.failureClasses".to_string());
    }

    let mut missing_operation_routes = Vec::new();
    let mut missing_morphisms = Vec::new();
    let mut observed_registry_routes = Map::new();
    for (route_id, route) in &contract_runtime_routes {
        let operation_id = route
            .get("operationId")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if operation_id.is_empty() {
            continue;
        }
        let required_morphisms =
            as_sorted_strings(route.get("requiredMorphisms").unwrap_or(&Value::Null));
        let Some(operation_row) = doctrine_operations.get(operation_id) else {
            missing_operation_routes.push(json!({
                "routeId": route_id,
                "operationId": operation_id
            }));
            continue;
        };
        observed_registry_routes.insert(
            route_id.to_string(),
            json!({
                "operationId": operation_id,
                "path": operation_row.path,
                "actualMorphisms": operation_row.morphisms
            }),
        );
        let route_missing_morphisms = required_morphisms
            .iter()
            .filter(|morphism| !operation_row.morphisms.contains(*morphism))
            .cloned()
            .collect::<Vec<_>>();
        if !route_missing_morphisms.is_empty() {
            missing_morphisms.push(json!({
                "routeId": route_id,
                "operationId": operation_id,
                "missingMorphisms": route_missing_morphisms,
                "requiredMorphisms": required_morphisms,
                "actualMorphisms": operation_row.morphisms
            }));
        }
    }

    if !missing_operation_routes.is_empty() || !missing_morphisms.is_empty() {
        reasons.push(
            "DOCTRINE-OP-REGISTRY missing required runtime-route bindings or morphisms".to_string(),
        );
    }

    let details = json!({
        "reasons": reasons,
        "contractRuntimeRouteBindings": contract_runtime_routes,
        "contractRuntimeRouteFailureClasses": contract_runtime_failure_classes,
        "observedDoctrineRegistryRoutes": observed_registry_routes,
        "missingOperationRoutes": missing_operation_routes,
        "missingRequiredMorphisms": missing_morphisms
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    (failed, details)
}

fn check_control_plane_kcir_mappings(control_plane_contract: &Map<String, Value>) -> (bool, Value) {
    let mut reasons = Vec::new();
    let contract_mappings = control_plane_contract
        .get("controlPlaneKcirMappings")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| {
            reasons.push("CONTROL-PLANE-CONTRACT missing controlPlaneKcirMappings".to_string());
            Map::new()
        });

    let contract_profile_id = contract_mappings
        .get("profileId")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    if contract_profile_id.is_empty() {
        reasons.push(
            "CONTROL-PLANE-CONTRACT controlPlaneKcirMappings.profileId must be non-empty"
                .to_string(),
        );
    }

    let contract_mapping_table = normalize_kcir_mapping_table(
        contract_mappings
            .get("mappingTable")
            .unwrap_or(&Value::Null),
    );
    let mut missing_required_rows = Vec::new();
    let mut row_invalid_fields = Vec::new();
    for row_id in REQUIRED_KCIR_MAPPING_ROWS {
        let Some(row) = contract_mapping_table.get(row_id) else {
            missing_required_rows.push(row_id.to_string());
            continue;
        };
        let mut drift_fields = Vec::new();
        for field in REQUIRED_KCIR_MAPPING_ROW_FIELDS {
            if row
                .get(field)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                drift_fields.push(field.to_string());
            }
        }
        let identity_fields = row
            .get("identityFields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if identity_fields.is_empty() {
            drift_fields.push("identityFields".to_string());
        }
        if !drift_fields.is_empty() {
            row_invalid_fields.push(json!({
                "rowId": row_id,
                "driftFields": drift_fields,
                "contract": row
            }));
        }
    }
    if !missing_required_rows.is_empty() || !row_invalid_fields.is_empty() {
        reasons.push(
            "CONTROL-PLANE-CONTRACT controlPlaneKcirMappings.mappingTable is missing required rows or required row fields".to_string(),
        );
    }

    let legacy_policy = normalize_kcir_legacy_policy(
        contract_mappings
            .get("compatibilityPolicy")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("legacyNonKcirEncodings"))
            .unwrap_or(&Value::Null),
    );
    let legacy_policy_missing_fields =
        ["mode", "authorityMode", "supportUntilEpoch", "failureClass"]
            .iter()
            .filter(|field| {
                legacy_policy
                    .get(**field)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
            })
            .map(|field| field.to_string())
            .collect::<Vec<_>>();
    if !legacy_policy_missing_fields.is_empty() {
        reasons.push(
            "CONTROL-PLANE-CONTRACT controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings missing required fields".to_string(),
        );
    }

    let details = json!({
        "reasons": reasons,
        "contractProfileId": contract_profile_id,
        "contractMappingTable": contract_mapping_table,
        "missingRequiredRows": missing_required_rows,
        "rowInvalidFields": row_invalid_fields,
        "contractLegacyPolicy": legacy_policy,
        "legacyPolicyMissingFields": legacy_policy_missing_fields
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    (failed, details)
}

fn check_coherence_required_obligations(
    coherence_contract: &Map<String, Value>,
    scope_noncontradiction_details: &Map<String, Value>,
) -> Result<(bool, Value), String> {
    let mut reasons = Vec::new();
    let contract_required_obligations = parse_required_obligation_ids(coherence_contract)?;
    let contract_required_bidir = parse_required_bidir_obligations(coherence_contract)?;
    let contract_registry_kind = coherence_contract
        .get("surfaces")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("obligationRegistryKind"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    let checker_required_obligations = as_sorted_strings(
        scope_noncontradiction_details
            .get("requiredCoherenceObligations")
            .unwrap_or(&Value::Null),
    );
    let checker_required_bidir = as_sorted_strings(
        scope_noncontradiction_details
            .get("requiredBidirObligations")
            .unwrap_or(&Value::Null),
    );
    let checker_registry_kind = scope_noncontradiction_details
        .get("obligationRegistryKind")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    if contract_required_obligations != checker_required_obligations {
        reasons.push(
            "coherence required obligation set drifts between contract and checker".to_string(),
        );
    }
    if contract_required_bidir != checker_required_bidir {
        reasons.push("requiredBidirObligations drifts between contract and checker".to_string());
    }
    if contract_registry_kind != checker_registry_kind {
        reasons.push("obligation registry kind drifts between contract and checker".to_string());
    }

    let details = json!({
        "reasons": reasons,
        "contractRequiredObligations": contract_required_obligations,
        "checkerRequiredObligations": checker_required_obligations,
        "contractRequiredBidirObligations": contract_required_bidir,
        "checkerRequiredBidirObligations": checker_required_bidir,
        "contractObligationRegistryKind": contract_registry_kind,
        "checkerObligationRegistryKind": checker_registry_kind
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    Ok((failed, details))
}

fn check_sigpi_notation(repo_root: &Path) -> Result<(bool, Value), String> {
    let mut reasons = Vec::new();
    let mut alias_hits = Vec::new();
    let mut canonical_sigpi_docs = Vec::new();
    let mut canonical_latex_docs = Vec::new();
    let alias_re = RegexBuilder::new(r"\bSig/Pi\b")
        .case_insensitive(true)
        .build()
        .map_err(|error| format!("failed to compile Sig/Pi alias regex: {error}"))?;

    for rel in SIGPI_NORMATIVE_DOCS {
        let path = repo_root.join(rel);
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if alias_re.is_match(&text) {
            alias_hits.push(rel.to_string());
        }
        if text.contains("SigPi") {
            canonical_sigpi_docs.push(rel.to_string());
        }
        if text.contains("sig\\Pi") {
            canonical_latex_docs.push(rel.to_string());
        }
    }

    if !alias_hits.is_empty() {
        reasons.push("normative docs still use Sig/Pi alias".to_string());
    }
    if canonical_sigpi_docs.is_empty() {
        reasons.push("normative docs missing canonical SigPi spelling".to_string());
    }
    if canonical_latex_docs.is_empty() {
        reasons.push("normative docs missing canonical sig\\\\Pi notation".to_string());
    }

    let details = json!({
        "reasons": reasons,
        "checkedDocs": SIGPI_NORMATIVE_DOCS,
        "aliasHits": alias_hits,
        "canonicalSigPiDocs": canonical_sigpi_docs,
        "canonicalLatexDocs": canonical_latex_docs
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    Ok((failed, details))
}

fn resolve_rooted_path(repo_root: &Path, value: &str) -> PathBuf {
    let candidate = PathBuf::from(value);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    }
}

fn normalize_path_for_set(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        canonical
    } else {
        path.to_path_buf()
    }
}

fn load_coherence_contract_input_paths(
    repo_root: &Path,
    coherence_contract: &Map<String, Value>,
) -> BTreeSet<PathBuf> {
    let mut base_paths = vec![
        repo_root.join("tools/conformance/run_fixture_suites.py"),
        repo_root.join("tools/ci/control_plane_contract.py"),
        repo_root.join("specs/premath/draft/COHERENCE-CONTRACT.json"),
        repo_root.join("Cargo.toml"),
        repo_root.join("Cargo.lock"),
        repo_root.join("tests/conformance/fixtures/coherence-transport"),
        repo_root.join("tests/conformance/fixtures/coherence-site"),
        repo_root.join("crates/premath-kernel/src"),
        repo_root.join("crates/premath-coherence/src"),
        repo_root.join("crates/premath-cli/src/commands/coherence_check.rs"),
    ];

    if let Some(surfaces) = coherence_contract
        .get("surfaces")
        .and_then(Value::as_object)
    {
        for (key, value) in surfaces {
            if !(key.ends_with("Path") || key.ends_with("Root")) {
                continue;
            }
            let Some(path_text) = value.as_str() else {
                continue;
            };
            base_paths.push(resolve_rooted_path(repo_root, path_text));
        }
    }

    if let Some(expected_operation_paths) = coherence_contract
        .get("expectedOperationPaths")
        .and_then(Value::as_array)
    {
        for value in expected_operation_paths {
            if let Some(path_text) = value.as_str() {
                base_paths.push(resolve_rooted_path(repo_root, path_text));
            }
        }
    }

    if let Some(overlay_docs) = coherence_contract
        .get("overlayDocs")
        .and_then(Value::as_array)
    {
        for value in overlay_docs {
            if let Some(doc) = value.as_str().map(str::trim)
                && !doc.is_empty()
            {
                base_paths.push(resolve_rooted_path(
                    repo_root,
                    &format!("specs/premath/{doc}.md"),
                ));
            }
        }
    }

    base_paths
        .into_iter()
        .map(|path| normalize_path_for_set(&path))
        .collect::<BTreeSet<_>>()
}

fn check_cache_input_closure(
    repo_root: &Path,
    coherence_contract: &Map<String, Value>,
) -> (bool, Value) {
    let mut reasons = Vec::new();
    let closure_paths = load_coherence_contract_input_paths(repo_root, coherence_contract);
    let required_paths = CACHE_CLOSURE_REQUIRED_PATHS
        .iter()
        .map(|rel| normalize_path_for_set(&repo_root.join(rel)))
        .collect::<Vec<_>>();
    let missing = CACHE_CLOSURE_REQUIRED_PATHS
        .iter()
        .zip(required_paths.iter())
        .filter_map(|(rel, abs_path)| {
            (!closure_paths.contains(abs_path)).then_some(rel.to_string())
        })
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        reasons.push(
            "coherence-contract cache input closure missing required loader inputs".to_string(),
        );
    }

    let details = json!({
        "reasons": reasons,
        "requiredPaths": CACHE_CLOSURE_REQUIRED_PATHS,
        "missingPaths": missing,
        "closureSize": closure_paths.len()
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    (failed, details)
}

fn extract_frontmatter_status(path: &Path) -> Result<Option<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if !text.starts_with("---\n") {
        return Ok(None);
    }
    let mut parts = text.splitn(3, "---\n");
    let _ = parts.next();
    let Some(frontmatter) = parts.next() else {
        return Ok(None);
    };
    for raw in frontmatter.lines() {
        let line = raw.trim();
        if let Some((key, value)) = line.split_once(':')
            && key.trim() == "status"
        {
            return Ok(Some(value.trim().to_string()));
        }
    }
    Ok(None)
}

fn count_promoted_draft_specs(draft_dir: &Path) -> Result<i64, String> {
    let entries = fs::read_dir(draft_dir)
        .map_err(|error| format!("failed to read {}: {error}", draft_dir.display()))?;
    let mut count = 0_i64;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("failed to list {}: {error}", draft_dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "README.md") {
            continue;
        }
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("md") => {
                if extract_frontmatter_status(&path)?.as_deref() == Some("draft") {
                    count += 1;
                }
            }
            Some("json") => {
                count += 1;
            }
            _ => {}
        }
    }
    Ok(count)
}

fn count_traceability_rows(matrix_path: &Path) -> Result<i64, String> {
    let lines = fs::read_to_string(matrix_path)
        .map_err(|error| format!("failed to read {}: {error}", matrix_path.display()))?
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let code_ref_re = Regex::new(r"`([^`]+)`")
        .map_err(|error| format!("failed to compile code-ref regex: {error}"))?;
    let separator_re = Regex::new(r"^\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|$")
        .map_err(|error| format!("failed to compile markdown-separator regex: {error}"))?;
    let mut in_matrix = false;
    let mut count = 0_i64;
    for line in lines {
        if line.starts_with("## 3. Traceability Matrix") {
            in_matrix = true;
            continue;
        }
        if in_matrix && line.starts_with("## ") {
            break;
        }
        if !in_matrix {
            continue;
        }
        let stripped = line.trim();
        if !stripped.starts_with('|') {
            continue;
        }
        if stripped.starts_with("| Draft spec") {
            continue;
        }
        if separator_re.is_match(stripped) {
            continue;
        }
        let parts = stripped
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if parts.len() != 4 {
            continue;
        }
        if !code_ref_re.is_match(parts[0]) {
            continue;
        }
        count += 1;
    }
    Ok(count)
}

fn parse_optional_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let Some(items) = value.as_array() else {
        return Err(format!("{label} must be a list"));
    };
    let mut out = Vec::new();
    for (idx, row) in items.iter().enumerate() {
        let text = row.as_str().map(str::trim).unwrap_or("");
        if text.is_empty() {
            return Err(format!("{label}[{idx}] must be a non-empty string"));
        }
        out.push(text.to_string());
    }
    let set = out.iter().cloned().collect::<BTreeSet<_>>();
    if set.len() != out.len() {
        return Err(format!("{label} must not contain duplicates"));
    }
    Ok(out)
}

fn parse_topology_threshold(value: &Value, label: &str) -> Result<TopologyThreshold, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!("{label} must be an object"));
    };
    let parse_opt_i64 = |field: &str| -> Result<Option<i64>, String> {
        match obj.get(field) {
            None | Some(Value::Null) => Ok(None),
            Some(number) => number
                .as_i64()
                .ok_or_else(|| format!("{label}.{field} must be an integer"))
                .map(Some),
        }
    };
    let threshold = TopologyThreshold {
        warn_above: parse_opt_i64("warnAbove")?,
        fail_above: parse_opt_i64("failAbove")?,
        warn_below: parse_opt_i64("warnBelow")?,
        fail_below: parse_opt_i64("failBelow")?,
    };

    if let (Some(warn_above), Some(fail_above)) = (threshold.warn_above, threshold.fail_above)
        && warn_above > fail_above
    {
        return Err(format!("{label}: warnAbove must be <= failAbove"));
    }
    if let (Some(warn_below), Some(fail_below)) = (threshold.warn_below, threshold.fail_below)
        && warn_below < fail_below
    {
        return Err(format!("{label}: warnBelow must be >= failBelow"));
    }
    if threshold.warn_above.is_none()
        && threshold.fail_above.is_none()
        && threshold.warn_below.is_none()
        && threshold.fail_below.is_none()
    {
        return Err(format!("{label} must declare at least one threshold bound"));
    }

    Ok(threshold)
}

fn load_topology_budget_contract(path: &Path) -> Result<TopologyBudgetContract, String> {
    let payload = load_json_object(path)?;
    if payload.get("schema").and_then(Value::as_i64) != Some(TOPOLOGY_BUDGET_SCHEMA) {
        return Err(format!(
            "{path}: schema must be {TOPOLOGY_BUDGET_SCHEMA}",
            path = path.display()
        ));
    }
    if payload
        .get("budgetKind")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        != TOPOLOGY_BUDGET_KIND
    {
        return Err(format!(
            "{path}: budgetKind must be {kind:?}",
            path = path.display(),
            kind = TOPOLOGY_BUDGET_KIND
        ));
    }
    let metrics_raw = payload
        .get("metrics")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{}: metrics must be a non-empty object", path.display()))?;
    if metrics_raw.is_empty() {
        return Err(format!(
            "{}: metrics must be a non-empty object",
            path.display()
        ));
    }
    let mut metrics = BTreeMap::new();
    for (metric_id, threshold_raw) in metrics_raw {
        let metric_id = metric_id.trim();
        if metric_id.is_empty() {
            return Err(format!(
                "{}: metric IDs must be non-empty strings",
                path.display()
            ));
        }
        if metrics.contains_key(metric_id) {
            return Err(format!(
                "{}: duplicate metric ID {:?}",
                path.display(),
                metric_id
            ));
        }
        let threshold = parse_topology_threshold(
            threshold_raw,
            &format!("{}:metrics.{}", path.display(), metric_id),
        )?;
        metrics.insert(metric_id.to_string(), threshold);
    }

    Ok(TopologyBudgetContract {
        metrics,
        deprecated_design_fragments: parse_optional_string_list(
            payload.get("deprecatedDesignFragments"),
            "deprecatedDesignFragments",
        )?,
        doctrine_site_authority_inputs: parse_optional_string_list(
            payload.get("doctrineSiteAuthorityInputs"),
            "doctrineSiteAuthorityInputs",
        )?,
        doctrine_site_generated_views: parse_optional_string_list(
            payload.get("doctrineSiteGeneratedViews"),
            "doctrineSiteGeneratedViews",
        )?,
    })
}

fn evaluate_topology_threshold(value: i64, threshold: &TopologyThreshold) -> (String, Vec<String>) {
    let mut messages = Vec::new();
    if let Some(fail_above) = threshold.fail_above
        && value > fail_above
    {
        messages.push(format!("value {value} exceeds failAbove {fail_above}"));
    }
    if let Some(fail_below) = threshold.fail_below
        && value < fail_below
    {
        messages.push(format!("value {value} is below failBelow {fail_below}"));
    }
    if !messages.is_empty() {
        return ("fail".to_string(), messages);
    }

    if let Some(warn_above) = threshold.warn_above
        && value > warn_above
    {
        messages.push(format!("value {value} exceeds warnAbove {warn_above}"));
    }
    if let Some(warn_below) = threshold.warn_below
        && value < warn_below
    {
        messages.push(format!("value {value} is below warnBelow {warn_below}"));
    }
    if !messages.is_empty() {
        return ("warn".to_string(), messages);
    }
    ("ok".to_string(), Vec::new())
}

fn collect_topology_metrics(
    repo_root: &Path,
    contract: &TopologyBudgetContract,
) -> Result<BTreeMap<String, i64>, String> {
    let draft_dir = repo_root.join("specs/premath/draft");
    let design_dir = repo_root.join("docs/design");
    let traceability_path = draft_dir.join("SPEC-TRACEABILITY.md");
    let doctrine_site_path = draft_dir.join("DOCTRINE-SITE.json");
    let doctrine_site = load_json_object(&doctrine_site_path)?;
    let doctrine_edges = doctrine_site
        .get("edges")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{}: edges must be a list", doctrine_site_path.display()))?;

    let design_doc_nodes = fs::read_dir(&design_dir)
        .map_err(|error| format!("failed to read {}: {error}", design_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .filter(|path| path.file_name().is_some_and(|name| name != "README.md"))
        .count() as i64;

    let mut metrics = BTreeMap::new();
    metrics.insert(
        "draftSpecNodes".to_string(),
        count_promoted_draft_specs(&draft_dir)?,
    );
    metrics.insert(
        "specTraceabilityRows".to_string(),
        count_traceability_rows(&traceability_path)?,
    );
    metrics.insert("designDocNodes".to_string(), design_doc_nodes);
    metrics.insert(
        "doctrineSiteEdgeCount".to_string(),
        doctrine_edges.len() as i64,
    );
    metrics.insert(
        "doctrineSiteAuthorityInputCount".to_string(),
        contract
            .doctrine_site_authority_inputs
            .iter()
            .filter(|rel| repo_root.join(rel).exists())
            .count() as i64,
    );
    metrics.insert(
        "doctrineSiteGeneratedViewCount".to_string(),
        contract
            .doctrine_site_generated_views
            .iter()
            .filter(|rel| repo_root.join(rel).exists())
            .count() as i64,
    );
    metrics.insert(
        "deprecatedDesignFragmentCount".to_string(),
        contract
            .deprecated_design_fragments
            .iter()
            .filter(|rel| repo_root.join(rel).exists())
            .count() as i64,
    );
    Ok(metrics)
}

fn check_topology_budget(
    repo_root: &Path,
    topology_budget_path: &Path,
) -> Result<(bool, bool, Value), String> {
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();
    let contract = load_topology_budget_contract(topology_budget_path)?;
    let thresholds = contract.metrics.clone();
    let metrics = collect_topology_metrics(repo_root, &contract)?;

    let mut details_metrics = Map::new();
    let unknown_threshold_metrics = thresholds
        .keys()
        .filter(|metric_id| !metrics.contains_key(*metric_id))
        .cloned()
        .collect::<Vec<_>>();
    for metric_id in &unknown_threshold_metrics {
        reasons.push(format!("topology metric {metric_id:?} has no evaluator"));
    }
    let unbudgeted_metrics = metrics
        .keys()
        .filter(|metric_id| !thresholds.contains_key(*metric_id))
        .cloned()
        .collect::<Vec<_>>();

    for metric_id in metrics
        .keys()
        .filter(|metric_id| thresholds.contains_key(*metric_id))
    {
        let value = *metrics.get(metric_id).unwrap_or(&0);
        let threshold = thresholds.get(metric_id).cloned().unwrap_or_default();
        let (status, messages) = evaluate_topology_threshold(value, &threshold);
        details_metrics.insert(
            metric_id.clone(),
            json!({
                "value": value,
                "status": status,
                "threshold": {
                    "warnAbove": threshold.warn_above,
                    "failAbove": threshold.fail_above,
                    "warnBelow": threshold.warn_below,
                    "failBelow": threshold.fail_below
                },
                "messages": messages
            }),
        );
        if status == "fail" {
            for message in &messages {
                reasons.push(format!("{metric_id}: {message}"));
            }
        } else if status == "warn" {
            for message in &messages {
                warnings.push(format!("{metric_id}: {message}"));
            }
        }
    }

    let details = json!({
        "reasons": reasons,
        "warnings": warnings,
        "budgetPath": topology_budget_path.display().to_string(),
        "metrics": details_metrics,
        "unbudgetedMetrics": unbudgeted_metrics
    });
    let failed = details
        .get("reasons")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let warned = details
        .get("warnings")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    Ok((failed, warned, details))
}

fn build_drift_budget_payload(
    repo_root: &Path,
    coherence_witness: &Map<String, Value>,
    coherence_contract: &Map<String, Value>,
    control_plane_contract: &Map<String, Value>,
    topology_budget_path: &Path,
) -> Result<Value, String> {
    let capability_registry_path = repo_root.join("specs/premath/draft/CAPABILITY-REGISTRY.json");
    let doctrine_op_registry_path = repo_root.join("specs/premath/draft/DOCTRINE-OP-REGISTRY.json");
    let spec_index_path = repo_root.join("specs/premath/draft/SPEC-INDEX.md");
    let conformance_path = repo_root.join("specs/premath/draft/CONFORMANCE.md");

    let spec_map = parse_spec_index_capability_doc_map(&spec_index_path)?;
    let registry_contract = parse_capability_registry(&capability_registry_path)?;
    let doctrine_operations = parse_doctrine_operation_registry(&doctrine_op_registry_path)?;
    let conformance_overlay_claims = parse_conformance_profile_overlay_claims(&conformance_path)?;
    let conditional_docs = parse_conditional_capability_docs(coherence_contract)?;

    let scope_details = obligation_details(coherence_witness, "scope_noncontradiction")?;
    let gate_chain_details = obligation_details(coherence_witness, "gate_chain_parity")?;

    let mut checks: Vec<(String, bool, bool, Value)> = Vec::new();

    let (profile_failed, profile_details) = check_profile_overlay_claims(
        &registry_contract.profile_overlay_claims,
        &conformance_overlay_claims,
    );
    checks.push((
        DRIFT_CLASS_PROFILE_OVERLAYS.to_string(),
        profile_failed,
        false,
        profile_details,
    ));

    let (spec_failed, spec_details) = check_spec_index_capability_map(
        &spec_map,
        &registry_contract.executable_capabilities,
        &conditional_docs,
    );
    checks.push((
        DRIFT_CLASS_SPEC_INDEX.to_string(),
        spec_failed,
        false,
        spec_details,
    ));

    let (lane_failed, lane_details) =
        check_control_plane_lane_bindings(control_plane_contract, &gate_chain_details);
    checks.push((
        DRIFT_CLASS_LANE_BINDINGS.to_string(),
        lane_failed,
        false,
        lane_details,
    ));

    let (kcir_mapping_failed, kcir_mapping_details) =
        check_control_plane_kcir_mappings(control_plane_contract);
    checks.push((
        DRIFT_CLASS_KCIR_MAPPINGS.to_string(),
        kcir_mapping_failed,
        false,
        kcir_mapping_details,
    ));

    let (runtime_route_failed, runtime_route_details) =
        check_runtime_route_bindings(control_plane_contract, &doctrine_operations);
    checks.push((
        DRIFT_CLASS_RUNTIME_ROUTE_BINDINGS.to_string(),
        runtime_route_failed,
        false,
        runtime_route_details,
    ));

    let (required_failed, required_details) =
        check_coherence_required_obligations(coherence_contract, &scope_details)?;
    checks.push((
        DRIFT_CLASS_REQUIRED_OBLIGATIONS.to_string(),
        required_failed,
        false,
        required_details,
    ));

    let (sigpi_failed, sigpi_details) = check_sigpi_notation(repo_root)?;
    checks.push((
        DRIFT_CLASS_SIGPI_NOTATION.to_string(),
        sigpi_failed,
        false,
        sigpi_details,
    ));

    let (closure_failed, closure_details) =
        check_cache_input_closure(repo_root, coherence_contract);
    checks.push((
        DRIFT_CLASS_CACHE_CLOSURE.to_string(),
        closure_failed,
        false,
        closure_details,
    ));

    let (topology_failed, topology_warned, topology_details) =
        check_topology_budget(repo_root, topology_budget_path)?;
    checks.push((
        DRIFT_CLASS_TOPOLOGY_BUDGET.to_string(),
        topology_failed,
        topology_warned,
        topology_details,
    ));

    let drift_classes = checks
        .iter()
        .filter_map(|(class_id, failed, _, _)| failed.then_some(class_id.clone()))
        .collect::<Vec<_>>();
    let warning_classes = checks
        .iter()
        .filter_map(|(class_id, failed, warned, _)| {
            if !failed && *warned {
                if class_id == DRIFT_CLASS_TOPOLOGY_BUDGET {
                    Some(WARN_CLASS_TOPOLOGY_BUDGET.to_string())
                } else {
                    Some(class_id.clone())
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let mut details = Map::new();
    for (class_id, _, _, detail) in checks {
        details.insert(class_id, detail);
    }

    Ok(json!({
        "schema": SCHEMA,
        "checkKind": CHECK_KIND,
        "result": if drift_classes.is_empty() { "accepted" } else { "rejected" },
        "driftClasses": drift_classes.clone(),
        "warningClasses": warning_classes.clone(),
        "summary": {
            "checkCount": details.len(),
            "driftCount": drift_classes.len(),
            "driftDetected": !drift_classes.is_empty(),
            "warningCount": warning_classes.len(),
            "warningDetected": !warning_classes.is_empty()
        },
        "details": details
    }))
}

fn render_json_payload(payload: &Value) {
    let rendered = serde_json::to_string_pretty(payload).unwrap_or_else(|error| {
        eprintln!("error: failed to render drift-budget payload: {error}");
        std::process::exit(2);
    });
    println!("{rendered}");
}

pub fn run(
    repo_root: String,
    coherence_json: Option<String>,
    topology_budget: Option<String>,
    json_output: bool,
) {
    let repo_root = resolve_repo_root(&repo_root);
    let coherence_contract_path = repo_root.join("specs/premath/draft/COHERENCE-CONTRACT.json");
    let control_plane_contract_path =
        repo_root.join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");
    let topology_budget_path = topology_budget
        .map(|value| resolve_rel_path(&repo_root, &value))
        .unwrap_or_else(|| repo_root.join("specs/process/TOPOLOGY-BUDGET.json"));

    let payload = (|| -> Result<Value, String> {
        let coherence_contract = load_json_object(&coherence_contract_path)?;
        let control_plane_contract = load_json_object(&control_plane_contract_path)?;
        let coherence_witness = if let Some(coherence_json) = coherence_json {
            load_json_object(&resolve_rel_path(&repo_root, &coherence_json))?
        } else {
            let witness = run_coherence_check(&repo_root, &coherence_contract_path)
                .map_err(|error| format!("coherence-check command failed: {error}"))?;
            serde_json::to_value(witness)
                .map_err(|error| format!("failed to serialize coherence witness: {error}"))?
                .as_object()
                .cloned()
                .ok_or_else(|| "coherence witness JSON output must be an object".to_string())?
        };

        build_drift_budget_payload(
            &repo_root,
            &coherence_witness,
            &coherence_contract,
            &control_plane_contract,
            &topology_budget_path,
        )
    })()
    .unwrap_or_else(|error| {
        if json_output {
            render_json_payload(&json!({
                "schema": SCHEMA,
                "checkKind": CHECK_KIND,
                "result": "rejected",
                "driftClasses": ["drift_budget_command_failed"],
                "warningClasses": [],
                "summary": {
                    "checkCount": 0,
                    "driftCount": 1,
                    "driftDetected": true,
                    "warningCount": 0,
                    "warningDetected": false
                },
                "details": {
                    "error": error
                }
            }));
        } else {
            println!("[drift-budget-check] FAIL ({error})");
        }
        std::process::exit(1);
    });

    if json_output {
        render_json_payload(&payload);
    } else if payload
        .get("result")
        .and_then(Value::as_str)
        .is_some_and(|result| result == "accepted")
    {
        let warning_classes = payload
            .get("warningClasses")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if warning_classes.is_empty() {
            let check_count = payload
                .get("summary")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("checkCount"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            println!("[drift-budget-check] OK (checks={check_count}, drift=0)");
        } else {
            println!(
                "[drift-budget-check] WARN (checks={}, drift=0, warnings={})",
                payload
                    .get("summary")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("checkCount"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                serde_json::to_string(&warning_classes).unwrap_or_else(|_| "[]".to_string())
            );
        }
    } else {
        let drift_classes = payload
            .get("driftClasses")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        println!("[drift-budget-check] FAIL (driftClasses={drift_classes})");
    }

    if payload
        .get("result")
        .and_then(Value::as_str)
        .is_some_and(|result| result != "accepted")
    {
        std::process::exit(1);
    }
}
