use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const OBSERVATION_SCHEMA: u64 = 1;
pub const OBSERVATION_KIND: &str = "ci.observation.surface.v0";
pub const REQUIRED_WITNESS_KIND: &str = "ci.required.v1";
pub const REQUIRED_DECISION_KIND: &str = "ci.required.decision.v1";
pub const INSTRUCTION_WITNESS_KIND: &str = "ci.instruction.v1";
pub const REQUIRED_EVENT_KIND: &str = "ci.required.v1.summary";
pub const REQUIRED_DECISION_EVENT_KIND: &str = "ci.required.decision.v1.summary";
pub const INSTRUCTION_EVENT_KIND: &str = "ci.instruction.v1.summary";
const BLOCKING_DEP_TYPES: &[&str] = &["blocks", "parent-child", "conditional-blocks", "waits-for"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSummary {
    pub state: String,
    pub needs_attention: bool,
    pub top_failure_class: Option<String>,
    pub latest_projection_digest: Option<String>,
    pub latest_instruction_id: Option<String>,
    pub required_check_count: u64,
    pub executed_check_count: u64,
    pub changed_path_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coherence: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeltaSummary {
    pub r#ref: String,
    pub projection_policy: Option<String>,
    pub projection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_core_projection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_payload_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalizer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_digest: Option<String>,
    pub delta_source: Option<String>,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub changed_paths: Vec<String>,
    pub changed_path_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredSummary {
    pub r#ref: String,
    pub witness_kind: Option<String>,
    pub projection_policy: Option<String>,
    pub projection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_core_projection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_payload_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalizer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_digest: Option<String>,
    pub verdict_class: Option<String>,
    pub required_checks: Vec<String>,
    pub executed_checks: Vec<String>,
    pub failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionSummary {
    pub r#ref: String,
    pub decision_kind: Option<String>,
    pub projection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_core_projection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_payload_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalizer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_digest: Option<String>,
    pub decision: Option<String>,
    pub reason_class: Option<String>,
    pub witness_path: Option<String>,
    pub delta_snapshot_path: Option<String>,
    pub required_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionSummary {
    pub r#ref: String,
    pub witness_kind: Option<String>,
    pub instruction_id: String,
    pub instruction_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_core_projection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_payload_digest: Option<String>,
    pub instruction_classification: Option<serde_json::Value>,
    pub intent: Option<String>,
    pub scope: Option<serde_json::Value>,
    pub policy_digest: Option<String>,
    pub verdict_class: Option<String>,
    pub required_checks: Vec<String>,
    pub executed_checks: Vec<String>,
    pub failure_classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_ingest: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LatestObservation {
    pub delta: Option<DeltaSummary>,
    pub required: Option<RequiredSummary>,
    pub decision: Option<DecisionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSurface {
    pub schema: u64,
    pub surface_kind: String,
    pub summary: ObservationSummary,
    pub latest: LatestObservation,
    pub instructions: Vec<InstructionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionView {
    pub projection_digest: String,
    pub required: Option<RequiredSummary>,
    pub delta: Option<DeltaSummary>,
    pub decision: Option<DecisionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEvent {
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("failed to read observation surface: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse observation surface JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid observation surface: {0}")]
    Invalid(String),
}

fn load_json_object(
    path: &Path,
) -> Result<Option<serde_json::Map<String, Value>>, ObservationError> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    let value = serde_json::from_slice::<Value>(&bytes)?;
    let Value::Object(map) = value else {
        return Err(ObservationError::Invalid(format!(
            "expected object JSON: {}",
            path.display()
        )));
    };
    Ok(Some(map))
}

fn string_opt(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    let mut out = BTreeSet::new();
    if let Some(Value::Array(items)) = value {
        for item in items {
            if let Some(text) = item.as_str() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    out.insert(trimmed.to_string());
                }
            }
        }
    }
    out.into_iter().collect()
}

fn canonical_dep_type(value: Option<&Value>) -> Option<String> {
    string_opt(value).map(|text| text.to_lowercase().replace('_', "-"))
}

fn is_blocking_dep(dep: &serde_json::Map<String, Value>) -> bool {
    let dep_type =
        canonical_dep_type(dep.get("type")).or_else(|| canonical_dep_type(dep.get("dep_type")));
    dep_type
        .as_deref()
        .map(|dep_type| BLOCKING_DEP_TYPES.contains(&dep_type))
        .unwrap_or(false)
}

fn issue_depends_on_id(dep: &serde_json::Map<String, Value>) -> Option<String> {
    string_opt(dep.get("depends_on_id")).or_else(|| string_opt(dep.get("dependsOnId")))
}

fn lease_expires_at(lease: &serde_json::Map<String, Value>) -> Option<DateTime<Utc>> {
    parse_rfc3339(string_opt(lease.get("expires_at")).as_deref())
        .or_else(|| parse_rfc3339(string_opt(lease.get("expiresAt")).as_deref()))
}

fn parse_rfc3339(value: Option<&str>) -> Option<DateTime<Utc>> {
    let text = value?.trim();
    if text.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn round_f64(value: f64, places: i32) -> f64 {
    let factor = 10f64.powi(places);
    (value * factor).round() / factor
}

fn relative_path(path: &Path, root: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

fn load_issue_rows(path: &Path) -> Result<Vec<serde_json::Map<String, Value>>, ObservationError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    if !path.is_file() {
        return Err(ObservationError::Invalid(format!(
            "issues path is not a file: {}",
            path.display()
        )));
    }

    let contents = fs::read_to_string(path)?;
    let mut by_id: BTreeMap<String, serde_json::Map<String, Value>> = BTreeMap::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let value = serde_json::from_str::<Value>(line).map_err(|err| {
            ObservationError::Invalid(format!(
                "invalid jsonl at {}:{}: {err}",
                path.display(),
                line_index + 1
            ))
        })?;
        let Value::Object(map) = value else {
            return Err(ObservationError::Invalid(format!(
                "jsonl row must be object at {}:{}",
                path.display(),
                line_index + 1
            )));
        };
        let Some(issue_id) = string_opt(map.get("id")) else {
            return Err(ObservationError::Invalid(format!(
                "missing issue id at {}:{}",
                path.display(),
                line_index + 1
            )));
        };
        by_id.insert(issue_id, map);
    }
    Ok(by_id.into_values().collect())
}

fn normalize_delta(payload: &serde_json::Map<String, Value>, rel_path: String) -> DeltaSummary {
    let changed_paths = string_list(payload.get("changedPaths"));
    DeltaSummary {
        r#ref: rel_path,
        projection_policy: string_opt(payload.get("projectionPolicy")),
        projection_digest: string_opt(payload.get("projectionDigest")),
        typed_core_projection_digest: string_opt(payload.get("typedCoreProjectionDigest")),
        authority_payload_digest: string_opt(payload.get("authorityPayloadDigest")),
        normalizer_id: string_opt(payload.get("normalizerId")),
        policy_digest: string_opt(payload.get("policyDigest")),
        delta_source: string_opt(payload.get("deltaSource")),
        from_ref: string_opt(payload.get("fromRef")),
        to_ref: string_opt(payload.get("toRef")),
        changed_path_count: changed_paths.len() as u64,
        changed_paths,
    }
}

fn normalize_required(
    payload: &serde_json::Map<String, Value>,
    rel_path: String,
) -> RequiredSummary {
    RequiredSummary {
        r#ref: rel_path,
        witness_kind: string_opt(payload.get("witnessKind")),
        projection_policy: string_opt(payload.get("projectionPolicy")),
        projection_digest: string_opt(payload.get("projectionDigest")),
        typed_core_projection_digest: string_opt(payload.get("typedCoreProjectionDigest")),
        authority_payload_digest: string_opt(payload.get("authorityPayloadDigest")),
        normalizer_id: string_opt(payload.get("normalizerId")),
        policy_digest: string_opt(payload.get("policyDigest")),
        verdict_class: string_opt(payload.get("verdictClass")),
        required_checks: string_list(payload.get("requiredChecks")),
        executed_checks: string_list(payload.get("executedChecks")),
        failure_classes: string_list(payload.get("failureClasses")),
    }
}

fn normalize_decision(
    payload: &serde_json::Map<String, Value>,
    rel_path: String,
) -> DecisionSummary {
    DecisionSummary {
        r#ref: rel_path,
        decision_kind: string_opt(payload.get("decisionKind")),
        projection_digest: string_opt(payload.get("projectionDigest")),
        typed_core_projection_digest: string_opt(payload.get("typedCoreProjectionDigest")),
        authority_payload_digest: string_opt(payload.get("authorityPayloadDigest")),
        normalizer_id: string_opt(payload.get("normalizerId")),
        policy_digest: string_opt(payload.get("policyDigest")),
        decision: string_opt(payload.get("decision")),
        reason_class: string_opt(payload.get("reasonClass")),
        witness_path: string_opt(payload.get("witnessPath")),
        delta_snapshot_path: string_opt(payload.get("deltaSnapshotPath")),
        required_checks: string_list(payload.get("requiredChecks")),
    }
}

fn normalize_instruction(
    payload: &serde_json::Map<String, Value>,
    rel_path: String,
) -> InstructionSummary {
    let fallback_instruction_id = Path::new(&rel_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| rel_path.clone());
    let instruction_id =
        string_opt(payload.get("instructionId")).unwrap_or(fallback_instruction_id);
    let proposal_ingest = payload
        .get("proposalIngest")
        .filter(|value| value.is_object())
        .cloned();
    InstructionSummary {
        r#ref: rel_path,
        witness_kind: string_opt(payload.get("witnessKind")),
        instruction_id,
        instruction_digest: string_opt(payload.get("instructionDigest")),
        typed_core_projection_digest: string_opt(payload.get("typedCoreProjectionDigest")),
        authority_payload_digest: string_opt(payload.get("authorityPayloadDigest")),
        instruction_classification: payload.get("instructionClassification").cloned(),
        intent: string_opt(payload.get("intent")),
        scope: payload.get("scope").cloned(),
        policy_digest: string_opt(payload.get("policyDigest")),
        verdict_class: string_opt(payload.get("verdictClass")),
        required_checks: string_list(payload.get("requiredChecks")),
        executed_checks: string_list(payload.get("executedChecks")),
        failure_classes: string_list(payload.get("failureClasses")),
        run_started_at: string_opt(payload.get("runStartedAt")),
        run_finished_at: string_opt(payload.get("runFinishedAt")),
        proposal_ingest,
    }
}

struct DerivedState {
    state: String,
    needs_attention: bool,
    top_failure_class: Option<String>,
}

fn derive_state(
    required: Option<&RequiredSummary>,
    decision: Option<&DecisionSummary>,
    instructions: &[InstructionSummary],
) -> DerivedState {
    let mut top_failure: Option<String> = None;
    let state = if let Some(decision) = decision {
        top_failure = decision.reason_class.clone();
        match decision.decision.as_deref() {
            Some("accept") => "accepted".to_string(),
            Some("reject") => "rejected".to_string(),
            _ => "error".to_string(),
        }
    } else if let Some(required) = required {
        top_failure = required.failure_classes.first().cloned();
        match required.verdict_class.as_deref() {
            Some("accepted") => "running".to_string(),
            Some("rejected") => "rejected".to_string(),
            _ => "error".to_string(),
        }
    } else if let Some(latest) = instructions.last() {
        top_failure = latest.failure_classes.first().cloned();
        match latest.verdict_class.as_deref() {
            Some("accepted") => "running".to_string(),
            Some("rejected") => "rejected".to_string(),
            _ => "error".to_string(),
        }
    } else {
        "empty".to_string()
    };

    if top_failure.is_none() && state == "rejected" {
        top_failure = Some("rejected_without_reason".to_string());
    }

    let needs_attention = matches!(state.as_str(), "rejected" | "error");
    DerivedState {
        state,
        needs_attention,
        top_failure_class: top_failure,
    }
}

fn coherence_policy_drift(
    delta: Option<&DeltaSummary>,
    required: Option<&RequiredSummary>,
    decision: Option<&DecisionSummary>,
    instructions: &[InstructionSummary],
) -> Value {
    let mut projection_policies = BTreeSet::new();
    if let Some(policy) = delta.and_then(|row| row.projection_policy.clone()) {
        projection_policies.insert(policy);
    }
    if let Some(policy) = required.and_then(|row| row.projection_policy.clone()) {
        projection_policies.insert(policy);
    }

    let mut typed_projection_digests = BTreeSet::new();
    if let Some(digest) = delta.and_then(|row| row.typed_core_projection_digest.clone()) {
        typed_projection_digests.insert(digest);
    }
    if let Some(digest) = required.and_then(|row| row.typed_core_projection_digest.clone()) {
        typed_projection_digests.insert(digest);
    }
    if let Some(digest) = decision.and_then(|row| row.typed_core_projection_digest.clone()) {
        typed_projection_digests.insert(digest);
    }

    let mut alias_projection_digests = BTreeSet::new();
    if let Some(digest) = delta.and_then(|row| row.projection_digest.clone()) {
        alias_projection_digests.insert(digest);
    }
    if let Some(digest) = required.and_then(|row| row.projection_digest.clone()) {
        alias_projection_digests.insert(digest);
    }
    if let Some(digest) = decision.and_then(|row| row.projection_digest.clone()) {
        alias_projection_digests.insert(digest);
    }
    let effective_projection_digests = if typed_projection_digests.is_empty() {
        alias_projection_digests.clone()
    } else {
        typed_projection_digests.clone()
    };

    let mut instruction_policy_digests = BTreeSet::new();
    let mut missing_instruction_policy_ids = BTreeSet::new();
    for row in instructions {
        if let Some(policy) = row.policy_digest.clone() {
            instruction_policy_digests.insert(policy);
        } else {
            missing_instruction_policy_ids.insert(row.instruction_id.clone());
        }
    }

    let mut drift_classes = Vec::new();
    if projection_policies.len() > 1 {
        drift_classes.push("projection_policy_drift".to_string());
    }
    if effective_projection_digests.len() > 1 {
        drift_classes.push("projection_digest_drift".to_string());
    }
    if instruction_policy_digests.len() > 1 {
        drift_classes.push("instruction_policy_drift".to_string());
    }

    json!({
        "projectionPolicies": projection_policies.into_iter().collect::<Vec<_>>(),
        "projectionDigests": effective_projection_digests.into_iter().collect::<Vec<_>>(),
        "typedCoreProjectionDigests": typed_projection_digests.into_iter().collect::<Vec<_>>(),
        "aliasProjectionDigests": alias_projection_digests.into_iter().collect::<Vec<_>>(),
        "instructionPolicyDigests": instruction_policy_digests.into_iter().collect::<Vec<_>>(),
        "missingInstructionPolicyIds": missing_instruction_policy_ids.into_iter().collect::<Vec<_>>(),
        "driftClasses": drift_classes,
        "driftDetected": !drift_classes.is_empty()
    })
}

fn coherence_instruction_typing(instructions: &[InstructionSummary]) -> Value {
    let mut unknown_instruction_ids = BTreeSet::new();
    let mut unknown_rejected_ids = BTreeSet::new();
    let mut typed_instruction_ids = BTreeSet::new();

    for row in instructions {
        let instruction_id = row.instruction_id.clone();
        let state = row
            .instruction_classification
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("state"))
            .and_then(Value::as_str);
        match state {
            Some("unknown") => {
                unknown_instruction_ids.insert(instruction_id.clone());
                if row.verdict_class.as_deref() == Some("rejected") {
                    unknown_rejected_ids.insert(instruction_id);
                }
            }
            Some("typed") => {
                typed_instruction_ids.insert(instruction_id);
            }
            _ => {}
        }
    }

    let instruction_count = instructions.len();
    let unknown_count = unknown_instruction_ids.len();
    let unknown_rate = if instruction_count == 0 {
        0.0
    } else {
        unknown_count as f64 / instruction_count as f64
    };

    json!({
        "instructionCount": instruction_count,
        "typedCount": typed_instruction_ids.len(),
        "unknownCount": unknown_count,
        "unknownRejectedCount": unknown_rejected_ids.len(),
        "unknownRate": round_f64(unknown_rate, 6),
        "unknownRatePercent": round_f64(unknown_rate * 100.0, 2),
        "unknownInstructionIds": unknown_instruction_ids.into_iter().collect::<Vec<_>>()
    })
}

fn coherence_proposal_reject_classes(instructions: &[InstructionSummary]) -> Value {
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut instruction_ids = BTreeSet::new();

    for row in instructions {
        let proposal_failures: BTreeSet<String> = row
            .failure_classes
            .iter()
            .filter(|item| item.starts_with("proposal_"))
            .cloned()
            .collect();
        if proposal_failures.is_empty() {
            continue;
        }
        instruction_ids.insert(row.instruction_id.clone());
        for failure in proposal_failures {
            *counts.entry(failure).or_insert(0) += 1;
        }
    }

    let total_reject_count = counts.values().sum::<u64>();
    let mut ranked: Vec<(String, u64)> = counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let top_classes = ranked
        .into_iter()
        .take(5)
        .map(|(name, _)| name)
        .collect::<Vec<_>>();

    json!({
        "totalRejectCount": total_reject_count,
        "classCounts": counts,
        "topClasses": top_classes,
        "instructionIds": instruction_ids.into_iter().collect::<Vec<_>>()
    })
}

fn coherence_issue_partition(issue_rows: &[serde_json::Map<String, Value>]) -> Value {
    let mut by_id = BTreeMap::new();
    for row in issue_rows {
        if let Some(issue_id) = string_opt(row.get("id")) {
            by_id.insert(issue_id, row);
        }
    }

    let open_ids = by_id
        .iter()
        .filter_map(|(issue_id, row)| {
            if string_opt(row.get("status")).as_deref() == Some("open") {
                Some(issue_id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut ready_ids = Vec::new();
    let mut blocked_ids = Vec::new();
    for issue_id in &open_ids {
        let row = by_id.get(issue_id).expect("open issue id should resolve");
        let mut blocked = false;
        if let Some(Value::Array(dependencies)) = row.get("dependencies") {
            for dep in dependencies {
                let Value::Object(dep) = dep else {
                    continue;
                };
                if !is_blocking_dep(dep) {
                    continue;
                }
                let Some(depends_on_id) = issue_depends_on_id(dep) else {
                    blocked = true;
                    break;
                };
                let blocker_status = by_id
                    .get(&depends_on_id)
                    .and_then(|blocker| string_opt(blocker.get("status")));
                if blocker_status.as_deref() == Some("closed") {
                    continue;
                }
                blocked = true;
                break;
            }
        }
        if blocked {
            blocked_ids.push(issue_id.clone());
        } else {
            ready_ids.push(issue_id.clone());
        }
    }

    let ready_set = ready_ids.iter().cloned().collect::<BTreeSet<_>>();
    let blocked_set = blocked_ids.iter().cloned().collect::<BTreeSet<_>>();
    let open_set = open_ids.iter().cloned().collect::<BTreeSet<_>>();
    let overlap_ids = ready_set
        .intersection(&blocked_set)
        .cloned()
        .collect::<Vec<_>>();
    let union = ready_set
        .union(&blocked_set)
        .cloned()
        .collect::<BTreeSet<_>>();
    let open_partition_gap_ids = open_set.difference(&union).cloned().collect::<Vec<_>>();

    json!({
        "openIssueCount": open_ids.len(),
        "readyCount": ready_ids.len(),
        "blockedCount": blocked_ids.len(),
        "readyIssueIds": ready_ids,
        "blockedIssueIds": blocked_ids,
        "overlapIssueIds": overlap_ids,
        "openPartitionGapIssueIds": open_partition_gap_ids,
        "isCoherent": overlap_ids.is_empty() && open_partition_gap_ids.is_empty()
    })
}

fn derive_reference_time(
    instructions: &[InstructionSummary],
    issue_rows: &[serde_json::Map<String, Value>],
) -> Option<DateTime<Utc>> {
    let mut candidates = Vec::new();
    for row in instructions {
        if let Some(parsed) = parse_rfc3339(row.run_finished_at.as_deref()) {
            candidates.push(parsed);
            continue;
        }
        if let Some(parsed) = parse_rfc3339(row.run_started_at.as_deref()) {
            candidates.push(parsed);
        }
    }
    for row in issue_rows {
        if let Some(parsed) = parse_rfc3339(string_opt(row.get("updated_at")).as_deref()) {
            candidates.push(parsed);
            continue;
        }
        if let Some(parsed) = parse_rfc3339(string_opt(row.get("updatedAt")).as_deref()) {
            candidates.push(parsed);
        }
    }
    candidates.into_iter().max()
}

fn coherence_lease_health(
    issue_rows: &[serde_json::Map<String, Value>],
    reference_time: Option<DateTime<Utc>>,
) -> Value {
    let mut stale_issue_ids = BTreeSet::new();
    let mut contended_issue_ids = BTreeSet::new();
    let mut unknown_evaluation_issue_ids = BTreeSet::new();
    let mut active_lease_count: u64 = 0;
    let mut lease_issue_count: u64 = 0;

    for row in issue_rows {
        let Some(Value::Object(lease)) = row.get("lease") else {
            continue;
        };
        lease_issue_count += 1;
        let issue_id = string_opt(row.get("id")).unwrap_or_else(|| "(unknown)".to_string());

        let Some(reference_time) = reference_time else {
            unknown_evaluation_issue_ids.insert(issue_id);
            continue;
        };

        let Some(expires_at) = lease_expires_at(lease) else {
            unknown_evaluation_issue_ids.insert(issue_id);
            continue;
        };

        if expires_at <= reference_time {
            stale_issue_ids.insert(issue_id);
            continue;
        }

        active_lease_count += 1;
        let owner = string_opt(lease.get("owner")).unwrap_or_default();
        let status = string_opt(row.get("status")).unwrap_or_default();
        let assignee = string_opt(row.get("assignee")).unwrap_or_default();
        if status != "in_progress" || assignee != owner {
            contended_issue_ids.insert(issue_id);
        }
    }

    json!({
        "referenceTime": reference_time.map(rfc3339),
        "leaseIssueCount": lease_issue_count,
        "activeLeaseCount": active_lease_count,
        "staleCount": stale_issue_ids.len(),
        "staleIssueIds": stale_issue_ids.into_iter().collect::<Vec<_>>(),
        "contendedCount": contended_issue_ids.len(),
        "contendedIssueIds": contended_issue_ids.into_iter().collect::<Vec<_>>(),
        "unknownEvaluationCount": unknown_evaluation_issue_ids.len(),
        "unknownEvaluationIssueIds": unknown_evaluation_issue_ids.into_iter().collect::<Vec<_>>()
    })
}

fn coherence_summary(
    delta: Option<&DeltaSummary>,
    required: Option<&RequiredSummary>,
    decision: Option<&DecisionSummary>,
    instructions: &[InstructionSummary],
    issue_rows: &[serde_json::Map<String, Value>],
) -> Value {
    let policy_drift = coherence_policy_drift(delta, required, decision, instructions);
    let instruction_typing = coherence_instruction_typing(instructions);
    let proposal_reject_classes = coherence_proposal_reject_classes(instructions);
    let issue_partition = coherence_issue_partition(issue_rows);
    let reference_time = derive_reference_time(instructions, issue_rows);
    let lease_health = coherence_lease_health(issue_rows, reference_time);

    let mut attention_reasons = Vec::new();
    if policy_drift
        .get("driftDetected")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        attention_reasons.push("policy_drift".to_string());
    }
    if instruction_typing
        .get("unknownCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        attention_reasons.push("instruction_unknown_classification".to_string());
    }
    if proposal_reject_classes
        .get("totalRejectCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        attention_reasons.push("proposal_reject_classes_present".to_string());
    }
    if !issue_partition
        .get("isCoherent")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        attention_reasons.push("issue_partition_incoherent".to_string());
    }
    if lease_health
        .get("staleCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        attention_reasons.push("stale_claims".to_string());
    }
    if lease_health
        .get("contendedCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        attention_reasons.push("contended_claims".to_string());
    }

    json!({
        "policyDrift": policy_drift,
        "instructionTyping": instruction_typing,
        "proposalRejectClasses": proposal_reject_classes,
        "issuePartition": issue_partition,
        "leaseHealth": lease_health,
        "needsAttention": !attention_reasons.is_empty(),
        "attentionReasons": attention_reasons
    })
}

pub fn build_surface(
    repo_root: &Path,
    ciwitness_dir: &Path,
    issues_path: Option<&Path>,
) -> Result<ObservationSurface, ObservationError> {
    let required_path = ciwitness_dir.join("latest-required.json");
    let delta_path = ciwitness_dir.join("latest-delta.json");
    let decision_path = ciwitness_dir.join("latest-decision.json");
    let resolved_issues_path: PathBuf = issues_path
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| repo_root.join(".premath/issues.jsonl"));

    let required_payload = load_json_object(&required_path)?;
    let delta_payload = load_json_object(&delta_path)?;
    let decision_payload = load_json_object(&decision_path)?;

    let required = required_payload
        .as_ref()
        .map(|payload| normalize_required(payload, relative_path(&required_path, repo_root)));
    let delta = delta_payload
        .as_ref()
        .map(|payload| normalize_delta(payload, relative_path(&delta_path, repo_root)));
    let decision = decision_payload
        .as_ref()
        .map(|payload| normalize_decision(payload, relative_path(&decision_path, repo_root)));

    let mut instructions = Vec::new();
    if ciwitness_dir.exists() {
        let mut files = fs::read_dir(ciwitness_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        files.sort();

        for path in files {
            let Some(name) = path.file_name().and_then(|file| file.to_str()) else {
                continue;
            };
            if name.starts_with("latest-") || name.starts_with("proj1_") {
                continue;
            }

            let Some(payload) = load_json_object(&path)? else {
                continue;
            };
            if string_opt(payload.get("witnessKind")) != Some(INSTRUCTION_WITNESS_KIND.to_string())
            {
                continue;
            }

            instructions.push(normalize_instruction(
                &payload,
                relative_path(&path, repo_root),
            ));
        }
    }
    instructions.sort_by(|left, right| left.instruction_id.cmp(&right.instruction_id));

    let issue_rows = load_issue_rows(&resolved_issues_path)?;

    let latest_projection_digest = decision
        .as_ref()
        .and_then(|row| {
            row.typed_core_projection_digest
                .clone()
                .or_else(|| row.projection_digest.clone())
        })
        .or_else(|| {
            required.as_ref().and_then(|row| {
                row.typed_core_projection_digest
                    .clone()
                    .or_else(|| row.projection_digest.clone())
            })
        })
        .or_else(|| {
            delta.as_ref().and_then(|row| {
                row.typed_core_projection_digest
                    .clone()
                    .or_else(|| row.projection_digest.clone())
            })
        });
    let latest_instruction_id = instructions.last().map(|row| row.instruction_id.clone());

    let state = derive_state(required.as_ref(), decision.as_ref(), &instructions);
    let coherence = coherence_summary(
        delta.as_ref(),
        required.as_ref(),
        decision.as_ref(),
        &instructions,
        &issue_rows,
    );
    let coherence_needs_attention = coherence
        .get("needsAttention")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let attention_reasons = coherence
        .get("attentionReasons")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut top_failure_class = state.top_failure_class;
    if !attention_reasons.is_empty()
        && (matches!(state.state.as_str(), "accepted" | "running" | "empty")
            || top_failure_class.is_none())
    {
        top_failure_class = attention_reasons
            .first()
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
    }

    Ok(ObservationSurface {
        schema: OBSERVATION_SCHEMA,
        surface_kind: OBSERVATION_KIND.to_string(),
        summary: ObservationSummary {
            state: state.state,
            needs_attention: state.needs_attention || coherence_needs_attention,
            top_failure_class,
            latest_projection_digest,
            latest_instruction_id,
            required_check_count: required
                .as_ref()
                .map(|row| row.required_checks.len() as u64)
                .unwrap_or(0),
            executed_check_count: required
                .as_ref()
                .map(|row| row.executed_checks.len() as u64)
                .unwrap_or(0),
            changed_path_count: delta
                .as_ref()
                .map(|row| row.changed_path_count)
                .unwrap_or(0),
            coherence: Some(coherence),
        },
        latest: LatestObservation {
            delta,
            required,
            decision,
        },
        instructions,
    })
}

pub fn build_events(surface: &ObservationSurface) -> Vec<ObservationEvent> {
    let mut events = Vec::new();

    if let Some(delta) = surface.latest.delta.as_ref() {
        events.push(ObservationEvent {
            kind: "ci.delta.v1.summary".to_string(),
            payload: serde_json::to_value(delta).unwrap_or(Value::Null),
        });
    }
    if let Some(required) = surface.latest.required.as_ref() {
        events.push(ObservationEvent {
            kind: REQUIRED_EVENT_KIND.to_string(),
            payload: serde_json::to_value(required).unwrap_or(Value::Null),
        });
    }
    if let Some(decision) = surface.latest.decision.as_ref() {
        events.push(ObservationEvent {
            kind: REQUIRED_DECISION_EVENT_KIND.to_string(),
            payload: serde_json::to_value(decision).unwrap_or(Value::Null),
        });
    }
    for row in &surface.instructions {
        events.push(ObservationEvent {
            kind: INSTRUCTION_EVENT_KIND.to_string(),
            payload: serde_json::to_value(row).unwrap_or(Value::Null),
        });
    }
    events.push(ObservationEvent {
        kind: "ci.observation.surface.v0.summary".to_string(),
        payload: serde_json::to_value(&surface.summary).unwrap_or(Value::Null),
    });
    if let Some(coherence) = surface.summary.coherence.as_ref()
        && coherence.is_object()
    {
        events.push(ObservationEvent {
            kind: "ci.observation.surface.v0.coherence".to_string(),
            payload: coherence.clone(),
        });
    }

    events
}

#[derive(Debug, Clone)]
pub struct ObservationIndex {
    surface: ObservationSurface,
    instruction_lookup: BTreeMap<String, usize>,
}

impl ObservationIndex {
    pub fn from_surface(mut surface: ObservationSurface) -> Result<Self, ObservationError> {
        if surface.schema != OBSERVATION_SCHEMA {
            return Err(ObservationError::Invalid(format!(
                "schema mismatch (expected={OBSERVATION_SCHEMA}, actual={})",
                surface.schema
            )));
        }
        if surface.surface_kind != OBSERVATION_KIND {
            return Err(ObservationError::Invalid(format!(
                "surfaceKind mismatch (expected={OBSERVATION_KIND}, actual={})",
                surface.surface_kind
            )));
        }

        surface
            .instructions
            .sort_by(|a, b| a.instruction_id.cmp(&b.instruction_id));

        let mut instruction_lookup = BTreeMap::new();
        for (idx, row) in surface.instructions.iter().enumerate() {
            instruction_lookup.insert(row.instruction_id.clone(), idx);
        }

        Ok(Self {
            surface,
            instruction_lookup,
        })
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ObservationError> {
        let bytes = fs::read(path)?;
        let surface = serde_json::from_slice::<ObservationSurface>(&bytes)?;
        Self::from_surface(surface)
    }

    pub fn summary(&self) -> &ObservationSummary {
        &self.surface.summary
    }

    pub fn latest(&self) -> &LatestObservation {
        &self.surface.latest
    }

    pub fn surface(&self) -> &ObservationSurface {
        &self.surface
    }

    pub fn instruction(&self, instruction_id: &str) -> Option<&InstructionSummary> {
        self.instruction_lookup
            .get(instruction_id)
            .and_then(|idx| self.surface.instructions.get(*idx))
    }

    pub fn projection(&self, projection_digest: &str) -> Option<ProjectionView> {
        let matches_projection = |typed: Option<&String>, alias: Option<&String>| {
            typed.is_some_and(|digest| digest == projection_digest)
                || alias.is_some_and(|digest| digest == projection_digest)
        };
        let required = self.surface.latest.required.clone().filter(|row| {
            matches_projection(
                row.typed_core_projection_digest.as_ref(),
                row.projection_digest.as_ref(),
            )
        });
        let delta = self.surface.latest.delta.clone().filter(|row| {
            matches_projection(
                row.typed_core_projection_digest.as_ref(),
                row.projection_digest.as_ref(),
            )
        });
        let decision = self.surface.latest.decision.clone().filter(|row| {
            matches_projection(
                row.typed_core_projection_digest.as_ref(),
                row.projection_digest.as_ref(),
            )
        });

        if required.is_none() && delta.is_none() && decision.is_none() {
            return None;
        }

        Some(ProjectionView {
            projection_digest: projection_digest.to_string(),
            required,
            delta,
            decision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_surface() -> ObservationSurface {
        ObservationSurface {
            schema: OBSERVATION_SCHEMA,
            surface_kind: OBSERVATION_KIND.to_string(),
            summary: ObservationSummary {
                state: "accepted".to_string(),
                needs_attention: false,
                top_failure_class: Some("verified_accept".to_string()),
                latest_projection_digest: Some("proj1_alpha".to_string()),
                latest_instruction_id: Some("i1".to_string()),
                required_check_count: 1,
                executed_check_count: 1,
                changed_path_count: 2,
                coherence: None,
            },
            latest: LatestObservation {
                delta: Some(DeltaSummary {
                    r#ref: "artifacts/ciwitness/latest-delta.json".to_string(),
                    projection_policy: Some("ci-topos-v0".to_string()),
                    projection_digest: Some("proj1_alpha".to_string()),
                    typed_core_projection_digest: Some("ev1_alpha".to_string()),
                    authority_payload_digest: Some("proj1_alpha".to_string()),
                    normalizer_id: Some("normalizer.ci.required.v1".to_string()),
                    policy_digest: Some("ci-topos-v0".to_string()),
                    delta_source: Some("git_diff+workspace".to_string()),
                    from_ref: Some("origin/main".to_string()),
                    to_ref: Some("HEAD".to_string()),
                    changed_paths: vec!["README.md".to_string(), "tools/ci/README.md".to_string()],
                    changed_path_count: 2,
                }),
                required: Some(RequiredSummary {
                    r#ref: "artifacts/ciwitness/latest-required.json".to_string(),
                    witness_kind: Some("ci.required.v1".to_string()),
                    projection_policy: Some("ci-topos-v0".to_string()),
                    projection_digest: Some("proj1_alpha".to_string()),
                    typed_core_projection_digest: Some("ev1_alpha".to_string()),
                    authority_payload_digest: Some("proj1_alpha".to_string()),
                    normalizer_id: Some("normalizer.ci.required.v1".to_string()),
                    policy_digest: Some("ci-topos-v0".to_string()),
                    verdict_class: Some("accepted".to_string()),
                    required_checks: vec!["baseline".to_string()],
                    executed_checks: vec!["baseline".to_string()],
                    failure_classes: vec![],
                }),
                decision: Some(DecisionSummary {
                    r#ref: "artifacts/ciwitness/latest-decision.json".to_string(),
                    decision_kind: Some("ci.required.decision.v1".to_string()),
                    projection_digest: Some("proj1_alpha".to_string()),
                    typed_core_projection_digest: Some("ev1_alpha".to_string()),
                    authority_payload_digest: Some("proj1_alpha".to_string()),
                    normalizer_id: Some("normalizer.ci.required.v1".to_string()),
                    policy_digest: Some("ci-topos-v0".to_string()),
                    decision: Some("accept".to_string()),
                    reason_class: Some("verified_accept".to_string()),
                    witness_path: None,
                    delta_snapshot_path: None,
                    required_checks: vec!["baseline".to_string()],
                }),
            },
            instructions: vec![InstructionSummary {
                r#ref: "artifacts/ciwitness/20260221T010000Z-ci-wiring-golden.json".to_string(),
                witness_kind: Some("ci.instruction.v1".to_string()),
                instruction_id: "20260221T010000Z-ci-wiring-golden".to_string(),
                instruction_digest: Some("instr1_alpha".to_string()),
                typed_core_projection_digest: Some("ev1_instr_alpha".to_string()),
                authority_payload_digest: Some("instr1_alpha".to_string()),
                instruction_classification: None,
                intent: Some("validate wiring".to_string()),
                scope: None,
                policy_digest: Some("policy.ci.v1".to_string()),
                verdict_class: Some("accepted".to_string()),
                required_checks: vec!["ci-wiring-check".to_string()],
                executed_checks: vec!["ci-wiring-check".to_string()],
                failure_classes: vec![],
                run_started_at: None,
                run_finished_at: None,
                proposal_ingest: None,
            }],
        }
    }

    #[test]
    fn instruction_lookup_and_projection_query() {
        let surface = sample_surface();
        let index = ObservationIndex::from_surface(surface).expect("surface should be valid");
        assert_eq!(index.summary().state, "accepted");
        assert!(
            index
                .instruction("20260221T010000Z-ci-wiring-golden")
                .is_some()
        );
        assert!(index.projection("proj1_alpha").is_some());
        assert!(index.projection("proj1_missing").is_none());
    }

    #[test]
    fn invalid_surface_kind_rejected() {
        let mut surface = sample_surface();
        surface.surface_kind = "wrong.kind".to_string();
        let err = ObservationIndex::from_surface(surface).expect_err("surface should be invalid");
        assert!(matches!(err, ObservationError::Invalid(_)));
    }

    fn write_json(path: &Path, payload: Value) {
        let parent = path.parent().expect("parent path should exist");
        fs::create_dir_all(parent).expect("parent directory should create");
        fs::write(
            path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&payload).expect("payload should render")
            ),
        )
        .expect("payload should write");
    }

    fn write_issue_jsonl(path: &Path, issue_rows: &[Value]) {
        let parent = path.parent().expect("issues parent should exist");
        fs::create_dir_all(parent).expect("issues parent should create");
        let mut file = fs::File::create(path).expect("issues file should create");
        for row in issue_rows {
            writeln!(
                file,
                "{}",
                serde_json::to_string(row).expect("issue row should render")
            )
            .expect("issue row should write");
        }
    }

    fn base_issue(id: &str) -> Value {
        json!({
            "id": id,
            "title": format!("Issue {id}"),
            "description": "",
            "status": "open",
            "priority": 2,
            "issue_type": "task",
            "assignee": "",
            "owner": "",
            "created_at": "2026-02-22T00:00:00Z",
            "updated_at": "2026-02-22T00:00:00Z",
            "dependencies": []
        })
    }

    #[test]
    fn build_surface_projects_expected_summary() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let repo_root = std::env::temp_dir().join(format!(
            "premath-surreal-observation-{unique}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&repo_root).expect("temp root should create");
        let ciwitness = repo_root.join("artifacts/ciwitness");
        let issues = repo_root.join(".premath/issues.jsonl");

        write_json(
            &ciwitness.join("latest-delta.json"),
            json!({
                "projectionPolicy": "ci-topos-v0",
                "projectionDigest": "proj1_alpha",
                "deltaSource": "explicit",
                "changedPaths": ["README.md"]
            }),
        );
        write_json(
            &ciwitness.join("latest-required.json"),
            json!({
                "witnessKind": REQUIRED_WITNESS_KIND,
                "projectionPolicy": "ci-topos-v0",
                "projectionDigest": "proj1_alpha",
                "verdictClass": "accepted",
                "requiredChecks": ["baseline"],
                "executedChecks": ["baseline"],
                "failureClasses": []
            }),
        );
        write_json(
            &ciwitness.join("latest-decision.json"),
            json!({
                "decisionKind": REQUIRED_DECISION_KIND,
                "projectionDigest": "proj1_alpha",
                "decision": "accept",
                "reasonClass": "verified_accept"
            }),
        );
        write_json(
            &ciwitness.join("20260222T010000Z-ci.json"),
            json!({
                "witnessKind": INSTRUCTION_WITNESS_KIND,
                "instructionId": "20260222T010000Z-ci",
                "instructionDigest": "instr1_alpha",
                "policyDigest": "pol1_alpha",
                "verdictClass": "accepted",
                "requiredChecks": ["baseline"],
                "executedChecks": ["baseline"],
                "failureClasses": [],
                "runFinishedAt": "2026-02-22T01:00:00Z"
            }),
        );

        let issue = base_issue("bd-root");
        write_issue_jsonl(&issues, &[issue]);

        let surface =
            build_surface(&repo_root, &ciwitness, Some(&issues)).expect("surface should build");
        assert_eq!(surface.summary.state, "accepted");
        assert!(!surface.summary.needs_attention);
        assert_eq!(
            surface.summary.latest_projection_digest.as_deref(),
            Some("proj1_alpha")
        );
        assert_eq!(surface.summary.required_check_count, 1);
        assert_eq!(surface.summary.changed_path_count, 1);
        assert_eq!(surface.instructions.len(), 1);

        let _ = fs::remove_dir_all(&repo_root);
    }

    #[test]
    fn build_events_emits_coherence_row() {
        let mut surface = sample_surface();
        surface.summary.coherence = Some(json!({
            "needsAttention": false,
            "attentionReasons": []
        }));

        let events = build_events(&surface);
        let kinds = events.into_iter().map(|row| row.kind).collect::<Vec<_>>();
        assert!(kinds.contains(&REQUIRED_EVENT_KIND.to_string()));
        assert!(kinds.contains(&REQUIRED_DECISION_EVENT_KIND.to_string()));
        assert!(kinds.contains(&INSTRUCTION_EVENT_KIND.to_string()));
        assert!(kinds.contains(&"ci.observation.surface.v0.summary".to_string()));
        assert!(kinds.contains(&"ci.observation.surface.v0.coherence".to_string()));
    }
}
