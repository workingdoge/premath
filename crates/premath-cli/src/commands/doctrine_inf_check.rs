use premath_kernel::world_registry::{
    RequiredRouteBinding, parse_operation_route_rows,
    validate_world_route_bindings_with_requirements,
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::PathBuf;

const GOVERNANCE_PROFILE_CLAIM_ID: &str = "profile.doctrine_inf_governance.v0";
const REQUIRED_GUARDRAIL_STAGES: [&str; 3] = ["pre_flight", "input", "output"];
const VALID_OBSERVABILITY_MODES: [&str; 3] = ["dashboard", "internal_processor", "disabled"];
const VALID_RISK_TIERS: [&str; 3] = ["low", "moderate", "high"];
const REQUIRED_EVAL_LINEAGE_FIELDS: [&str; 3] = [
    "datasetLineageRef",
    "graderConfigLineageRef",
    "metricThresholdsRef",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineInfCheckOutput {
    action: &'static str,
    result: String,
    failure_classes: Vec<String>,
}

fn emit_error(failure_class: &str, message: impl Into<String>) -> ! {
    eprintln!("{failure_class}: {}", message.into());
    std::process::exit(2);
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    let raw = value?.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(raw.to_string())
}

fn ensure_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>, String> {
    let Some(rows) = value.and_then(Value::as_array) else {
        return Err(format!("{label} must be a list"));
    };
    let mut out = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let Some(token) = row
            .as_str()
            .map(str::trim)
            .filter(|token| !token.is_empty())
        else {
            return Err(format!("{label}[{idx}] must be a non-empty string"));
        };
        out.push(token.to_string());
    }
    Ok(out)
}

fn evaluate_boundary_artifacts(case: &Map<String, Value>) -> Result<BTreeSet<String>, String> {
    let artifacts = case
        .get("artifacts")
        .and_then(Value::as_object)
        .ok_or_else(|| "artifacts must be an object".to_string())?;
    let registry = ensure_string_list(
        artifacts.get("doctrineRegistry"),
        "artifacts.doctrineRegistry",
    )?;
    if registry.is_empty() {
        return Err("artifacts.doctrineRegistry must be non-empty".to_string());
    }
    let registry_set = registry.into_iter().collect::<HashSet<_>>();

    let declares = artifacts
        .get("destinationDeclares")
        .and_then(Value::as_object)
        .ok_or_else(|| "artifacts.destinationDeclares must be an object".to_string())?;
    let preserved = ensure_string_list(
        declares.get("preserved"),
        "artifacts.destinationDeclares.preserved",
    )?
    .into_iter()
    .collect::<HashSet<_>>();
    let not_preserved = ensure_string_list(
        declares.get("notPreserved"),
        "artifacts.destinationDeclares.notPreserved",
    )?
    .into_iter()
    .collect::<HashSet<_>>();
    let edge_morphisms =
        ensure_string_list(artifacts.get("edgeMorphisms"), "artifacts.edgeMorphisms")?;

    let mut failures = BTreeSet::new();

    if !preserved.is_disjoint(&not_preserved) {
        failures.insert("doctrine_declaration_overlap".to_string());
    }
    if !(preserved
        .union(&not_preserved)
        .all(|item| registry_set.contains(item)))
    {
        failures.insert("doctrine_unknown_morphism".to_string());
    }
    if edge_morphisms
        .iter()
        .any(|item| !registry_set.contains(item))
    {
        failures.insert("doctrine_unknown_morphism".to_string());
    }

    for morphism in &edge_morphisms {
        if not_preserved.contains(morphism) {
            failures.insert("doctrine_boundary_not_preserved".to_string());
        } else if !preserved.contains(morphism) {
            failures.insert("doctrine_boundary_not_declared_preserved".to_string());
        }
    }

    Ok(failures)
}

fn evaluate_governance_profile(profile: &Map<String, Value>) -> Result<BTreeSet<String>, String> {
    let claim_id = non_empty_string(profile.get("claimId"))
        .ok_or_else(|| "governanceProfile.claimId must be a non-empty string".to_string())?;
    if claim_id != GOVERNANCE_PROFILE_CLAIM_ID {
        return Err(format!(
            "governanceProfile.claimId must be {GOVERNANCE_PROFILE_CLAIM_ID:?}"
        ));
    }
    let claimed = profile
        .get("claimed")
        .and_then(Value::as_bool)
        .ok_or_else(|| "governanceProfile.claimed must be a boolean".to_string())?;
    if !claimed {
        return Ok(BTreeSet::new());
    }

    let mut failures = BTreeSet::new();

    let policy = profile
        .get("policyProvenance")
        .and_then(Value::as_object)
        .ok_or_else(|| "governanceProfile.policyProvenance must be an object".to_string())?;
    let pinned = policy
        .get("pinned")
        .and_then(Value::as_bool)
        .ok_or_else(|| "governanceProfile.policyProvenance.pinned must be a boolean".to_string())?;
    let package_ref = non_empty_string(policy.get("packageRef"));
    let expected_digest = non_empty_string(policy.get("expectedDigest"));
    let bound_digest = non_empty_string(policy.get("boundDigest"));
    if !pinned || package_ref.is_none() || expected_digest.is_none() || bound_digest.is_none() {
        failures.insert("governance.policy_package_unpinned".to_string());
    }
    if let (Some(expected), Some(bound)) = (expected_digest, bound_digest)
        && expected != bound
    {
        failures.insert("governance.policy_package_mismatch".to_string());
    }

    let stages = ensure_string_list(
        profile.get("guardrailStages"),
        "governanceProfile.guardrailStages",
    )?;
    if REQUIRED_GUARDRAIL_STAGES
        .iter()
        .any(|required| !stages.iter().any(|stage| stage == required))
    {
        failures.insert("governance.guardrail_stage_missing".to_string());
    } else {
        let expected = REQUIRED_GUARDRAIL_STAGES
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        if stages != expected {
            failures.insert("governance.guardrail_stage_order_invalid".to_string());
        }
    }

    let eval_gate = profile
        .get("evalGate")
        .and_then(Value::as_object)
        .ok_or_else(|| "governanceProfile.evalGate must be an object".to_string())?;
    if eval_gate.get("passed").and_then(Value::as_bool) != Some(true) {
        failures.insert("governance.eval_gate_unmet".to_string());
    }

    let eval_evidence = profile
        .get("evalEvidence")
        .and_then(Value::as_object)
        .ok_or_else(|| "governanceProfile.evalEvidence must be an object".to_string())?;
    for field in REQUIRED_EVAL_LINEAGE_FIELDS {
        if non_empty_string(eval_evidence.get(field)).is_none() {
            failures.insert("governance.eval_lineage_missing".to_string());
        }
    }

    let observability_mode = non_empty_string(profile.get("observabilityMode"));
    if !observability_mode
        .as_deref()
        .map(|mode| VALID_OBSERVABILITY_MODES.contains(&mode))
        .unwrap_or(false)
    {
        failures.insert("governance.trace_mode_violation".to_string());
    }

    let risk_tier = profile
        .get("riskTier")
        .and_then(Value::as_object)
        .ok_or_else(|| "governanceProfile.riskTier must be an object".to_string())?;
    let tier = non_empty_string(risk_tier.get("tier"));
    let bound = risk_tier
        .get("controlProfileBound")
        .and_then(Value::as_bool)
        == Some(true);
    if !tier
        .as_deref()
        .map(|value| VALID_RISK_TIERS.contains(&value))
        .unwrap_or(false)
        || !bound
    {
        failures.insert("governance.risk_tier_profile_missing".to_string());
    }

    let self_evolution = profile
        .get("selfEvolution")
        .and_then(Value::as_object)
        .ok_or_else(|| "governanceProfile.selfEvolution must be an object".to_string())?;
    let max_attempts = self_evolution.get("maxAttempts").and_then(Value::as_i64);
    if !max_attempts.map(|value| value >= 1).unwrap_or(false) {
        failures.insert("governance.self_evolution_retry_missing".to_string());
    }
    if non_empty_string(self_evolution.get("terminalEscalation")).is_none() {
        failures.insert("governance.self_evolution_escalation_missing".to_string());
    }
    if non_empty_string(self_evolution.get("rollbackRef")).is_none() {
        failures.insert("governance.self_evolution_rollback_missing".to_string());
    }

    Ok(failures)
}

fn evaluate_route_consolidation(
    route_consolidation: &Map<String, Value>,
) -> Result<BTreeSet<String>, String> {
    let site_input = route_consolidation
        .get("siteInput")
        .ok_or_else(|| "routeConsolidation.siteInput must be present".to_string())?;
    let operations_raw = route_consolidation
        .get("operations")
        .ok_or_else(|| "routeConsolidation.operations must be present".to_string())?;
    let operations = parse_operation_route_rows(operations_raw)
        .map_err(|err| format!("routeConsolidation.operations invalid: {err}"))?;

    let required_route_families = route_consolidation
        .get("requiredRouteFamilies")
        .map(|value| ensure_string_list(Some(value), "routeConsolidation.requiredRouteFamilies"))
        .transpose()?
        .unwrap_or_default();

    let required_route_bindings = route_consolidation
        .get("requiredRouteBindings")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .enumerate()
                .map(|(idx, row)| {
                    let obj = row.as_object().ok_or_else(|| {
                        format!("routeConsolidation.requiredRouteBindings[{idx}] must be an object")
                    })?;
                    let route_family_id = non_empty_string(obj.get("routeFamilyId")).ok_or_else(|| {
                        format!(
                            "routeConsolidation.requiredRouteBindings[{idx}].routeFamilyId must be a non-empty string"
                        )
                    })?;
                    let operation_ids = ensure_string_list(
                        obj.get("operationIds"),
                        &format!("routeConsolidation.requiredRouteBindings[{idx}].operationIds"),
                    )?;
                    Ok(RequiredRouteBinding {
                        route_family_id,
                        operation_ids,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();

    let report = validate_world_route_bindings_with_requirements(
        site_input,
        &operations,
        &required_route_families,
        &required_route_bindings,
    );
    Ok(report.failure_classes.into_iter().collect())
}

fn evaluate_case(case: &Map<String, Value>) -> Result<DoctrineInfCheckOutput, String> {
    let mut failures = evaluate_boundary_artifacts(case)?;

    if let Some(governance_profile) = case.get("governanceProfile") {
        let Some(profile) = governance_profile.as_object() else {
            return Err("governanceProfile must be an object when provided".to_string());
        };
        failures.extend(evaluate_governance_profile(profile)?);
    }

    if let Some(route_consolidation) = case.get("routeConsolidation") {
        let Some(route_consolidation) = route_consolidation.as_object() else {
            return Err("routeConsolidation must be an object when provided".to_string());
        };
        failures.extend(evaluate_route_consolidation(route_consolidation)?);
    }

    let failure_classes = failures.into_iter().collect::<Vec<_>>();
    Ok(DoctrineInfCheckOutput {
        action: "doctrine-inf-check",
        result: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes,
    })
}

pub fn run(input: String, json_output: bool) {
    let input_path = PathBuf::from(&input);
    let bytes = fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(
            "doctrine_inf_check_invalid",
            format!(
                "failed to read doctrine-inf input {}: {err}",
                input_path.display()
            ),
        )
    });
    let case_value: Value = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(
            "doctrine_inf_check_invalid",
            format!(
                "failed to parse doctrine-inf input json {}: {err}",
                input_path.display()
            ),
        )
    });
    let Some(case) = case_value.as_object() else {
        emit_error("doctrine_inf_check_invalid", "input root must be an object");
    };

    let output = evaluate_case(case).unwrap_or_else(|err| {
        emit_error("doctrine_inf_check_invalid", err);
    });

    if json_output {
        let rendered = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
            emit_error(
                "doctrine_inf_check_invalid",
                format!("failed to render doctrine-inf output: {err}"),
            )
        });
        println!("{rendered}");
        return;
    }

    if output.failure_classes.is_empty() {
        println!("premath doctrine-inf-check: ACCEPT");
        return;
    }
    println!(
        "premath doctrine-inf-check: REJECT (failureClasses={})",
        output.failure_classes.len()
    );
    for class in output.failure_classes {
        println!("  - {class}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preserved_boundary_accepts() {
        let case = json!({
            "artifacts": {
                "doctrineRegistry": ["dm.identity", "dm.refine.context"],
                "destinationDeclares": {
                    "preserved": ["dm.identity", "dm.refine.context"],
                    "notPreserved": []
                },
                "edgeMorphisms": ["dm.identity"]
            }
        });
        let output = evaluate_case(case.as_object().expect("object")).expect("evaluated");
        assert_eq!(output.result, "accepted");
        assert!(output.failure_classes.is_empty());
    }

    #[test]
    fn route_consolidation_missing_binding_rejects() {
        let case = json!({
            "artifacts": {
                "doctrineRegistry": ["dm.identity"],
                "destinationDeclares": {
                    "preserved": ["dm.identity"],
                    "notPreserved": []
                },
                "edgeMorphisms": ["dm.identity"]
            },
            "routeConsolidation": {
                "siteInput": {
                    "worldRouteBindings": {
                        "schema": 1,
                        "bindingKind": "premath.world_route_bindings.v1",
                        "rows": [{
                            "routeFamilyId": "route.transport.dispatch",
                            "operationIds": ["op/transport.other"],
                            "worldId": "world.transport.v1",
                            "morphismRowId": "wm.control.transport.dispatch",
                            "requiredMorphisms": ["dm.transport.world"],
                            "failureClassUnbound": "world_route_unbound"
                        }]
                    }
                },
                "operations": [{
                    "operationId": "op/transport.world_route_binding",
                    "morphisms": ["dm.transport.world"]
                }],
                "requiredRouteBindings": [{
                    "routeFamilyId": "route.transport.dispatch",
                    "operationIds": ["op/transport.world_route_binding"]
                }]
            }
        });
        let output = evaluate_case(case.as_object().expect("object")).expect("evaluated");
        assert_eq!(output.result, "rejected");
        assert!(
            output
                .failure_classes
                .contains(&"world_route_unbound".to_string())
        );
    }
}
