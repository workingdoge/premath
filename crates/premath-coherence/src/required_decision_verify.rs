use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const REQUIRED_DECISION_KIND: &str = "ci.required.decision.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredDecisionVerifyRequest {
    pub decision: Value,
    #[serde(default)]
    pub witness: Option<Value>,
    #[serde(default, rename = "deltaSnapshot")]
    pub delta_snapshot: Option<Value>,
    #[serde(default)]
    pub actual_witness_sha256: Option<String>,
    #[serde(default)]
    pub actual_delta_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredDecisionVerifyDerived {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typed_core_projection_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_payload_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalizer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_checks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredDecisionVerifyResult {
    pub errors: Vec<String>,
    pub derived: RequiredDecisionVerifyDerived,
}

fn parse_non_empty_string(
    value: Option<&Value>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<String> {
    let Some(raw) = value else {
        errors.push(format!("{label} must be a non-empty string"));
        return None;
    };
    let Some(text) = raw.as_str() else {
        errors.push(format!("{label} must be a non-empty string"));
        return None;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        errors.push(format!("{label} must be a non-empty string"));
        return None;
    }
    Some(trimmed.to_string())
}

fn parse_string_list(
    value: Option<&Value>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<Vec<String>> {
    let start_errors = errors.len();
    let Some(Value::Array(items)) = value else {
        errors.push(format!("{label} must be a list"));
        return None;
    };
    let mut out = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let Some(text) = item.as_str() else {
            errors.push(format!("{label}[{idx}] must be a non-empty string"));
            continue;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            errors.push(format!("{label}[{idx}] must be a non-empty string"));
            continue;
        }
        out.push(trimmed.to_string());
    }
    if errors.len() != start_errors {
        return None;
    }
    Some(out)
}

fn parse_non_empty_owned_string(
    value: Option<&String>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<String> {
    let Some(text) = value else {
        errors.push(format!("{label} must be a non-empty string"));
        return None;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        errors.push(format!("{label} must be a non-empty string"));
        return None;
    }
    Some(trimmed.to_string())
}

fn as_object<'a>(
    value: &'a Value,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<&'a Map<String, Value>> {
    let Some(obj) = value.as_object() else {
        errors.push(format!("{label} must be an object"));
        return None;
    };
    Some(obj)
}

pub fn verify_required_decision_request(
    request: &RequiredDecisionVerifyRequest,
) -> RequiredDecisionVerifyResult {
    let mut errors: Vec<String> = Vec::new();
    let mut derived = RequiredDecisionVerifyDerived {
        decision: None,
        projection_digest: None,
        typed_core_projection_digest: None,
        authority_payload_digest: None,
        normalizer_id: None,
        policy_digest: None,
        required_checks: None,
    };

    let Some(decision_obj) = as_object(&request.decision, "decision", &mut errors) else {
        return RequiredDecisionVerifyResult { errors, derived };
    };

    let decision_kind = parse_non_empty_string(
        decision_obj.get("decisionKind"),
        "decision.decisionKind",
        &mut errors,
    );
    if decision_kind.as_deref() != Some(REQUIRED_DECISION_KIND) {
        errors.push(format!(
            "decisionKind must be {REQUIRED_DECISION_KIND:?} (actual={decision_kind:?})"
        ));
    }

    let decision_value = parse_non_empty_string(
        decision_obj.get("decision"),
        "decision.decision",
        &mut errors,
    );
    if let Some(value) = decision_value.as_deref() {
        if value != "accept" && value != "reject" {
            errors.push("decision must be 'accept' or 'reject'".to_string());
        } else {
            derived.decision = Some(value.to_string());
        }
    }

    let projection_digest = parse_non_empty_string(
        decision_obj.get("projectionDigest"),
        "decision.projectionDigest",
        &mut errors,
    );
    if let Some(value) = projection_digest.clone() {
        derived.projection_digest = Some(value);
    }
    let typed_core_projection_digest = parse_non_empty_string(
        decision_obj.get("typedCoreProjectionDigest"),
        "decision.typedCoreProjectionDigest",
        &mut errors,
    );
    if let Some(value) = typed_core_projection_digest.clone() {
        derived.typed_core_projection_digest = Some(value);
    }
    let authority_payload_digest = parse_non_empty_string(
        decision_obj.get("authorityPayloadDigest"),
        "decision.authorityPayloadDigest",
        &mut errors,
    );
    if let Some(value) = authority_payload_digest.clone() {
        derived.authority_payload_digest = Some(value);
    }
    let normalizer_id = parse_non_empty_string(
        decision_obj.get("normalizerId"),
        "decision.normalizerId",
        &mut errors,
    );
    if let Some(value) = normalizer_id.clone() {
        derived.normalizer_id = Some(value);
    }
    let policy_digest = parse_non_empty_string(
        decision_obj.get("policyDigest"),
        "decision.policyDigest",
        &mut errors,
    );
    if let Some(value) = policy_digest.clone() {
        derived.policy_digest = Some(value);
    }

    let decision_required_checks = parse_string_list(
        decision_obj.get("requiredChecks"),
        "decision.requiredChecks",
        &mut errors,
    );
    if let Some(required_checks) = decision_required_checks.clone() {
        derived.required_checks = Some(required_checks);
    }

    let expected_witness_sha = parse_non_empty_string(
        decision_obj.get("witnessSha256"),
        "decision.witnessSha256",
        &mut errors,
    );
    if let Some(expected_sha) = expected_witness_sha {
        let actual_sha = parse_non_empty_owned_string(
            request.actual_witness_sha256.as_ref(),
            "actualWitnessSha256",
            &mut errors,
        );
        if let Some(actual_sha) = actual_sha
            && actual_sha != expected_sha
        {
            errors.push(format!(
                "witness sha mismatch (decision={expected_sha}, actual={actual_sha})"
            ));
        }
    }

    let expected_delta_sha = parse_non_empty_string(
        decision_obj.get("deltaSha256"),
        "decision.deltaSha256",
        &mut errors,
    );
    if let Some(expected_sha) = expected_delta_sha {
        let actual_sha = parse_non_empty_owned_string(
            request.actual_delta_sha256.as_ref(),
            "actualDeltaSha256",
            &mut errors,
        );
        if let Some(actual_sha) = actual_sha
            && actual_sha != expected_sha
        {
            errors.push(format!(
                "delta sha mismatch (decision={expected_sha}, actual={actual_sha})"
            ));
        }
    }

    if let Some(witness) = request.witness.as_ref()
        && let Some(witness_obj) = as_object(witness, "witness", &mut errors)
    {
        if let Some(typed_core_projection_digest) = typed_core_projection_digest.as_deref() {
            let witness_typed = parse_non_empty_string(
                witness_obj.get("typedCoreProjectionDigest"),
                "witness.typedCoreProjectionDigest",
                &mut errors,
            );
            if witness_typed.as_deref() != Some(typed_core_projection_digest) {
                errors.push(
                    "typedCoreProjectionDigest mismatch between decision and witness".to_string(),
                );
            }
        }
        if let Some(authority_payload_digest) = authority_payload_digest.as_deref() {
            let witness_alias = parse_non_empty_string(
                witness_obj.get("authorityPayloadDigest"),
                "witness.authorityPayloadDigest",
                &mut errors,
            );
            if witness_alias.as_deref() != Some(authority_payload_digest) {
                errors.push(
                    "authorityPayloadDigest mismatch between decision and witness".to_string(),
                );
            }
        }
        if let Some(normalizer_id) = normalizer_id.as_deref() {
            let witness_normalizer = parse_non_empty_string(
                witness_obj.get("normalizerId"),
                "witness.normalizerId",
                &mut errors,
            );
            if witness_normalizer.as_deref() != Some(normalizer_id) {
                errors.push("normalizerId mismatch between decision and witness".to_string());
            }
        }
        if let Some(policy_digest) = policy_digest.as_deref() {
            let witness_policy = parse_non_empty_string(
                witness_obj.get("policyDigest"),
                "witness.policyDigest",
                &mut errors,
            );
            if witness_policy.as_deref() != Some(policy_digest) {
                errors.push("policyDigest mismatch between decision and witness".to_string());
            }
        }
        if let Some(projection_digest) = projection_digest.as_deref() {
            let witness_projection = parse_non_empty_string(
                witness_obj.get("projectionDigest"),
                "witness.projectionDigest",
                &mut errors,
            );
            if witness_projection.as_deref() != Some(projection_digest) {
                errors.push("projectionDigest mismatch between decision and witness".to_string());
            }
        }
        let witness_required_checks = parse_string_list(
            witness_obj.get("requiredChecks"),
            "witness.requiredChecks",
            &mut errors,
        );
        if let (Some(decision_required_checks), Some(witness_required_checks)) = (
            decision_required_checks.as_ref(),
            witness_required_checks.as_ref(),
        ) && decision_required_checks != witness_required_checks
        {
            errors.push("requiredChecks mismatch between decision and witness".to_string());
        }
    }

    if let Some(delta_snapshot) = request.delta_snapshot.as_ref()
        && let Some(delta_obj) = as_object(delta_snapshot, "deltaSnapshot", &mut errors)
    {
        if let Some(typed_core_projection_digest) = typed_core_projection_digest.as_deref() {
            let delta_typed = parse_non_empty_string(
                delta_obj.get("typedCoreProjectionDigest"),
                "deltaSnapshot.typedCoreProjectionDigest",
                &mut errors,
            );
            if delta_typed.as_deref() != Some(typed_core_projection_digest) {
                errors.push(
                    "typedCoreProjectionDigest mismatch between decision and delta snapshot"
                        .to_string(),
                );
            }
        }
        if let Some(authority_payload_digest) = authority_payload_digest.as_deref() {
            let delta_alias = parse_non_empty_string(
                delta_obj.get("authorityPayloadDigest"),
                "deltaSnapshot.authorityPayloadDigest",
                &mut errors,
            );
            if delta_alias.as_deref() != Some(authority_payload_digest) {
                errors.push(
                    "authorityPayloadDigest mismatch between decision and delta snapshot"
                        .to_string(),
                );
            }
        }
        if let Some(normalizer_id) = normalizer_id.as_deref() {
            let delta_normalizer = parse_non_empty_string(
                delta_obj.get("normalizerId"),
                "deltaSnapshot.normalizerId",
                &mut errors,
            );
            if delta_normalizer.as_deref() != Some(normalizer_id) {
                errors
                    .push("normalizerId mismatch between decision and delta snapshot".to_string());
            }
        }
        if let Some(policy_digest) = policy_digest.as_deref() {
            let delta_policy = parse_non_empty_string(
                delta_obj.get("policyDigest"),
                "deltaSnapshot.policyDigest",
                &mut errors,
            );
            if delta_policy.as_deref() != Some(policy_digest) {
                errors
                    .push("policyDigest mismatch between decision and delta snapshot".to_string());
            }
        }
        if let Some(projection_digest) = projection_digest.as_deref() {
            let delta_projection = parse_non_empty_string(
                delta_obj.get("projectionDigest"),
                "deltaSnapshot.projectionDigest",
                &mut errors,
            );
            if delta_projection.as_deref() != Some(projection_digest) {
                errors.push(
                    "projectionDigest mismatch between decision and delta snapshot".to_string(),
                );
            }
        }
        let delta_required_checks = parse_string_list(
            delta_obj.get("requiredChecks"),
            "deltaSnapshot.requiredChecks",
            &mut errors,
        );
        if let (Some(decision_required_checks), Some(delta_required_checks)) = (
            decision_required_checks.as_ref(),
            delta_required_checks.as_ref(),
        ) && decision_required_checks != delta_required_checks
        {
            errors.push("requiredChecks mismatch between decision and delta snapshot".to_string());
        }
    }

    RequiredDecisionVerifyResult { errors, derived }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn accepted_request() -> RequiredDecisionVerifyRequest {
        let typed = "ev1_demo";
        let alias = "proj1_demo";
        let normalizer = "normalizer.ci.required.v1";
        let policy = "ci-topos-v0";
        RequiredDecisionVerifyRequest {
            decision: json!({
                "decisionKind": "ci.required.decision.v1",
                "decision": "accept",
                "projectionDigest": "proj1_demo",
                "typedCoreProjectionDigest": typed,
                "authorityPayloadDigest": alias,
                "normalizerId": normalizer,
                "policyDigest": policy,
                "requiredChecks": ["baseline"],
                "witnessSha256": "witness_hash",
                "deltaSha256": "delta_hash"
            }),
            witness: Some(json!({
                "typedCoreProjectionDigest": typed,
                "authorityPayloadDigest": alias,
                "normalizerId": normalizer,
                "policyDigest": policy,
                "projectionDigest": "proj1_demo",
                "requiredChecks": ["baseline"]
            })),
            delta_snapshot: Some(json!({
                "typedCoreProjectionDigest": typed,
                "authorityPayloadDigest": alias,
                "normalizerId": normalizer,
                "policyDigest": policy,
                "projectionDigest": "proj1_demo",
                "requiredChecks": ["baseline"]
            })),
            actual_witness_sha256: Some("witness_hash".to_string()),
            actual_delta_sha256: Some("delta_hash".to_string()),
        }
    }

    #[test]
    fn verify_required_decision_accepts_valid_chain() {
        let request = accepted_request();
        let result = verify_required_decision_request(&request);
        assert!(result.errors.is_empty());
        assert_eq!(result.derived.decision.as_deref(), Some("accept"));
        assert_eq!(
            result.derived.projection_digest.as_deref(),
            Some("proj1_demo")
        );
        assert_eq!(
            result.derived.required_checks,
            Some(vec!["baseline".to_string()])
        );
    }

    #[test]
    fn verify_required_decision_rejects_projection_mismatch() {
        let mut request = accepted_request();
        request.witness = Some(json!({
            "typedCoreProjectionDigest": "ev1_demo",
            "authorityPayloadDigest": "proj1_demo",
            "normalizerId": "normalizer.ci.required.v1",
            "policyDigest": "ci-topos-v0",
            "projectionDigest": "proj1_wrong",
            "requiredChecks": ["baseline"]
        }));
        let result = verify_required_decision_request(&request);
        assert!(
            result
                .errors
                .iter()
                .any(|row| row.contains("projectionDigest mismatch between decision and witness"))
        );
    }
}
