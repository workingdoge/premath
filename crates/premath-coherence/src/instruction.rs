use crate::{
    CanonicalProposal, ProposalBinding, ProposalDischarge, ProposalError, ProposalObligation,
    ProposalTargetJudgment, compile_proposal_obligations, discharge_proposal_obligations,
    validate_proposal_payload,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const POLICY_KIND: &str = "ci.instruction.policy.v1";
const POLICY_DIGEST_PREFIX: &str = "pol1_";
const SUPPORTED_INSTRUCTION_TYPES: [&str; 3] =
    ["ci.gate.check", "ci.gate.pre_commit", "ci.gate.pre_push"];
const INSTRUCTION_WITNESS_KIND: &str = "ci.instruction.v1";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{failure_class}: {message}")]
pub struct InstructionError {
    pub failure_class: String,
    pub message: String,
}

impl InstructionError {
    fn new(failure_class: &str, message: impl Into<String>) -> Self {
        Self {
            failure_class: failure_class.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionTypingPolicy {
    pub allow_unknown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum InstructionClassification {
    Typed { kind: String },
    Unknown { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum InstructionExecutionDecision {
    Execute,
    Reject {
        source: String,
        reason: String,
        #[serde(rename = "operationalFailureClasses")]
        operational_failure_classes: Vec<String>,
        #[serde(rename = "semanticFailureClasses")]
        semantic_failure_classes: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedInstructionProposal {
    pub canonical: CanonicalProposal,
    pub digest: String,
    pub kcir_ref: String,
    pub obligations: Vec<ProposalObligation>,
    pub discharge: ProposalDischarge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedInstructionEnvelope {
    pub intent: String,
    pub scope: Value,
    pub normalizer_id: String,
    pub policy_digest: String,
    pub instruction_digest: String,
    pub requested_checks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_type: Option<String>,
    pub instruction_classification: InstructionClassification,
    pub execution_decision: InstructionExecutionDecision,
    pub typing_policy: InstructionTypingPolicy,
    pub capability_claims: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<ValidatedInstructionProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutedInstructionCheck {
    pub check_id: String,
    pub status: String,
    pub exit_code: i64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionWitnessRuntime {
    pub instruction_id: String,
    pub instruction_ref: String,
    pub instruction_digest: String,
    pub squeak_site_profile: String,
    pub run_started_at: String,
    pub run_finished_at: String,
    pub run_duration_ms: u64,
    pub results: Vec<ExecutedInstructionCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionProposalIngest {
    pub state: String,
    pub kind: String,
    pub proposal_digest: String,
    pub proposal_kcir_ref: String,
    pub binding: ProposalBinding,
    pub target_ctx_ref: String,
    pub target_judgment: ProposalTargetJudgment,
    pub obligations: Vec<ProposalObligation>,
    pub discharge: ProposalDischarge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionWitness {
    pub ci_schema: u32,
    pub witness_kind: String,
    pub instruction_id: String,
    pub instruction_ref: String,
    pub instruction_digest: String,
    pub instruction_classification: InstructionClassification,
    pub typing_policy: InstructionTypingPolicy,
    pub intent: String,
    pub scope: Value,
    pub normalizer_id: Option<String>,
    pub policy_digest: Option<String>,
    pub capability_claims: Vec<String>,
    pub required_checks: Vec<String>,
    pub executed_checks: Vec<String>,
    pub results: Vec<ExecutedInstructionCheck>,
    pub verdict_class: String,
    pub operational_failure_classes: Vec<String>,
    pub semantic_failure_classes: Vec<String>,
    pub failure_classes: Vec<String>,
    pub squeak_site_profile: String,
    pub run_started_at: String,
    pub run_finished_at: String,
    pub run_duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reject_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_ingest: Option<InstructionProposalIngest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PolicyArtifact {
    policy_digest: String,
    allowed_checks: BTreeSet<String>,
    allowed_normalizers: BTreeSet<String>,
}

fn sort_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut sorted = Map::new();
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

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(&sort_json_value(value))
        .expect("canonical json serialization should succeed")
}

fn stable_hash(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json(value).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn ensure_non_empty_string(
    value: Option<&Value>,
    label: &str,
    failure_class: &str,
) -> Result<String, InstructionError> {
    let Some(raw) = value else {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    };
    let Some(text) = raw.as_str() else {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    }
    Ok(trimmed.to_string())
}

fn ensure_non_empty_trimmed_string(
    value: Option<&Value>,
    label: &str,
    failure_class: &str,
) -> Result<String, InstructionError> {
    let Some(raw) = value else {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    };
    let Some(text) = raw.as_str() else {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    }
    if trimmed != text {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must not include leading/trailing whitespace"),
        ));
    }
    Ok(trimmed.to_string())
}

fn ensure_non_empty_string_list(
    value: Option<&Value>,
    label: &str,
    failure_class: &str,
) -> Result<Vec<String>, InstructionError> {
    let Some(raw) = value else {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty list"),
        ));
    };
    let Some(list) = raw.as_array() else {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty list"),
        ));
    };
    if list.is_empty() {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a non-empty list"),
        ));
    }

    let mut out = Vec::with_capacity(list.len());
    let mut seen = BTreeSet::new();
    for (idx, item) in list.iter().enumerate() {
        let Some(text) = item.as_str() else {
            return Err(InstructionError::new(
                failure_class,
                format!("{label}[{idx}] must be a non-empty string"),
            ));
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(InstructionError::new(
                failure_class,
                format!("{label}[{idx}] must be a non-empty string"),
            ));
        }
        if !seen.insert(trimmed.to_string()) {
            return Err(InstructionError::new(
                failure_class,
                format!("{label} must not contain duplicates"),
            ));
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}

fn ensure_optional_unique_string_list(
    value: Option<&Value>,
    label: &str,
    failure_class: &str,
) -> Result<Vec<String>, InstructionError> {
    let Some(raw) = value else {
        return Ok(Vec::new());
    };
    let Some(list) = raw.as_array() else {
        return Err(InstructionError::new(
            failure_class,
            format!("{label} must be a list when provided"),
        ));
    };
    let mut out = Vec::with_capacity(list.len());
    let mut seen = BTreeSet::new();
    for (idx, item) in list.iter().enumerate() {
        let Some(text) = item.as_str() else {
            return Err(InstructionError::new(
                failure_class,
                format!("{label}[{idx}] must be a non-empty string"),
            ));
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(InstructionError::new(
                failure_class,
                format!("{label}[{idx}] must be a non-empty string"),
            ));
        }
        if !seen.insert(trimmed.to_string()) {
            return Err(InstructionError::new(
                failure_class,
                format!("{label} must not contain duplicates"),
            ));
        }
        out.push(trimmed.to_string());
    }
    out.sort();
    Ok(out)
}

fn to_rel_or_abs(repo_root: &Path, path: &Path) -> String {
    match path.strip_prefix(repo_root) {
        Ok(rel) => rel.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

fn policy_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("policies").join("instruction")
}

fn compute_policy_digest(canonical_policy: &Value) -> String {
    format!("{}{}", POLICY_DIGEST_PREFIX, stable_hash(canonical_policy))
}

fn canonicalize_policy(
    payload: &Value,
    path: &Path,
    repo_root: &Path,
) -> Result<PolicyArtifact, InstructionError> {
    let Some(root) = payload.as_object() else {
        return Err(InstructionError::new(
            "instruction_policy_invalid_shape",
            format!(
                "{}: policy artifact root must be an object",
                to_rel_or_abs(repo_root, path)
            ),
        ));
    };

    let schema = root.get("schema").and_then(Value::as_i64).ok_or_else(|| {
        InstructionError::new(
            "instruction_policy_invalid_shape",
            format!("{}: schema must be 1", to_rel_or_abs(repo_root, path)),
        )
    })?;
    if schema != 1 {
        return Err(InstructionError::new(
            "instruction_policy_invalid_shape",
            format!("{}: schema must be 1", to_rel_or_abs(repo_root, path)),
        ));
    }

    let policy_kind = ensure_non_empty_string(
        root.get("policyKind"),
        &format!("{}: policyKind", to_rel_or_abs(repo_root, path)),
        "instruction_policy_invalid_shape",
    )?;
    if policy_kind != POLICY_KIND {
        return Err(InstructionError::new(
            "instruction_policy_invalid_shape",
            format!(
                "{}: policyKind must be {POLICY_KIND:?}",
                to_rel_or_abs(repo_root, path)
            ),
        ));
    }

    let policy_id = ensure_non_empty_string(
        root.get("policyId"),
        &format!("{}: policyId", to_rel_or_abs(repo_root, path)),
        "instruction_policy_invalid_shape",
    )?;

    let mut allowed_checks = ensure_non_empty_string_list(
        root.get("allowedChecks"),
        &format!("{}: allowedChecks", to_rel_or_abs(repo_root, path)),
        "instruction_policy_invalid_shape",
    )?;
    let mut allowed_normalizers = ensure_non_empty_string_list(
        root.get("allowedNormalizers"),
        &format!("{}: allowedNormalizers", to_rel_or_abs(repo_root, path)),
        "instruction_policy_invalid_shape",
    )?;
    allowed_checks.sort();
    allowed_normalizers.sort();

    let canonical_policy = json!({
        "schema": 1,
        "policyKind": POLICY_KIND,
        "policyId": policy_id,
        "allowedChecks": allowed_checks,
        "allowedNormalizers": allowed_normalizers,
    });
    let computed_digest = compute_policy_digest(&canonical_policy);

    let declared_digest = ensure_non_empty_string(
        root.get("policyDigest"),
        &format!("{}: policyDigest", to_rel_or_abs(repo_root, path)),
        "instruction_policy_invalid_shape",
    )?;
    if !declared_digest.starts_with(POLICY_DIGEST_PREFIX) {
        return Err(InstructionError::new(
            "instruction_policy_digest_mismatch",
            format!(
                "{}: policyDigest must start with {:?}",
                to_rel_or_abs(repo_root, path),
                POLICY_DIGEST_PREFIX
            ),
        ));
    }
    if declared_digest != computed_digest {
        return Err(InstructionError::new(
            "instruction_policy_digest_mismatch",
            format!(
                "{}: policyDigest mismatch (declared={}, computed={})",
                to_rel_or_abs(repo_root, path),
                declared_digest,
                computed_digest
            ),
        ));
    }

    Ok(PolicyArtifact {
        policy_digest: declared_digest,
        allowed_checks: allowed_checks.into_iter().collect(),
        allowed_normalizers: allowed_normalizers.into_iter().collect(),
    })
}

fn load_policy_registry(
    repo_root: &Path,
) -> Result<BTreeMap<String, PolicyArtifact>, InstructionError> {
    let dir = policy_dir(repo_root);
    if !dir.exists() || !dir.is_dir() {
        return Err(InstructionError::new(
            "instruction_unknown_policy",
            format!("policy registry not found at {}", dir.display()),
        ));
    }

    let mut registry = BTreeMap::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|err| {
            InstructionError::new(
                "instruction_unknown_policy",
                format!("failed to read policy registry {}: {err}", dir.display()),
            )
        })?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();

    for path in entries {
        let bytes = fs::read(&path).map_err(|err| {
            InstructionError::new(
                "instruction_policy_invalid_shape",
                format!("failed to read policy artifact {}: {err}", path.display()),
            )
        })?;
        let payload: Value = serde_json::from_slice(&bytes).map_err(|err| {
            InstructionError::new(
                "instruction_policy_invalid_shape",
                format!("invalid json policy artifact {}: {err}", path.display()),
            )
        })?;
        let artifact = canonicalize_policy(&payload, &path, repo_root)?;
        if registry.contains_key(&artifact.policy_digest) {
            return Err(InstructionError::new(
                "instruction_policy_invalid_shape",
                format!(
                    "{}: duplicate policyDigest {:?}",
                    to_rel_or_abs(repo_root, &path),
                    artifact.policy_digest
                ),
            ));
        }
        registry.insert(artifact.policy_digest.clone(), artifact);
    }

    if registry.is_empty() {
        return Err(InstructionError::new(
            "instruction_unknown_policy",
            format!("no policy artifacts found under {}", dir.display()),
        ));
    }
    Ok(registry)
}

fn resolve_instruction_policy(
    repo_root: &Path,
    policy_digest: &str,
) -> Result<PolicyArtifact, InstructionError> {
    if policy_digest.trim().is_empty() {
        return Err(InstructionError::new(
            "instruction_unknown_policy",
            "policyDigest must be a non-empty string",
        ));
    }
    if !policy_digest.starts_with(POLICY_DIGEST_PREFIX) {
        return Err(InstructionError::new(
            "instruction_unknown_policy",
            "policyDigest must be a canonical digest (pol1_...) referencing a registry artifact",
        ));
    }
    let registry = load_policy_registry(repo_root)?;
    registry.get(policy_digest).cloned().ok_or_else(|| {
        InstructionError::new(
            "instruction_unknown_policy",
            format!("policyDigest {:?} is not registered", policy_digest),
        )
    })
}

fn validate_requested_checks(
    repo_root: &Path,
    policy_digest: &str,
    requested_checks: &[String],
    normalizer_id: &str,
) -> Result<PolicyArtifact, InstructionError> {
    let policy = resolve_instruction_policy(repo_root, policy_digest)?;
    let disallowed: Vec<String> = requested_checks
        .iter()
        .filter(|check_id| !policy.allowed_checks.contains(*check_id))
        .cloned()
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    if !disallowed.is_empty() {
        return Err(InstructionError::new(
            "instruction_check_not_allowed",
            format!(
                "requestedChecks include check IDs not allowed by policyDigest {:?}: {:?}",
                policy_digest, disallowed
            ),
        ));
    }
    if !policy.allowed_normalizers.contains(normalizer_id) {
        return Err(InstructionError::new(
            "instruction_normalizer_not_allowed",
            format!(
                "normalizerId is not allowed by policyDigest {:?}: {:?}",
                policy_digest, normalizer_id
            ),
        ));
    }
    Ok(policy)
}

fn map_proposal_error(err: ProposalError) -> InstructionError {
    InstructionError::new(err.failure_class.as_str(), err.message)
}

fn extract_instruction_proposal(
    raw: &Map<String, Value>,
) -> Result<Option<Value>, InstructionError> {
    let proposal = raw.get("proposal");
    let llm_proposal = raw.get("llmProposal");
    if proposal.is_some() && llm_proposal.is_some() {
        return Err(InstructionError::new(
            "proposal_invalid_shape",
            "instruction may include only one of proposal or llmProposal",
        ));
    }
    let selected = proposal.or(llm_proposal);
    let Some(value) = selected else {
        return Ok(None);
    };
    if !value.is_object() {
        return Err(InstructionError::new(
            "proposal_invalid_shape",
            "instruction proposal must be an object",
        ));
    }
    Ok(Some(value.clone()))
}

fn classify_instruction(
    instruction_type: Option<&str>,
    requested_checks: &[String],
) -> InstructionClassification {
    if let Some(kind) = instruction_type {
        if SUPPORTED_INSTRUCTION_TYPES.contains(&kind) {
            return InstructionClassification::Typed {
                kind: kind.to_string(),
            };
        }
        return InstructionClassification::Unknown {
            reason: "unsupported_instruction_type".to_string(),
        };
    }

    if !requested_checks.is_empty()
        && requested_checks
            .iter()
            .all(|check| check.starts_with("hk-"))
    {
        if requested_checks.len() == 1 && requested_checks[0] == "hk-pre-commit" {
            return InstructionClassification::Typed {
                kind: "ci.gate.pre_commit".to_string(),
            };
        }
        if requested_checks.len() == 1 && requested_checks[0] == "hk-pre-push" {
            return InstructionClassification::Typed {
                kind: "ci.gate.pre_push".to_string(),
            };
        }
        return InstructionClassification::Typed {
            kind: "ci.gate.check".to_string(),
        };
    }

    InstructionClassification::Unknown {
        reason: "unrecognized_requested_checks".to_string(),
    }
}

fn sorted_unique_non_empty(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

fn derive_execution_decision(
    instruction_classification: &InstructionClassification,
    typing_policy: &InstructionTypingPolicy,
    proposal: Option<&ValidatedInstructionProposal>,
) -> InstructionExecutionDecision {
    if let InstructionClassification::Unknown { reason } = instruction_classification
        && !typing_policy.allow_unknown
    {
        return InstructionExecutionDecision::Reject {
            source: "instruction_classification".to_string(),
            reason: reason.clone(),
            operational_failure_classes: vec!["instruction_unknown_unroutable".to_string()],
            semantic_failure_classes: Vec::new(),
        };
    }

    if let Some(candidate) = proposal
        && candidate.discharge.outcome == "rejected"
    {
        return InstructionExecutionDecision::Reject {
            source: "proposal_discharge".to_string(),
            reason: "proposal_discharge_rejected".to_string(),
            operational_failure_classes: Vec::new(),
            semantic_failure_classes: sorted_unique_non_empty(&candidate.discharge.failure_classes),
        };
    }

    InstructionExecutionDecision::Execute
}

fn ensure_runtime_non_empty(value: &str, label: &str) -> Result<String, InstructionError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(InstructionError::new(
            "instruction_runtime_invalid",
            format!("{label} must be a non-empty string"),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_executed_checks(results: &[ExecutedInstructionCheck]) -> Vec<String> {
    results.iter().map(|row| row.check_id.clone()).collect()
}

fn proposal_ingest_from_checked(
    proposal: Option<&ValidatedInstructionProposal>,
) -> Option<InstructionProposalIngest> {
    proposal.map(|item| InstructionProposalIngest {
        state: "typed".to_string(),
        kind: format!("proposal.{}", item.canonical.proposal_kind),
        proposal_digest: item.digest.clone(),
        proposal_kcir_ref: item.kcir_ref.clone(),
        binding: item.canonical.binding.clone(),
        target_ctx_ref: item.canonical.target_ctx_ref.clone(),
        target_judgment: item.canonical.target_judgment.clone(),
        obligations: item.obligations.clone(),
        discharge: item.discharge.clone(),
    })
}

fn optional_trimmed_non_empty(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalized_typing_policy_from_envelope(envelope: Option<&Value>) -> InstructionTypingPolicy {
    let Some(raw) = envelope else {
        return InstructionTypingPolicy {
            allow_unknown: false,
        };
    };
    let Some(root) = raw.as_object() else {
        return InstructionTypingPolicy {
            allow_unknown: false,
        };
    };
    let allow_unknown = root
        .get("typingPolicy")
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("allowUnknown"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    InstructionTypingPolicy { allow_unknown }
}

fn normalized_requested_checks_from_envelope(envelope: Option<&Value>) -> Vec<String> {
    let Some(raw) = envelope else {
        return Vec::new();
    };
    let Some(root) = raw.as_object() else {
        return Vec::new();
    };
    let Some(values) = root.get("requestedChecks").and_then(Value::as_array) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

fn normalized_capability_claims_from_envelope(envelope: Option<&Value>) -> Vec<String> {
    let Some(raw) = envelope else {
        return Vec::new();
    };
    let Some(root) = raw.as_object() else {
        return Vec::new();
    };
    let Some(values) = root.get("capabilityClaims").and_then(Value::as_array) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

fn normalized_intent_from_envelope(envelope: Option<&Value>) -> String {
    optional_trimmed_non_empty(
        envelope
            .and_then(Value::as_object)
            .and_then(|root| root.get("intent")),
    )
    .unwrap_or_else(|| "(invalid envelope)".to_string())
}

fn normalized_scope_from_envelope(envelope: Option<&Value>) -> Value {
    envelope
        .and_then(Value::as_object)
        .and_then(|root| root.get("scope").cloned())
        .unwrap_or(Value::Null)
}

pub fn build_pre_execution_reject_witness(
    envelope: Option<&Value>,
    runtime: InstructionWitnessRuntime,
    failure_class: &str,
    reason: &str,
) -> Result<InstructionWitness, InstructionError> {
    let instruction_id = ensure_runtime_non_empty(&runtime.instruction_id, "instructionId")?;
    let instruction_ref = ensure_runtime_non_empty(&runtime.instruction_ref, "instructionRef")?;
    let instruction_digest =
        ensure_runtime_non_empty(&runtime.instruction_digest, "instructionDigest")?;
    let squeak_site_profile =
        ensure_runtime_non_empty(&runtime.squeak_site_profile, "squeakSiteProfile")?;
    let run_started_at = ensure_runtime_non_empty(&runtime.run_started_at, "runStartedAt")?;
    let run_finished_at = ensure_runtime_non_empty(&runtime.run_finished_at, "runFinishedAt")?;
    let failure_class = ensure_runtime_non_empty(failure_class, "failureClass")?;
    let reason = ensure_runtime_non_empty(reason, "reason")?;

    let normalizer_id = optional_trimmed_non_empty(
        envelope
            .and_then(Value::as_object)
            .and_then(|root| root.get("normalizerId")),
    );
    let policy_digest = optional_trimmed_non_empty(
        envelope
            .and_then(Value::as_object)
            .and_then(|root| root.get("policyDigest")),
    );

    Ok(InstructionWitness {
        ci_schema: 1,
        witness_kind: INSTRUCTION_WITNESS_KIND.to_string(),
        instruction_id,
        instruction_ref,
        instruction_digest,
        instruction_classification: InstructionClassification::Unknown {
            reason: "pre_execution_invalid".to_string(),
        },
        typing_policy: normalized_typing_policy_from_envelope(envelope),
        intent: normalized_intent_from_envelope(envelope),
        scope: normalized_scope_from_envelope(envelope),
        normalizer_id,
        policy_digest,
        capability_claims: normalized_capability_claims_from_envelope(envelope),
        required_checks: normalized_requested_checks_from_envelope(envelope),
        executed_checks: Vec::new(),
        results: Vec::new(),
        verdict_class: "rejected".to_string(),
        operational_failure_classes: vec![failure_class.clone()],
        semantic_failure_classes: Vec::new(),
        failure_classes: vec![failure_class],
        squeak_site_profile,
        run_started_at,
        run_finished_at,
        run_duration_ms: runtime.run_duration_ms,
        reject_stage: Some("pre_execution".to_string()),
        reject_reason: Some(reason),
        proposal_ingest: None,
    })
}

pub fn build_instruction_witness(
    checked: &ValidatedInstructionEnvelope,
    runtime: InstructionWitnessRuntime,
) -> Result<InstructionWitness, InstructionError> {
    let instruction_id = ensure_runtime_non_empty(&runtime.instruction_id, "instructionId")?;
    let instruction_ref = ensure_runtime_non_empty(&runtime.instruction_ref, "instructionRef")?;
    let instruction_digest =
        ensure_runtime_non_empty(&runtime.instruction_digest, "instructionDigest")?;
    let squeak_site_profile =
        ensure_runtime_non_empty(&runtime.squeak_site_profile, "squeakSiteProfile")?;
    let run_started_at = ensure_runtime_non_empty(&runtime.run_started_at, "runStartedAt")?;
    let run_finished_at = ensure_runtime_non_empty(&runtime.run_finished_at, "runFinishedAt")?;

    let results = runtime.results;
    let executed_checks = normalize_executed_checks(&results);
    let failed = results.iter().any(|row| row.exit_code != 0);

    let (verdict_class, mut operational_failure_classes, mut semantic_failure_classes) =
        match &checked.execution_decision {
            InstructionExecutionDecision::Execute => {
                if failed {
                    (
                        "rejected".to_string(),
                        vec!["check_failed".to_string()],
                        Vec::new(),
                    )
                } else {
                    ("accepted".to_string(), Vec::new(), Vec::new())
                }
            }
            InstructionExecutionDecision::Reject {
                operational_failure_classes,
                semantic_failure_classes,
                ..
            } => (
                "rejected".to_string(),
                operational_failure_classes.clone(),
                semantic_failure_classes.clone(),
            ),
        };

    operational_failure_classes = sorted_unique_non_empty(&operational_failure_classes);
    semantic_failure_classes = sorted_unique_non_empty(&semantic_failure_classes);
    let failure_classes = sorted_unique_non_empty(
        &operational_failure_classes
            .iter()
            .chain(semantic_failure_classes.iter())
            .cloned()
            .collect::<Vec<String>>(),
    );

    Ok(InstructionWitness {
        ci_schema: 1,
        witness_kind: INSTRUCTION_WITNESS_KIND.to_string(),
        instruction_id,
        instruction_ref,
        instruction_digest,
        instruction_classification: checked.instruction_classification.clone(),
        typing_policy: checked.typing_policy.clone(),
        intent: checked.intent.clone(),
        scope: checked.scope.clone(),
        normalizer_id: Some(checked.normalizer_id.clone()),
        policy_digest: Some(checked.policy_digest.clone()),
        capability_claims: checked.capability_claims.clone(),
        required_checks: checked.requested_checks.clone(),
        executed_checks,
        results,
        verdict_class,
        operational_failure_classes,
        semantic_failure_classes,
        failure_classes,
        squeak_site_profile,
        run_started_at,
        run_finished_at,
        run_duration_ms: runtime.run_duration_ms,
        reject_stage: None,
        reject_reason: None,
        proposal_ingest: proposal_ingest_from_checked(checked.proposal.as_ref()),
    })
}

fn validate_filename(path: &Path) -> Result<(), InstructionError> {
    if path.extension().is_none_or(|ext| ext != "json") {
        return Err(InstructionError::new(
            "instruction_filename_invalid",
            "filename must end with .json",
        ));
    }
    let stem = path
        .file_stem()
        .and_then(|item| item.to_str())
        .unwrap_or_default();
    if !stem.contains('-') {
        return Err(InstructionError::new(
            "instruction_filename_invalid",
            "filename stem must follow <ts>-<id> format",
        ));
    }
    Ok(())
}

pub fn validate_instruction_envelope_payload(
    raw: &Value,
    instruction_path: &Path,
    repo_root: &Path,
) -> Result<ValidatedInstructionEnvelope, InstructionError> {
    validate_filename(instruction_path)?;
    let root = raw.as_object().ok_or_else(|| {
        InstructionError::new(
            "instruction_envelope_invalid_shape",
            "root must be a JSON object",
        )
    })?;

    let schema = root.get("schema").and_then(Value::as_i64).ok_or_else(|| {
        InstructionError::new(
            "instruction_invalid_schema",
            "schema must be a positive integer",
        )
    })?;
    if schema <= 0 {
        return Err(InstructionError::new(
            "instruction_invalid_schema",
            "schema must be a positive integer",
        ));
    }

    let intent = ensure_non_empty_trimmed_string(
        root.get("intent"),
        "intent",
        "instruction_invalid_intent",
    )?;
    let normalizer_id = ensure_non_empty_trimmed_string(
        root.get("normalizerId"),
        "normalizerId",
        "instruction_invalid_normalizer",
    )?;
    let policy_digest = ensure_non_empty_trimmed_string(
        root.get("policyDigest"),
        "policyDigest",
        "instruction_invalid_policy_digest",
    )?;

    let Some(scope) = root.get("scope") else {
        return Err(InstructionError::new(
            "instruction_scope_missing",
            "scope is required",
        ));
    };
    if scope.is_null() || scope.as_str().is_some_and(str::is_empty) {
        return Err(InstructionError::new(
            "instruction_scope_invalid",
            "scope must be non-empty",
        ));
    }

    let requested_checks = ensure_non_empty_string_list(
        root.get("requestedChecks"),
        "requestedChecks",
        "instruction_requested_checks_invalid",
    )?;
    let _policy = validate_requested_checks(
        repo_root,
        policy_digest.as_str(),
        &requested_checks,
        normalizer_id.as_str(),
    )?;

    let instruction_type = match root.get("instructionType") {
        Some(_) => Some(ensure_non_empty_trimmed_string(
            root.get("instructionType"),
            "instructionType",
            "instruction_instruction_type_invalid",
        )?),
        None => None,
    };

    let typing_policy = match root.get("typingPolicy") {
        None => InstructionTypingPolicy {
            allow_unknown: false,
        },
        Some(policy_raw) => {
            let Some(policy) = policy_raw.as_object() else {
                return Err(InstructionError::new(
                    "instruction_typing_policy_invalid",
                    "typingPolicy must be an object when provided",
                ));
            };
            let allow_unknown = match policy.get("allowUnknown") {
                None => false,
                Some(item) => item.as_bool().ok_or_else(|| {
                    InstructionError::new(
                        "instruction_typing_policy_invalid",
                        "typingPolicy.allowUnknown must be a boolean when provided",
                    )
                })?,
            };
            InstructionTypingPolicy { allow_unknown }
        }
    };

    let capability_claims = ensure_optional_unique_string_list(
        root.get("capabilityClaims"),
        "capabilityClaims",
        "instruction_capability_claims_invalid",
    )?;

    let proposal = match extract_instruction_proposal(root)? {
        None => None,
        Some(raw_proposal) => {
            let validated = validate_proposal_payload(&raw_proposal).map_err(map_proposal_error)?;
            if validated.canonical.binding.normalizer_id != normalizer_id {
                return Err(InstructionError::new(
                    "proposal_binding_mismatch",
                    format!(
                        "proposal.binding.normalizerId must match instruction normalizerId ({:?} != {:?})",
                        validated.canonical.binding.normalizer_id, normalizer_id
                    ),
                ));
            }
            if validated.canonical.binding.policy_digest != policy_digest {
                return Err(InstructionError::new(
                    "proposal_binding_mismatch",
                    format!(
                        "proposal.binding.policyDigest must match instruction policyDigest ({:?} != {:?})",
                        validated.canonical.binding.policy_digest, policy_digest
                    ),
                ));
            }
            let obligations = compile_proposal_obligations(&validated.canonical);
            let discharge = discharge_proposal_obligations(&validated.canonical, &obligations);
            Some(ValidatedInstructionProposal {
                canonical: validated.canonical,
                digest: validated.digest,
                kcir_ref: validated.kcir_ref,
                obligations,
                discharge,
            })
        }
    };

    let instruction_classification =
        classify_instruction(instruction_type.as_deref(), &requested_checks);
    let execution_decision = derive_execution_decision(
        &instruction_classification,
        &typing_policy,
        proposal.as_ref(),
    );
    let instruction_digest = format!("instr1_{}", stable_hash(raw));

    Ok(ValidatedInstructionEnvelope {
        intent,
        scope: scope.clone(),
        normalizer_id,
        policy_digest,
        instruction_digest,
        requested_checks,
        instruction_type,
        instruction_classification,
        execution_decision,
        typing_policy,
        capability_claims,
        proposal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("repo root should resolve")
            .to_path_buf()
    }

    fn runtime_for(instruction_id: &str, failed: bool) -> InstructionWitnessRuntime {
        InstructionWitnessRuntime {
            instruction_id: instruction_id.to_string(),
            instruction_ref: format!("instructions/{instruction_id}.json"),
            instruction_digest: "instr1_demo".to_string(),
            squeak_site_profile: "local".to_string(),
            run_started_at: "2026-02-22T00:00:00Z".to_string(),
            run_finished_at: "2026-02-22T00:00:01Z".to_string(),
            run_duration_ms: 1000,
            results: vec![ExecutedInstructionCheck {
                check_id: "ci-wiring-check".to_string(),
                status: if failed { "failed" } else { "passed" }.to_string(),
                exit_code: if failed { 1 } else { 0 },
                duration_ms: 25,
            }],
        }
    }

    #[test]
    fn validate_instruction_envelope_accepts_fixture() {
        let root = repo_root();
        let fixture_path = root
            .join("tests")
            .join("ci")
            .join("fixtures")
            .join("instructions")
            .join("20260221T010000Z-ci-wiring-golden.json");
        let payload: Value =
            serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
                .expect("fixture json should parse");

        let checked = validate_instruction_envelope_payload(&payload, &fixture_path, &root)
            .expect("fixture should validate");
        assert_eq!(checked.normalizer_id, "normalizer.ci.v1");
        assert!(!checked.requested_checks.is_empty());
        assert_eq!(
            checked.instruction_classification,
            InstructionClassification::Typed {
                kind: "ci.gate.check".to_string()
            }
        );
        assert_eq!(
            checked.execution_decision,
            InstructionExecutionDecision::Execute
        );
        assert!(checked.proposal.is_some());
    }

    #[test]
    fn validate_instruction_envelope_rejects_proposal_binding_mismatch() {
        let root = repo_root();
        let fixture_path = root
            .join("tests")
            .join("ci")
            .join("fixtures")
            .join("instructions")
            .join("20260221T010000Z-ci-wiring-golden.json");
        let mut payload: Value =
            serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
                .expect("fixture json should parse");
        payload["proposal"]["binding"]["policyDigest"] = Value::String("pol1_other".to_string());

        let err = validate_instruction_envelope_payload(&payload, &fixture_path, &root)
            .expect_err("mismatched binding should fail");
        assert_eq!(err.failure_class, "proposal_binding_mismatch");
    }

    #[test]
    fn validate_instruction_envelope_rejects_unknown_classification_without_allow_unknown() {
        let root = repo_root();
        let fixture_path = root
            .join("tests")
            .join("ci")
            .join("fixtures")
            .join("instructions")
            .join("20260221T010000Z-ci-wiring-golden.json");
        let mut payload: Value =
            serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
                .expect("fixture json should parse");
        payload["instructionType"] = Value::String("ci.gate.unknown".to_string());

        let checked = validate_instruction_envelope_payload(&payload, &fixture_path, &root)
            .expect("fixture should validate with unknown classification");
        assert_eq!(
            checked.execution_decision,
            InstructionExecutionDecision::Reject {
                source: "instruction_classification".to_string(),
                reason: "unsupported_instruction_type".to_string(),
                operational_failure_classes: vec!["instruction_unknown_unroutable".to_string()],
                semantic_failure_classes: Vec::new(),
            }
        );
    }

    #[test]
    fn validate_instruction_envelope_rejects_on_proposal_discharge_before_execution() {
        let root = repo_root();
        let fixture_path = root
            .join("tests")
            .join("ci")
            .join("fixtures")
            .join("instructions")
            .join("20260221T010000Z-ci-wiring-golden.json");
        let mut payload: Value =
            serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
                .expect("fixture json should parse");
        payload["proposal"]["candidateRefs"] = Value::Array(Vec::new());

        let checked = validate_instruction_envelope_payload(&payload, &fixture_path, &root)
            .expect("proposal with empty candidate refs should validate");
        let expected_semantic_failure_classes = checked
            .proposal
            .as_ref()
            .expect("proposal should be present")
            .discharge
            .failure_classes
            .clone();
        assert_eq!(
            checked.execution_decision,
            InstructionExecutionDecision::Reject {
                source: "proposal_discharge".to_string(),
                reason: "proposal_discharge_rejected".to_string(),
                operational_failure_classes: Vec::new(),
                semantic_failure_classes: expected_semantic_failure_classes,
            }
        );
    }

    #[test]
    fn build_instruction_witness_marks_check_failure_as_operational_reject() {
        let root = repo_root();
        let fixture_path = root
            .join("tests")
            .join("ci")
            .join("fixtures")
            .join("instructions")
            .join("20260221T010000Z-ci-wiring-golden.json");
        let payload: Value =
            serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
                .expect("fixture json should parse");
        let checked = validate_instruction_envelope_payload(&payload, &fixture_path, &root)
            .expect("fixture should validate");

        let witness = build_instruction_witness(
            &checked,
            runtime_for("20260221T010000Z-ci-wiring-golden", true),
        )
        .expect("witness should build");
        assert_eq!(witness.verdict_class, "rejected");
        assert_eq!(
            witness.operational_failure_classes,
            vec!["check_failed".to_string()]
        );
        assert_eq!(witness.semantic_failure_classes, Vec::<String>::new());
        assert_eq!(witness.failure_classes, vec!["check_failed".to_string()]);
        assert!(witness.proposal_ingest.is_some());
    }

    #[test]
    fn build_instruction_witness_preserves_semantic_reject_from_execution_decision() {
        let root = repo_root();
        let fixture_path = root
            .join("tests")
            .join("ci")
            .join("fixtures")
            .join("instructions")
            .join("20260221T010000Z-ci-wiring-golden.json");
        let mut payload: Value =
            serde_json::from_slice(&fs::read(&fixture_path).expect("fixture should be readable"))
                .expect("fixture json should parse");
        payload["proposal"]["candidateRefs"] = Value::Array(Vec::new());
        let checked = validate_instruction_envelope_payload(&payload, &fixture_path, &root)
            .expect("proposal with empty candidate refs should validate");
        assert!(matches!(
            checked.execution_decision,
            InstructionExecutionDecision::Reject { .. }
        ));

        let witness = build_instruction_witness(
            &checked,
            InstructionWitnessRuntime {
                results: Vec::new(),
                ..runtime_for("20260221T010000Z-ci-wiring-golden", false)
            },
        )
        .expect("witness should build");
        assert_eq!(witness.verdict_class, "rejected");
        assert_eq!(witness.operational_failure_classes, Vec::<String>::new());
        assert_eq!(
            witness.semantic_failure_classes,
            checked
                .proposal
                .as_ref()
                .expect("proposal should be present")
                .discharge
                .failure_classes
        );
        assert_eq!(witness.failure_classes, witness.semantic_failure_classes);
    }

    #[test]
    fn build_pre_execution_reject_witness_preserves_invalid_envelope_projection() {
        let envelope = json!({
            "schema": 1,
            "intent": "  ",
            "scope": {"kind": "repo"},
            "policyDigest": "pol1_demo",
            "requestedChecks": ["ci-wiring-check", "ci-wiring-check", "   "],
            "typingPolicy": {
                "allowUnknown": true
            },
            "capabilityClaims": ["capabilities.instruction_typing", "capabilities.instruction_typing"]
        });

        let witness = build_pre_execution_reject_witness(
            Some(&envelope),
            InstructionWitnessRuntime {
                instruction_id: "20260222T000001Z-invalid-normalizer".to_string(),
                instruction_ref: "instructions/20260222T000001Z-invalid-normalizer.json"
                    .to_string(),
                instruction_digest: "instr1_demo".to_string(),
                squeak_site_profile: "local".to_string(),
                run_started_at: "2026-02-22T00:00:00Z".to_string(),
                run_finished_at: "2026-02-22T00:00:01Z".to_string(),
                run_duration_ms: 1000,
                results: Vec::new(),
            },
            "instruction_invalid_normalizer",
            "normalizerId must be a non-empty string",
        )
        .expect("pre-execution witness should build");

        assert_eq!(witness.verdict_class, "rejected");
        assert_eq!(
            witness.operational_failure_classes,
            vec!["instruction_invalid_normalizer".to_string()]
        );
        assert_eq!(witness.reject_stage, Some("pre_execution".to_string()));
        assert_eq!(
            witness.reject_reason,
            Some("normalizerId must be a non-empty string".to_string())
        );
        assert_eq!(witness.normalizer_id, None);
        assert_eq!(witness.policy_digest, Some("pol1_demo".to_string()));
        assert_eq!(witness.intent, "(invalid envelope)".to_string());
        assert_eq!(witness.required_checks, vec!["ci-wiring-check".to_string()]);
        assert_eq!(
            witness.capability_claims,
            vec!["capabilities.instruction_typing".to_string()]
        );
    }
}
