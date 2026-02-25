use crate::support::read_json_file_or_exit;
use premath_kernel::{
    OperationRouteRow, RequiredRouteBinding, ValidationIssue, ValidationReport, WorldRegistry,
    WorldRouteBindingRow, parse_operation_route_rows, parse_world_route_binding_rows,
    validate_world_bindings_against_operations, validate_world_registry,
    validate_world_route_bindings_with_requirements, world_failure_class,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const WORLD_REGISTRY_CHECK_KIND: &str = "premath.world_registry_check.v1";
const CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX: &str = "controlPlaneContract";
const REQUIRED_WORLD_ROUTE_FAMILIES: &[&str] = &[
    "route.gate_execution",
    "route.instruction_execution",
    "route.required_decision_attestation",
    "route.fiber.lifecycle",
    "route.issue_claim_lease",
    "route.session_projection",
];
const REQUIRED_WORLD_ROUTE_ACTION_BINDINGS: &[(&str, &[&str])] = &[
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

pub fn run(
    registry: Option<String>,
    site_input: Option<String>,
    operations: Option<String>,
    control_plane_contract: Option<String>,
    required_route_families: Vec<String>,
    required_route_bindings: Vec<String>,
    json_output: bool,
) {
    match (registry, site_input) {
        (Some(registry_path), None) => {
            if control_plane_contract.is_some() {
                eprintln!(
                    "error: --control-plane-contract can only be used together with --site-input"
                );
                std::process::exit(1);
            }
            run_registry_mode(registry_path, operations, json_output);
        }
        (None, Some(site_input_path)) => {
            run_site_input_mode(
                site_input_path,
                operations,
                control_plane_contract,
                required_route_families,
                required_route_bindings,
                json_output,
            );
        }
        (Some(_), Some(_)) => {
            eprintln!("error: exactly one of --registry or --site-input must be provided");
            std::process::exit(1);
        }
        (None, None) => {
            eprintln!("error: one of --registry or --site-input must be provided");
            std::process::exit(1);
        }
    }
}

fn run_registry_mode(registry: String, operations: Option<String>, json_output: bool) {
    let registry_payload: WorldRegistry = read_json_file_or_exit(&registry, "world registry");
    let operations_rows = operations
        .as_deref()
        .map(load_operation_rows_or_exit)
        .unwrap_or_default();
    let report = if operations.is_some() {
        validate_world_bindings_against_operations(&registry_payload, &operations_rows)
    } else {
        validate_world_registry(&registry_payload)
    };
    emit_registry_report(report, &registry, operations.as_deref(), json_output);
}

fn run_site_input_mode(
    site_input_path: String,
    operations: Option<String>,
    control_plane_contract: Option<String>,
    required_route_families: Vec<String>,
    required_route_bindings: Vec<String>,
    json_output: bool,
) {
    let Some(operations_path) = operations else {
        eprintln!("error: --operations is required when using --site-input");
        std::process::exit(1);
    };

    let site_input_payload = read_json_file_or_exit(&site_input_path, "doctrine site input");
    let operations_rows = load_operation_rows_or_exit(&operations_path);
    let required: Vec<String> = required_route_families
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let required_bindings =
        parse_required_route_bindings_or_exit(&required, &required_route_bindings);
    let mut merged_required_families = required;
    let mut merged_required_bindings = required_bindings;
    let mut contract_issues: Vec<ValidationIssue> = Vec::new();
    if let Some(path) = control_plane_contract.as_deref() {
        let control_plane_payload: Value = read_json_file_or_exit(path, "control-plane contract");
        let (contract_requirements, issues) =
            derive_required_world_requirements_from_control_plane(&control_plane_payload);
        merged_required_families = merge_required_route_families(
            &merged_required_families,
            &contract_requirements.families,
        );
        merged_required_bindings = merge_required_route_bindings(
            &merged_required_bindings,
            &contract_requirements.bindings,
        );
        contract_issues = issues;
    }
    let report = validate_world_route_bindings_with_requirements(
        &site_input_payload,
        &operations_rows,
        &merged_required_families,
        &merged_required_bindings,
    );
    let report = merge_report_with_contract_issues(report, contract_issues);
    let world_route_rows = parse_world_route_binding_rows(&site_input_payload).unwrap_or_default();
    emit_site_input_report(
        report,
        SiteInputReportContext {
            site_input_path: &site_input_path,
            operations_path: &operations_path,
            control_plane_contract_path: control_plane_contract.as_deref(),
            required_route_families: &merged_required_families,
            required_route_bindings: &merged_required_bindings,
            world_route_rows: &world_route_rows,
        },
        json_output,
    );
}

fn parse_required_route_bindings_or_exit(
    required_route_families: &[String],
    required_route_bindings: &[String],
) -> Vec<RequiredRouteBinding> {
    let mut rows: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for family in required_route_families {
        let family_id = family.trim();
        if family_id.is_empty() {
            continue;
        }
        rows.entry(family_id.to_string()).or_default();
    }

    for binding in required_route_bindings {
        let raw = binding.trim();
        if raw.is_empty() {
            continue;
        }
        let Some((family_raw, operation_raw)) = raw.split_once('=') else {
            eprintln!(
                "error: --required-route-binding must use `<route-family-id>=<operation-id>`, got {:?}",
                raw
            );
            std::process::exit(1);
        };
        let family_id = family_raw.trim();
        let operation_id = operation_raw.trim();
        if family_id.is_empty() || operation_id.is_empty() {
            eprintln!(
                "error: --required-route-binding must use non-empty `<route-family-id>=<operation-id>`, got {:?}",
                raw
            );
            std::process::exit(1);
        }
        rows.entry(family_id.to_string())
            .or_default()
            .insert(operation_id.to_string());
    }

    rows.into_iter()
        .map(|(route_family_id, operation_ids)| RequiredRouteBinding {
            route_family_id,
            operation_ids: operation_ids.into_iter().collect(),
        })
        .collect()
}

#[derive(Debug, Clone, Default)]
struct DerivedWorldRequirements {
    families: Vec<String>,
    bindings: Vec<RequiredRouteBinding>,
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
        route_families.insert((*route_family_id).to_string());
        let route_entry = route_bindings
            .entry((*route_family_id).to_string())
            .or_default();
        for host_action_id in *host_action_ids {
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
            let Some(operation_id) = action_obj
                .get("operationId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                issues.push(control_plane_contract_issue(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions.{host_action_id}.operationId"
                    ),
                    "must be a non-empty string",
                ));
                continue;
            };
            route_entry.insert(operation_id.to_string());
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

fn merge_required_route_families(current: &[String], additional: &[String]) -> Vec<String> {
    current
        .iter()
        .chain(additional.iter())
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn merge_required_route_bindings(
    current: &[RequiredRouteBinding],
    additional: &[RequiredRouteBinding],
) -> Vec<RequiredRouteBinding> {
    let mut merged: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in current.iter().chain(additional.iter()) {
        let family = row.route_family_id.trim();
        if family.is_empty() {
            continue;
        }
        let entry = merged.entry(family.to_string()).or_default();
        for operation_id in &row.operation_ids {
            let operation_id = operation_id.trim();
            if operation_id.is_empty() {
                continue;
            }
            entry.insert(operation_id.to_string());
        }
    }
    route_bindings_to_rows(merged)
}

fn merge_report_with_contract_issues(
    mut report: ValidationReport,
    mut contract_issues: Vec<ValidationIssue>,
) -> ValidationReport {
    if contract_issues.is_empty() {
        return report;
    }
    report.issues.append(&mut contract_issues);
    report.issues.sort_by(|a, b| {
        (&a.path, &a.failure_class, &a.message).cmp(&(&b.path, &b.failure_class, &b.message))
    });
    report
        .failure_classes
        .push(world_failure_class::WORLD_ROUTE_UNBOUND.to_string());
    report.failure_classes.sort();
    report.failure_classes.dedup();
    report.result = "rejected".to_string();
    report
}

fn load_operation_rows_or_exit(path: &str) -> Vec<OperationRouteRow> {
    let raw = read_json_file_or_exit(path, "operation route rows");
    parse_operation_route_rows(&raw).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to parse operation route rows at {}: {}",
            path, err
        );
        std::process::exit(1);
    })
}

fn emit_registry_report(
    report: ValidationReport,
    registry_path: &str,
    operations_path: Option<&str>,
    json_output: bool,
) {
    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": WORLD_REGISTRY_CHECK_KIND,
            "registryPath": registry_path,
            "operationsPath": operations_path,
            "result": report.result,
            "failureClasses": report.failure_classes,
            "issues": report.issues,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render world-registry-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    println!("premath world-registry-check");
    println!("  Registry path: {registry_path}");
    if let Some(path) = operations_path {
        println!("  Operations path: {path}");
    }
    println!("  Result: {}", report.result);
    println!("  Failure classes: {}", report.failure_classes.len());
    println!("  Issues: {}", report.issues.len());
}

struct SiteInputReportContext<'a> {
    site_input_path: &'a str,
    operations_path: &'a str,
    control_plane_contract_path: Option<&'a str>,
    required_route_families: &'a [String],
    required_route_bindings: &'a [RequiredRouteBinding],
    world_route_rows: &'a [WorldRouteBindingRow],
}

fn emit_site_input_report(
    report: ValidationReport,
    context: SiteInputReportContext<'_>,
    json_output: bool,
) {
    let projected_rows = project_world_route_rows(context.world_route_rows, &report.issues);
    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": WORLD_REGISTRY_CHECK_KIND,
            "siteInputPath": context.site_input_path,
            "operationsPath": context.operations_path,
            "controlPlaneContractPath": context.control_plane_contract_path,
            "requiredRouteFamilies": context.required_route_families,
            "requiredRouteBindings": context.required_route_bindings,
            "result": report.result,
            "failureClasses": report.failure_classes,
            "issues": report.issues,
            "worldRouteBindings": projected_rows,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render world-registry-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    println!("premath world-registry-check");
    println!("  Site input path: {}", context.site_input_path);
    println!("  Operations path: {}", context.operations_path);
    if let Some(path) = context.control_plane_contract_path {
        println!("  Control-plane contract path: {path}");
    }
    println!(
        "  Required route families: {}",
        context.required_route_families.len()
    );
    println!(
        "  Required route operation bindings: {}",
        context.required_route_bindings.len()
    );
    println!("  Checked world route families: {}", projected_rows.len());
    println!("  Result: {}", report.result);
    println!("  Failure classes: {}", report.failure_classes.len());
    println!("  Issues: {}", report.issues.len());
}

fn project_world_route_rows(
    world_route_rows: &[WorldRouteBindingRow],
    issues: &[ValidationIssue],
) -> Vec<serde_json::Value> {
    let mut family_errors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let route_families_by_index: Vec<String> = world_route_rows
        .iter()
        .map(|row| row.route_family_id.clone())
        .collect();
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
    sorted_rows.sort_by(|a, b| a.route_family_id.cmp(&b.route_family_id));
    sorted_rows
        .into_iter()
        .map(|row| {
            let mut errors = family_errors
                .remove(&row.route_family_id)
                .unwrap_or_default();
            errors.sort();
            errors.dedup();
            json!({
                "routeFamilyId": row.route_family_id,
                "worldId": row.world_id,
                "morphismRowId": row.morphism_row_id,
                "operationIds": row.operation_ids,
                "requiredMorphisms": row.required_morphisms,
                "status": if errors.is_empty() { "ok" } else { "invalid" },
                "errors": errors,
            })
        })
        .collect()
}

fn parse_route_binding_index(path: &str) -> Option<usize> {
    let suffix = path.strip_prefix("routeBindings[")?;
    let end = suffix.find(']')?;
    suffix.get(..end)?.parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use premath_kernel::world_registry::WORLD_REGISTRY_KIND;
    use premath_kernel::{RouteBindingRow, WorldMorphismRow, WorldRow};

    fn valid_registry() -> WorldRegistry {
        WorldRegistry {
            schema: 1,
            registry_kind: WORLD_REGISTRY_KIND.to_string(),
            worlds: vec![WorldRow {
                world_id: "world.kernel.semantic.v1".to_string(),
                role: "semantic_authority".to_string(),
                context_family_id: "ctx.kernel".to_string(),
                definable_family_id: "def.kernel".to_string(),
                cover_kind: "site_cover".to_string(),
                equality_mode: "strict".to_string(),
                source_refs: Vec::new(),
            }],
            morphisms: vec![WorldMorphismRow {
                morphism_row_id: "wm.kernel.semantic.runtime_gate".to_string(),
                from_world_id: "world.kernel.semantic.v1".to_string(),
                to_world_id: "world.kernel.semantic.v1".to_string(),
                doctrine_morphisms: vec![
                    "dm.identity".to_string(),
                    "dm.profile.execution".to_string(),
                ],
                preservation_claims: Vec::new(),
            }],
            route_bindings: vec![RouteBindingRow {
                route_family_id: "route.gate_execution".to_string(),
                operation_ids: vec!["op/ci.run_gate".to_string()],
                world_id: "world.kernel.semantic.v1".to_string(),
                morphism_row_id: "wm.kernel.semantic.runtime_gate".to_string(),
                failure_class_unbound: "world_route_unbound".to_string(),
            }],
        }
    }

    #[test]
    fn parses_operation_registry_rows() {
        let raw = json!({
            "operations": [
                {"id": "op/ci.run_gate", "morphisms": ["dm.identity", "dm.profile.execution"]},
                {"operationId": "op/ci.run_instruction", "morphisms": ["dm.identity"]},
            ],
        });
        let rows = parse_operation_route_rows(&raw).expect("rows should parse");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].operation_id, "op/ci.run_gate");
        assert_eq!(rows[1].operation_id, "op/ci.run_instruction");
    }

    #[test]
    fn validates_registry_against_operations() {
        let registry = valid_registry();
        let operations = vec![OperationRouteRow {
            operation_id: "op/ci.run_gate".to_string(),
            morphisms: vec![
                "dm.identity".to_string(),
                "dm.profile.execution".to_string(),
            ],
        }];
        let report = validate_world_bindings_against_operations(&registry, &operations);
        assert_eq!(report.result, "accepted");
    }

    #[test]
    fn project_world_route_rows_marks_route_with_indexed_issue_as_invalid() {
        let rows = vec![WorldRouteBindingRow {
            route_family_id: "route.gate_execution".to_string(),
            operation_ids: vec!["op/ci.run_gate".to_string()],
            world_id: "world.kernel.semantic.v1".to_string(),
            morphism_row_id: "wm.kernel.semantic.runtime_gate".to_string(),
            required_morphisms: vec!["dm.identity".to_string()],
            failure_class_unbound: "world_route_unbound".to_string(),
        }];
        let projected = project_world_route_rows(
            &rows,
            &[ValidationIssue {
                failure_class: "world_route_unbound".to_string(),
                path: "routeBindings[0].operationIds[0]".to_string(),
                message: "unknown operationId op/ci.run_gate".to_string(),
            }],
        );
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0]["status"], json!("invalid"));
    }

    #[test]
    fn parse_required_route_bindings_merges_families_and_bindings() {
        let parsed = parse_required_route_bindings_or_exit(
            &[
                "route.gate_execution".to_string(),
                "route.instruction_execution".to_string(),
            ],
            &[
                "route.instruction_execution=op/ci.run_instruction".to_string(),
                "route.instruction_execution=op/mcp.instruction_run".to_string(),
            ],
        );
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].route_family_id, "route.gate_execution");
        assert!(parsed[0].operation_ids.is_empty());
        assert_eq!(parsed[1].route_family_id, "route.instruction_execution");
        assert_eq!(
            parsed[1].operation_ids,
            vec![
                "op/ci.run_instruction".to_string(),
                "op/mcp.instruction_run".to_string(),
            ]
        );
    }

    #[test]
    fn derive_required_world_requirements_from_control_plane_extracts_bindings() {
        let control_plane = json!({
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
            }
        });
        let (requirements, issues) =
            derive_required_world_requirements_from_control_plane(&control_plane);
        assert!(issues.is_empty());
        assert_eq!(requirements.families.len(), 6);
        let mut by_family: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for row in requirements.bindings {
            by_family.insert(row.route_family_id, row.operation_ids);
        }
        assert_eq!(
            by_family.get("route.instruction_execution"),
            Some(&vec!["op/mcp.instruction_run".to_string()])
        );
        assert_eq!(
            by_family.get("route.required_decision_attestation"),
            Some(&vec![
                "op/ci.decide_required".to_string(),
                "op/ci.verify_required_witness".to_string()
            ])
        );
        assert_eq!(
            by_family.get("route.issue_claim_lease"),
            Some(&vec![
                "op/mcp.issue_claim".to_string(),
                "op/mcp.issue_discover".to_string(),
                "op/mcp.issue_lease_release".to_string(),
                "op/mcp.issue_lease_renew".to_string(),
                "op/transport.issue_claim_next".to_string()
            ])
        );
        assert_eq!(
            by_family.get("route.fiber.lifecycle"),
            Some(&vec![
                "op/transport.fiber_cancel".to_string(),
                "op/transport.fiber_join".to_string(),
                "op/transport.fiber_spawn".to_string()
            ])
        );
    }

    #[test]
    fn derive_required_world_requirements_from_control_plane_reports_unbound_operation_rows() {
        let control_plane = json!({
            "hostActionSurface": {
                "requiredActions": {
                    "instruction.run": {"operationId": "op/mcp.instruction_run"},
                    "required.witness_verify": {},
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
            }
        });
        let (_requirements, issues) =
            derive_required_world_requirements_from_control_plane(&control_plane);
        assert!(!issues.is_empty());
        assert_eq!(
            issues[0].failure_class,
            world_failure_class::WORLD_ROUTE_UNBOUND.to_string()
        );
        assert!(issues.iter().any(|issue| {
            issue.path
                == "controlPlaneContract.hostActionSurface.requiredActions.required.witness_verify.operationId"
        }));
    }
}
