//! Deterministic runtime orchestration check semantics.

use crate::world_registry::{
    RequiredRouteBinding, ValidationIssue, ValidationReport, WorldRouteBindingRow,
    failure_class as world_failure_class, parse_operation_route_rows,
    parse_world_route_binding_rows, validate_world_route_bindings_with_requirements,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub mod failure_class {
    pub const ROUTE_MISSING: &str = "runtime_route_missing";
    pub const MORPHISM_DRIFT: &str = "runtime_route_morphism_drift";
    pub const CONTRACT_UNBOUND: &str = "runtime_route_contract_unbound";
    pub const KCIR_MAPPING_CONTRACT_VIOLATION: &str = "kcir_mapping_contract_violation";
}

pub const RUNTIME_ORCHESTRATION_SCHEMA: u32 = 1;
pub const RUNTIME_ORCHESTRATION_CHECK_KIND: &str = "conformance.runtime_orchestration.v1";

const REQUIRED_HANDOFF_HEADING: &str = "## 1.2 Harness-Squeak composition boundary (required)";
const REQUIRED_HANDOFF_TOKENS: [&str; 4] = [
    "Harness computes deterministic work context and witness lineage refs.",
    "Squeak performs transport/runtime-placement mapping",
    "Destination Tusk/Gate performs destination-local admissibility checks",
    "Harness records the resulting references in session/trajectory projections.",
];

const REQUIRED_KCIR_MAPPING_ROWS: [&str; 7] = [
    "instructionEnvelope",
    "proposalPayload",
    "coherenceCheckPayload",
    "requiredDecisionInput",
    "coherenceObligations",
    "doctrineRouteBinding",
    "fiberLifecycleAction",
];

const REQUIRED_KCIR_MAPPING_ROW_FIELDS: [&str; 3] = ["sourceKind", "targetDomain", "targetKind"];

const REQUIRED_PHASE3_COMMAND_SURFACES: [(&str, &[&str]); 2] = [
    (
        "governancePromotionCheck",
        &[
            "cargo",
            "run",
            "--package",
            "premath-cli",
            "--",
            "governance-promotion-check",
        ],
    ),
    (
        "kcirMappingCheck",
        &[
            "cargo",
            "run",
            "--package",
            "premath-cli",
            "--",
            "kcir-mapping-check",
        ],
    ),
];

const CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX: &str = "controlPlaneContract";
const REQUIRED_WORLD_ROUTE_FAMILIES: [&str; 7] = [
    "route.gate_execution",
    "route.instruction_execution",
    "route.required_decision_attestation",
    "route.fiber.lifecycle",
    "route.issue_claim_lease",
    "route.session_projection",
    "route.transport.dispatch",
];

const REQUIRED_WORLD_ROUTE_ACTION_BINDINGS: [(&str, &[&str]); 4] = [
    ("route.instruction_execution", &["instruction.run"]),
    (
        "route.required_decision_attestation",
        &["required.witness_verify", "required.witness_decide"],
    ),
    (
        "route.fiber.lifecycle",
        &["fiber.spawn", "fiber.join", "fiber.cancel"],
    ),
    (
        "route.issue_claim_lease",
        &[
            "issue.claim_next",
            "issue.claim",
            "issue.lease_renew",
            "issue.lease_release",
            "issue.discover",
        ],
    ),
];

const REQUIRED_WORLD_ROUTE_STATIC_BINDINGS: [(&str, &[&str]); 1] = [(
    "route.transport.dispatch",
    &["op/transport.world_route_binding"],
)];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOrchestrationSummary {
    pub required_routes: usize,
    pub checked_routes: usize,
    pub checked_kcir_mapping_rows: usize,
    pub checked_phase3_command_surfaces: usize,
    pub checked_world_route_families: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRouteCheckRow {
    pub route_id: String,
    pub operation_id: String,
    pub operation_path: String,
    pub status: String,
    pub required_morphisms: Vec<String>,
    pub actual_morphisms: Vec<String>,
    pub missing_morphisms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KcirMappingCheckRow {
    pub row_id: String,
    pub status: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phase3CommandSurfaceCheckRow {
    pub surface_id: String,
    pub status: String,
    pub expected_entrypoint: Vec<String>,
    pub actual_entrypoint: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldRouteBindingCheckRow {
    pub route_family_id: String,
    pub world_id: String,
    pub morphism_row_id: String,
    pub operation_ids: Vec<String>,
    pub required_morphisms: Vec<String>,
    pub status: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOrchestrationReport {
    pub schema: u32,
    pub check_kind: String,
    pub result: String,
    pub failure_classes: Vec<String>,
    pub summary: RuntimeOrchestrationSummary,
    pub routes: Vec<RuntimeRouteCheckRow>,
    pub kcir_mapping_rows: Vec<KcirMappingCheckRow>,
    pub phase3_command_surfaces: Vec<Phase3CommandSurfaceCheckRow>,
    pub world_route_bindings: Vec<WorldRouteBindingCheckRow>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
struct RuntimeRouteRequirement {
    operation_id: String,
    required_morphisms: Vec<String>,
}

#[derive(Debug, Clone)]
struct RegistryOperation {
    path: String,
    morphisms: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct DerivedWorldRequirements {
    families: Vec<String>,
    bindings: Vec<RequiredRouteBinding>,
}

pub fn evaluate_runtime_orchestration(
    control_plane_contract: &Value,
    operation_registry: &Value,
    harness_runtime_text: &str,
    doctrine_site_input: Option<&Value>,
) -> RuntimeOrchestrationReport {
    let mut errors: Vec<String> = Vec::new();
    let mut failure_classes: BTreeSet<String> = BTreeSet::new();
    let mut route_rows: Vec<RuntimeRouteCheckRow> = Vec::new();
    let mut world_rows: Vec<WorldRouteBindingCheckRow> = Vec::new();

    let runtime_routes = match extract_runtime_routes(control_plane_contract) {
        Ok(routes) => routes,
        Err(err) => {
            errors.push(err);
            failure_classes.insert(failure_class::CONTRACT_UNBOUND.to_string());
            BTreeMap::new()
        }
    };

    let registry_operations = match extract_registry_operations(operation_registry) {
        Ok(operations) => operations,
        Err(err) => {
            errors.push(err);
            failure_classes.insert(failure_class::CONTRACT_UNBOUND.to_string());
            BTreeMap::new()
        }
    };

    let handoff_errors = check_handoff_contract(harness_runtime_text);
    if !handoff_errors.is_empty() {
        errors.extend(handoff_errors);
        failure_classes.insert(failure_class::CONTRACT_UNBOUND.to_string());
    }

    let (mapping_errors, mapping_rows) = check_kcir_mapping_rows(control_plane_contract);
    if !mapping_errors.is_empty() {
        errors.extend(mapping_errors);
        failure_classes.insert(failure_class::KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
    }

    let (command_errors, command_rows) = check_phase3_command_surfaces(control_plane_contract);
    if !command_errors.is_empty() {
        errors.extend(command_errors);
        failure_classes.insert(failure_class::CONTRACT_UNBOUND.to_string());
    }

    if let Some(site_input) = doctrine_site_input {
        let (world_errors, checked_world_rows, world_failure_classes) =
            check_world_route_bindings(site_input, operation_registry, control_plane_contract);
        if !world_errors.is_empty() {
            errors.extend(world_errors);
            failure_classes.extend(world_failure_classes);
        }
        world_rows = checked_world_rows;
    }

    for (route_id, route) in runtime_routes {
        let operation_id = route.operation_id;
        let required_morphisms = route.required_morphisms;
        let Some(operation_row) = registry_operations.get(operation_id.as_str()) else {
            errors.push(format!(
                "missing runtime route operation in DOCTRINE-OP-REGISTRY: {}",
                operation_id
            ));
            failure_classes.insert(failure_class::ROUTE_MISSING.to_string());
            route_rows.push(RuntimeRouteCheckRow {
                route_id,
                operation_id,
                operation_path: String::new(),
                status: "missing_operation".to_string(),
                required_morphisms: required_morphisms.clone(),
                actual_morphisms: Vec::new(),
                missing_morphisms: required_morphisms,
            });
            continue;
        };

        let actual_morphisms = operation_row.morphisms.clone();
        let operation_path = operation_row.path.clone();
        let mut status_fragments: Vec<String> = Vec::new();

        if !operation_path.starts_with("tools/ci/") {
            errors.push(format!(
                "runtime route {} operation path outside canonical CI adapter boundary: {:?}",
                route_id, operation_path
            ));
            failure_classes.insert(failure_class::CONTRACT_UNBOUND.to_string());
            status_fragments.push("path_unbound".to_string());
        }

        let actual_set: BTreeSet<String> = actual_morphisms.iter().cloned().collect();
        let missing_morphisms: Vec<String> = required_morphisms
            .iter()
            .filter(|morphism| !actual_set.contains(*morphism))
            .cloned()
            .collect();
        if !missing_morphisms.is_empty() {
            errors.push(format!(
                "runtime route {} missing morphisms on {}: {}",
                route_id,
                operation_id,
                missing_morphisms.join(", ")
            ));
            failure_classes.insert(failure_class::MORPHISM_DRIFT.to_string());
            status_fragments.push("missing_morphisms".to_string());
        }

        route_rows.push(RuntimeRouteCheckRow {
            route_id,
            operation_id,
            operation_path,
            status: if status_fragments.is_empty() {
                "ok".to_string()
            } else {
                status_fragments.join("+")
            },
            required_morphisms,
            actual_morphisms,
            missing_morphisms,
        });
    }

    route_rows.sort_by(|left, right| left.route_id.cmp(&right.route_id));

    RuntimeOrchestrationReport {
        schema: RUNTIME_ORCHESTRATION_SCHEMA,
        check_kind: RUNTIME_ORCHESTRATION_CHECK_KIND.to_string(),
        result: if errors.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: failure_classes.into_iter().collect(),
        summary: RuntimeOrchestrationSummary {
            required_routes: route_rows.len(),
            checked_routes: route_rows.len(),
            checked_kcir_mapping_rows: mapping_rows.len(),
            checked_phase3_command_surfaces: command_rows.len(),
            checked_world_route_families: world_rows.len(),
            errors: errors.len(),
        },
        routes: route_rows,
        kcir_mapping_rows: mapping_rows,
        phase3_command_surfaces: command_rows,
        world_route_bindings: world_rows,
        errors,
    }
}

fn check_handoff_contract(harness_runtime_text: &str) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    if !harness_runtime_text.contains(REQUIRED_HANDOFF_HEADING) {
        errors.push(
            "HARNESS-RUNTIME missing required Harness-Squeak composition boundary heading"
                .to_string(),
        );
    }
    let mut missing_tokens: Vec<&str> = Vec::new();
    for token in REQUIRED_HANDOFF_TOKENS {
        if !harness_runtime_text.contains(token) {
            missing_tokens.push(token);
        }
    }
    if !missing_tokens.is_empty() {
        errors.push(format!(
            "HARNESS-RUNTIME missing required handoff tokens: {}",
            missing_tokens.join(", ")
        ));
    }
    errors
}

fn check_kcir_mapping_rows(
    control_plane_contract: &Value,
) -> (Vec<String>, Vec<KcirMappingCheckRow>) {
    let mut errors: Vec<String> = Vec::new();
    let mut rows: Vec<KcirMappingCheckRow> = Vec::new();

    let Some(contract_obj) = control_plane_contract.as_object() else {
        return (
            vec!["controlPlaneContract must be an object".to_string()],
            rows,
        );
    };

    let Some(mappings) = contract_obj.get("controlPlaneKcirMappings") else {
        return (errors, rows);
    };
    let Some(mappings_obj) = mappings.as_object() else {
        return (
            vec!["controlPlaneKcirMappings must be an object when provided".to_string()],
            rows,
        );
    };

    let Some(mapping_table) = mappings_obj.get("mappingTable") else {
        return (
            vec!["controlPlaneKcirMappings.mappingTable must be an object".to_string()],
            rows,
        );
    };
    let Some(mapping_table_obj) = mapping_table.as_object() else {
        return (
            vec!["controlPlaneKcirMappings.mappingTable must be an object".to_string()],
            rows,
        );
    };

    for row_id in REQUIRED_KCIR_MAPPING_ROWS {
        let mut row_errors: Vec<String> = Vec::new();
        let Some(row_value) = mapping_table_obj.get(row_id) else {
            row_errors.push("missing row".to_string());
            rows.push(KcirMappingCheckRow {
                row_id: row_id.to_string(),
                status: "missing".to_string(),
                errors: row_errors,
            });
            errors.push(format!(
                "controlPlaneKcirMappings.mappingTable missing required row: {}",
                row_id
            ));
            continue;
        };

        let Some(row_obj) = row_value.as_object() else {
            row_errors.push("missing row".to_string());
            rows.push(KcirMappingCheckRow {
                row_id: row_id.to_string(),
                status: "missing".to_string(),
                errors: row_errors,
            });
            errors.push(format!(
                "controlPlaneKcirMappings.mappingTable missing required row: {}",
                row_id
            ));
            continue;
        };

        for field in REQUIRED_KCIR_MAPPING_ROW_FIELDS {
            if non_empty_string(row_obj.get(field)).is_none() {
                row_errors.push(format!("missing field {}", field));
            }
        }

        let Some(identity_fields) = row_obj.get("identityFields").and_then(Value::as_array) else {
            row_errors.push("identityFields must be a non-empty list".to_string());
            rows.push(KcirMappingCheckRow {
                row_id: row_id.to_string(),
                status: "invalid".to_string(),
                errors: row_errors.clone(),
            });
            errors.push(format!(
                "controlPlaneKcirMappings.mappingTable.{} invalid: {}",
                row_id,
                row_errors.join(", ")
            ));
            continue;
        };

        if identity_fields.is_empty() {
            row_errors.push("identityFields must be a non-empty list".to_string());
        } else {
            for (idx, value) in identity_fields.iter().enumerate() {
                if non_empty_string(Some(value)).is_none() {
                    row_errors.push(format!("identityFields[{idx}] must be a non-empty string"));
                }
            }
        }

        let status = if row_errors.is_empty() {
            "ok".to_string()
        } else {
            "invalid".to_string()
        };
        rows.push(KcirMappingCheckRow {
            row_id: row_id.to_string(),
            status,
            errors: row_errors.clone(),
        });
        if !row_errors.is_empty() {
            errors.push(format!(
                "controlPlaneKcirMappings.mappingTable.{} invalid: {}",
                row_id,
                row_errors.join(", ")
            ));
        }
    }

    (errors, rows)
}

fn check_phase3_command_surfaces(
    control_plane_contract: &Value,
) -> (Vec<String>, Vec<Phase3CommandSurfaceCheckRow>) {
    let mut rows: Vec<Phase3CommandSurfaceCheckRow> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let contract_obj = control_plane_contract.as_object();
    let mut command_surface_obj: Option<&Map<String, Value>> = None;

    match contract_obj.and_then(|obj| obj.get("commandSurface")) {
        Some(raw) if raw.is_object() => {
            command_surface_obj = raw.as_object();
        }
        Some(_) => {
            errors.push("commandSurface must be an object".to_string());
        }
        None => {
            errors.push(
                "commandSurface is required for phase-3 command-surface parity checks".to_string(),
            );
        }
    }

    for (surface_id, expected_tokens) in REQUIRED_PHASE3_COMMAND_SURFACES {
        let mut row_errors: Vec<String> = Vec::new();
        let mut actual_tokens: Vec<String> = Vec::new();
        let status =
            if let Some(surface) = command_surface_obj.and_then(|obj| obj.get(surface_id)) {
                if let Some(surface_obj) = surface.as_object() {
                    match surface_obj.get("canonicalEntrypoint") {
                        Some(Value::Array(values)) if !values.is_empty() => {
                            for (idx, value) in values.iter().enumerate() {
                                if let Some(token) = non_empty_string(Some(value)) {
                                    actual_tokens.push(token);
                                } else {
                                    row_errors.push(format!(
                                        "canonicalEntrypoint[{idx}] must be a non-empty string"
                                    ));
                                }
                            }
                            if row_errors.is_empty()
                                && actual_tokens
                                    != expected_tokens
                                        .iter()
                                        .map(|token| token.to_string())
                                        .collect::<Vec<_>>()
                            {
                                row_errors.push("canonicalEntrypoint mismatch".to_string());
                            }
                        }
                        _ => row_errors
                            .push("canonicalEntrypoint must be a non-empty list".to_string()),
                    }
                    if row_errors.is_empty() {
                        "ok".to_string()
                    } else {
                        "invalid".to_string()
                    }
                } else {
                    row_errors.push("missing row".to_string());
                    "missing".to_string()
                }
            } else {
                row_errors.push("missing row".to_string());
                "missing".to_string()
            };

        if !row_errors.is_empty() {
            errors.push(format!(
                "commandSurface.{} invalid: {}",
                surface_id,
                row_errors.join(", ")
            ));
        }

        rows.push(Phase3CommandSurfaceCheckRow {
            surface_id: surface_id.to_string(),
            status,
            expected_entrypoint: expected_tokens
                .iter()
                .map(|token| token.to_string())
                .collect(),
            actual_entrypoint: actual_tokens,
            errors: row_errors,
        });
    }

    (errors, rows)
}

fn check_world_route_bindings(
    doctrine_site_input: &Value,
    operation_registry: &Value,
    control_plane_contract: &Value,
) -> (
    Vec<String>,
    Vec<WorldRouteBindingCheckRow>,
    BTreeSet<String>,
) {
    let mut errors: Vec<String> = Vec::new();
    let mut failure_classes: BTreeSet<String> = BTreeSet::new();

    let operations = match parse_operation_route_rows(operation_registry) {
        Ok(rows) => rows,
        Err(err) => {
            errors.push(format!("worldRouteBindings kernel check failed: {err}"));
            failure_classes.insert(world_failure_class::WORLD_ROUTE_UNBOUND.to_string());
            return (errors, Vec::new(), failure_classes);
        }
    };

    let (requirements, contract_issues) =
        derive_required_world_requirements_from_control_plane(control_plane_contract);
    let report = validate_world_route_bindings_with_requirements(
        doctrine_site_input,
        &operations,
        &requirements.families,
        &requirements.bindings,
    );
    let report = merge_report_with_contract_issues(report, contract_issues);

    for issue in &report.issues {
        let path = issue.path.trim();
        let message = issue.message.trim();
        if path.is_empty() || message.is_empty() {
            continue;
        }
        errors.push(format!("{path}: {message}"));
        if path.starts_with("controlPlaneContract.") {
            failure_classes.insert(failure_class::CONTRACT_UNBOUND.to_string());
        }
    }
    failure_classes.extend(report.failure_classes.iter().cloned());

    let world_rows = parse_world_route_binding_rows(doctrine_site_input)
        .map(|rows| project_world_route_rows(&rows, &report.issues))
        .unwrap_or_default();

    (errors, world_rows, failure_classes)
}

fn derive_required_world_requirements_from_control_plane(
    control_plane_contract: &Value,
) -> (DerivedWorldRequirements, Vec<ValidationIssue>) {
    let mut route_families: BTreeSet<String> = REQUIRED_WORLD_ROUTE_FAMILIES
        .iter()
        .map(|family| family.to_string())
        .collect();
    let mut route_bindings: BTreeMap<String, BTreeSet<String>> = route_families
        .iter()
        .map(|family| (family.clone(), BTreeSet::new()))
        .collect();
    let mut issues: Vec<ValidationIssue> = Vec::new();

    for (route_family_id, operation_ids) in REQUIRED_WORLD_ROUTE_STATIC_BINDINGS {
        route_families.insert(route_family_id.to_string());
        let route_entry = route_bindings
            .entry(route_family_id.to_string())
            .or_default();
        for operation_id in operation_ids {
            route_entry.insert((*operation_id).to_string());
        }
    }

    let Some(contract_obj) = control_plane_contract.as_object() else {
        issues.push(control_plane_contract_issue(
            CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX.to_string(),
            "must be an object",
        ));
        return (
            DerivedWorldRequirements {
                families: route_families.into_iter().collect(),
                bindings: route_bindings_to_rows(route_bindings),
            },
            issues,
        );
    };

    if let Ok(runtime_routes) = extract_runtime_routes(control_plane_contract) {
        let gate_entry = route_bindings
            .entry("route.gate_execution".to_string())
            .or_default();
        for route in runtime_routes.values() {
            gate_entry.insert(route.operation_id.clone());
        }
    }

    let Some(host_action_surface) = contract_obj.get("hostActionSurface") else {
        issues.push(control_plane_contract_issue(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface"),
            "missing required object",
        ));
        return (
            DerivedWorldRequirements {
                families: route_families.into_iter().collect(),
                bindings: route_bindings_to_rows(route_bindings),
            },
            issues,
        );
    };
    let Some(host_action_surface_obj) = host_action_surface.as_object() else {
        issues.push(control_plane_contract_issue(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface"),
            "must be an object",
        ));
        return (
            DerivedWorldRequirements {
                families: route_families.into_iter().collect(),
                bindings: route_bindings_to_rows(route_bindings),
            },
            issues,
        );
    };

    let Some(required_actions) = host_action_surface_obj.get("requiredActions") else {
        issues.push(control_plane_contract_issue(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions"),
            "missing required object",
        ));
        return (
            DerivedWorldRequirements {
                families: route_families.into_iter().collect(),
                bindings: route_bindings_to_rows(route_bindings),
            },
            issues,
        );
    };
    let Some(required_actions_obj) = required_actions.as_object() else {
        issues.push(control_plane_contract_issue(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions"),
            "must be an object",
        ));
        return (
            DerivedWorldRequirements {
                families: route_families.into_iter().collect(),
                bindings: route_bindings_to_rows(route_bindings),
            },
            issues,
        );
    };

    for (route_family_id, host_action_ids) in REQUIRED_WORLD_ROUTE_ACTION_BINDINGS {
        route_families.insert(route_family_id.to_string());
        let route_entry = route_bindings
            .entry(route_family_id.to_string())
            .or_default();
        for host_action_id in host_action_ids {
            let Some(action_row) = required_actions_obj.get(*host_action_id) else {
                issues.push(control_plane_contract_issue(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions.{host_action_id}"
                    ),
                    "missing required host-action row",
                ));
                continue;
            };
            let Some(action_obj) = action_row.as_object() else {
                issues.push(control_plane_contract_issue(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions.{host_action_id}"
                    ),
                    "must be an object",
                ));
                continue;
            };
            let Some(operation_id) = non_empty_string(action_obj.get("operationId")) else {
                issues.push(control_plane_contract_issue(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions.{host_action_id}.operationId"
                    ),
                    "must be a non-empty string",
                ));
                continue;
            };
            route_entry.insert(operation_id);
        }
    }

    (
        DerivedWorldRequirements {
            families: route_families.into_iter().collect(),
            bindings: route_bindings_to_rows(route_bindings),
        },
        issues,
    )
}

fn route_bindings_to_rows(rows: BTreeMap<String, BTreeSet<String>>) -> Vec<RequiredRouteBinding> {
    rows.into_iter()
        .map(|(route_family_id, operation_ids)| RequiredRouteBinding {
            route_family_id,
            operation_ids: operation_ids.into_iter().collect(),
        })
        .collect()
}

fn control_plane_contract_issue(path: String, message: &str) -> ValidationIssue {
    ValidationIssue {
        failure_class: world_failure_class::WORLD_ROUTE_UNBOUND.to_string(),
        path,
        message: message.to_string(),
    }
}

fn merge_report_with_contract_issues(
    mut report: ValidationReport,
    mut contract_issues: Vec<ValidationIssue>,
) -> ValidationReport {
    if contract_issues.is_empty() {
        return report;
    }
    report.issues.append(&mut contract_issues);
    report.issues.sort_by(|left, right| {
        (&left.path, &left.failure_class, &left.message).cmp(&(
            &right.path,
            &right.failure_class,
            &right.message,
        ))
    });
    report
        .failure_classes
        .push(world_failure_class::WORLD_ROUTE_UNBOUND.to_string());
    report.failure_classes.sort();
    report.failure_classes.dedup();
    report.result = "rejected".to_string();
    report
}

fn project_world_route_rows(
    world_route_rows: &[WorldRouteBindingRow],
    issues: &[ValidationIssue],
) -> Vec<WorldRouteBindingCheckRow> {
    let route_families_by_index: Vec<String> = world_route_rows
        .iter()
        .map(|row| row.route_family_id.clone())
        .collect();
    let mut family_errors: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for issue in issues {
        if let Some(index) = parse_route_binding_index(issue.path.as_str())
            && let Some(family) = route_families_by_index.get(index)
        {
            family_errors
                .entry(family.clone())
                .or_default()
                .push(issue.message.clone());
        }
    }

    let mut sorted_rows = world_route_rows.to_vec();
    sorted_rows.sort_by(|left, right| left.route_family_id.cmp(&right.route_family_id));
    sorted_rows
        .into_iter()
        .map(|row| {
            let mut errors = family_errors
                .remove(&row.route_family_id)
                .unwrap_or_default();
            errors.sort();
            errors.dedup();
            WorldRouteBindingCheckRow {
                route_family_id: row.route_family_id,
                world_id: row.world_id,
                morphism_row_id: row.morphism_row_id,
                operation_ids: row.operation_ids,
                required_morphisms: row.required_morphisms,
                status: if errors.is_empty() {
                    "ok".to_string()
                } else {
                    "invalid".to_string()
                },
                errors,
            }
        })
        .collect()
}

fn parse_route_binding_index(path: &str) -> Option<usize> {
    let suffix = path.strip_prefix("routeBindings[")?;
    let end = suffix.find(']')?;
    suffix.get(..end)?.parse::<usize>().ok()
}

fn extract_runtime_routes(
    control_plane_contract: &Value,
) -> Result<BTreeMap<String, RuntimeRouteRequirement>, String> {
    let Some(contract_obj) = control_plane_contract.as_object() else {
        return Err("runtimeRouteBindings must be an object".to_string());
    };
    let Some(runtime) = contract_obj.get("runtimeRouteBindings") else {
        return Err("runtimeRouteBindings must be an object".to_string());
    };
    let Some(runtime_obj) = runtime.as_object() else {
        return Err("runtimeRouteBindings must be an object".to_string());
    };
    let Some(routes) = runtime_obj.get("requiredOperationRoutes") else {
        return Err(
            "runtimeRouteBindings.requiredOperationRoutes must be a non-empty object".to_string(),
        );
    };
    let Some(routes_obj) = routes.as_object() else {
        return Err(
            "runtimeRouteBindings.requiredOperationRoutes must be a non-empty object".to_string(),
        );
    };
    if routes_obj.is_empty() {
        return Err(
            "runtimeRouteBindings.requiredOperationRoutes must be a non-empty object".to_string(),
        );
    }

    let mut out: BTreeMap<String, RuntimeRouteRequirement> = BTreeMap::new();
    for (route_id, route_value) in routes_obj {
        let route_id = route_id.trim();
        if route_id.is_empty() {
            return Err(
                "runtimeRouteBindings.requiredOperationRoutes keys must be non-empty".to_string(),
            );
        }
        let Some(route_obj) = route_value.as_object() else {
            return Err(format!(
                "runtimeRouteBindings.requiredOperationRoutes.{} must be an object",
                route_id
            ));
        };
        let Some(operation_id) = non_empty_string(route_obj.get("operationId")) else {
            return Err(format!(
                "runtimeRouteBindings.requiredOperationRoutes.{}.operationId must be non-empty",
                route_id
            ));
        };
        let required_morphisms = non_empty_string_list(
            route_obj.get("requiredMorphisms"),
            &format!(
                "runtimeRouteBindings.requiredOperationRoutes.{}.requiredMorphisms",
                route_id
            ),
        )?;
        out.insert(
            route_id.to_string(),
            RuntimeRouteRequirement {
                operation_id,
                required_morphisms,
            },
        );
    }

    Ok(out)
}

fn extract_registry_operations(
    operation_registry: &Value,
) -> Result<BTreeMap<String, RegistryOperation>, String> {
    let Some(registry_obj) = operation_registry.as_object() else {
        return Err("DOCTRINE-OP-REGISTRY.operations must be a non-empty list".to_string());
    };
    let Some(operations_value) = registry_obj.get("operations") else {
        return Err("DOCTRINE-OP-REGISTRY.operations must be a non-empty list".to_string());
    };
    let Some(operations) = operations_value.as_array() else {
        return Err("DOCTRINE-OP-REGISTRY.operations must be a non-empty list".to_string());
    };
    if operations.is_empty() {
        return Err("DOCTRINE-OP-REGISTRY.operations must be a non-empty list".to_string());
    }

    let mut out: BTreeMap<String, RegistryOperation> = BTreeMap::new();
    for (idx, row) in operations.iter().enumerate() {
        let Some(row_obj) = row.as_object() else {
            return Err(format!(
                "DOCTRINE-OP-REGISTRY.operations[{}] must be an object",
                idx
            ));
        };
        let Some(operation_id) = non_empty_string(row_obj.get("id")) else {
            return Err(format!(
                "DOCTRINE-OP-REGISTRY.operations[{}].id must be non-empty",
                idx
            ));
        };
        if out.contains_key(operation_id.as_str()) {
            return Err(format!(
                "duplicate operation id in DOCTRINE-OP-REGISTRY: {:?}",
                operation_id
            ));
        }

        let path = row_obj
            .get("path")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_string();

        let morphisms = sorted_unique_strings(row_obj.get("morphisms"));

        out.insert(operation_id, RegistryOperation { path, morphisms });
    }

    Ok(out)
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn sorted_unique_strings(value: Option<&Value>) -> Vec<String> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn non_empty_string_list(value: Option<&Value>, path: &str) -> Result<Vec<String>, String> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Err(format!("{path} must be non-empty"));
    };
    if values.is_empty() {
        return Err(format!("{path} must be non-empty"));
    }
    let mut out: Vec<String> = Vec::with_capacity(values.len());
    for (idx, value) in values.iter().enumerate() {
        let Some(token) = value
            .as_str()
            .map(str::trim)
            .filter(|token| !token.is_empty())
        else {
            return Err(format!("{path}[{idx}] must be a non-empty string"));
        };
        out.push(token.to_string());
    }
    Ok(out
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_harness_runtime_text() -> &'static str {
        "## 1.2 Harness-Squeak composition boundary (required)\nHarness computes deterministic work context and witness lineage refs.\nSqueak performs transport/runtime-placement mapping and emits transport-class witness outcomes.\nDestination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.\nHarness records the resulting references in session/trajectory projections."
    }

    fn command_surface() -> Value {
        json!({
            "governancePromotionCheck": {
                "canonicalEntrypoint": [
                    "cargo", "run", "--package", "premath-cli", "--", "governance-promotion-check"
                ]
            },
            "kcirMappingCheck": {
                "canonicalEntrypoint": [
                    "cargo", "run", "--package", "premath-cli", "--", "kcir-mapping-check"
                ]
            }
        })
    }

    #[test]
    fn accepts_valid_runtime_orchestration_payload() {
        let control_plane = json!({
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"]
                    }
                }
            },
            "commandSurface": command_surface(),
        });
        let operation_registry = json!({
            "operations": [
                {
                    "id": "op/ci.run_gate",
                    "path": "tools/ci/run_gate.sh",
                    "morphisms": ["dm.identity"]
                }
            ]
        });

        let report = evaluate_runtime_orchestration(
            &control_plane,
            &operation_registry,
            valid_harness_runtime_text(),
            None,
        );

        assert_eq!(report.result, "accepted");
        assert!(report.failure_classes.is_empty());
        assert_eq!(report.summary.required_routes, 1);
        assert_eq!(report.summary.checked_phase3_command_surfaces, 2);
    }

    #[test]
    fn rejects_when_runtime_route_operation_path_is_outside_ci_boundary() {
        let control_plane = json!({
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"]
                    }
                }
            },
            "commandSurface": command_surface(),
        });
        let operation_registry = json!({
            "operations": [
                {
                    "id": "op/ci.run_gate",
                    "path": "scripts/run_gate.sh",
                    "morphisms": ["dm.identity"]
                }
            ]
        });

        let report = evaluate_runtime_orchestration(
            &control_plane,
            &operation_registry,
            valid_harness_runtime_text(),
            None,
        );

        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == failure_class::CONTRACT_UNBOUND)
        );
        assert!(
            report
                .routes
                .iter()
                .any(|row| row.status.contains("path_unbound"))
        );
    }

    #[test]
    fn world_route_check_rejects_missing_transport_dispatch_family() {
        let control_plane = json!({
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"]
                    },
                    "runGateTerraform": {
                        "operationId": "op/ci.run_gate_terraform",
                        "requiredMorphisms": ["dm.identity"]
                    }
                }
            },
            "hostActionSurface": {
                "requiredActions": {
                    "instruction.run": {"operationId": "op/mcp.instruction_run"},
                    "required.witness_verify": {"operationId": "op/ci.verify_required_witness"},
                    "required.witness_decide": {"operationId": "op/ci.decide_required"},
                    "fiber.spawn": {"operationId": "op/transport.fiber_spawn"},
                    "fiber.join": {"operationId": "op/transport.fiber_join"},
                    "fiber.cancel": {"operationId": "op/transport.fiber_cancel"},
                    "issue.claim_next": {"operationId": "op/transport.issue_claim_next"},
                    "issue.claim": {"operationId": "op/mcp.issue_claim"},
                    "issue.lease_renew": {"operationId": "op/mcp.issue_lease_renew"},
                    "issue.lease_release": {"operationId": "op/mcp.issue_lease_release"},
                    "issue.discover": {"operationId": "op/mcp.issue_discover"}
                }
            },
            "commandSurface": command_surface(),
        });
        let operation_registry = json!({
            "operations": [
                {"id": "op/ci.run_gate", "path": "tools/ci/run_gate.sh", "morphisms": ["dm.identity"]},
                {"id": "op/ci.run_gate_terraform", "path": "tools/ci/run_gate_terraform.sh", "morphisms": ["dm.identity"]},
                {"id": "op/mcp.instruction_run", "path": "crates/premath-cli/src/commands/mcp_serve.rs", "morphisms": ["dm.identity"]},
                {"id": "op/ci.verify_required_witness", "path": "tools/ci/verify_required_witness.py", "morphisms": ["dm.identity"]},
                {"id": "op/ci.decide_required", "path": "tools/ci/decide_required.py", "morphisms": ["dm.identity"]},
                {"id": "op/transport.fiber_spawn", "path": "crates/premath-cli/src/commands/transport.rs", "morphisms": ["dm.identity"]},
                {"id": "op/transport.fiber_join", "path": "crates/premath-cli/src/commands/transport.rs", "morphisms": ["dm.identity"]},
                {"id": "op/transport.fiber_cancel", "path": "crates/premath-cli/src/commands/transport.rs", "morphisms": ["dm.identity"]},
                {"id": "op/transport.issue_claim_next", "path": "crates/premath-cli/src/commands/transport.rs", "morphisms": ["dm.identity"]},
                {"id": "op/mcp.issue_claim", "path": "crates/premath-cli/src/commands/mcp_serve.rs", "morphisms": ["dm.identity"]},
                {"id": "op/mcp.issue_lease_renew", "path": "crates/premath-cli/src/commands/mcp_serve.rs", "morphisms": ["dm.identity"]},
                {"id": "op/mcp.issue_lease_release", "path": "crates/premath-cli/src/commands/mcp_serve.rs", "morphisms": ["dm.identity"]},
                {"id": "op/mcp.issue_discover", "path": "crates/premath-cli/src/commands/mcp_serve.rs", "morphisms": ["dm.identity"]},
                {"id": "op/mcp.observe_latest", "path": "crates/premath-cli/src/commands/mcp_serve.rs", "morphisms": ["dm.identity"]},
                {"id": "op/transport.world_route_binding", "path": "crates/premath-cli/src/commands/transport.rs", "morphisms": ["dm.identity"]}
            ]
        });
        let doctrine_site_input = json!({
            "worldRouteBindings": {
                "schema": 1,
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": [
                    {
                        "routeFamilyId": "route.gate_execution",
                        "operationIds": ["op/ci.run_gate", "op/ci.run_gate_terraform"],
                        "worldId": "world.kernel.semantic.v1",
                        "morphismRowId": "wm.kernel.semantic.runtime_gate",
                        "requiredMorphisms": ["dm.identity"],
                        "failureClassUnbound": "world_route_unbound"
                    },
                    {
                        "routeFamilyId": "route.instruction_execution",
                        "operationIds": ["op/mcp.instruction_run"],
                        "worldId": "world.instruction.v1",
                        "morphismRowId": "wm.control.instruction.execution",
                        "requiredMorphisms": ["dm.identity"],
                        "failureClassUnbound": "world_route_unbound"
                    },
                    {
                        "routeFamilyId": "route.required_decision_attestation",
                        "operationIds": ["op/ci.verify_required_witness", "op/ci.decide_required"],
                        "worldId": "world.ci_witness.v1",
                        "morphismRowId": "wm.control.ci_witness.attest",
                        "requiredMorphisms": ["dm.identity"],
                        "failureClassUnbound": "world_route_unbound"
                    },
                    {
                        "routeFamilyId": "route.fiber.lifecycle",
                        "operationIds": ["op/transport.fiber_spawn", "op/transport.fiber_join", "op/transport.fiber_cancel"],
                        "worldId": "world.fiber.v1",
                        "morphismRowId": "wm.control.fiber.lifecycle",
                        "requiredMorphisms": ["dm.identity"],
                        "failureClassUnbound": "world_route_unbound"
                    },
                    {
                        "routeFamilyId": "route.issue_claim_lease",
                        "operationIds": [
                            "op/transport.issue_claim_next",
                            "op/mcp.issue_claim",
                            "op/mcp.issue_lease_renew",
                            "op/mcp.issue_lease_release",
                            "op/mcp.issue_discover"
                        ],
                        "worldId": "world.lease.v1",
                        "morphismRowId": "wm.control.lease.mutation",
                        "requiredMorphisms": ["dm.identity"],
                        "failureClassUnbound": "world_route_unbound"
                    },
                    {
                        "routeFamilyId": "route.session_projection",
                        "operationIds": ["op/mcp.observe_latest"],
                        "worldId": "world.control_plane.bundle.v0",
                        "morphismRowId": "wm.control.bundle.projection",
                        "requiredMorphisms": ["dm.identity"],
                        "failureClassUnbound": "world_route_unbound"
                    }
                ]
            }
        });

        let report = evaluate_runtime_orchestration(
            &control_plane,
            &operation_registry,
            valid_harness_runtime_text(),
            Some(&doctrine_site_input),
        );

        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == world_failure_class::WORLD_ROUTE_UNBOUND)
        );
        assert!(
            report.errors.iter().any(
                |row| row.contains("missing required route families: route.transport.dispatch")
            )
        );
    }

    #[test]
    fn world_route_check_surfaces_world_failure_classes() {
        let control_plane = json!({
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"]
                    }
                }
            },
            "commandSurface": command_surface(),
        });
        let operation_registry = json!({
            "operations": [
                {
                    "id": "op/ci.run_gate",
                    "path": "tools/ci/run_gate.sh",
                    "morphisms": ["dm.identity"]
                }
            ]
        });
        let doctrine_site_input = json!({
            "worldRouteBindings": {}
        });

        let report = evaluate_runtime_orchestration(
            &control_plane,
            &operation_registry,
            valid_harness_runtime_text(),
            Some(&doctrine_site_input),
        );

        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == world_failure_class::WORLD_ROUTE_UNBOUND)
        );
        assert_eq!(report.summary.checked_world_route_families, 0);
    }
}
