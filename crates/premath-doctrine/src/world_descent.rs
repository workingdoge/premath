use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX: &str = "controlPlaneContract";
const WORLD_DESCENT_CONTRACT_ID: &str = "doctrine.world_descent.v1";
const WORLD_ROUTE_IDENTITY_MISSING: &str = "world_route_identity_missing";
const WORLD_DESCENT_DATA_MISSING: &str = "world_descent_data_missing";
const KCIR_HANDOFF_IDENTITY_MISSING: &str = "kcir_handoff_identity_missing";
const DEFAULT_WORLD_ROUTE_FAMILIES: [&str; 7] = [
    "route.gate_execution",
    "route.instruction_execution",
    "route.required_decision_attestation",
    "route.fiber.lifecycle",
    "route.issue_claim_lease",
    "route.session_projection",
    "route.transport.dispatch",
];
const DEFAULT_WORLD_ROUTE_ACTION_BINDINGS: [(&str, &[&str]); 4] = [
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
const DEFAULT_WORLD_ROUTE_STATIC_BINDINGS: [(&str, &[&str]); 1] = [(
    "route.transport.dispatch",
    &["op/transport.world_route_binding"],
)];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DoctrineValidationIssue {
    pub failure_class: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DoctrineRequiredRouteBinding {
    pub route_family_id: String,
    #[serde(default)]
    pub operation_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DerivedWorldRequirements {
    pub families: Vec<String>,
    pub bindings: Vec<DoctrineRequiredRouteBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldDescentContractProjection {
    pub contract_id: String,
    pub required_route_families: Vec<String>,
    pub required_action_route_bindings: BTreeMap<String, Vec<String>>,
    pub required_static_operation_bindings: BTreeMap<String, Vec<String>>,
    pub failure_classes: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct WorldDescentConfig {
    required_families: BTreeSet<String>,
    required_action_bindings: BTreeMap<String, BTreeSet<String>>,
    required_static_bindings: BTreeMap<String, BTreeSet<String>>,
    failure_class_identity_missing: String,
    failure_class_descent_data_missing: String,
    failure_class_kcir_handoff_identity_missing: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeriveMode {
    RuntimeOrchestration,
    WorldRegistryCheck,
}

impl DeriveMode {
    fn include_runtime_route_operations(self) -> bool {
        matches!(self, Self::RuntimeOrchestration)
    }
}

pub fn derive_world_descent_requirements_for_runtime_orchestration(
    control_plane_contract: &Value,
) -> (DerivedWorldRequirements, Vec<DoctrineValidationIssue>) {
    derive_world_descent_requirements(control_plane_contract, DeriveMode::RuntimeOrchestration)
}

pub fn derive_world_descent_requirements_for_world_registry_check(
    control_plane_contract: &Value,
) -> (DerivedWorldRequirements, Vec<DoctrineValidationIssue>) {
    derive_world_descent_requirements(control_plane_contract, DeriveMode::WorldRegistryCheck)
}

pub fn validate_world_descent_contract_projection(
    control_plane_contract: &Value,
) -> (WorldDescentContractProjection, Vec<DoctrineValidationIssue>) {
    let mut config = default_world_descent_config();
    let mut issues: Vec<DoctrineValidationIssue> = Vec::new();

    let Some(contract_obj) = control_plane_contract.as_object() else {
        issues.push(control_plane_contract_issue_with_failure(
            CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX.to_string(),
            config.failure_class_descent_data_missing.as_str(),
            "must be an object",
        ));
        return (project_world_descent_contract(&config), issues);
    };

    if let Err(mut parse_issues) = parse_world_descent_contract(contract_obj, &mut config) {
        issues.append(&mut parse_issues);
    }

    (project_world_descent_contract(&config), issues)
}

fn derive_world_descent_requirements(
    control_plane_contract: &Value,
    mode: DeriveMode,
) -> (DerivedWorldRequirements, Vec<DoctrineValidationIssue>) {
    let mut issues: Vec<DoctrineValidationIssue> = Vec::new();
    let mut world_descent_config = default_world_descent_config();

    let Some(contract_obj) = control_plane_contract.as_object() else {
        let mut route_bindings: BTreeMap<String, BTreeSet<String>> = world_descent_config
            .required_families
            .iter()
            .map(|family| (family.clone(), BTreeSet::new()))
            .collect();
        for (route_family_id, operation_ids) in &world_descent_config.required_static_bindings {
            route_bindings
                .entry(route_family_id.clone())
                .or_default()
                .extend(operation_ids.iter().cloned());
        }
        issues.push(control_plane_contract_issue_with_failure(
            CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX.to_string(),
            world_descent_config
                .failure_class_descent_data_missing
                .as_str(),
            "must be an object",
        ));
        return (
            DerivedWorldRequirements {
                families: world_descent_config.required_families.into_iter().collect(),
                bindings: route_bindings_to_rows(route_bindings),
            },
            issues,
        );
    };

    if let Err(mut parse_issues) =
        parse_world_descent_contract(contract_obj, &mut world_descent_config)
    {
        issues.append(&mut parse_issues);
    }

    let mut route_families = world_descent_config.required_families.clone();
    let mut route_bindings: BTreeMap<String, BTreeSet<String>> = route_families
        .iter()
        .map(|family| (family.clone(), BTreeSet::new()))
        .collect();

    for (route_family_id, operation_ids) in &world_descent_config.required_static_bindings {
        route_families.insert(route_family_id.clone());
        let route_entry = route_bindings.entry(route_family_id.clone()).or_default();
        route_entry.extend(operation_ids.iter().cloned());
    }

    if mode.include_runtime_route_operations() {
        let gate_entry = route_bindings
            .entry("route.gate_execution".to_string())
            .or_default();
        for operation_id in parse_runtime_route_operation_ids(control_plane_contract) {
            gate_entry.insert(operation_id);
        }
    }

    let Some(host_action_surface) = contract_obj.get("hostActionSurface") else {
        issues.push(control_plane_contract_issue_with_failure(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface"),
            world_descent_config
                .failure_class_descent_data_missing
                .as_str(),
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
        issues.push(control_plane_contract_issue_with_failure(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface"),
            world_descent_config
                .failure_class_descent_data_missing
                .as_str(),
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
        issues.push(control_plane_contract_issue_with_failure(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions"),
            world_descent_config
                .failure_class_descent_data_missing
                .as_str(),
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
        issues.push(control_plane_contract_issue_with_failure(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions"),
            world_descent_config
                .failure_class_descent_data_missing
                .as_str(),
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

    for (route_family_id, host_action_ids) in &world_descent_config.required_action_bindings {
        route_families.insert(route_family_id.clone());
        let route_entry = route_bindings.entry(route_family_id.clone()).or_default();
        for host_action_id in host_action_ids {
            let Some(action_row) = required_actions_obj.get(host_action_id.as_str()) else {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions.{host_action_id}"
                    ),
                    world_descent_config.failure_class_descent_data_missing.as_str(),
                    "missing required host-action row",
                ));
                continue;
            };
            let Some(action_obj) = action_row.as_object() else {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions.{host_action_id}"
                    ),
                    world_descent_config.failure_class_descent_data_missing.as_str(),
                    "must be an object",
                ));
                continue;
            };
            let Some(operation_id) = non_empty_string(action_obj.get("operationId")) else {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.hostActionSurface.requiredActions.{host_action_id}.operationId"
                    ),
                    world_descent_config.failure_class_identity_missing.as_str(),
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

fn default_world_descent_config() -> WorldDescentConfig {
    let required_families = DEFAULT_WORLD_ROUTE_FAMILIES
        .iter()
        .map(|family| family.to_string())
        .collect::<BTreeSet<_>>();
    let required_action_bindings = DEFAULT_WORLD_ROUTE_ACTION_BINDINGS
        .iter()
        .map(|(route_family_id, host_action_ids)| {
            (
                (*route_family_id).to_string(),
                host_action_ids
                    .iter()
                    .map(|host_action_id| (*host_action_id).to_string())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let required_static_bindings = DEFAULT_WORLD_ROUTE_STATIC_BINDINGS
        .iter()
        .map(|(route_family_id, operation_ids)| {
            (
                (*route_family_id).to_string(),
                operation_ids
                    .iter()
                    .map(|operation_id| (*operation_id).to_string())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    WorldDescentConfig {
        required_families,
        required_action_bindings,
        required_static_bindings,
        failure_class_identity_missing: WORLD_ROUTE_IDENTITY_MISSING.to_string(),
        failure_class_descent_data_missing: WORLD_DESCENT_DATA_MISSING.to_string(),
        failure_class_kcir_handoff_identity_missing: KCIR_HANDOFF_IDENTITY_MISSING.to_string(),
    }
}

fn parse_world_descent_contract(
    contract_obj: &serde_json::Map<String, Value>,
    config: &mut WorldDescentConfig,
) -> Result<(), Vec<DoctrineValidationIssue>> {
    let mut issues: Vec<DoctrineValidationIssue> = Vec::new();

    let Some(world_descent_value) = contract_obj.get("worldDescentContract") else {
        issues.push(control_plane_contract_issue_with_failure(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract"),
            config.failure_class_descent_data_missing.as_str(),
            "missing required object",
        ));
        return Err(issues);
    };
    let Some(world_descent_obj) = world_descent_value.as_object() else {
        issues.push(control_plane_contract_issue_with_failure(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract"),
            config.failure_class_descent_data_missing.as_str(),
            "must be an object",
        ));
        return Err(issues);
    };

    let contract_id = non_empty_string(world_descent_obj.get("contractId"));
    if contract_id.as_deref() != Some(WORLD_DESCENT_CONTRACT_ID) {
        issues.push(control_plane_contract_issue_with_failure(
            format!("{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.contractId"),
            config.failure_class_descent_data_missing.as_str(),
            "must equal doctrine.world_descent.v1",
        ));
    }

    let failure_classes_obj = world_descent_obj
        .get("failureClasses")
        .and_then(Value::as_object);
    match failure_classes_obj {
        Some(row) => {
            let expected_failure_classes = expected_failure_classes(config);
            for key in [
                "identityMissing",
                "descentDataMissing",
                "kcirHandoffIdentityMissing",
            ] {
                let path = format!(
                    "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.failureClasses.{key}"
                );
                let Some(value) = non_empty_string(row.get(key)) else {
                    issues.push(control_plane_contract_issue_with_failure(
                        path,
                        config.failure_class_descent_data_missing.as_str(),
                        "must be a non-empty string",
                    ));
                    continue;
                };
                if let Some(expected_value) = expected_failure_classes.get(key)
                    && &value != expected_value
                {
                    issues.push(control_plane_contract_issue_with_failure(
                        format!(
                            "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.failureClasses.{key}"
                        ),
                        config.failure_class_descent_data_missing.as_str(),
                        &format!("must equal {expected_value}"),
                    ));
                }
            }
            let unknown_failure_class_keys: Vec<String> = row
                .keys()
                .filter(|key| {
                    !matches!(
                        key.as_str(),
                        "identityMissing" | "descentDataMissing" | "kcirHandoffIdentityMissing"
                    )
                })
                .cloned()
                .collect();
            if !unknown_failure_class_keys.is_empty() {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.failureClasses"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "must include only identityMissing, descentDataMissing, and kcirHandoffIdentityMissing",
                ));
            }
        }
        None => {
            issues.push(control_plane_contract_issue_with_failure(
                format!(
                    "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.failureClasses"
                ),
                config.failure_class_descent_data_missing.as_str(),
                "must be an object",
            ));
        }
    }

    if let Some(required_families) = world_descent_obj
        .get("requiredRouteFamilies")
        .and_then(Value::as_array)
    {
        let parsed = required_families
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|family| !family.is_empty())
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>();
        if parsed.is_empty() {
            issues.push(control_plane_contract_issue_with_failure(
                format!(
                    "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredRouteFamilies"
                ),
                config.failure_class_descent_data_missing.as_str(),
                "must be a non-empty list",
            ));
        } else if parsed != config.required_families {
            issues.push(control_plane_contract_issue_with_failure(
                format!(
                    "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredRouteFamilies"
                ),
                config.failure_class_descent_data_missing.as_str(),
                "must match canonical route-family set",
            ));
        }
    } else {
        issues.push(control_plane_contract_issue_with_failure(
            format!(
                "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredRouteFamilies"
            ),
            config.failure_class_descent_data_missing.as_str(),
            "must be a non-empty list",
        ));
    }

    if let Some(action_bindings) = world_descent_obj
        .get("requiredActionRouteBindings")
        .and_then(Value::as_object)
    {
        let mut parsed_bindings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (route_family_id, host_action_ids) in action_bindings {
            let route_family_id = route_family_id.trim();
            if route_family_id.is_empty() {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredActionRouteBindings.<routeFamilyId>"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "route family id must be non-empty",
                ));
                continue;
            }
            let Some(host_action_ids) = host_action_ids.as_array() else {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredActionRouteBindings.{route_family_id}"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "must be a non-empty list",
                ));
                continue;
            };
            let parsed = host_action_ids
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|host_action_id| !host_action_id.is_empty())
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>();
            if parsed.is_empty() {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredActionRouteBindings.{route_family_id}"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "must be a non-empty list",
                ));
                continue;
            }
            if !config.required_families.contains(route_family_id) {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredActionRouteBindings.{route_family_id}"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "must reference requiredRouteFamilies",
                ));
                continue;
            }
            parsed_bindings.insert(route_family_id.to_string(), parsed);
        }
        if parsed_bindings != config.required_action_bindings {
            issues.push(control_plane_contract_issue_with_failure(
                format!(
                    "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredActionRouteBindings"
                ),
                config.failure_class_descent_data_missing.as_str(),
                "must match canonical route-family host-action bindings",
            ));
        }
    } else {
        issues.push(control_plane_contract_issue_with_failure(
            format!(
                "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredActionRouteBindings"
            ),
            config.failure_class_descent_data_missing.as_str(),
            "must be an object",
        ));
    }

    if let Some(static_bindings) = world_descent_obj
        .get("requiredStaticOperationBindings")
        .and_then(Value::as_object)
    {
        let mut parsed_bindings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (route_family_id, operation_ids) in static_bindings {
            let route_family_id = route_family_id.trim();
            if route_family_id.is_empty() {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredStaticOperationBindings.<routeFamilyId>"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "route family id must be non-empty",
                ));
                continue;
            }
            let Some(operation_ids) = operation_ids.as_array() else {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredStaticOperationBindings.{route_family_id}"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "must be a non-empty list",
                ));
                continue;
            };
            let parsed = operation_ids
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|operation_id| !operation_id.is_empty())
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>();
            if parsed.is_empty() {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredStaticOperationBindings.{route_family_id}"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "must be a non-empty list",
                ));
                continue;
            }
            if !config.required_families.contains(route_family_id) {
                issues.push(control_plane_contract_issue_with_failure(
                    format!(
                        "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredStaticOperationBindings.{route_family_id}"
                    ),
                    config.failure_class_descent_data_missing.as_str(),
                    "must reference requiredRouteFamilies",
                ));
                continue;
            }
            parsed_bindings.insert(route_family_id.to_string(), parsed);
        }
        if parsed_bindings != config.required_static_bindings {
            issues.push(control_plane_contract_issue_with_failure(
                format!(
                    "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredStaticOperationBindings"
                ),
                config.failure_class_descent_data_missing.as_str(),
                "must match canonical route-family operation bindings",
            ));
        }
    } else {
        issues.push(control_plane_contract_issue_with_failure(
            format!(
                "{CONTROL_PLANE_CONTRACT_ISSUE_PATH_PREFIX}.worldDescentContract.requiredStaticOperationBindings"
            ),
            config.failure_class_descent_data_missing.as_str(),
            "must be an object",
        ));
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn project_world_descent_contract(config: &WorldDescentConfig) -> WorldDescentContractProjection {
    WorldDescentContractProjection {
        contract_id: WORLD_DESCENT_CONTRACT_ID.to_string(),
        required_route_families: config.required_families.iter().cloned().collect(),
        required_action_route_bindings: config
            .required_action_bindings
            .iter()
            .map(|(route_family_id, host_action_ids)| {
                (
                    route_family_id.clone(),
                    host_action_ids.iter().cloned().collect::<Vec<_>>(),
                )
            })
            .collect(),
        required_static_operation_bindings: config
            .required_static_bindings
            .iter()
            .map(|(route_family_id, operation_ids)| {
                (
                    route_family_id.clone(),
                    operation_ids.iter().cloned().collect::<Vec<_>>(),
                )
            })
            .collect(),
        failure_classes: expected_failure_classes(config),
    }
}

fn expected_failure_classes(config: &WorldDescentConfig) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    out.insert(
        "identityMissing".to_string(),
        config.failure_class_identity_missing.clone(),
    );
    out.insert(
        "descentDataMissing".to_string(),
        config.failure_class_descent_data_missing.clone(),
    );
    out.insert(
        "kcirHandoffIdentityMissing".to_string(),
        config.failure_class_kcir_handoff_identity_missing.clone(),
    );
    out
}

fn parse_runtime_route_operation_ids(control_plane_contract: &Value) -> Vec<String> {
    let Some(contract_obj) = control_plane_contract.as_object() else {
        return Vec::new();
    };
    let Some(runtime_route_bindings) = contract_obj.get("runtimeRouteBindings") else {
        return Vec::new();
    };
    let Some(runtime_route_bindings_obj) = runtime_route_bindings.as_object() else {
        return Vec::new();
    };
    let Some(required_operation_routes) = runtime_route_bindings_obj.get("requiredOperationRoutes")
    else {
        return Vec::new();
    };
    let Some(required_operation_routes_obj) = required_operation_routes.as_object() else {
        return Vec::new();
    };
    required_operation_routes_obj
        .values()
        .filter_map(Value::as_object)
        .filter_map(|route_obj| non_empty_string(route_obj.get("operationId")))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn route_bindings_to_rows(
    rows: BTreeMap<String, BTreeSet<String>>,
) -> Vec<DoctrineRequiredRouteBinding> {
    rows.into_iter()
        .map(
            |(route_family_id, operation_ids)| DoctrineRequiredRouteBinding {
                route_family_id,
                operation_ids: operation_ids.into_iter().collect(),
            },
        )
        .collect()
}

fn control_plane_contract_issue_with_failure(
    path: String,
    failure_class: &str,
    message: &str,
) -> DoctrineValidationIssue {
    DoctrineValidationIssue {
        failure_class: failure_class.to_string(),
        path,
        message: message.to_string(),
    }
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn world_registry_mode_extracts_bindings() {
        let control_plane = serde_json::json!({
            "worldDescentContract": {
                "contractId": "doctrine.world_descent.v1",
                "requiredRouteFamilies": [
                    "route.gate_execution",
                    "route.instruction_execution",
                    "route.required_decision_attestation",
                    "route.fiber.lifecycle",
                    "route.issue_claim_lease",
                    "route.session_projection",
                    "route.transport.dispatch"
                ],
                "requiredActionRouteBindings": {
                    "route.instruction_execution": ["instruction.run"],
                    "route.required_decision_attestation": ["required.witness_verify", "required.witness_decide"],
                    "route.fiber.lifecycle": ["fiber.spawn", "fiber.join", "fiber.cancel"],
                    "route.issue_claim_lease": [
                        "issue.claim_next",
                        "issue.claim",
                        "issue.lease_renew",
                        "issue.lease_release",
                        "issue.discover"
                    ]
                },
                "requiredStaticOperationBindings": {
                    "route.transport.dispatch": ["op/transport.world_route_binding"]
                },
                "failureClasses": {
                    "identityMissing": "world_route_identity_missing",
                    "descentDataMissing": "world_descent_data_missing",
                    "kcirHandoffIdentityMissing": "kcir_handoff_identity_missing"
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
            }
        });

        let (requirements, issues) =
            derive_world_descent_requirements_for_world_registry_check(&control_plane);
        assert!(issues.is_empty());
        assert_eq!(requirements.families.len(), 7);
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
        assert_eq!(
            by_family.get("route.transport.dispatch"),
            Some(&vec!["op/transport.world_route_binding".to_string()])
        );
    }

    #[test]
    fn world_registry_mode_reports_unbound_operation_rows() {
        let control_plane = serde_json::json!({
            "worldDescentContract": {
                "contractId": "doctrine.world_descent.v1",
                "requiredRouteFamilies": [
                    "route.gate_execution",
                    "route.instruction_execution",
                    "route.required_decision_attestation",
                    "route.fiber.lifecycle",
                    "route.issue_claim_lease",
                    "route.session_projection",
                    "route.transport.dispatch"
                ],
                "requiredActionRouteBindings": {
                    "route.instruction_execution": ["instruction.run"],
                    "route.required_decision_attestation": ["required.witness_verify", "required.witness_decide"],
                    "route.fiber.lifecycle": ["fiber.spawn", "fiber.join", "fiber.cancel"],
                    "route.issue_claim_lease": [
                        "issue.claim_next",
                        "issue.claim",
                        "issue.lease_renew",
                        "issue.lease_release",
                        "issue.discover"
                    ]
                },
                "requiredStaticOperationBindings": {
                    "route.transport.dispatch": ["op/transport.world_route_binding"]
                },
                "failureClasses": {
                    "identityMissing": "world_route_identity_missing",
                    "descentDataMissing": "world_descent_data_missing",
                    "kcirHandoffIdentityMissing": "kcir_handoff_identity_missing"
                }
            },
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
            derive_world_descent_requirements_for_world_registry_check(&control_plane);
        assert!(!issues.is_empty());
        assert!(
            issues
                .iter()
                .any(|issue| issue.failure_class == WORLD_ROUTE_IDENTITY_MISSING)
        );
        assert!(issues.iter().any(|issue| {
            issue.path
                == "controlPlaneContract.hostActionSurface.requiredActions.required.witness_verify.operationId"
        }));
    }

    #[test]
    fn runtime_mode_requires_failure_class_fields() {
        let control_plane = serde_json::json!({
            "worldDescentContract": {
                "contractId": "doctrine.world_descent.v1",
                "requiredRouteFamilies": ["route.gate_execution"],
                "requiredActionRouteBindings": {"route.instruction_execution": ["instruction.run"]},
                "requiredStaticOperationBindings": {"route.transport.dispatch": ["op/transport.world_route_binding"]},
                "failureClasses": {}
            },
            "hostActionSurface": {
                "requiredActions": {
                    "instruction.run": {"operationId": "op/mcp.instruction_run"}
                }
            }
        });

        let (_requirements, issues) =
            derive_world_descent_requirements_for_runtime_orchestration(&control_plane);
        assert!(issues.iter().any(|issue| {
            issue.path == "controlPlaneContract.worldDescentContract.failureClasses.identityMissing"
        }));
        assert!(issues.iter().any(|issue| {
            issue.path
                == "controlPlaneContract.worldDescentContract.failureClasses.descentDataMissing"
        }));
        assert!(issues.iter().any(|issue| {
            issue.path
                == "controlPlaneContract.worldDescentContract.failureClasses.kcirHandoffIdentityMissing"
        }));
    }

    #[test]
    fn world_registry_mode_requires_failure_class_fields() {
        let control_plane = serde_json::json!({
            "worldDescentContract": {
                "contractId": "doctrine.world_descent.v1",
                "requiredRouteFamilies": ["route.gate_execution"],
                "requiredActionRouteBindings": {"route.instruction_execution": ["instruction.run"]},
                "requiredStaticOperationBindings": {"route.transport.dispatch": ["op/transport.world_route_binding"]},
                "failureClasses": {}
            },
            "hostActionSurface": {
                "requiredActions": {
                    "instruction.run": {"operationId": "op/mcp.instruction_run"}
                }
            }
        });

        let (_requirements, issues) =
            derive_world_descent_requirements_for_world_registry_check(&control_plane);
        assert!(issues.iter().any(|issue| {
            issue.path == "controlPlaneContract.worldDescentContract.failureClasses.identityMissing"
        }));
        assert!(issues.iter().any(|issue| {
            issue.path
                == "controlPlaneContract.worldDescentContract.failureClasses.descentDataMissing"
        }));
        assert!(issues.iter().any(|issue| {
            issue.path
                == "controlPlaneContract.worldDescentContract.failureClasses.kcirHandoffIdentityMissing"
        }));
    }

    #[test]
    fn runtime_mode_includes_runtime_route_gate_operations() {
        let control_plane = serde_json::json!({
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"]
                    }
                }
            },
            "worldDescentContract": {
                "contractId": "doctrine.world_descent.v1",
                "requiredRouteFamilies": [
                    "route.gate_execution",
                    "route.instruction_execution",
                    "route.required_decision_attestation",
                    "route.fiber.lifecycle",
                    "route.issue_claim_lease",
                    "route.session_projection",
                    "route.transport.dispatch"
                ],
                "requiredActionRouteBindings": {
                    "route.instruction_execution": ["instruction.run"],
                    "route.required_decision_attestation": ["required.witness_verify", "required.witness_decide"],
                    "route.fiber.lifecycle": ["fiber.spawn", "fiber.join", "fiber.cancel"],
                    "route.issue_claim_lease": [
                        "issue.claim_next",
                        "issue.claim",
                        "issue.lease_renew",
                        "issue.lease_release",
                        "issue.discover"
                    ]
                },
                "requiredStaticOperationBindings": {
                    "route.transport.dispatch": ["op/transport.world_route_binding"]
                },
                "failureClasses": {
                    "identityMissing": "world_route_identity_missing",
                    "descentDataMissing": "world_descent_data_missing",
                    "kcirHandoffIdentityMissing": "kcir_handoff_identity_missing"
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
            }
        });

        let (requirements, _issues) =
            derive_world_descent_requirements_for_runtime_orchestration(&control_plane);
        let gate_binding = requirements
            .bindings
            .iter()
            .find(|row| row.route_family_id == "route.gate_execution")
            .expect("missing route.gate_execution binding");
        assert_eq!(
            gate_binding.operation_ids,
            vec!["op/ci.run_gate".to_string()]
        );
    }
}
