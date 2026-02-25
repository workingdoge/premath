//! World registry and route-binding validation primitives.
//!
//! This module provides a single kernel-backed semantics path for world rows,
//! morphism rows, and route bindings.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub mod failure_class {
    pub const WORLD_ROUTE_UNBOUND: &str = "world_route_unbound";
    pub const WORLD_ROUTE_UNKNOWN_WORLD: &str = "world_route_unknown_world";
    pub const WORLD_ROUTE_UNKNOWN_MORPHISM: &str = "world_route_unknown_morphism";
    pub const WORLD_ROUTE_MORPHISM_DRIFT: &str = "world_route_morphism_drift";
}

pub const WORLD_REGISTRY_KIND: &str = "premath.world_registry.v1";
pub const WORLD_REGISTRY_SCHEMA: u32 = 1;
pub const WORLD_ROUTE_BINDINGS_KIND: &str = "premath.world_route_bindings.v1";
pub const WORLD_ROUTE_BINDINGS_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldRegistry {
    pub schema: u32,
    pub registry_kind: String,
    pub worlds: Vec<WorldRow>,
    pub morphisms: Vec<WorldMorphismRow>,
    pub route_bindings: Vec<RouteBindingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldRow {
    pub world_id: String,
    pub role: String,
    pub context_family_id: String,
    pub definable_family_id: String,
    pub cover_kind: String,
    pub equality_mode: String,
    #[serde(default)]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldMorphismRow {
    pub morphism_row_id: String,
    pub from_world_id: String,
    pub to_world_id: String,
    #[serde(default)]
    pub doctrine_morphisms: Vec<String>,
    #[serde(default)]
    pub preservation_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouteBindingRow {
    pub route_family_id: String,
    #[serde(default)]
    pub operation_ids: Vec<String>,
    pub world_id: String,
    pub morphism_row_id: String,
    pub failure_class_unbound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationRouteRow {
    pub operation_id: String,
    #[serde(default)]
    pub morphisms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub failure_class: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub result: String,
    pub failure_classes: Vec<String>,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    fn from_issues(mut issues: Vec<ValidationIssue>) -> Self {
        issues.sort_by(|a, b| {
            (&a.path, &a.failure_class, &a.message).cmp(&(&b.path, &b.failure_class, &b.message))
        });
        let failure_classes: Vec<String> = issues
            .iter()
            .map(|issue| issue.failure_class.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            result: if issues.is_empty() {
                "accepted".to_string()
            } else {
                "rejected".to_string()
            },
            failure_classes,
            issues,
        }
    }
}

fn push_issue(
    issues: &mut Vec<ValidationIssue>,
    failure_class: &str,
    path: String,
    message: String,
) {
    issues.push(ValidationIssue {
        failure_class: failure_class.to_string(),
        path,
        message,
    });
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldRouteBindingRow {
    pub route_family_id: String,
    pub operation_ids: Vec<String>,
    pub world_id: String,
    pub morphism_row_id: String,
    pub required_morphisms: Vec<String>,
    pub failure_class_unbound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredRouteBinding {
    pub route_family_id: String,
    #[serde(default)]
    pub operation_ids: Vec<String>,
}

pub fn parse_operation_route_rows(raw: &Value) -> Result<Vec<OperationRouteRow>, String> {
    if let Some(rows) = raw.as_array() {
        return parse_operation_route_row_array(rows);
    }
    let Some(obj) = raw.as_object() else {
        return Err("root must be an array or object".to_string());
    };
    let Some(operations_value) = obj.get("operations") else {
        return Err("object root must contain an `operations` array".to_string());
    };
    let Some(rows) = operations_value.as_array() else {
        return Err("`operations` must be an array".to_string());
    };
    parse_operation_route_row_array(rows)
}

fn parse_operation_route_row_array(rows: &[Value]) -> Result<Vec<OperationRouteRow>, String> {
    let mut parsed = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let Some(obj) = row.as_object() else {
            return Err(format!("operations[{idx}] must be an object"));
        };
        let operation_id = obj
            .get("operationId")
            .or_else(|| obj.get("id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("operations[{idx}] is missing operationId/id"))?
            .to_string();
        let morphisms = obj
            .get("morphisms")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        parsed.push(OperationRouteRow {
            operation_id,
            morphisms,
        });
    }
    Ok(parsed)
}

fn parse_non_empty_string_list(value: Option<&Value>, path: &str) -> Result<Vec<String>, String> {
    let Some(rows) = value.and_then(Value::as_array) else {
        return Err(format!("{path} must be a non-empty list"));
    };
    if rows.is_empty() {
        return Err(format!("{path} must be a non-empty list"));
    }
    let mut out = Vec::with_capacity(rows.len());
    for (idx, item) in rows.iter().enumerate() {
        let Some(token) = item
            .as_str()
            .map(str::trim)
            .filter(|token| !token.is_empty())
        else {
            return Err(format!("{path}[{idx}] must be a non-empty string"));
        };
        out.push(token.to_string());
    }
    Ok(out)
}

pub fn parse_world_route_binding_rows(
    site_input: &Value,
) -> Result<Vec<WorldRouteBindingRow>, String> {
    let Some(site_obj) = site_input.as_object() else {
        return Err("DOCTRINE-SITE-INPUT root must be an object".to_string());
    };
    let Some(block_raw) = site_obj.get("worldRouteBindings") else {
        return Err("DOCTRINE-SITE-INPUT.worldRouteBindings must be an object".to_string());
    };
    let Some(block) = block_raw.as_object() else {
        return Err("DOCTRINE-SITE-INPUT.worldRouteBindings must be an object".to_string());
    };

    if let Some(schema) = block.get("schema").and_then(Value::as_u64)
        && schema != WORLD_ROUTE_BINDINGS_SCHEMA as u64
    {
        return Err(format!(
            "DOCTRINE-SITE-INPUT.worldRouteBindings.schema must equal {WORLD_ROUTE_BINDINGS_SCHEMA}, got {schema}"
        ));
    }

    let Some(binding_kind) = block
        .get("bindingKind")
        .and_then(Value::as_str)
        .map(str::trim)
    else {
        return Err(
            "DOCTRINE-SITE-INPUT.worldRouteBindings.bindingKind must be a string".to_string(),
        );
    };
    if binding_kind != WORLD_ROUTE_BINDINGS_KIND {
        return Err(format!(
            "DOCTRINE-SITE-INPUT.worldRouteBindings.bindingKind must equal {:?}",
            WORLD_ROUTE_BINDINGS_KIND
        ));
    }

    let Some(rows) = block.get("rows").and_then(Value::as_array) else {
        return Err(
            "DOCTRINE-SITE-INPUT.worldRouteBindings.rows must be a non-empty list".to_string(),
        );
    };
    if rows.is_empty() {
        return Err(
            "DOCTRINE-SITE-INPUT.worldRouteBindings.rows must be a non-empty list".to_string(),
        );
    }

    let mut out = Vec::with_capacity(rows.len());
    let mut seen_route_families: BTreeSet<String> = BTreeSet::new();
    for (idx, row) in rows.iter().enumerate() {
        let path = format!("DOCTRINE-SITE-INPUT.worldRouteBindings.rows[{idx}]");
        let Some(obj) = row.as_object() else {
            return Err(format!("{path} must be an object"));
        };
        let Some(route_family_id) = obj
            .get("routeFamilyId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(format!("{path}.routeFamilyId must be a non-empty string"));
        };
        if !seen_route_families.insert(route_family_id.to_string()) {
            return Err(format!(
                "{path}.routeFamilyId duplicates existing family {:?}",
                route_family_id
            ));
        }
        let operation_ids =
            parse_non_empty_string_list(obj.get("operationIds"), &format!("{path}.operationIds"))?;
        let Some(world_id) = obj
            .get("worldId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(format!("{path}.worldId must be a non-empty string"));
        };
        let Some(morphism_row_id) = obj
            .get("morphismRowId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(format!("{path}.morphismRowId must be a non-empty string"));
        };
        let required_morphisms = parse_non_empty_string_list(
            obj.get("requiredMorphisms"),
            &format!("{path}.requiredMorphisms"),
        )?;
        let Some(failure_class_unbound) = obj
            .get("failureClassUnbound")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(format!(
                "{path}.failureClassUnbound must be a non-empty string"
            ));
        };

        out.push(WorldRouteBindingRow {
            route_family_id: route_family_id.to_string(),
            operation_ids,
            world_id: world_id.to_string(),
            morphism_row_id: morphism_row_id.to_string(),
            required_morphisms,
            failure_class_unbound: failure_class_unbound.to_string(),
        });
    }
    Ok(out)
}

fn synthesize_world_registry(
    rows: &[WorldRouteBindingRow],
) -> (WorldRegistry, Vec<ValidationIssue>) {
    let mut issues = Vec::new();
    let mut worlds: BTreeMap<String, WorldRow> = BTreeMap::new();
    let mut morphisms: BTreeMap<String, WorldMorphismRow> = BTreeMap::new();
    let mut route_bindings = Vec::with_capacity(rows.len());

    for (idx, row) in rows.iter().enumerate() {
        let world_id = row.world_id.trim().to_string();
        worlds.entry(world_id.clone()).or_insert_with(|| WorldRow {
            world_id: world_id.clone(),
            role: "world_route_binding".to_string(),
            context_family_id: "ctx.world_route".to_string(),
            definable_family_id: "def.world_route".to_string(),
            cover_kind: "route_cover".to_string(),
            equality_mode: "strict".to_string(),
            source_refs: vec!["worldRouteBindings".to_string()],
        });

        let required_morphisms_set: BTreeSet<String> = row
            .required_morphisms
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        let candidate = WorldMorphismRow {
            morphism_row_id: row.morphism_row_id.trim().to_string(),
            from_world_id: world_id.clone(),
            to_world_id: world_id.clone(),
            doctrine_morphisms: required_morphisms_set.iter().cloned().collect(),
            preservation_claims: Vec::new(),
        };
        if let Some(existing) = morphisms.get(&candidate.morphism_row_id) {
            if existing.from_world_id != candidate.from_world_id
                || existing.to_world_id != candidate.to_world_id
                || existing.doctrine_morphisms != candidate.doctrine_morphisms
            {
                push_issue(
                    &mut issues,
                    failure_class::WORLD_ROUTE_UNBOUND,
                    format!("routeBindings[{idx}].morphismRowId"),
                    format!(
                        "morphismRowId {} has conflicting worldId/requiredMorphisms declarations",
                        candidate.morphism_row_id
                    ),
                );
            }
        } else {
            morphisms.insert(candidate.morphism_row_id.clone(), candidate);
        }

        route_bindings.push(RouteBindingRow {
            route_family_id: row.route_family_id.trim().to_string(),
            operation_ids: row
                .operation_ids
                .iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
            world_id,
            morphism_row_id: row.morphism_row_id.trim().to_string(),
            failure_class_unbound: row.failure_class_unbound.trim().to_string(),
        });
    }

    (
        WorldRegistry {
            schema: WORLD_REGISTRY_SCHEMA,
            registry_kind: WORLD_REGISTRY_KIND.to_string(),
            worlds: worlds.into_values().collect(),
            morphisms: morphisms.into_values().collect(),
            route_bindings,
        },
        issues,
    )
}

pub fn validate_world_route_bindings(
    site_input: &Value,
    operations: &[OperationRouteRow],
) -> ValidationReport {
    validate_world_route_bindings_with_requirements(site_input, operations, &[], &[])
}

pub fn validate_world_route_bindings_with_required_families(
    site_input: &Value,
    operations: &[OperationRouteRow],
    required_route_families: &[String],
) -> ValidationReport {
    validate_world_route_bindings_with_requirements(
        site_input,
        operations,
        required_route_families,
        &[],
    )
}

pub fn validate_world_route_bindings_with_requirements(
    site_input: &Value,
    operations: &[OperationRouteRow],
    required_route_families: &[String],
    required_route_bindings: &[RequiredRouteBinding],
) -> ValidationReport {
    let parsed_rows = match parse_world_route_binding_rows(site_input) {
        Ok(rows) => rows,
        Err(message) => {
            return ValidationReport::from_issues(vec![ValidationIssue {
                failure_class: failure_class::WORLD_ROUTE_UNBOUND.to_string(),
                path: "worldRouteBindings".to_string(),
                message,
            }]);
        }
    };

    let route_families: BTreeSet<String> = parsed_rows
        .iter()
        .map(|row| row.route_family_id.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let mut required_families: BTreeSet<String> = required_route_families
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let mut route_index_by_family: BTreeMap<String, usize> = BTreeMap::new();
    let mut route_operation_ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (idx, row) in parsed_rows.iter().enumerate() {
        let family_id = row.route_family_id.trim().to_string();
        route_index_by_family.insert(family_id.clone(), idx);
        route_operation_ids.insert(
            family_id,
            row.operation_ids
                .iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        );
    }
    for required in required_route_bindings {
        let family_id = required.route_family_id.trim();
        if family_id.is_empty() {
            continue;
        }
        required_families.insert(family_id.to_string());
    }
    let missing_families: Vec<String> = required_families
        .difference(&route_families)
        .cloned()
        .collect();

    let (registry, mut synthesis_issues) = synthesize_world_registry(&parsed_rows);
    for required in required_route_bindings {
        let family_id = required.route_family_id.trim();
        if family_id.is_empty() {
            continue;
        }
        let required_operation_ids: BTreeSet<String> = required
            .operation_ids
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if required_operation_ids.is_empty() {
            continue;
        }
        let Some(actual_operation_ids) = route_operation_ids.get(family_id) else {
            continue;
        };
        let missing_operation_ids: Vec<String> = required_operation_ids
            .difference(actual_operation_ids)
            .cloned()
            .collect();
        if missing_operation_ids.is_empty() {
            continue;
        }
        let path = route_index_by_family
            .get(family_id)
            .map(|idx| format!("routeBindings[{idx}].operationIds"))
            .unwrap_or_else(|| "worldRouteBindings.rows".to_string());
        push_issue(
            &mut synthesis_issues,
            failure_class::WORLD_ROUTE_UNBOUND,
            path,
            format!(
                "routeFamilyId {family_id} missing required operationIds: {}",
                missing_operation_ids.join(", ")
            ),
        );
    }
    if !missing_families.is_empty() {
        push_issue(
            &mut synthesis_issues,
            failure_class::WORLD_ROUTE_UNBOUND,
            "worldRouteBindings.rows".to_string(),
            format!(
                "missing required route families: {}",
                missing_families.join(", ")
            ),
        );
    }
    let mut report = validate_world_bindings_against_operations(&registry, operations);
    if synthesis_issues.is_empty() {
        return report;
    }
    synthesis_issues.append(&mut report.issues);
    ValidationReport::from_issues(synthesis_issues)
}

pub fn validate_world_registry(registry: &WorldRegistry) -> ValidationReport {
    let mut issues = Vec::new();
    if registry.schema != WORLD_REGISTRY_SCHEMA {
        push_issue(
            &mut issues,
            failure_class::WORLD_ROUTE_UNBOUND,
            "schema".to_string(),
            format!(
                "schema must be {WORLD_REGISTRY_SCHEMA}, got {}",
                registry.schema
            ),
        );
    }
    if registry.registry_kind.trim() != WORLD_REGISTRY_KIND {
        push_issue(
            &mut issues,
            failure_class::WORLD_ROUTE_UNBOUND,
            "registryKind".to_string(),
            format!("registryKind must equal {:?}", WORLD_REGISTRY_KIND),
        );
    }
    if registry.worlds.is_empty() {
        push_issue(
            &mut issues,
            failure_class::WORLD_ROUTE_UNBOUND,
            "worlds".to_string(),
            "worlds must be non-empty".to_string(),
        );
    }
    if registry.morphisms.is_empty() {
        push_issue(
            &mut issues,
            failure_class::WORLD_ROUTE_UNBOUND,
            "morphisms".to_string(),
            "morphisms must be non-empty".to_string(),
        );
    }
    if registry.route_bindings.is_empty() {
        push_issue(
            &mut issues,
            failure_class::WORLD_ROUTE_UNBOUND,
            "routeBindings".to_string(),
            "routeBindings must be non-empty".to_string(),
        );
    }

    let mut world_ids = BTreeSet::new();
    for (idx, world) in registry.worlds.iter().enumerate() {
        let path = format!("worlds[{idx}]");
        if world.world_id.trim().is_empty() {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("{path}.worldId"),
                "worldId must be non-empty".to_string(),
            );
            continue;
        }
        if !world_ids.insert(world.world_id.trim().to_string()) {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("{path}.worldId"),
                format!("duplicate worldId {}", world.world_id.trim()),
            );
        }
    }

    let world_ids_ref = world_ids.clone();
    let mut morphism_ids = BTreeSet::new();
    for (idx, morphism) in registry.morphisms.iter().enumerate() {
        let path = format!("morphisms[{idx}]");
        if morphism.morphism_row_id.trim().is_empty() {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("{path}.morphismRowId"),
                "morphismRowId must be non-empty".to_string(),
            );
        } else if !morphism_ids.insert(morphism.morphism_row_id.trim().to_string()) {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("{path}.morphismRowId"),
                format!(
                    "duplicate morphismRowId {}",
                    morphism.morphism_row_id.trim()
                ),
            );
        }
        if !world_ids_ref.contains(morphism.from_world_id.trim()) {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNKNOWN_WORLD,
                format!("{path}.fromWorldId"),
                format!("unknown worldId {}", morphism.from_world_id.trim()),
            );
        }
        if !world_ids_ref.contains(morphism.to_world_id.trim()) {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNKNOWN_WORLD,
                format!("{path}.toWorldId"),
                format!("unknown worldId {}", morphism.to_world_id.trim()),
            );
        }
    }

    let mut route_families = BTreeSet::new();
    let morphism_ids_ref = morphism_ids.clone();
    let mut operation_binding_owner: BTreeMap<String, String> = BTreeMap::new();
    for (idx, binding) in registry.route_bindings.iter().enumerate() {
        let path = format!("routeBindings[{idx}]");
        if binding.route_family_id.trim().is_empty() {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("{path}.routeFamilyId"),
                "routeFamilyId must be non-empty".to_string(),
            );
        } else if !route_families.insert(binding.route_family_id.trim().to_string()) {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("{path}.routeFamilyId"),
                format!("duplicate routeFamilyId {}", binding.route_family_id.trim()),
            );
        }
        if binding.operation_ids.is_empty() {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("{path}.operationIds"),
                "operationIds must be non-empty".to_string(),
            );
        }
        if !world_ids_ref.contains(binding.world_id.trim()) {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNKNOWN_WORLD,
                format!("{path}.worldId"),
                format!("unknown worldId {}", binding.world_id.trim()),
            );
        }
        if !morphism_ids_ref.contains(binding.morphism_row_id.trim()) {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNKNOWN_MORPHISM,
                format!("{path}.morphismRowId"),
                format!("unknown morphismRowId {}", binding.morphism_row_id.trim()),
            );
        }
        for (op_idx, operation_id) in binding.operation_ids.iter().enumerate() {
            if operation_id.trim().is_empty() {
                push_issue(
                    &mut issues,
                    failure_class::WORLD_ROUTE_UNBOUND,
                    format!("{path}.operationIds[{op_idx}]"),
                    "operationId must be non-empty".to_string(),
                );
                continue;
            }
            let route_family_id = binding.route_family_id.trim().to_string();
            if let Some(existing) = operation_binding_owner
                .insert(operation_id.trim().to_string(), route_family_id.clone())
                && existing != route_family_id
            {
                push_issue(
                    &mut issues,
                    failure_class::WORLD_ROUTE_UNBOUND,
                    format!("{path}.operationIds[{op_idx}]"),
                    format!(
                        "operation {operation_id} already bound under routeFamilyId {existing}"
                    ),
                );
            }
        }
    }

    ValidationReport::from_issues(issues)
}

pub fn validate_world_bindings_against_operations(
    registry: &WorldRegistry,
    operations: &[OperationRouteRow],
) -> ValidationReport {
    let mut issues = validate_world_registry(registry).issues;

    let mut operation_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (idx, operation) in operations.iter().enumerate() {
        if operation.operation_id.trim().is_empty() {
            push_issue(
                &mut issues,
                failure_class::WORLD_ROUTE_UNBOUND,
                format!("operations[{idx}].operationId"),
                "operationId must be non-empty".to_string(),
            );
            continue;
        }
        operation_map.insert(
            operation.operation_id.trim().to_string(),
            operation
                .morphisms
                .iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect(),
        );
    }

    let morphism_map: BTreeMap<String, BTreeSet<String>> = registry
        .morphisms
        .iter()
        .map(|row| {
            (
                row.morphism_row_id.trim().to_string(),
                row.doctrine_morphisms
                    .iter()
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect(),
            )
        })
        .collect();

    for (idx, binding) in registry.route_bindings.iter().enumerate() {
        let Some(required_morphisms) = morphism_map.get(binding.morphism_row_id.trim()) else {
            continue;
        };
        for (op_idx, operation_id) in binding.operation_ids.iter().enumerate() {
            let Some(actual_morphisms) = operation_map.get(operation_id.trim()) else {
                push_issue(
                    &mut issues,
                    failure_class::WORLD_ROUTE_UNBOUND,
                    format!("routeBindings[{idx}].operationIds[{op_idx}]"),
                    format!("unknown operationId {}", operation_id.trim()),
                );
                continue;
            };
            let missing: Vec<String> = required_morphisms
                .difference(actual_morphisms)
                .cloned()
                .collect();
            if !missing.is_empty() {
                push_issue(
                    &mut issues,
                    failure_class::WORLD_ROUTE_MORPHISM_DRIFT,
                    format!("routeBindings[{idx}].operationIds[{op_idx}]"),
                    format!(
                        "operation {} missing morphisms: {}",
                        operation_id.trim(),
                        missing.join(", ")
                    ),
                );
            }
        }
    }

    ValidationReport::from_issues(issues)
}

pub fn resolve_route_family<'a>(
    registry: &'a WorldRegistry,
    route_family_id: &str,
) -> Option<&'a RouteBindingRow> {
    registry
        .route_bindings
        .iter()
        .find(|row| row.route_family_id.trim() == route_family_id.trim())
}

pub fn resolve_operation_binding<'a>(
    registry: &'a WorldRegistry,
    operation_id: &str,
) -> Option<&'a RouteBindingRow> {
    registry.route_bindings.iter().find(|row| {
        row.operation_ids
            .iter()
            .any(|item| item.trim() == operation_id.trim())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_registry() -> WorldRegistry {
        WorldRegistry {
            schema: WORLD_REGISTRY_SCHEMA,
            registry_kind: WORLD_REGISTRY_KIND.to_string(),
            worlds: vec![
                WorldRow {
                    world_id: "world.kernel.semantic.v1".to_string(),
                    role: "semantic_authority".to_string(),
                    context_family_id: "c.kernel".to_string(),
                    definable_family_id: "e.kernel".to_string(),
                    cover_kind: "site_cover".to_string(),
                    equality_mode: "semantic".to_string(),
                    source_refs: vec!["draft/PREMATH-KERNEL".to_string()],
                },
                WorldRow {
                    world_id: "world.control_plane.bundle.v0".to_string(),
                    role: "control_plane_projection".to_string(),
                    context_family_id: "c.control".to_string(),
                    definable_family_id: "e.control".to_string(),
                    cover_kind: "route_cover".to_string(),
                    equality_mode: "strict".to_string(),
                    source_refs: vec!["draft/CONTROL-PLANE-CONTRACT".to_string()],
                },
            ],
            morphisms: vec![WorldMorphismRow {
                morphism_row_id: "wm.kernel.semantic.runtime_gate".to_string(),
                from_world_id: "world.kernel.semantic.v1".to_string(),
                to_world_id: "world.control_plane.bundle.v0".to_string(),
                doctrine_morphisms: vec![
                    "dm.identity".to_string(),
                    "dm.profile.execution".to_string(),
                    "dm.transport.location".to_string(),
                    "dm.transport.world".to_string(),
                ],
                preservation_claims: vec!["transport_functoriality".to_string()],
            }],
            route_bindings: vec![RouteBindingRow {
                route_family_id: "route.gate_execution".to_string(),
                operation_ids: vec!["op/ci.run_gate".to_string()],
                world_id: "world.kernel.semantic.v1".to_string(),
                morphism_row_id: "wm.kernel.semantic.runtime_gate".to_string(),
                failure_class_unbound: failure_class::WORLD_ROUTE_UNBOUND.to_string(),
            }],
        }
    }

    #[test]
    fn valid_registry_is_accepted() {
        let report = validate_world_registry(&valid_registry());
        assert_eq!(report.result, "accepted");
        assert!(report.issues.is_empty());
    }

    #[test]
    fn rejects_unknown_world_and_morphism_refs() {
        let mut registry = valid_registry();
        registry.route_bindings[0].world_id = "world.unknown".to_string();
        registry.route_bindings[0].morphism_row_id = "wm.unknown".to_string();
        let report = validate_world_registry(&registry);
        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_UNKNOWN_WORLD.to_string())
        );
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_UNKNOWN_MORPHISM.to_string())
        );
    }

    #[test]
    fn rejects_duplicate_operation_binding_across_families() {
        let mut registry = valid_registry();
        registry.route_bindings.push(RouteBindingRow {
            route_family_id: "route.other".to_string(),
            operation_ids: vec!["op/ci.run_gate".to_string()],
            world_id: "world.kernel.semantic.v1".to_string(),
            morphism_row_id: "wm.kernel.semantic.runtime_gate".to_string(),
            failure_class_unbound: failure_class::WORLD_ROUTE_UNBOUND.to_string(),
        });
        let report = validate_world_registry(&registry);
        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_UNBOUND.to_string())
        );
    }

    #[test]
    fn detects_morphism_drift_against_operation_rows() {
        let registry = valid_registry();
        let operations = vec![OperationRouteRow {
            operation_id: "op/ci.run_gate".to_string(),
            morphisms: vec![
                "dm.identity".to_string(),
                "dm.profile.execution".to_string(),
                "dm.transport.world".to_string(),
            ],
        }];
        let report = validate_world_bindings_against_operations(&registry, &operations);
        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_MORPHISM_DRIFT.to_string())
        );
    }

    #[test]
    fn resolve_helpers_are_deterministic() {
        let registry = valid_registry();
        assert!(resolve_route_family(&registry, "route.gate_execution").is_some());
        assert!(resolve_operation_binding(&registry, "op/ci.run_gate").is_some());
        assert!(resolve_route_family(&registry, "route.missing").is_none());
        assert!(resolve_operation_binding(&registry, "op/missing").is_none());
    }

    #[test]
    fn parse_operation_route_rows_supports_registry_object_shape() {
        let raw = json!({
            "operations": [
                { "id": "op/ci.run_gate", "morphisms": ["dm.identity"] },
                { "operationId": "op/ci.run_instruction", "morphisms": ["dm.identity", "dm.profile.execution"] },
            ],
        });
        let rows = parse_operation_route_rows(&raw).expect("operation rows should parse");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].operation_id, "op/ci.run_gate");
        assert_eq!(rows[1].operation_id, "op/ci.run_instruction");
    }

    #[test]
    fn validate_world_route_bindings_detects_morphism_drift() {
        let site_input = json!({
            "worldRouteBindings": {
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": [
                    {
                        "routeFamilyId": "route.gate_execution",
                        "operationIds": ["op/ci.run_gate"],
                        "worldId": "world.kernel.semantic.v1",
                        "morphismRowId": "wm.kernel.semantic.runtime_gate",
                        "requiredMorphisms": ["dm.identity", "dm.profile.execution", "dm.transport.world"],
                        "failureClassUnbound": "world_route_unbound"
                    }
                ]
            }
        });
        let operations = vec![OperationRouteRow {
            operation_id: "op/ci.run_gate".to_string(),
            morphisms: vec![
                "dm.identity".to_string(),
                "dm.profile.execution".to_string(),
            ],
        }];
        let report = validate_world_route_bindings(&site_input, &operations);
        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_MORPHISM_DRIFT.to_string())
        );
    }

    #[test]
    fn validate_world_route_bindings_rejects_missing_required_families() {
        let site_input = json!({
            "worldRouteBindings": {
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": [
                    {
                        "routeFamilyId": "route.gate_execution",
                        "operationIds": ["op/ci.run_gate"],
                        "worldId": "world.kernel.semantic.v1",
                        "morphismRowId": "wm.kernel.semantic.runtime_gate",
                        "requiredMorphisms": ["dm.identity"],
                        "failureClassUnbound": "world_route_unbound"
                    }
                ]
            }
        });
        let operations = vec![OperationRouteRow {
            operation_id: "op/ci.run_gate".to_string(),
            morphisms: vec!["dm.identity".to_string()],
        }];
        let report = validate_world_route_bindings_with_required_families(
            &site_input,
            &operations,
            &["route.instruction_execution".to_string()],
        );
        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_UNBOUND.to_string())
        );
    }

    #[test]
    fn validate_world_route_bindings_rejects_missing_required_route_operations() {
        let site_input = json!({
            "worldRouteBindings": {
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": [
                    {
                        "routeFamilyId": "route.instruction_execution",
                        "operationIds": ["op/ci.run_instruction"],
                        "worldId": "world.instruction.v1",
                        "morphismRowId": "wm.control.instruction.execution",
                        "requiredMorphisms": ["dm.identity", "dm.profile.execution"],
                        "failureClassUnbound": "world_route_unbound"
                    }
                ]
            }
        });
        let operations = vec![OperationRouteRow {
            operation_id: "op/ci.run_instruction".to_string(),
            morphisms: vec![
                "dm.identity".to_string(),
                "dm.profile.execution".to_string(),
            ],
        }];
        let report = validate_world_route_bindings_with_requirements(
            &site_input,
            &operations,
            &[],
            &[RequiredRouteBinding {
                route_family_id: "route.instruction_execution".to_string(),
                operation_ids: vec!["op/mcp.instruction_run".to_string()],
            }],
        );
        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_UNBOUND.to_string())
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "routeBindings[0].operationIds")
        );
    }

    #[test]
    fn validate_world_route_bindings_rejects_missing_declaration_block() {
        let site_input = json!({
            "schema": 1,
            "inputKind": "premath.doctrine_operation_site.input.v1"
        });
        let report = validate_world_route_bindings(&site_input, &[]);
        assert_eq!(report.result, "rejected");
        assert!(
            report
                .failure_classes
                .contains(&failure_class::WORLD_ROUTE_UNBOUND.to_string())
        );
    }
}
