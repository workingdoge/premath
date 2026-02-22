use crate::{
    CanonicalProposal, ProposalDischarge, ProposalError, ProposalObligation,
    compile_proposal_obligations, discharge_proposal_obligations, validate_proposal_payload,
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
    pub requested_checks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_type: Option<String>,
    pub typing_policy: InstructionTypingPolicy,
    pub capability_claims: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<ValidatedInstructionProposal>,
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

    Ok(ValidatedInstructionEnvelope {
        intent,
        scope: scope.clone(),
        normalizer_id,
        policy_digest,
        requested_checks,
        instruction_type,
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
}
