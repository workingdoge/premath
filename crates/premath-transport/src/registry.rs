use premath_kernel::{
    RequiredRouteBinding, parse_operation_route_rows,
    validate_world_route_bindings_with_requirements, world_failure_class,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::types::*;
use crate::*;

pub(crate) fn validate_transport_registry(
    actions: &[TransportActionRegistryRow],
) -> Vec<TransportCheckIssue> {
    let mut issues: Vec<TransportCheckIssue> = Vec::new();
    let mut seen_actions: BTreeSet<String> = BTreeSet::new();
    let mut seen_action_ids: BTreeSet<String> = BTreeSet::new();
    let expected_actions: BTreeSet<String> = TRANSPORT_ACTION_SPECS
        .iter()
        .map(|spec| spec.action.to_string())
        .collect();
    let mut present_actions: BTreeSet<String> = BTreeSet::new();
    let spec_rows: BTreeMap<String, TransportActionRegistryRow> = TRANSPORT_ACTION_SPECS
        .iter()
        .map(|spec| (spec.action.to_string(), transport_action_row(spec)))
        .collect();

    for row in actions {
        present_actions.insert(row.action.clone());

        if row.action.trim().is_empty()
            || row.action_id.trim().is_empty()
            || row.operation_id.trim().is_empty()
            || row.route_family_id.trim().is_empty()
            || row.world_id.trim().is_empty()
            || row.morphism_row_id.trim().is_empty()
            || row.semantic_digest.trim().is_empty()
        {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_EMPTY_FIELD.to_string(),
                path: format!("actions/{}", row.action),
                message: "action row must provide non-empty typed fields".to_string(),
            });
        }

        if !seen_actions.insert(row.action.clone()) {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION.to_string(),
                path: format!("actions/{}", row.action),
                message: "duplicate action row".to_string(),
            });
        }
        if !seen_action_ids.insert(row.action_id.clone()) {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION_ID.to_string(),
                path: format!("actions/{}", row.action_id),
                message: "duplicate actionId row".to_string(),
            });
        }

        if let Some(expected) = spec_rows.get(&row.action)
            && row.semantic_digest != expected.semantic_digest
        {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_DIGEST_MISMATCH.to_string(),
                path: format!("actions/{}/semanticDigest", row.action),
                message: format!(
                    "semanticDigest mismatch (expected={}, got={})",
                    expected.semantic_digest, row.semantic_digest
                ),
            });
        }
    }

    for action in expected_actions {
        if !present_actions.contains(&action) {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_MISSING_ACTION.to_string(),
                path: "actions".to_string(),
                message: format!("missing required action row: {action}"),
            });
        }
    }

    issues.sort_by(|a, b| {
        (&a.failure_class, &a.path, &a.message).cmp(&(&b.failure_class, &b.path, &b.message))
    });
    issues
}

pub(crate) fn validate_transport_action_binding_with_kernel(
    spec: &TransportActionSpec,
) -> Result<(), TransportKernelBindingError> {
    let site_input: Value = serde_json::from_str(DOCTRINE_SITE_INPUT_JSON).map_err(|source| {
        TransportKernelBindingError {
            failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
            diagnostic: format!("failed to parse DOCTRINE-SITE-INPUT: {source}"),
        }
    })?;
    let operation_registry: Value =
        serde_json::from_str(DOCTRINE_OP_REGISTRY_JSON).map_err(|source| {
            TransportKernelBindingError {
                failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
                diagnostic: format!("failed to parse DOCTRINE-OP-REGISTRY: {source}"),
            }
        })?;
    let operations = parse_operation_route_rows(&operation_registry).map_err(|source| {
        TransportKernelBindingError {
            failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
            diagnostic: format!("failed to parse operation rows: {source}"),
        }
    })?;
    let required_families = vec![spec.route_family_id.to_string()];
    let required_bindings = vec![RequiredRouteBinding {
        route_family_id: spec.route_family_id.to_string(),
        operation_ids: vec![spec.operation_id.to_string()],
    }];
    let report = validate_world_route_bindings_with_requirements(
        &site_input,
        &operations,
        &required_families,
        &required_bindings,
    );
    if report.result == "accepted" {
        let resolver = resolve_site_for_spec(spec)?;
        if resolver.result != "accepted" {
            return Err(TransportKernelBindingError {
                failure_class: resolver
                    .failure_classes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| world_failure_class::WORLD_ROUTE_UNBOUND.to_string()),
                diagnostic: format!(
                    "site-resolve rejected action={} operation={} failures={:?}",
                    spec.action, spec.operation_id, resolver.failure_classes
                ),
            });
        }
        let Some(selected) = resolver.selected else {
            return Err(TransportKernelBindingError {
                failure_class: world_failure_class::WORLD_ROUTE_UNBOUND.to_string(),
                diagnostic: format!(
                    "site-resolve missing selected binding for action={} operation={}",
                    spec.action, spec.operation_id
                ),
            });
        };
        let binding_matches = selected.operation_id == spec.operation_id
            && selected.route_family_id == spec.route_family_id
            && selected.world_id == spec.world_id
            && selected.morphism_row_id == spec.morphism_row_id;
        if !binding_matches {
            return Err(TransportKernelBindingError {
                failure_class: world_failure_class::WORLD_ROUTE_UNBOUND.to_string(),
                diagnostic: format!(
                    "site-resolve binding drift for action={} expected=({}, {}, {}, {}) got=({}, {}, {}, {})",
                    spec.action,
                    spec.operation_id,
                    spec.route_family_id,
                    spec.world_id,
                    spec.morphism_row_id,
                    selected.operation_id,
                    selected.route_family_id,
                    selected.world_id,
                    selected.morphism_row_id
                ),
            });
        }
        return Ok(());
    }
    let failure_class = report
        .failure_classes
        .first()
        .cloned()
        .unwrap_or_else(|| world_failure_class::WORLD_ROUTE_UNBOUND.to_string());
    let diagnostic = report.issues.first().map_or_else(
        || {
            format!(
                "kernel route validation rejected action={} operation={} route={}",
                spec.action, spec.operation_id, spec.route_family_id
            )
        },
        |issue| {
            format!(
                "kernel route validation rejected action={} at {}: {}",
                spec.action, issue.path, issue.message
            )
        },
    );
    Err(TransportKernelBindingError {
        failure_class,
        diagnostic,
    })
}

pub fn transport_check() -> TransportCheckReport {
    let actions = transport_action_registry_rows();
    let mut issues = validate_transport_registry(&actions);
    for spec in TRANSPORT_ACTION_SPECS {
        if !spec.route_bound {
            continue; // read_only_projection — no kernel binding validation needed
        }
        if let Err(err) = validate_transport_action_binding_with_kernel(&spec) {
            issues.push(TransportCheckIssue {
                failure_class: err.failure_class,
                path: format!("actions/{}/kernelBinding", spec.action),
                message: err.diagnostic,
            });
        }
    }
    issues.sort_by(|a, b| {
        (&a.failure_class, &a.path, &a.message).cmp(&(&b.failure_class, &b.path, &b.message))
    });
    let mut failure_classes: Vec<String> = issues
        .iter()
        .map(|issue| issue.failure_class.clone())
        .collect();
    failure_classes.sort();
    failure_classes.dedup();
    let result = if issues.is_empty() {
        "accepted".to_string()
    } else {
        "rejected".to_string()
    };
    let semantic_digest = transport_check_digest(&result, &failure_classes, &actions);

    TransportCheckReport {
        schema: 1,
        check_kind: TRANSPORT_CHECK_KIND.to_string(),
        registry_kind: TRANSPORT_ACTION_REGISTRY_KIND.to_string(),
        profile_id: TRANSPORT_PROFILE_ID.to_string(),
        result,
        failure_classes,
        issues,
        action_count: actions.len(),
        actions,
        semantic_digest,
    }
}
