use premath_coherence::run_coherence_check;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "premath.drift_budget.v1";
const TOPOLOGY_BUDGET_KIND: &str = "premath.topology_budget.v1";
const WARN_CLASS_TOPOLOGY_BUDGET: &str = "topology_budget_watch";

const SIGPI_NORMATIVE_DOCS: &[&str] = &[
    "specs/premath/draft/SPEC-INDEX.md",
    "specs/premath/draft/UNIFICATION-DOCTRINE.md",
    "specs/premath/profile/ADJOINTS-AND-SITES.md",
];

const CACHE_CLOSURE_REQUIRED_PATHS: &[&str] = &[
    "specs/premath/draft/COHERENCE-CONTRACT.json",
    "specs/premath/draft/CONTROL-PLANE-CONTRACT.json",
    "crates/premath-coherence/src",
    "crates/premath-cli/src/commands/coherence_check.rs",
];

pub struct Args {
    pub repo_root: String,
    pub coherence_json: Option<String>,
    pub topology_budget: Option<String>,
    pub json: bool,
}

struct Check {
    class_id: &'static str,
    failed: bool,
    warned: bool,
    details: Value,
}

pub fn run(args: Args) {
    let repo_root = PathBuf::from(args.repo_root);
    let root = repo_root.canonicalize().unwrap_or_else(|err| {
        eprintln!(
            "error: failed to resolve repo root `{}`: {err}",
            repo_root.display()
        );
        std::process::exit(2);
    });
    let topology_budget = args
        .topology_budget
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("specs/process/TOPOLOGY-BUDGET.json"));

    let payload = build_payload(&root, args.coherence_json.as_deref(), &topology_budget)
        .unwrap_or_else(|err| {
            eprintln!("[drift-budget-check] FAIL ({err})");
            std::process::exit(1);
        });

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("payload should serialize")
        );
    } else {
        let result = payload
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("rejected");
        if result == "accepted" {
            let warning_count = payload
                .pointer("/summary/warningCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if warning_count == 0 {
                println!("[drift-budget-check] OK");
            } else {
                println!(
                    "[drift-budget-check] WARN (warnings={})",
                    payload
                        .get("warningClasses")
                        .cloned()
                        .unwrap_or_else(|| json!([]))
                );
            }
        } else {
            println!(
                "[drift-budget-check] FAIL (driftClasses={})",
                payload
                    .get("driftClasses")
                    .cloned()
                    .unwrap_or_else(|| json!([]))
            );
        }
    }

    if payload.get("result").and_then(Value::as_str) != Some("accepted") {
        std::process::exit(1);
    }
}

fn build_payload(
    repo_root: &Path,
    coherence_json: Option<&str>,
    topology_budget_path: &Path,
) -> Result<Value, String> {
    let coherence_contract_path = repo_root.join("specs/premath/draft/COHERENCE-CONTRACT.json");
    let control_plane_contract_path =
        repo_root.join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

    let coherence_contract = load_json(&coherence_contract_path)?;
    let control_plane_contract = load_json(&control_plane_contract_path)?;
    let witness = if let Some(path) = coherence_json {
        load_json(&PathBuf::from(path))?
    } else {
        serde_json::to_value(
            run_coherence_check(repo_root, &coherence_contract_path)
                .map_err(|err| format!("coherence-check failed: {err}"))?,
        )
        .map_err(|err| format!("failed to encode coherence witness: {err}"))?
    };

    let scope_details = obligation_details(&witness, "scope_noncontradiction")?;
    let gate_chain_details = obligation_details(&witness, "gate_chain_parity")?;

    let capability_registry =
        load_json(&repo_root.join("specs/premath/draft/CAPABILITY-REGISTRY.json"))?;
    let spec_index_text = read_text(&repo_root.join("specs/premath/draft/SPEC-INDEX.md"))?;
    let checker_claims_text = read_text(&repo_root.join("specs/premath/draft/CHECKER-CLAIMS.md"))?;

    let checks = vec![
        check_profile_overlay_claims(&capability_registry, &checker_claims_text)?,
        check_spec_index_capability_map(
            &spec_index_text,
            &capability_registry,
            &coherence_contract,
        )?,
        check_control_plane_lane_bindings(&control_plane_contract, gate_chain_details),
        check_control_plane_kcir_mappings(&control_plane_contract),
        check_coherence_required_obligations(&coherence_contract, scope_details),
        check_sigpi_notation(repo_root)?,
        check_cache_input_closure(repo_root, &coherence_contract),
        check_topology_budget(repo_root, topology_budget_path)?,
    ];

    let drift_classes: Vec<&str> = checks
        .iter()
        .filter(|check| check.failed)
        .map(|check| check.class_id)
        .collect();
    let warning_classes: Vec<&str> = checks
        .iter()
        .filter(|check| !check.failed && check.warned)
        .map(|check| {
            if check.class_id == "topology_budget_drift" {
                WARN_CLASS_TOPOLOGY_BUDGET
            } else {
                check.class_id
            }
        })
        .collect();
    let details = checks
        .into_iter()
        .map(|check| (check.class_id.to_string(), check.details))
        .collect::<Map<String, Value>>();

    Ok(json!({
        "schema": 1,
        "checkKind": CHECK_KIND,
        "result": if drift_classes.is_empty() { "accepted" } else { "rejected" },
        "driftClasses": drift_classes,
        "warningClasses": warning_classes,
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

fn load_json(path: &Path) -> Result<Value, String> {
    let text = read_text(path)?;
    serde_json::from_str(&text).map_err(|err| format!("{}: invalid JSON: {err}", path.display()))
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("{}: read failed: {err}", path.display()))
}

fn obligation_details<'a>(witness: &'a Value, id: &str) -> Result<&'a Value, String> {
    witness
        .get("obligations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("obligationId").and_then(Value::as_str) == Some(id))
        .and_then(|row| row.get("details"))
        .ok_or_else(|| format!("coherence witness missing obligation `{id}`"))
}

fn string_set(value: &Value) -> BTreeSet<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn string_vec(value: &Value) -> Vec<String> {
    string_set(value).into_iter().collect()
}

fn string_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn normalize_lane_artifact_kinds(value: Option<&Value>) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    if let Some(map) = value.and_then(Value::as_object) {
        for (key, value) in map {
            out.insert(key.clone(), string_vec(value));
        }
    }
    out
}

fn values_as_sorted_strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.values()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_default()
}

fn extract_heading_section<'a>(text: &'a str, heading_prefix: &str) -> Result<&'a str, String> {
    let marker = format!("### {heading_prefix}");
    let start = text
        .lines()
        .scan(0usize, |offset, line| {
            let current = *offset;
            *offset += line.len() + 1;
            Some((current, line))
        })
        .find(|(_, line)| line.starts_with(&marker))
        .map(|(offset, line)| offset + line.len() + 1)
        .ok_or_else(|| format!("missing heading: {heading_prefix}"))?;
    let tail = &text[start..];
    let end = tail.find("\n### ").unwrap_or(tail.len());
    Ok(&tail[..end])
}

fn parse_backticked_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let token = after_start[..end].trim();
        if !token.is_empty() {
            out.push(token.to_string());
        }
        rest = &after_start[end + 1..];
    }
    out
}

fn parse_spec_index_capability_doc_map(text: &str) -> Result<BTreeMap<String, String>, String> {
    let section = extract_heading_section(text, "5.4")?;
    let mut out = BTreeMap::new();
    for line in section.lines() {
        let tokens = parse_backticked_tokens(line);
        if tokens.len() >= 2 && line.contains("(for `") {
            out.insert(tokens[0].clone(), tokens[1].clone());
        }
    }
    if out.is_empty() {
        return Err("SPEC-INDEX §5.4 capability doc map is empty".to_string());
    }
    Ok(out)
}

fn parse_checker_profile_overlay_claims(text: &str) -> Result<Vec<String>, String> {
    let section = extract_heading_section(text, "2.4")?;
    Ok(parse_backticked_tokens(section)
        .into_iter()
        .filter(|token| token.starts_with("profile."))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn check_profile_overlay_claims(
    capability_registry: &Value,
    checker_claims_text: &str,
) -> Result<Check, String> {
    let registry = string_vec(
        capability_registry
            .get("profileOverlayClaims")
            .unwrap_or(&Value::Array(vec![])),
    );
    let checker_claims = parse_checker_profile_overlay_claims(checker_claims_text)?;
    let registry_set = registry.iter().cloned().collect::<BTreeSet<_>>();
    let checker_claims_set = checker_claims.iter().cloned().collect::<BTreeSet<_>>();
    let missing_in_checker_claims: Vec<String> = registry_set
        .difference(&checker_claims_set)
        .cloned()
        .collect();
    let missing_in_registry: Vec<String> = checker_claims_set
        .difference(&registry_set)
        .cloned()
        .collect();
    let mut reasons = Vec::new();
    if !missing_in_checker_claims.is_empty() || !missing_in_registry.is_empty() {
        reasons.push("CHECKER-CLAIMS §2.4 profile-overlay claims diverge from CAPABILITY-REGISTRY");
    }
    Ok(Check {
        class_id: "profile_overlay_claim_drift",
        failed: !reasons.is_empty(),
        warned: false,
        details: json!({
            "reasons": reasons,
            "registryProfileOverlayClaims": registry,
            "checkerProfileOverlayClaims": checker_claims,
            "missingInCheckerClaims": missing_in_checker_claims,
            "missingInRegistry": missing_in_registry
        }),
    })
}

fn check_spec_index_capability_map(
    spec_index_text: &str,
    capability_registry: &Value,
    coherence_contract: &Value,
) -> Result<Check, String> {
    let spec_map = parse_spec_index_capability_doc_map(spec_index_text)?;
    let executable = string_set(
        capability_registry
            .get("executableCapabilities")
            .unwrap_or(&Value::Array(vec![])),
    );
    let conditional_docs = coherence_contract
        .get("conditionalCapabilityDocs")
        .and_then(Value::as_array)
        .ok_or_else(|| "COHERENCE-CONTRACT conditionalCapabilityDocs must be a list".to_string())?;
    let mut conditional_map = BTreeMap::new();
    for row in conditional_docs {
        let doc_ref = string_value(row.get("docRef"));
        let capability_id = string_value(row.get("capabilityId"));
        if !doc_ref.is_empty() && !capability_id.is_empty() {
            conditional_map.insert(doc_ref, capability_id);
        }
    }

    let mut reasons = Vec::new();
    let unknown_capabilities: Vec<String> = spec_map
        .values()
        .filter(|capability| !executable.contains(*capability))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if !unknown_capabilities.is_empty() {
        reasons.push("spec-index references capabilities not present in CAPABILITY-REGISTRY");
    }

    let mut missing_conditional_docs = Vec::new();
    let mut conditional_mismatches = Vec::new();
    for (doc_ref, expected) in &conditional_map {
        match spec_map.get(doc_ref) {
            Some(actual) if actual == expected => {}
            Some(actual) => conditional_mismatches.push(json!({
                "docRef": doc_ref,
                "expected": expected,
                "actual": actual
            })),
            None => missing_conditional_docs.push(doc_ref.clone()),
        }
    }
    if !missing_conditional_docs.is_empty() || !conditional_mismatches.is_empty() {
        reasons.push("SPEC-INDEX §5.4 conditional capability docs diverge from COHERENCE-CONTRACT");
    }

    Ok(Check {
        class_id: "spec_index_capability_map_drift",
        failed: !reasons.is_empty(),
        warned: false,
        details: json!({
            "reasons": reasons,
            "specIndexCapabilityDocMap": spec_map,
            "conditionalCapabilityDocs": conditional_map,
            "unknownCapabilities": unknown_capabilities,
            "missingConditionalDocs": missing_conditional_docs,
            "conditionalMismatches": conditional_mismatches
        }),
    })
}

fn check_control_plane_lane_bindings(contract: &Value, gate_details: &Value) -> Check {
    let mut reasons = Vec::new();
    let lane_registry = gate_details.get("laneRegistry").unwrap_or(&Value::Null);
    if !lane_registry.is_object() {
        reasons.push("coherence witness missing gate_chain_parity laneRegistry details");
    }

    let contract_evidence_lanes = contract
        .get("evidenceLanes")
        .cloned()
        .unwrap_or(Value::Null);
    let checker_evidence_lanes = lane_registry
        .get("evidenceLanes")
        .cloned()
        .unwrap_or(Value::Null);
    if checker_evidence_lanes.is_object() && checker_evidence_lanes != contract_evidence_lanes {
        reasons.push("coherence checker lane IDs differ from CONTROL-PLANE-CONTRACT evidenceLanes");
    }

    let contract_lane_artifact_kinds =
        normalize_lane_artifact_kinds(contract.get("laneArtifactKinds"));
    let checker_lane_artifact_kinds =
        normalize_lane_artifact_kinds(lane_registry.get("laneArtifactKinds"));
    if !checker_lane_artifact_kinds.is_empty()
        && checker_lane_artifact_kinds != contract_lane_artifact_kinds
    {
        reasons.push("coherence checker laneArtifactKinds differ from CONTROL-PLANE-CONTRACT");
    }

    let contract_checker_core = string_vec(
        contract
            .pointer("/laneOwnership/checkerCoreOnlyObligations")
            .unwrap_or(&Value::Array(vec![])),
    );
    let checker_expected_core = string_vec(
        lane_registry
            .get("expectedCheckerCoreOnlyObligations")
            .unwrap_or(&Value::Array(vec![])),
    );
    if !checker_expected_core.is_empty() && checker_expected_core != contract_checker_core {
        reasons.push(
            "checker expected checker-core-only obligations differ from CONTROL-PLANE-CONTRACT laneOwnership",
        );
    }

    let contract_required_route = string_value(
        contract.pointer("/laneOwnership/requiredCrossLaneWitnessRoute/pullbackBaseChange"),
    );
    let checker_required_route = string_value(lane_registry.get("requiredCrossLaneWitnessRoute"));
    if !checker_required_route.is_empty() && checker_required_route != contract_required_route {
        reasons.push(
            "checker required cross-lane witness route differs from CONTROL-PLANE-CONTRACT laneOwnership",
        );
    }

    let contract_lane_failure_classes = string_set(
        contract
            .get("laneFailureClasses")
            .unwrap_or(&Value::Array(vec![])),
    );
    let checker_required_failures = string_vec(
        lane_registry
            .get("requiredLaneFailureClasses")
            .unwrap_or(&Value::Array(vec![])),
    );
    if !checker_required_failures
        .iter()
        .all(|class_id| contract_lane_failure_classes.contains(class_id))
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT laneFailureClasses missing checker-required failure classes",
        );
    }

    let stage1_parity = gate_details.get("stage1Parity").unwrap_or(&Value::Null);
    let stage1_rollback = gate_details.get("stage1Rollback").unwrap_or(&Value::Null);
    let stage2_authority = gate_details.get("stage2Authority").unwrap_or(&Value::Null);
    compare_sorted_values(
        &mut reasons,
        values_as_sorted_strings(contract.pointer("/evidenceStage1Parity/failureClasses")),
        values_as_sorted_strings(stage1_parity.get("requiredFailureClasses")),
        "CONTROL-PLANE-CONTRACT evidenceStage1Parity.failureClasses differ from checker-required classes",
    );
    compare_sorted_values(
        &mut reasons,
        values_as_sorted_strings(contract.pointer("/evidenceStage1Rollback/failureClasses")),
        values_as_sorted_strings(stage1_rollback.get("requiredFailureClasses")),
        "CONTROL-PLANE-CONTRACT evidenceStage1Rollback.failureClasses differ from checker-required classes",
    );
    let contract_stage1_triggers = string_set(
        contract
            .pointer("/evidenceStage1Rollback/triggerFailureClasses")
            .unwrap_or(&Value::Array(vec![])),
    );
    let checker_stage1_triggers = string_vec(
        stage1_rollback
            .get("requiredTriggerFailureClasses")
            .unwrap_or(&Value::Array(vec![])),
    );
    if !checker_stage1_triggers
        .iter()
        .all(|class_id| contract_stage1_triggers.contains(class_id))
    {
        reasons.push(
            "CONTROL-PLANE-CONTRACT evidenceStage1Rollback.triggerFailureClasses missing checker-required trigger classes",
        );
    }
    compare_sorted_values(
        &mut reasons,
        values_as_sorted_strings(contract.pointer("/evidenceStage2Authority/failureClasses")),
        values_as_sorted_strings(stage2_authority.get("requiredFailureClasses")),
        "CONTROL-PLANE-CONTRACT evidenceStage2Authority.failureClasses differ from checker-required classes",
    );
    compare_sorted_values(
        &mut reasons,
        string_vec(
            contract
                .pointer("/evidenceStage2Authority/coreObligationEvidenceRoute/requiredObligations")
                .unwrap_or(&Value::Array(vec![])),
        ),
        string_vec(
            stage2_authority
                .get("requiredCoreObligationKinds")
                .unwrap_or(&Value::Array(vec![])),
        ),
        "CONTROL-PLANE-CONTRACT evidenceStage2Authority.coreObligationEvidenceRoute.requiredObligations differ from checker-observed values",
    );
    compare_sorted_values(
        &mut reasons,
        values_as_sorted_strings(
            contract.pointer("/evidenceStage2Authority/coreObligationEvidenceRoute/failureClasses"),
        ),
        values_as_sorted_strings(
            stage2_authority.get("requiredCoreObligationEvidenceFailureClasses"),
        ),
        "CONTROL-PLANE-CONTRACT evidenceStage2Authority.coreObligationEvidenceRoute.failureClasses differ from checker-required classes",
    );

    Check {
        class_id: "control_plane_lane_binding_drift",
        failed: !reasons.is_empty(),
        warned: false,
        details: json!({
            "reasons": reasons,
            "contract": {
                "evidenceLanes": contract_evidence_lanes,
                "laneArtifactKinds": contract_lane_artifact_kinds,
                "checkerCoreOnlyObligations": contract_checker_core,
                "requiredCrossLaneWitnessRoute": contract_required_route,
                "laneFailureClasses": string_vec(contract.get("laneFailureClasses").unwrap_or(&Value::Array(vec![]))),
                "stage1": {
                    "parityFailureClasses": values_as_sorted_strings(contract.pointer("/evidenceStage1Parity/failureClasses")),
                    "rollbackTriggerFailureClasses": string_vec(contract.pointer("/evidenceStage1Rollback/triggerFailureClasses").unwrap_or(&Value::Array(vec![]))),
                    "rollbackFailureClasses": values_as_sorted_strings(contract.pointer("/evidenceStage1Rollback/failureClasses"))
                },
                "stage2": {
                    "authorityFailureClasses": values_as_sorted_strings(contract.pointer("/evidenceStage2Authority/failureClasses")),
                    "coreObligationRequiredKinds": string_vec(contract.pointer("/evidenceStage2Authority/coreObligationEvidenceRoute/requiredObligations").unwrap_or(&Value::Array(vec![]))),
                    "coreObligationFailureClasses": values_as_sorted_strings(contract.pointer("/evidenceStage2Authority/coreObligationEvidenceRoute/failureClasses"))
                }
            },
            "checker": {
                "evidenceLanes": checker_evidence_lanes,
                "laneArtifactKinds": checker_lane_artifact_kinds,
                "expectedCheckerCoreOnlyObligations": checker_expected_core,
                "requiredCrossLaneWitnessRoute": checker_required_route,
                "requiredLaneFailureClasses": checker_required_failures,
                "stage1": {
                    "parityRequiredFailureClasses": values_as_sorted_strings(stage1_parity.get("requiredFailureClasses")),
                    "rollbackRequiredTriggerFailureClasses": checker_stage1_triggers,
                    "rollbackRequiredFailureClasses": values_as_sorted_strings(stage1_rollback.get("requiredFailureClasses"))
                },
                "stage2": {
                    "authorityRequiredFailureClasses": values_as_sorted_strings(stage2_authority.get("requiredFailureClasses")),
                    "coreObligationRequiredKinds": string_vec(stage2_authority.get("requiredCoreObligationKinds").unwrap_or(&Value::Array(vec![]))),
                    "coreObligationRequiredFailureClasses": values_as_sorted_strings(stage2_authority.get("requiredCoreObligationEvidenceFailureClasses"))
                }
            }
        }),
    }
}

fn compare_sorted_values(
    reasons: &mut Vec<&'static str>,
    left: Vec<String>,
    right: Vec<String>,
    reason: &'static str,
) {
    if !left.is_empty() && !right.is_empty() && left != right {
        reasons.push(reason);
    }
}

fn check_control_plane_kcir_mappings(contract: &Value) -> Check {
    let mut reasons = Vec::new();
    let mappings = contract
        .get("controlPlaneKcirMappings")
        .and_then(Value::as_object);
    let mut row_errors = Vec::new();
    let mut mapping_table = BTreeMap::new();
    if let Some(mappings) = mappings {
        let profile_id = string_value(mappings.get("profileId"));
        if profile_id.is_empty() {
            reasons.push("CONTROL-PLANE-CONTRACT controlPlaneKcirMappings.profileId is empty");
        }
        if let Some(table) = mappings.get("mappingTable").and_then(Value::as_object) {
            if table.is_empty() {
                reasons
                    .push("CONTROL-PLANE-CONTRACT controlPlaneKcirMappings.mappingTable is empty");
            }
            for (row_id, row) in table {
                let source_kind = string_value(row.get("sourceKind"));
                let target_domain = string_value(row.get("targetDomain"));
                let target_kind = string_value(row.get("targetKind"));
                let identity_fields =
                    string_vec(row.get("identityFields").unwrap_or(&Value::Array(vec![])));
                if source_kind.is_empty()
                    || target_domain.is_empty()
                    || target_kind.is_empty()
                    || identity_fields.is_empty()
                {
                    row_errors.push(row_id.clone());
                }
                mapping_table.insert(
                    row_id.clone(),
                    json!({
                        "sourceKind": source_kind,
                        "targetDomain": target_domain,
                        "targetKind": target_kind,
                        "identityFields": identity_fields
                    }),
                );
            }
        } else {
            reasons.push("CONTROL-PLANE-CONTRACT missing controlPlaneKcirMappings.mappingTable");
        }
        let legacy = mappings
            .get("compatibilityPolicy")
            .and_then(|value| value.get("legacyNonKcirEncodings"));
        for field in ["mode", "authorityMode", "supportUntilEpoch", "failureClass"] {
            if string_value(legacy.and_then(|value| value.get(field))).is_empty() {
                reasons.push(
                    "CONTROL-PLANE-CONTRACT legacy non-KCIR compatibility policy is incomplete",
                );
                break;
            }
        }
    } else {
        reasons.push("CONTROL-PLANE-CONTRACT missing controlPlaneKcirMappings");
    }
    if !row_errors.is_empty() {
        reasons.push("CONTROL-PLANE-CONTRACT controlPlaneKcirMappings rows are incomplete");
    }
    Check {
        class_id: "control_plane_kcir_mapping_drift",
        failed: !reasons.is_empty(),
        warned: false,
        details: json!({
            "reasons": reasons,
            "contractProfileId": string_value(contract.pointer("/controlPlaneKcirMappings/profileId")),
            "contractMappingTable": mapping_table,
            "incompleteRows": row_errors,
            "legacyPolicy": contract.pointer("/controlPlaneKcirMappings/compatibilityPolicy/legacyNonKcirEncodings").cloned().unwrap_or(Value::Null)
        }),
    }
}

fn check_coherence_required_obligations(contract: &Value, scope_details: &Value) -> Check {
    let contract_required: Vec<String> = contract
        .get("obligations")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("id").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_default();
    let checker_required = string_vec(
        scope_details
            .get("requiredCoherenceObligations")
            .unwrap_or(&Value::Array(vec![])),
    );
    let contract_core = string_vec(
        contract
            .get("requiredCoreObligationKinds")
            .unwrap_or(&Value::Array(vec![])),
    );
    let checker_core = string_vec(
        scope_details
            .get("requiredCoreObligationKinds")
            .unwrap_or(&Value::Array(vec![])),
    );
    let contract_registry_kind = string_value(contract.pointer("/surfaces/obligationRegistryKind"));
    let checker_registry_kind = string_value(scope_details.get("obligationRegistryKind"));
    let mut reasons = Vec::new();
    if contract_required != checker_required {
        reasons.push("coherence required obligation set drifts between contract and checker");
    }
    if contract_core != checker_core {
        reasons.push("requiredCoreObligationKinds drifts between contract and checker");
    }
    if contract_registry_kind != checker_registry_kind {
        reasons.push("obligation registry kind drifts between contract and checker");
    }
    Check {
        class_id: "coherence_required_obligation_drift",
        failed: !reasons.is_empty(),
        warned: false,
        details: json!({
            "reasons": reasons,
            "contractRequiredObligations": contract_required,
            "checkerRequiredObligations": checker_required,
            "contractRequiredCoreObligationKinds": contract_core,
            "checkerRequiredCoreObligationKinds": checker_core,
            "contractObligationRegistryKind": contract_registry_kind,
            "checkerObligationRegistryKind": checker_registry_kind
        }),
    }
}

fn check_sigpi_notation(repo_root: &Path) -> Result<Check, String> {
    let mut reasons = Vec::new();
    let mut alias_hits = Vec::new();
    let mut canonical_sigpi_docs = Vec::new();
    let mut canonical_latex_docs = Vec::new();
    for rel in SIGPI_NORMATIVE_DOCS {
        let text = read_text(&repo_root.join(rel))?;
        if text.to_lowercase().contains("sig/pi") {
            alias_hits.push(*rel);
        }
        if text.contains("SigPi") {
            canonical_sigpi_docs.push(*rel);
        }
        if text.contains("sig\\Pi") {
            canonical_latex_docs.push(*rel);
        }
    }
    if !alias_hits.is_empty() {
        reasons.push("normative docs still use Sig/Pi alias");
    }
    if canonical_sigpi_docs.is_empty() {
        reasons.push("normative docs missing canonical SigPi spelling");
    }
    if canonical_latex_docs.is_empty() {
        reasons.push("normative docs missing canonical sig\\Pi notation");
    }
    Ok(Check {
        class_id: "sigpi_notation_drift",
        failed: !reasons.is_empty(),
        warned: false,
        details: json!({
            "reasons": reasons,
            "checkedDocs": SIGPI_NORMATIVE_DOCS,
            "aliasHits": alias_hits,
            "canonicalSigPiDocs": canonical_sigpi_docs,
            "canonicalLatexDocs": canonical_latex_docs
        }),
    })
}

fn check_cache_input_closure(repo_root: &Path, coherence_contract: &Value) -> Check {
    let closure_paths = load_coherence_contract_input_paths(repo_root, coherence_contract);
    let missing: Vec<&str> = CACHE_CLOSURE_REQUIRED_PATHS
        .iter()
        .copied()
        .filter(|rel| !closure_paths.contains(&repo_root.join(rel)))
        .collect();
    let mut reasons = Vec::new();
    if !missing.is_empty() {
        reasons.push("coherence-contract cache input closure missing required loader inputs");
    }
    Check {
        class_id: "coherence_cache_input_closure_drift",
        failed: !reasons.is_empty(),
        warned: false,
        details: json!({
            "reasons": reasons,
            "requiredPaths": CACHE_CLOSURE_REQUIRED_PATHS,
            "missingPaths": missing,
            "closureSize": closure_paths.len()
        }),
    }
}

fn load_coherence_contract_input_paths(repo_root: &Path, contract: &Value) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::from([
        repo_root.join("specs/premath/draft/COHERENCE-CONTRACT.json"),
        repo_root.join("Cargo.toml"),
        repo_root.join("Cargo.lock"),
        repo_root.join("tests/checker/fixtures/coherence-transport"),
        repo_root.join("tests/checker/fixtures/coherence-site"),
        repo_root.join("crates/premath-kernel/src"),
        repo_root.join("crates/premath-coherence/src"),
        repo_root.join("crates/premath-cli/src/commands/coherence_check.rs"),
    ]);
    if let Some(surfaces) = contract.get("surfaces").and_then(Value::as_object) {
        for (key, value) in surfaces {
            if (key.ends_with("Path") || key.ends_with("Root"))
                && value.as_str().is_some_and(|path| !path.trim().is_empty())
            {
                paths.insert(resolve_rooted_path(repo_root, value.as_str().unwrap()));
            }
        }
    }
    if let Some(paths_raw) = contract
        .get("expectedOperationPaths")
        .and_then(Value::as_array)
    {
        for value in paths_raw.iter().filter_map(Value::as_str) {
            paths.insert(resolve_rooted_path(repo_root, value));
        }
    }
    if let Some(docs) = contract.get("overlayDocs").and_then(Value::as_array) {
        for value in docs.iter().filter_map(Value::as_str) {
            if !value.trim().is_empty() {
                paths.insert(resolve_rooted_path(
                    repo_root,
                    format!("specs/premath/{value}.md").as_str(),
                ));
            }
        }
    }
    paths
}

fn resolve_rooted_path(repo_root: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    }
}

fn check_topology_budget(repo_root: &Path, budget_path: &Path) -> Result<Check, String> {
    let contract = load_json(budget_path)?;
    if contract.get("schema").and_then(Value::as_u64) != Some(1) {
        return Err(format!("{}: schema must be 1", budget_path.display()));
    }
    if contract.get("budgetKind").and_then(Value::as_str) != Some(TOPOLOGY_BUDGET_KIND) {
        return Err(format!(
            "{}: budgetKind must be `{TOPOLOGY_BUDGET_KIND}`",
            budget_path.display()
        ));
    }
    let thresholds = contract
        .get("metrics")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{}: metrics must be an object", budget_path.display()))?;
    let metrics = collect_topology_metrics(repo_root, &contract)?;
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();
    let mut details_metrics = Map::new();

    for metric_id in thresholds.keys() {
        if !metrics.contains_key(metric_id) {
            reasons.push(format!("topology metric `{metric_id}` has no evaluator"));
        }
    }
    let unbudgeted_metrics: Vec<String> = metrics
        .keys()
        .filter(|metric_id| !thresholds.contains_key(*metric_id))
        .cloned()
        .collect();
    for (metric_id, value) in metrics {
        let Some(threshold) = thresholds.get(&metric_id).and_then(Value::as_object) else {
            continue;
        };
        let (status, messages) = evaluate_topology_threshold(value, threshold);
        for message in &messages {
            if status == "fail" {
                reasons.push(format!("{metric_id}: {message}"));
            } else if status == "warn" {
                warnings.push(format!("{metric_id}: {message}"));
            }
        }
        details_metrics.insert(
            metric_id,
            json!({
                "value": value,
                "status": status,
                "threshold": threshold,
                "messages": messages
            }),
        );
    }

    Ok(Check {
        class_id: "topology_budget_drift",
        failed: !reasons.is_empty(),
        warned: !warnings.is_empty(),
        details: json!({
            "reasons": reasons,
            "warnings": warnings,
            "budgetPath": budget_path,
            "metrics": details_metrics,
            "unbudgetedMetrics": unbudgeted_metrics
        }),
    })
}

fn collect_topology_metrics(
    repo_root: &Path,
    contract: &Value,
) -> Result<BTreeMap<String, usize>, String> {
    let draft_dir = repo_root.join("specs/premath/draft");
    let design_dir = repo_root.join("docs/design");
    let doctrine_site = load_json(&draft_dir.join("DOCTRINE-SITE.json"))?;
    let doctrine_edges = doctrine_site
        .get("edges")
        .and_then(Value::as_array)
        .ok_or_else(|| "DOCTRINE-SITE.json edges must be a list".to_string())?;
    let authority_inputs = path_list(contract.get("doctrineSiteAuthorityInputs"));
    let generated_views = path_list(contract.get("doctrineSiteGeneratedViews"));
    let deprecated = path_list(contract.get("deprecatedDesignFragments"));
    Ok(BTreeMap::from([
        (
            "draftSpecNodes".to_string(),
            count_promoted_draft_specs(&draft_dir)?,
        ),
        (
            "specTraceabilityRows".to_string(),
            count_traceability_rows(&draft_dir.join("SPEC-TRACEABILITY.md"))?,
        ),
        (
            "designDocNodes".to_string(),
            fs::read_dir(&design_dir)
                .map_err(|err| format!("{}: read_dir failed: {err}", design_dir.display()))?
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
                .filter(|entry| entry.file_name() != "README.md")
                .count(),
        ),
        ("doctrineSiteEdgeCount".to_string(), doctrine_edges.len()),
        (
            "doctrineSiteAuthorityInputCount".to_string(),
            authority_inputs
                .iter()
                .filter(|rel| repo_root.join(rel).exists())
                .count(),
        ),
        (
            "doctrineSiteGeneratedViewCount".to_string(),
            generated_views
                .iter()
                .filter(|rel| repo_root.join(rel).exists())
                .count(),
        ),
        (
            "deprecatedDesignFragmentCount".to_string(),
            deprecated
                .iter()
                .filter(|rel| repo_root.join(rel).exists())
                .count(),
        ),
    ]))
}

fn path_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn count_promoted_draft_specs(draft_dir: &Path) -> Result<usize, String> {
    let mut count = 0;
    for entry in fs::read_dir(draft_dir)
        .map_err(|err| format!("{}: read_dir failed: {err}", draft_dir.display()))?
    {
        let entry = entry
            .map_err(|err| format!("{}: read_dir entry failed: {err}", draft_dir.display()))?;
        let path = entry.path();
        if path.is_dir() || entry.file_name() == "README.md" {
            continue;
        }
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => count += 1,
            Some("md") if frontmatter_status(&path)? == Some("draft".to_string()) => count += 1,
            _ => {}
        }
    }
    Ok(count)
}

fn frontmatter_status(path: &Path) -> Result<Option<String>, String> {
    let text = read_text(path)?;
    if !text.starts_with("---\n") {
        return Ok(None);
    }
    let Some(rest) = text.strip_prefix("---\n") else {
        return Ok(None);
    };
    let Some((frontmatter, _)) = rest.split_once("---\n") else {
        return Ok(None);
    };
    for line in frontmatter.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("status:") {
            return Ok(Some(value.trim().to_string()));
        }
    }
    Ok(None)
}

fn count_traceability_rows(path: &Path) -> Result<usize, String> {
    let text = read_text(path)?;
    let mut in_matrix = false;
    let mut count = 0;
    for line in text.lines() {
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
        if !stripped.starts_with('|') || stripped.starts_with("| Draft spec") {
            continue;
        }
        let cells: Vec<&str> = stripped
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 5 || cells.iter().all(|cell| cell.chars().all(|ch| ch == '-')) {
            continue;
        }
        if cells[0].contains('`') {
            count += 1;
        }
    }
    Ok(count)
}

fn evaluate_topology_threshold(
    value: usize,
    threshold: &Map<String, Value>,
) -> (&'static str, Vec<String>) {
    let fail_above = threshold
        .get("failAbove")
        .and_then(Value::as_u64)
        .map(|v| v as usize);
    let fail_below = threshold
        .get("failBelow")
        .and_then(Value::as_u64)
        .map(|v| v as usize);
    let warn_above = threshold
        .get("warnAbove")
        .and_then(Value::as_u64)
        .map(|v| v as usize);
    let warn_below = threshold
        .get("warnBelow")
        .and_then(Value::as_u64)
        .map(|v| v as usize);
    let mut messages = Vec::new();
    if fail_above.is_some_and(|limit| value > limit) {
        messages.push(format!(
            "value {value} exceeds failAbove {}",
            fail_above.unwrap()
        ));
    }
    if fail_below.is_some_and(|limit| value < limit) {
        messages.push(format!(
            "value {value} is below failBelow {}",
            fail_below.unwrap()
        ));
    }
    if !messages.is_empty() {
        return ("fail", messages);
    }
    if warn_above.is_some_and(|limit| value > limit) {
        messages.push(format!(
            "value {value} exceeds warnAbove {}",
            warn_above.unwrap()
        ));
    }
    if warn_below.is_some_and(|limit| value < limit) {
        messages.push(format!(
            "value {value} is below warnBelow {}",
            warn_below.unwrap()
        ));
    }
    if !messages.is_empty() {
        return ("warn", messages);
    }
    ("ok", messages)
}
