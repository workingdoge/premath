use premath_tusk::{ToolResultInput, TypestateEvidenceInput, normalize_typestate_evidence};
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const GOVERNANCE_PROFILE_CLAIM_ID: &str = "profile.doctrine_inf_governance.v0";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinCheckInput {
    evidence: TypestateEvidenceInput,
    #[serde(default)]
    governance_profile: Option<GovernanceProfileInput>,
    #[serde(default)]
    protocol_constraints: Option<ProtocolConstraintsInput>,
    #[serde(default)]
    handoff_constraints: Option<HandoffConstraintsInput>,
    #[serde(default)]
    truncation_constraints: Option<TruncationConstraintsInput>,
    #[serde(default)]
    decomposition_constraints: Option<DecompositionConstraintsInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GovernanceProfileInput {
    #[serde(default)]
    claim_id: Option<String>,
    claimed: bool,
    #[serde(default)]
    policy_provenance: Option<PolicyProvenanceInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PolicyProvenanceInput {
    pinned: bool,
    #[serde(default)]
    package_ref: Option<String>,
    #[serde(default)]
    expected_digest: Option<String>,
    #[serde(default)]
    bound_digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProtocolConstraintsInput {
    #[serde(default)]
    parallel_transport_order_valid: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandoffConstraintsInput {
    #[serde(default)]
    required_artifact_refs: Vec<String>,
    #[serde(default)]
    allowed_targets: Vec<String>,
    #[serde(default)]
    require_return_path: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TruncationConstraintsInput {
    #[serde(default)]
    required: bool,
    #[serde(default)]
    valid: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecompositionConstraintsInput {
    #[serde(default)]
    admissible: Option<bool>,
    #[serde(default)]
    expected_execution_pattern: Option<String>,
    #[serde(default)]
    expected_policy_digest: Option<String>,
}

fn emit_error(message: impl Into<String>) -> ! {
    eprintln!("{}", message.into());
    std::process::exit(2);
}

pub fn run(input: String, json_output: bool) {
    let input_path = PathBuf::from(&input);
    let bytes = fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(format!(
            "harness_join_check_invalid: failed to read input {}: {err}",
            input_path.display()
        ))
    });
    let request: JoinCheckInput = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(format!(
            "harness_join_check_invalid: failed to parse input json {}: {err}",
            input_path.display()
        ))
    });

    let evidence = request.evidence.clone();
    let normalized = normalize_typestate_evidence(request.evidence).unwrap_or_else(|err| {
        emit_error(format!("harness_join_check_invalid: {err}"));
    });

    let join_input = normalized.join_closed_input();
    let mutation_input = normalized.mutation_ready_input();

    let mut failure_classes: BTreeSet<String> = BTreeSet::new();
    if !join_input.missing_result_call_ids.is_empty() {
        failure_classes.insert("tool.result_missing".to_string());
    }
    if !join_input.orphan_result_call_ids.is_empty() {
        failure_classes.insert("tool.result_orphan".to_string());
    }
    if !join_input.missing_use_call_ids.is_empty() {
        failure_classes.insert("tool.use_missing".to_string());
    }
    if !join_input.unknown_use_call_ids.is_empty() {
        failure_classes.insert("tool.use_without_result".to_string());
    }
    if !join_input.join_closed {
        failure_classes.insert("tool.join_incomplete".to_string());
    }

    if !is_known_stop_reason(&normalized.protocol_state.stop_reason) {
        failure_classes.insert("protocol.stop_reason_unhandled".to_string());
    }

    failure_classes.extend(evaluate_protocol_constraints(
        request.protocol_constraints.as_ref(),
    ));
    failure_classes.extend(evaluate_decomposition_constraints(
        request.decomposition_constraints.as_ref(),
        &normalized,
    ));
    failure_classes.extend(evaluate_context_constraints(&normalized));
    failure_classes.extend(evaluate_handoff_constraints(
        request.handoff_constraints.as_ref(),
        normalized.handoff.as_ref(),
    ));
    failure_classes.extend(evaluate_truncation_constraints(
        request.truncation_constraints.as_ref(),
    ));
    failure_classes.extend(evaluate_error_envelope_constraints(&evidence));
    failure_classes.extend(evaluate_governance_profile(
        request.governance_profile.as_ref(),
    ));

    let failure_classes = failure_classes.into_iter().collect::<Vec<_>>();
    let result = if failure_classes.is_empty() {
        "accepted"
    } else {
        "rejected"
    };

    if json_output {
        let payload = json!({
            "action": "harness-join-check",
            "result": result,
            "failureClasses": failure_classes,
            "joinClosed": result == "accepted",
            "witness": {
                "schema": 1,
                "witnessKind": "premath.harness.join_check.v1",
                "digests": normalized.digests,
                "joinInput": join_input,
                "mutationInput": mutation_input
            },
            "normalized": normalized
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            emit_error(format!(
                "harness_join_check_invalid: failed to render output json: {err}"
            ))
        });
        println!("{rendered}");
        return;
    }

    println!("premath harness-join-check");
    println!("  Input: {}", input_path.display());
    println!("  Result: {result}");
    println!("  Failure Classes: {}", failure_classes.len());
    for class in &failure_classes {
        println!("  - {class}");
    }
}

fn is_known_stop_reason(value: &str) -> bool {
    matches!(value, "tool_use" | "pause_turn" | "max_tokens" | "end_turn")
}

fn evaluate_protocol_constraints(
    constraints: Option<&ProtocolConstraintsInput>,
) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    let Some(constraints) = constraints else {
        return failures;
    };

    if matches!(constraints.parallel_transport_order_valid, Some(false)) {
        failures.insert("protocol.parallel_transport_order_invalid".to_string());
    }
    failures
}

fn evaluate_handoff_constraints(
    constraints: Option<&HandoffConstraintsInput>,
    handoff: Option<&premath_tusk::NormalizedHandoffObservation>,
) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    let Some(constraints) = constraints else {
        return failures;
    };
    let required_refs = constraints
        .required_artifact_refs
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<BTreeSet<_>>();
    let allowed_targets = constraints
        .allowed_targets
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<BTreeSet<_>>();

    match handoff {
        None => {
            if !required_refs.is_empty() {
                failures.insert("handoff.required_artifact_missing".to_string());
            }
            if constraints.require_return_path {
                failures.insert("handoff.return_path_missing".to_string());
            }
            if !allowed_targets.is_empty() {
                failures.insert("handoff.target_not_allowed".to_string());
            }
        }
        Some(handoff) => {
            if !required_refs.is_empty() {
                let provided = handoff
                    .required_artifact_refs
                    .iter()
                    .map(|item| item.trim())
                    .filter(|item| !item.is_empty())
                    .collect::<BTreeSet<_>>();
                if !required_refs.is_subset(&provided) {
                    failures.insert("handoff.required_artifact_missing".to_string());
                }
            }
            if !allowed_targets.is_empty() && !allowed_targets.contains(handoff.target.trim()) {
                failures.insert("handoff.target_not_allowed".to_string());
            }
            if constraints.require_return_path
                && handoff
                    .return_path_ref
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
            {
                failures.insert("handoff.return_path_missing".to_string());
            }
        }
    }

    failures
}

fn evaluate_truncation_constraints(
    constraints: Option<&TruncationConstraintsInput>,
) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    let Some(constraints) = constraints else {
        return failures;
    };
    if constraints.required && !constraints.valid {
        failures.insert("tool.response_truncation_policy_violation".to_string());
    }
    failures
}

fn evaluate_decomposition_constraints(
    constraints: Option<&DecompositionConstraintsInput>,
    normalized: &premath_tusk::NormalizedTypestateEvidence,
) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    let Some(constraints) = constraints else {
        return failures;
    };

    if matches!(constraints.admissible, Some(false)) {
        failures.insert("coordination.decomposition_policy_violation".to_string());
        return failures;
    }

    if let Some(expected_pattern) = constraints
        .expected_execution_pattern
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        && normalized.call_spec.execution_pattern != expected_pattern
    {
        failures.insert("coordination.decomposition_policy_violation".to_string());
    }
    if let Some(expected_digest) = constraints
        .expected_policy_digest
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        && normalized.call_spec.decomposition_policy_digest != expected_digest
    {
        failures.insert("coordination.decomposition_policy_violation".to_string());
    }
    failures
}

fn evaluate_context_constraints(
    normalized: &premath_tusk::NormalizedTypestateEvidence,
) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    let requires_context_projection = normalized.protocol_state.continuation_allowed
        || normalized
            .tool_use
            .iter()
            .any(|item| item.disposition == "consumed");
    if !requires_context_projection {
        return failures;
    }

    let context = &normalized.context_state;
    if !context.missing_render_call_ids.is_empty() || !context.state_view_present {
        failures.insert("context.injection_point_missing".to_string());
    }

    if !context.queue_reduction_present
        || !context.render_policy_valid
        || !context.queue_policy_valid
        || !context.state_view_policy_valid
    {
        failures.insert("context.queue_policy_violation".to_string());
    }
    failures
}

fn evaluate_error_envelope_constraints(evidence: &TypestateEvidenceInput) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    for result in &evidence.tool_results {
        if !result_status_is_error(result) {
            continue;
        }
        if !has_machine_readable_error_envelope(result) {
            failures.insert("tool.schema_invalid".to_string());
        }
    }
    failures
}

fn result_status_is_error(result: &ToolResultInput) -> bool {
    let status = result
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    if matches!(status.as_deref(), Some("error")) {
        return true;
    }
    if matches!(status.as_deref(), Some("ok")) {
        return false;
    }

    result
        .error_code
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        || result.retryable.is_some()
        || result
            .error_message
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        || result.raw_error.is_some()
}

fn has_machine_readable_error_envelope(result: &ToolResultInput) -> bool {
    let code_present = result
        .error_code
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        || raw_error_string(result.raw_error.as_ref(), &["errorCode", "code"]).is_some();
    let retryable_present = result.retryable.is_some()
        || raw_error_bool(result.raw_error.as_ref(), "retryable").is_some();
    let message_present = result
        .error_message
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        || raw_error_string(result.raw_error.as_ref(), &["errorMessage", "message"]).is_some();

    code_present && retryable_present && message_present
}

fn raw_error_string(raw_error: Option<&Value>, keys: &[&str]) -> Option<String> {
    let Value::Object(map) = raw_error? else {
        return None;
    };
    for key in keys {
        let Some(value) = map.get(*key).and_then(Value::as_str) else {
            continue;
        };
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn raw_error_bool(raw_error: Option<&Value>, key: &str) -> Option<bool> {
    let Value::Object(map) = raw_error? else {
        return None;
    };
    map.get(key).and_then(Value::as_bool)
}

fn evaluate_governance_profile(profile: Option<&GovernanceProfileInput>) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    let Some(profile) = profile else {
        return failures;
    };

    if let Some(claim_id) = profile
        .claim_id
        .as_ref()
        .map(|raw| raw.trim())
        .filter(|raw| !raw.is_empty())
        && claim_id != GOVERNANCE_PROFILE_CLAIM_ID
    {
        failures.insert("governance.claim_id_invalid".to_string());
        return failures;
    }

    if !profile.claimed {
        return failures;
    }

    let Some(provenance) = profile.policy_provenance.as_ref() else {
        failures.insert("governance.policy_package_unpinned".to_string());
        return failures;
    };

    let package_ref = provenance.package_ref.as_deref().unwrap_or("").trim();
    let expected_digest = provenance.expected_digest.as_deref().unwrap_or("").trim();
    let bound_digest = provenance.bound_digest.as_deref().unwrap_or("").trim();

    if !provenance.pinned
        || package_ref.is_empty()
        || expected_digest.is_empty()
        || bound_digest.is_empty()
    {
        failures.insert("governance.policy_package_unpinned".to_string());
        return failures;
    }

    if expected_digest != bound_digest {
        failures.insert("governance.policy_package_mismatch".to_string());
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn governance_profile_is_claim_gated() {
        let profile = GovernanceProfileInput {
            claim_id: Some(GOVERNANCE_PROFILE_CLAIM_ID.to_string()),
            claimed: false,
            policy_provenance: None,
        };
        let failures = evaluate_governance_profile(Some(&profile));
        assert!(failures.is_empty());
    }

    #[test]
    fn governance_profile_detects_mismatch() {
        let profile = GovernanceProfileInput {
            claim_id: Some(GOVERNANCE_PROFILE_CLAIM_ID.to_string()),
            claimed: true,
            policy_provenance: Some(PolicyProvenanceInput {
                pinned: true,
                package_ref: Some("policy/governance/v1".to_string()),
                expected_digest: Some("sha256:a".to_string()),
                bound_digest: Some("sha256:b".to_string()),
            }),
        };
        let failures = evaluate_governance_profile(Some(&profile));
        assert_eq!(
            failures.into_iter().collect::<Vec<_>>(),
            vec!["governance.policy_package_mismatch".to_string()]
        );
    }

    #[test]
    fn handoff_constraints_detect_missing_required_artifact() {
        let constraints = HandoffConstraintsInput {
            required_artifact_refs: vec!["artifact://required".to_string()],
            allowed_targets: vec![],
            require_return_path: false,
        };
        let handoff = premath_tusk::NormalizedHandoffObservation {
            target: "worker.frontend".to_string(),
            required_artifact_refs: vec!["artifact://other".to_string()],
            return_path_ref: Some("handoff://return/main".to_string()),
        };
        let failures = evaluate_handoff_constraints(Some(&constraints), Some(&handoff));
        assert_eq!(
            failures.into_iter().collect::<Vec<_>>(),
            vec!["handoff.required_artifact_missing".to_string()]
        );
    }

    #[test]
    fn handoff_constraints_detect_missing_return_path() {
        let constraints = HandoffConstraintsInput {
            required_artifact_refs: vec![],
            allowed_targets: vec![],
            require_return_path: true,
        };
        let handoff = premath_tusk::NormalizedHandoffObservation {
            target: "worker.frontend".to_string(),
            required_artifact_refs: vec![],
            return_path_ref: None,
        };
        let failures = evaluate_handoff_constraints(Some(&constraints), Some(&handoff));
        assert_eq!(
            failures.into_iter().collect::<Vec<_>>(),
            vec!["handoff.return_path_missing".to_string()]
        );
    }

    #[test]
    fn error_envelope_constraints_detect_untyped_error() {
        let evidence = TypestateEvidenceInput {
            call_spec: premath_tusk::CallSpecInput {
                call_id: "call-1".to_string(),
                model_ref: "gpt-5".to_string(),
                action_mode: "code".to_string(),
                execution_pattern: "single".to_string(),
                normalizer_id: "nf.v1".to_string(),
                mutation_policy_digest: "mut.pol.v1".to_string(),
                governance_policy_digest: "gov.pol.v1".to_string(),
                tool_render_protocol_digest: "render.pol.v1".to_string(),
                reminder_queue_policy_digest: "queue.pol.v1".to_string(),
                state_view_policy_digest: "state.pol.v1".to_string(),
                decomposition_policy_digest: "decomp.pol.v1".to_string(),
            },
            tool_requests: vec![],
            tool_results: vec![ToolResultInput {
                tool_call_id: "tc-1".to_string(),
                status: Some("error".to_string()),
                result_digest: None,
                payload: None,
                error_code: None,
                retryable: None,
                error_message: None,
                raw_error: Some(json!({"unexpected": "shape"})),
            }],
            tool_use: vec![],
            tool_render: vec![],
            reminder_queue: vec![],
            state_views: vec![],
            protocol_state: premath_tusk::ProtocolStateInput {
                stop_reason: "tool_use".to_string(),
                continuation_allowed: true,
            },
            handoff: None,
            session_refs: vec![],
            trajectory_refs: vec![],
        };

        let failures = evaluate_error_envelope_constraints(&evidence);
        assert_eq!(
            failures.into_iter().collect::<Vec<_>>(),
            vec!["tool.schema_invalid".to_string()]
        );
    }

    fn context_fixture() -> premath_tusk::NormalizedTypestateEvidence {
        let evidence = TypestateEvidenceInput {
            call_spec: premath_tusk::CallSpecInput {
                call_id: "call-1".to_string(),
                model_ref: "gpt-5".to_string(),
                action_mode: "code".to_string(),
                execution_pattern: "single".to_string(),
                normalizer_id: "nf.v1".to_string(),
                mutation_policy_digest: "mut.pol.v1".to_string(),
                governance_policy_digest: "gov.pol.v1".to_string(),
                tool_render_protocol_digest: "render.pol.v1".to_string(),
                reminder_queue_policy_digest: "queue.pol.v1".to_string(),
                state_view_policy_digest: "state.pol.v1".to_string(),
                decomposition_policy_digest: "decomp.pol.v1".to_string(),
            },
            tool_requests: vec![premath_tusk::ToolRequestInput {
                tool_call_id: "tc-1".to_string(),
                tool_name: "fs.read".to_string(),
                schema_digest: None,
                caller_id: None,
                search_ref_digest: None,
            }],
            tool_results: vec![ToolResultInput {
                tool_call_id: "tc-1".to_string(),
                status: Some("ok".to_string()),
                result_digest: Some("sha256:result-a".to_string()),
                payload: None,
                error_code: None,
                retryable: None,
                error_message: None,
                raw_error: None,
            }],
            tool_use: vec![premath_tusk::ToolUseInput {
                tool_call_id: "tc-1".to_string(),
                disposition: "consumed".to_string(),
                result_digest: Some("sha256:result-a".to_string()),
            }],
            tool_render: vec![premath_tusk::ToolRenderObservationInput {
                tool_call_id: "tc-1".to_string(),
                operator_payload_digest: "sha256:operator-a".to_string(),
                reminder_render_digest: "sha256:reminder-a".to_string(),
                injection_point: "tool_response".to_string(),
                policy_digest: "render.pol.v1".to_string(),
            }],
            reminder_queue: vec![premath_tusk::ReminderQueueReductionInput {
                queue_id: "queue/default".to_string(),
                reduced_digest: "sha256:queue-a".to_string(),
                policy_digest: "queue.pol.v1".to_string(),
            }],
            state_views: vec![premath_tusk::StateViewObservationInput {
                view_id: "state/latest".to_string(),
                view_digest: "sha256:view-a".to_string(),
                policy_digest: "state.pol.v1".to_string(),
                source_refs: vec!["handoff://return/main".to_string()],
            }],
            protocol_state: premath_tusk::ProtocolStateInput {
                stop_reason: "tool_use".to_string(),
                continuation_allowed: true,
            },
            handoff: None,
            session_refs: vec![],
            trajectory_refs: vec![],
        };
        premath_tusk::normalize_typestate_evidence(evidence)
            .expect("context fixture should normalize")
    }

    #[test]
    fn context_constraints_accept_with_required_rows() {
        let normalized = context_fixture();
        let failures = evaluate_context_constraints(&normalized);
        assert!(failures.is_empty());
    }

    #[test]
    fn decomposition_constraints_reject_non_admissible_dispatch() {
        let normalized = context_fixture();
        let constraints = DecompositionConstraintsInput {
            admissible: Some(false),
            expected_execution_pattern: None,
            expected_policy_digest: None,
        };
        let failures = evaluate_decomposition_constraints(Some(&constraints), &normalized);
        assert_eq!(
            failures.into_iter().collect::<Vec<_>>(),
            vec!["coordination.decomposition_policy_violation".to_string()]
        );
    }

    #[test]
    fn context_constraints_reject_missing_render_row() {
        let mut evidence = context_fixture();
        evidence.tool_render.clear();
        evidence.context_state.missing_render_call_ids = vec!["tc-1".to_string()];
        evidence.context_state.continuation_context_ready = false;

        let failures = evaluate_context_constraints(&evidence);
        assert_eq!(
            failures.into_iter().collect::<Vec<_>>(),
            vec!["context.injection_point_missing".to_string()]
        );
    }

    #[test]
    fn context_constraints_reject_policy_mismatch() {
        let mut evidence = context_fixture();
        evidence.context_state.queue_policy_valid = false;
        evidence.context_state.continuation_context_ready = false;

        let failures = evaluate_context_constraints(&evidence);
        assert_eq!(
            failures.into_iter().collect::<Vec<_>>(),
            vec!["context.queue_policy_violation".to_string()]
        );
    }
}
