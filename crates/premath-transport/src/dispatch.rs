use chrono::{DateTime, Utc};
use premath_coherence::{
    ExecutedInstructionCheck, InstructionError, InstructionWitness, InstructionWitnessRuntime,
    ValidatedInstructionEnvelope, build_instruction_witness, build_pre_execution_reject_witness,
    validate_instruction_envelope_payload,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::fiber::{fiber_cancel_response, fiber_join_response, fiber_spawn_response};
use crate::lease::{issue_claim, issue_claim_next, issue_lease_release, issue_lease_renew};
use crate::registry::validate_transport_action_binding_with_kernel;
use crate::types::*;
use crate::*;

pub(crate) fn truncate_for_payload(value: &str, max_chars: usize) -> String {
    let total = value.chars().count();
    if total <= max_chars {
        return value.to_string();
    }
    let clipped: String = value.chars().take(max_chars).collect();
    let remaining = total.saturating_sub(max_chars);
    format!("{clipped}...<truncated {remaining} chars>")
}

pub(crate) fn resolve_repo_root(raw_repo_root: Option<String>) -> Result<PathBuf, String> {
    let candidate = raw_repo_root.unwrap_or_else(|| ".".to_string());
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return Err("repoRoot must be non-empty when provided".to_string());
    }
    let root = PathBuf::from(trimmed);
    if root.is_absolute() {
        Ok(root)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(root))
            .map_err(|err| format!("failed to resolve repoRoot from current directory: {err}"))
    }
}

pub(crate) fn resolve_instruction_path(
    repo_root: &Path,
    raw_instruction_path: &str,
) -> Result<PathBuf, String> {
    let trimmed = raw_instruction_path.trim();
    if trimmed.is_empty() {
        return Err("instructionPath must be non-empty".to_string());
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(repo_root.join(path))
    }
}

pub(crate) fn instruction_ref(repo_root: &Path, instruction_path: &Path) -> String {
    instruction_path
        .strip_prefix(repo_root)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| instruction_path.display().to_string())
}

pub(crate) fn fallback_instruction_id_from_path(path: &Path) -> String {
    if let Some(stem) = path.file_stem().and_then(|value| value.to_str())
        && !stem.is_empty()
    {
        return stem.to_string();
    }
    if let Some(name) = path.file_name().and_then(|value| value.to_str())
        && !name.is_empty()
    {
        return name.to_string();
    }
    "instruction-invalid".to_string()
}

pub(crate) fn instruction_id_from_validated_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| fallback_instruction_id_from_path(path))
}

pub(crate) fn sort_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                if let Some(item) = map.get(key) {
                    sorted.insert(key.clone(), sort_json_value(item));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_json_value).collect()),
        _ => value.clone(),
    }
}

pub(crate) fn normalized_instruction_digest(
    instruction_bytes: &[u8],
    envelope: Option<&Value>,
) -> String {
    if let Some(value) = envelope {
        let canonical = serde_json::to_string(&sort_json_value(value))
            .expect("canonical envelope serialization should succeed");
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        return format!("instr1_{:x}", hasher.finalize());
    }
    let mut hasher = Sha256::new();
    hasher.update(instruction_bytes);
    format!("instr1_{:x}", hasher.finalize())
}

pub(crate) fn instruction_runtime_payload(
    instruction_id: String,
    instruction_ref: String,
    instruction_digest: String,
    squeak_site_profile: String,
    run_started_at: DateTime<Utc>,
    run_finished_at: DateTime<Utc>,
    results: Vec<ExecutedInstructionCheck>,
) -> InstructionWitnessRuntime {
    let run_duration_ms = (run_finished_at - run_started_at).num_milliseconds();
    InstructionWitnessRuntime {
        instruction_id,
        instruction_ref,
        instruction_digest,
        squeak_site_profile,
        run_started_at: run_started_at.to_rfc3339(),
        run_finished_at: run_finished_at.to_rfc3339(),
        run_duration_ms: run_duration_ms.max(0) as u64,
        results,
    }
}

pub(crate) fn write_instruction_witness(
    path: &Path,
    witness: &InstructionWitness,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create witness directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let rendered = serde_json::to_string_pretty(witness)
        .map_err(|err| format!("failed to render witness json: {err}"))?;
    fs::write(path, format!("{rendered}\n"))
        .map_err(|err| format!("failed to write witness file {}: {err}", path.display()))
}

pub(crate) fn run_gate_check(
    repo_root: &Path,
    check_id: &str,
) -> Result<(ExecutedInstructionCheck, String, String), String> {
    let started = Instant::now();
    let output = Command::new("sh")
        .arg(repo_root.join("tools/ci/run_gate.sh"))
        .arg(check_id)
        .current_dir(repo_root)
        .output()
        .map_err(|err| format!("failed to execute check `{check_id}`: {err}"))?;
    let duration_ms = started.elapsed().as_millis();
    let exit_code = output.status.code().unwrap_or(1);
    let result = ExecutedInstructionCheck {
        check_id: check_id.to_string(),
        status: if exit_code == 0 {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        exit_code: i64::from(exit_code),
        duration_ms: duration_ms.min(u128::from(u64::MAX)) as u64,
    };
    Ok((
        result,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

pub(crate) fn decision_state(checked: &ValidatedInstructionEnvelope) -> Option<String> {
    serde_json::to_value(&checked.execution_decision)
        .ok()
        .and_then(|value| {
            value
                .get("state")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
}

pub(crate) fn decision_source_reason(
    checked: &ValidatedInstructionEnvelope,
) -> (Option<String>, Option<String>) {
    let value = serde_json::to_value(&checked.execution_decision).ok();
    let source = value
        .as_ref()
        .and_then(|item| item.get("source"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let reason = value
        .as_ref()
        .and_then(|item| item.get("reason"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    (source, reason)
}

pub(crate) fn instruction_run_rejected(
    action_id: TransportActionId,
    failure_class: &str,
    diagnostic: impl Into<String>,
) -> Value {
    transport_rejected(
        transport_action_spec(action_id).action,
        action_id.as_str(),
        failure_class,
        diagnostic.into(),
    )
}

pub(crate) fn instruction_run_response(payload: Value) -> Value {
    let action_id = TransportActionId::InstructionRun;
    let parsed = match serde_json::from_value::<InstructionRunRequest>(payload) {
        Ok(value) => value,
        Err(source) => {
            return instruction_run_rejected(
                action_id,
                FAILURE_INSTRUCTION_INVALID_PAYLOAD,
                format!("invalid instruction.run payload: {source}"),
            );
        }
    };

    let repo_root = match resolve_repo_root(parsed.repo_root) {
        Ok(path) => path,
        Err(err) => {
            return instruction_run_rejected(action_id, FAILURE_INSTRUCTION_INVALID_PAYLOAD, err);
        }
    };
    let instruction_path = match resolve_instruction_path(&repo_root, &parsed.instruction_path) {
        Ok(path) => path,
        Err(err) => {
            return instruction_run_rejected(action_id, FAILURE_INSTRUCTION_INVALID_PAYLOAD, err);
        }
    };
    let allow_failure = parsed.allow_failure.unwrap_or(false);
    if !instruction_path.exists() {
        return instruction_run_rejected(
            action_id,
            FAILURE_INSTRUCTION_EXECUTION_IO,
            format!("instruction file not found: {}", instruction_path.display()),
        );
    }
    if !instruction_path.is_file() {
        return instruction_run_rejected(
            action_id,
            FAILURE_INSTRUCTION_EXECUTION_IO,
            format!(
                "instruction path is not a file: {}",
                instruction_path.display()
            ),
        );
    }

    let instruction_ref = instruction_ref(&repo_root, &instruction_path);
    let squeak_site_profile = std::env::var("PREMATH_SQUEAK_SITE_PROFILE")
        .ok()
        .or_else(|| std::env::var("PREMATH_EXECUTOR_PROFILE").ok())
        .unwrap_or_else(|| "local".to_string());
    let run_started_at = Utc::now();
    let mut stdout_parts: Vec<String> = Vec::new();
    let mut stderr_parts: Vec<String> = Vec::new();

    let instruction_bytes = match fs::read(&instruction_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return instruction_run_rejected(
                action_id,
                FAILURE_INSTRUCTION_EXECUTION_IO,
                format!(
                    "failed to read instruction file {}: {err}",
                    instruction_path.display()
                ),
            );
        }
    };
    let envelope_value = serde_json::from_slice::<Value>(&instruction_bytes).ok();
    let validated = if let Some(value) = envelope_value.as_ref() {
        validate_instruction_envelope_payload(value, &instruction_path, &repo_root)
    } else {
        Err(InstructionError {
            failure_class: "instruction_envelope_invalid_json".to_string(),
            message: format!(
                "failed to parse instruction json {}",
                instruction_path.display()
            ),
        })
    };

    let (witness, exit_code) = match validated {
        Ok(checked) => {
            let instruction_id = instruction_id_from_validated_path(&instruction_path);
            let instruction_digest = checked.instruction_digest.clone();
            let mut results: Vec<ExecutedInstructionCheck> = Vec::new();
            if matches!(decision_state(&checked).as_deref(), Some("execute")) {
                for check_id in &checked.requested_checks {
                    stdout_parts.push(format!("[instruction] running check: {check_id}"));
                    let (result, stdout, stderr) = match run_gate_check(&repo_root, check_id) {
                        Ok(value) => value,
                        Err(err) => {
                            return instruction_run_rejected(
                                action_id,
                                FAILURE_INSTRUCTION_EXECUTION_IO,
                                err,
                            );
                        }
                    };
                    if !stdout.trim().is_empty() {
                        stdout_parts.push(stdout.trim_end().to_string());
                    }
                    if !stderr.trim().is_empty() {
                        stderr_parts.push(stderr.trim_end().to_string());
                    }
                    results.push(result);
                }
            } else {
                let (source, reason) = decision_source_reason(&checked);
                stderr_parts.push(format!(
                    "[instruction] execution decision rejected before execution (source={}, reason={})",
                    source.unwrap_or_else(|| "unknown".to_string()),
                    reason.unwrap_or_else(|| "unknown".to_string())
                ));
            }

            let run_finished_at = Utc::now();
            let runtime = instruction_runtime_payload(
                instruction_id,
                instruction_ref.clone(),
                instruction_digest,
                squeak_site_profile.clone(),
                run_started_at,
                run_finished_at,
                results,
            );
            let witness = match build_instruction_witness(&checked, runtime) {
                Ok(value) => value,
                Err(err) => {
                    return instruction_run_rejected(
                        action_id,
                        FAILURE_INSTRUCTION_RUNTIME_INVALID,
                        err.to_string(),
                    );
                }
            };
            let exit_code = if witness.verdict_class == "rejected" && !allow_failure {
                1
            } else {
                0
            };
            (witness, exit_code)
        }
        Err(err) => {
            let run_finished_at = Utc::now();
            let instruction_id = fallback_instruction_id_from_path(&instruction_path);
            let instruction_digest =
                normalized_instruction_digest(&instruction_bytes, envelope_value.as_ref());
            let runtime = instruction_runtime_payload(
                instruction_id,
                instruction_ref.clone(),
                instruction_digest,
                squeak_site_profile.clone(),
                run_started_at,
                run_finished_at,
                Vec::new(),
            );
            let witness = match build_pre_execution_reject_witness(
                envelope_value.as_ref(),
                runtime,
                err.failure_class.as_str(),
                err.message.as_str(),
            ) {
                Ok(value) => value,
                Err(build_err) => {
                    return instruction_run_rejected(
                        action_id,
                        FAILURE_INSTRUCTION_RUNTIME_INVALID,
                        build_err.to_string(),
                    );
                }
            };
            stderr_parts.push(format!(
                "[error] invalid instruction envelope: {}: {}",
                err.failure_class, err.message
            ));
            (witness, 2)
        }
    };

    let witness_path = repo_root
        .join("artifacts")
        .join("ciwitness")
        .join(format!("{}.json", witness.instruction_id));
    if let Err(err) = write_instruction_witness(&witness_path, &witness) {
        return instruction_run_rejected(action_id, FAILURE_INSTRUCTION_EXECUTION_IO, err);
    }
    stdout_parts.push(format!(
        "[instruction] witness written: {}",
        witness_path.display()
    ));

    let ok = exit_code == 0;
    let failure_classes: Vec<String> = if ok {
        Vec::new()
    } else {
        witness.failure_classes.clone()
    };

    serde_json::json!({
        "schema": 1,
        "result": if ok { "accepted" } else { "rejected" },
        "failureClasses": failure_classes,
        "action": transport_action_spec(action_id).action,
        "repoRoot": repo_root.display().to_string(),
        "instructionPath": instruction_path.display().to_string(),
        "allowFailure": allow_failure,
        "ok": ok,
        "exitCode": exit_code,
        "witnessPath": witness_path.display().to_string(),
        "witnessExists": witness_path.exists(),
        "instructionId": witness.instruction_id,
        "verdictClass": witness.verdict_class,
        "requiredChecks": witness.required_checks,
        "executedChecks": witness.executed_checks,
        "stdout": truncate_for_payload(&stdout_parts.join("\n"), 16_000),
        "stderr": truncate_for_payload(&stderr_parts.join("\n"), 16_000),
    })
}

pub fn world_route_binding_json(action: &str) -> String {
    let envelope = match TransportActionId::from_action(action) {
        Some(action_id) => {
            let spec = transport_action_spec(action_id);
            match validate_transport_action_binding_with_kernel(spec) {
                Ok(()) => serde_json::json!({
                    "schema": 1,
                    "result": "accepted",
                    "action": spec.action,
                    "binding": world_binding_for_action(action_id),
                }),
                Err(err) => serde_json::json!({
                    "schema": 1,
                    "result": "rejected",
                    "failureClasses": [err.failure_class],
                    "action": spec.action,
                    "diagnostic": err.diagnostic,
                }),
            }
        }
        None => serde_json::json!({
            "schema": 1,
            "result": "rejected",
            "failureClasses": [FAILURE_LEASE_UNKNOWN_ACTION],
            "action": action,
        }),
    };
    serde_json::to_string(&envelope).expect("world binding envelope should serialize")
}

pub fn transport_dispatch_json(request_json: &str) -> String {
    let parsed = serde_json::from_str::<TransportDispatchRequest>(request_json);
    let response = match parsed {
        Ok(request) => dispatch_transport_request(request),
        Err(source) => transport_rejected(
            "transport.dispatch",
            ACTION_ID_TRANSPORT_INVALID_REQUEST,
            FAILURE_TRANSPORT_INVALID_REQUEST,
            format!("invalid transport request: {source}"),
        ),
    };
    serde_json::to_string(&response).expect("transport dispatch response should serialize")
}

#[allow(dead_code)] // used behind cfg(test) and cfg(feature = "rustler_nif")
pub(crate) fn nif_dispatch_json(request_json: &str) -> String {
    transport_dispatch_json(request_json)
}

pub(crate) fn dispatch_transport_request(request: TransportDispatchRequest) -> Value {
    let action = request.action.trim().to_string();
    let Some(action_id) = TransportActionId::from_action(&action) else {
        return transport_rejected(
            &action,
            ACTION_ID_TRANSPORT_UNKNOWN,
            FAILURE_TRANSPORT_UNKNOWN_ACTION,
            format!("unsupported transport action: {action}"),
        );
    };
    let spec = transport_action_spec(action_id);
    // Bootstrap (CHANGE-SITE §8): site change actions share the site_change route,
    // so kernel binding validation is deferred to the apply semantics.
    if !matches!(
        action_id,
        TransportActionId::SiteApplyChange
            | TransportActionId::SiteCurrentDigest
            | TransportActionId::SiteBuildChange
            | TransportActionId::SiteComposeChanges
    ) && let Err(err) = validate_transport_action_binding_with_kernel(spec)
    {
        return transport_rejected(
            &action,
            action_id.as_str(),
            &err.failure_class,
            err.diagnostic,
        );
    }

    let mut response = dispatch_transport_action(action_id, request.payload);
    annotate_transport_dispatch_fields(&mut response, &action, action_id);
    response
}

pub(crate) fn dispatch_transport_action(action_id: TransportActionId, payload: Value) -> Value {
    match action_id {
        TransportActionId::IssueClaim => match serde_json::from_value::<IssueClaimRequest>(payload)
        {
            Ok(parsed) => serde_json::to_value(issue_claim(parsed))
                .expect("issue claim envelope should serialize"),
            Err(source) => transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_LEASE_INVALID_PAYLOAD,
                format!("invalid claim payload: {source}"),
            ),
        },
        TransportActionId::IssueClaimNext => {
            match serde_json::from_value::<IssueClaimNextRequest>(payload) {
                Ok(parsed) => serde_json::to_value(issue_claim_next(parsed))
                    .expect("issue claim-next envelope should serialize"),
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid claim-next payload: {source}"),
                ),
            }
        }
        TransportActionId::IssueLeaseRenew => {
            match serde_json::from_value::<IssueLeaseRenewRequest>(payload) {
                Ok(parsed) => serde_json::to_value(issue_lease_renew(parsed))
                    .expect("issue lease renew envelope should serialize"),
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid renew payload: {source}"),
                ),
            }
        }
        TransportActionId::IssueLeaseRelease => {
            match serde_json::from_value::<IssueLeaseReleaseRequest>(payload) {
                Ok(parsed) => serde_json::to_value(issue_lease_release(parsed))
                    .expect("issue lease release envelope should serialize"),
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid release payload: {source}"),
                ),
            }
        }
        TransportActionId::WorldRouteBinding => {
            match serde_json::from_value::<TransportWorldBindingRequest>(payload) {
                Ok(parsed) => {
                    let payload = world_route_binding_json(&parsed.operation_action);
                    let mut value = serde_json::from_str::<Value>(&payload)
                        .expect("world route binding payload should parse");
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(
                            "action".to_string(),
                            Value::String(transport_action_spec(action_id).action.to_string()),
                        );
                        obj.insert(
                            "operationAction".to_string(),
                            Value::String(parsed.operation_action),
                        );
                    }
                    value
                }
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid world binding payload: {source}"),
                ),
            }
        }
        TransportActionId::FiberSpawn => fiber_spawn_response(payload),
        TransportActionId::FiberJoin => fiber_join_response(payload),
        TransportActionId::FiberCancel => fiber_cancel_response(payload),
        TransportActionId::InstructionRun => instruction_run_response(payload),
        TransportActionId::SiteApplyChange => site_apply_change_response(payload),
        TransportActionId::SiteCurrentDigest => site_current_digest_response(payload),
        TransportActionId::SiteBuildChange => site_build_change_response(payload),
        TransportActionId::SiteComposeChanges => site_compose_changes_response(payload),
    }
}

// ── site.current_digest ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteCurrentDigestPayload {
    #[serde(default)]
    repo_root: Option<String>,
}

pub(crate) fn site_current_digest_response(payload: Value) -> Value {
    let action_id = TransportActionId::SiteCurrentDigest;
    let parsed = match serde_json::from_value::<SiteCurrentDigestPayload>(payload) {
        Ok(v) => v,
        Err(source) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                format!("invalid site.current_digest payload: {source}"),
            );
        }
    };

    let repo_root = match resolve_repo_root(parsed.repo_root) {
        Ok(p) => p,
        Err(err) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                err,
            );
        }
    };

    let package_path = repo_root.join(SITE_PACKAGE_REL);
    let package_json = match fs::read_to_string(&package_path) {
        Ok(s) => s,
        Err(err) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                format!("failed to read {}: {}", package_path.display(), err),
            );
        }
    };

    let response_json = premath_kernel::current_site_digest_json(&package_json);
    let mut response: Value =
        serde_json::from_str(&response_json).expect("site digest response should parse");
    if let Some(obj) = response.as_object_mut() {
        obj.insert(
            "action".to_string(),
            Value::String("site.current_digest".to_string()),
        );
    }
    response
}

// ── site.build_change ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteBuildChangePayload {
    mutations: Value,
    #[serde(default)]
    preservation_claims: Option<Vec<String>>,
    #[serde(default)]
    repo_root: Option<String>,
}

pub(crate) fn site_build_change_response(payload: Value) -> Value {
    let action_id = TransportActionId::SiteBuildChange;
    let parsed = match serde_json::from_value::<SiteBuildChangePayload>(payload) {
        Ok(v) => v,
        Err(source) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                format!("invalid site.build_change payload: {source}"),
            );
        }
    };

    let repo_root = match resolve_repo_root(parsed.repo_root) {
        Ok(p) => p,
        Err(err) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                err,
            );
        }
    };

    let package_path = repo_root.join(SITE_PACKAGE_REL);
    let package_json = match fs::read_to_string(&package_path) {
        Ok(s) => s,
        Err(err) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                format!("failed to read {}: {}", package_path.display(), err),
            );
        }
    };

    let claims = parsed.preservation_claims.unwrap_or_default();
    let mutations_json = serde_json::json!({
        "mutations": parsed.mutations,
        "preservationClaims": claims,
    })
    .to_string();

    let response_json = premath_kernel::build_site_change_json(&package_json, &mutations_json);
    let mut response: Value =
        serde_json::from_str(&response_json).expect("site build response should parse");
    if let Some(obj) = response.as_object_mut() {
        obj.insert(
            "action".to_string(),
            Value::String("site.build_change".to_string()),
        );
    }
    response
}

// ── site.compose_changes ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteComposeChangesPayload {
    request1: Value,
    request2: Value,
}

pub(crate) fn site_compose_changes_response(payload: Value) -> Value {
    let action_id = TransportActionId::SiteComposeChanges;
    let parsed = match serde_json::from_value::<SiteComposeChangesPayload>(payload) {
        Ok(v) => v,
        Err(source) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                format!("invalid site.compose_changes payload: {source}"),
            );
        }
    };

    let r1_json = serde_json::to_string(&parsed.request1).unwrap_or_default();
    let r2_json = serde_json::to_string(&parsed.request2).unwrap_or_default();

    let response_json = premath_kernel::compose_site_changes_json(&r1_json, &r2_json);
    let mut response: Value =
        serde_json::from_str(&response_json).expect("site compose response should parse");
    if let Some(obj) = response.as_object_mut() {
        obj.insert(
            "action".to_string(),
            Value::String("site.compose_changes".to_string()),
        );
    }
    response
}

// ── site.apply_change ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteApplyChangePayload {
    change_request: Value,
    #[serde(default)]
    repo_root: Option<String>,
    #[serde(default)]
    dry_run: Option<bool>,
}

const SITE_PACKAGE_REL: &str =
    "specs/premath/site-packages/premath.doctrine_operation_site.v0/SITE-PACKAGE.json";

pub(crate) fn site_apply_change_response(payload: Value) -> Value {
    let action_id = TransportActionId::SiteApplyChange;
    let parsed = match serde_json::from_value::<SiteApplyChangePayload>(payload) {
        Ok(v) => v,
        Err(source) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                format!("invalid site.apply_change payload: {source}"),
            );
        }
    };

    let repo_root = match resolve_repo_root(parsed.repo_root) {
        Ok(p) => p,
        Err(err) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                err,
            );
        }
    };

    let package_path = repo_root.join(SITE_PACKAGE_REL);
    let package_json = match fs::read_to_string(&package_path) {
        Ok(s) => s,
        Err(err) => {
            return transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_SITE_CHANGE_INVALID_PAYLOAD,
                format!("failed to read {}: {}", package_path.display(), err),
            );
        }
    };

    let request_json = serde_json::to_string(&parsed.change_request).unwrap_or_default();
    let response_json = premath_kernel::apply_site_change_json(&package_json, &request_json);
    let mut response: Value =
        serde_json::from_str(&response_json).expect("site change response should parse");

    // Write mutated package back if accepted and not dry-run
    let dry_run = parsed.dry_run.unwrap_or(false);
    if !dry_run && let Some("accepted") = response.get("result").and_then(Value::as_str) {
        // Re-apply to get the package value
        let package: Value = serde_json::from_str(&package_json).unwrap();
        let request: premath_kernel::SiteChangeRequest =
            serde_json::from_str(&request_json).unwrap();
        let (_resp, pkg) = premath_kernel::apply_site_change(&package, &request);
        if let Some(mutated_pkg) = pkg {
            let pretty =
                serde_json::to_string_pretty(&mutated_pkg).expect("package should serialize");
            if let Err(err) = fs::write(&package_path, format!("{pretty}\n")) {
                return transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    "site_change_write_failed",
                    format!("failed to write {}: {}", package_path.display(), err),
                );
            }
        }

        // Append to digest-chain log (CHANGE-SITE §15)
        let change_log_path = repo_root.join(".premath/site-change-log.jsonl");
        if let (Some(change_id), Some(from_digest), Some(to_digest)) = (
            response.get("changeId").and_then(Value::as_str),
            response.get("fromDigest").and_then(Value::as_str),
            response.get("toDigest").and_then(Value::as_str),
        ) {
            let entry = serde_json::json!({
                "changeId": change_id,
                "fromDigest": from_digest,
                "toDigest": to_digest,
                "timestamp": Utc::now().to_rfc3339(),
            });
            if let Some(parent) = change_log_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let line = serde_json::to_string(&entry).unwrap_or_default();
            use std::io::Write;
            if let Ok(mut file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&change_log_path)
            {
                let _ = writeln!(file, "{line}");
            }
        }
    }

    if let Some(obj) = response.as_object_mut() {
        obj.insert(
            "action".to_string(),
            Value::String("site.apply_change".to_string()),
        );
        obj.insert("dryRun".to_string(), Value::Bool(dry_run));
    }
    response
}
