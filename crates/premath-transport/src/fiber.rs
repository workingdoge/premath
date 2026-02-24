use serde_json::Value;

use crate::types::*;
use crate::*;

pub(crate) fn fiber_token(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "fiber".to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn derive_fiber_id(task_ref: &str, parent_fiber_id: Option<&str>) -> String {
    let digest = semantic_digest(&[
        TRANSPORT_PROFILE_ID,
        "fiber.spawn",
        task_ref,
        parent_fiber_id.unwrap_or(""),
    ]);
    let suffix = digest
        .strip_prefix(TRANSPORT_SEMANTIC_DIGEST_PREFIX)
        .unwrap_or("");
    format!("fib1_{}", &suffix[..16])
}

pub(crate) fn fiber_witness_ref(action: &str, fiber_id: &str) -> String {
    let digest = semantic_digest(&[TRANSPORT_PROFILE_ID, action, fiber_id]);
    format!(
        "fiber://dispatch/{action}/{}/{}",
        fiber_token(fiber_id),
        digest
    )
}

pub(crate) fn fiber_rejected(
    action_id: TransportActionId,
    failure_class: &str,
    diagnostic: &str,
) -> Value {
    transport_rejected(
        transport_action_spec(action_id).action,
        action_id.as_str(),
        failure_class,
        diagnostic.to_string(),
    )
}

pub(crate) fn fiber_spawn_response(payload: Value) -> Value {
    let action_id = TransportActionId::FiberSpawn;
    let parsed = match serde_json::from_value::<FiberSpawnRequest>(payload) {
        Ok(value) => value,
        Err(source) => {
            return fiber_rejected(
                action_id,
                FAILURE_FIBER_INVALID_PAYLOAD,
                &format!("invalid fiber.spawn payload: {source}"),
            );
        }
    };

    let task_ref = parsed.task_ref.trim().to_string();
    if task_ref.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.spawn requires taskRef",
        );
    }
    let parent_fiber_id = non_empty(parsed.parent_fiber_id);
    let scope_ref = non_empty(parsed.scope_ref);
    let fiber_id = non_empty(parsed.fiber_id)
        .unwrap_or_else(|| derive_fiber_id(&task_ref, parent_fiber_id.as_deref()));
    let witness_ref = fiber_witness_ref("fiber.spawn", &fiber_id);

    serde_json::json!({
        "schema": 1,
        "action": transport_action_spec(action_id).action,
        "result": "accepted",
        "failureClasses": [],
        "worldBinding": world_binding_for_action(action_id),
        "fiberId": fiber_id,
        "taskRef": task_ref,
        "parentFiberId": parent_fiber_id,
        "scopeRef": scope_ref,
        "fiberWitnessRef": witness_ref
    })
}

pub(crate) fn fiber_join_response(payload: Value) -> Value {
    let action_id = TransportActionId::FiberJoin;
    let parsed = match serde_json::from_value::<FiberJoinRequest>(payload) {
        Ok(value) => value,
        Err(source) => {
            return fiber_rejected(
                action_id,
                FAILURE_FIBER_INVALID_PAYLOAD,
                &format!("invalid fiber.join payload: {source}"),
            );
        }
    };

    let fiber_id = parsed.fiber_id.trim().to_string();
    if fiber_id.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.join requires fiberId",
        );
    }
    let join_set = parsed
        .join_set
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if join_set.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.join requires non-empty joinSet",
        );
    }
    let witness_ref = fiber_witness_ref("fiber.join", &fiber_id);

    serde_json::json!({
        "schema": 1,
        "action": transport_action_spec(action_id).action,
        "result": "accepted",
        "failureClasses": [],
        "worldBinding": world_binding_for_action(action_id),
        "fiberId": fiber_id,
        "joinSet": join_set,
        "resultRef": non_empty(parsed.result_ref),
        "fiberWitnessRef": witness_ref
    })
}

pub(crate) fn fiber_cancel_response(payload: Value) -> Value {
    let action_id = TransportActionId::FiberCancel;
    let parsed = match serde_json::from_value::<FiberCancelRequest>(payload) {
        Ok(value) => value,
        Err(source) => {
            return fiber_rejected(
                action_id,
                FAILURE_FIBER_INVALID_PAYLOAD,
                &format!("invalid fiber.cancel payload: {source}"),
            );
        }
    };

    let fiber_id = parsed.fiber_id.trim().to_string();
    if fiber_id.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.cancel requires fiberId",
        );
    }
    let witness_ref = fiber_witness_ref("fiber.cancel", &fiber_id);

    serde_json::json!({
        "schema": 1,
        "action": transport_action_spec(action_id).action,
        "result": "accepted",
        "failureClasses": [],
        "worldBinding": world_binding_for_action(action_id),
        "fiberId": fiber_id,
        "reason": non_empty(parsed.reason),
        "fiberWitnessRef": witness_ref
    })
}
