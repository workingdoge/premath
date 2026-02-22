use crate::required_verify::verify_required_witness_payload;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const DECISION_KIND: &str = "ci.required.decision.v1";
const VERDICT_ACCEPTED: &str = "accepted";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitnessDecideRequest {
    pub witness: Value,
    #[serde(default)]
    pub expected_changed_paths: Option<Vec<String>>,
    #[serde(default)]
    pub witness_root: Option<String>,
    #[serde(default)]
    pub gate_witness_payloads: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    pub native_required_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitnessDecideResult {
    pub decision_kind: String,
    pub decision: String,
    pub reason_class: String,
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
    pub errors: Vec<String>,
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    normalized
}

fn normalize_paths(paths: &[String]) -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for path in paths {
        let normalized = normalize_path(path);
        if !normalized.is_empty() {
            out.insert(normalized);
        }
    }
    out.into_iter().collect()
}

fn extract_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>, Vec<String>> {
    let Some(Value::Array(items)) = value else {
        return Err(vec![format!("{label} must be a list")]);
    };

    let mut out = Vec::new();
    let mut errors = Vec::new();
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
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

fn reject_result(
    reason_class: &str,
    errors: Vec<String>,
    projection_digest: Option<String>,
    typed_core_projection_digest: Option<String>,
    authority_payload_digest: Option<String>,
    normalizer_id: Option<String>,
    policy_digest: Option<String>,
    required_checks: Option<Vec<String>>,
) -> RequiredWitnessDecideResult {
    RequiredWitnessDecideResult {
        decision_kind: DECISION_KIND.to_string(),
        decision: "reject".to_string(),
        reason_class: reason_class.to_string(),
        projection_digest,
        typed_core_projection_digest,
        authority_payload_digest,
        normalizer_id,
        policy_digest,
        required_checks,
        errors,
    }
}

pub fn decide_required_witness_request(
    request: &RequiredWitnessDecideRequest,
) -> RequiredWitnessDecideResult {
    let Some(witness) = request.witness.as_object() else {
        return reject_result(
            "invalid_witness_shape",
            vec!["witness must be an object".to_string()],
            None,
            None,
            None,
            None,
            None,
            None,
        );
    };

    let changed_paths = match extract_string_list(witness.get("changedPaths"), "changedPaths") {
        Ok(paths) => paths,
        Err(errors) => {
            return reject_result(
                "invalid_witness_shape",
                errors,
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }
    };

    let witness_root = request.witness_root.as_ref().map(Path::new);
    let verify = verify_required_witness_payload(
        &request.witness,
        &changed_paths,
        witness_root,
        request.gate_witness_payloads.as_ref(),
        &request.native_required_checks,
    );

    let mut errors = verify.errors;
    if let Some(expected_paths_raw) = request.expected_changed_paths.as_ref() {
        let expected_paths = normalize_paths(expected_paths_raw);
        let witness_paths = normalize_paths(&changed_paths);
        if expected_paths != witness_paths {
            errors.push(format!(
                "delta comparison mismatch (detected={expected_paths:?}, witness={witness_paths:?})"
            ));
        }
    }

    let witness_verdict = witness.get("verdictClass").and_then(Value::as_str);
    if witness_verdict != Some(VERDICT_ACCEPTED) {
        errors.push(format!(
            "required witness verdict must be accepted for decision accept (actual={witness_verdict:?})"
        ));
    }

    let decision = if errors.is_empty() {
        "accept"
    } else {
        "reject"
    };
    let reason_class = if errors.is_empty() {
        "verified_accept"
    } else {
        "verification_reject"
    };

    RequiredWitnessDecideResult {
        decision_kind: DECISION_KIND.to_string(),
        decision: decision.to_string(),
        reason_class: reason_class.to_string(),
        projection_digest: Some(verify.derived.projection_digest),
        typed_core_projection_digest: verify.derived.typed_core_projection_digest,
        authority_payload_digest: verify.derived.authority_payload_digest,
        normalizer_id: verify.derived.normalizer_id,
        policy_digest: verify.derived.policy_digest,
        required_checks: Some(verify.derived.required_checks),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, Value, json};
    use sha2::{Digest, Sha256};

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

    fn stable_sha256(value: &Value) -> String {
        let mut hasher = Sha256::new();
        let rendered = serde_json::to_string(&sort_json_value(value))
            .expect("canonical json rendering should succeed");
        hasher.update(rendered.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn projection_digest_for(changed_paths: &[String], required_checks: &[String]) -> String {
        let digest = stable_sha256(&json!({
            "projectionPolicy": "ci-topos-v0",
            "changedPaths": changed_paths,
            "requiredChecks": required_checks,
        }));
        format!("proj1_{digest}")
    }

    fn accepted_fixture() -> (Value, BTreeMap<String, Value>) {
        let changed_paths: Vec<String> = Vec::new();
        let required_checks = vec!["baseline".to_string()];
        let projection_digest = projection_digest_for(&changed_paths, &required_checks);
        let normalizer_id = "normalizer.ci.required.v1";
        let typed_core_projection_digest = crate::required::compute_typed_core_projection_digest(
            projection_digest.as_str(),
            normalizer_id,
            "ci-topos-v0",
        );

        let gate_baseline = json!({
            "witnessKind": "gate",
            "runId": "run1_fixture_baseline",
            "result": "accepted",
            "failures": []
        });
        let gate_path = format!("gates/{projection_digest}/01-baseline.json");
        let mut gate_payloads = BTreeMap::new();
        gate_payloads.insert(gate_path.clone(), gate_baseline.clone());

        let witness = json!({
            "ciSchema": 1,
            "witnessKind": "ci.required.v1",
            "projectionPolicy": "ci-topos-v0",
            "projectionDigest": projection_digest,
            "typedCoreProjectionDigest": typed_core_projection_digest,
            "authorityPayloadDigest": projection_digest,
            "normalizerId": normalizer_id,
            "policyDigest": "ci-topos-v0",
            "changedPaths": [],
            "requiredChecks": ["baseline"],
            "executedChecks": ["baseline"],
            "results": [
                {"checkId": "baseline", "status": "passed", "exitCode": 0, "durationMs": 10}
            ],
            "gateWitnessRefs": [
                {
                    "checkId": "baseline",
                    "artifactRelPath": gate_path,
                    "sha256": stable_sha256(&gate_baseline),
                    "source": "native",
                    "runId": "run1_fixture_baseline",
                    "witnessKind": "gate",
                    "result": "accepted",
                    "failureClasses": []
                }
            ],
            "verdictClass": "accepted",
            "operationalFailureClasses": [],
            "semanticFailureClasses": [],
            "failureClasses": [],
            "docsOnly": true,
            "reasons": ["empty_delta_fallback_baseline"]
        });

        (witness, gate_payloads)
    }

    #[test]
    fn decide_required_witness_accepts_fixture() {
        let (witness, gate_payloads) = accepted_fixture();
        let request = RequiredWitnessDecideRequest {
            witness,
            expected_changed_paths: Some(Vec::new()),
            witness_root: None,
            gate_witness_payloads: Some(gate_payloads),
            native_required_checks: Vec::new(),
        };
        let result = decide_required_witness_request(&request);
        assert_eq!(result.decision, "accept");
        assert_eq!(result.reason_class, "verified_accept");
        assert!(result.errors.is_empty());
        assert!(result.projection_digest.is_some());
        assert_eq!(result.required_checks, Some(vec!["baseline".to_string()]));
    }

    #[test]
    fn decide_required_witness_rejects_delta_mismatch() {
        let (witness, gate_payloads) = accepted_fixture();
        let request = RequiredWitnessDecideRequest {
            witness,
            expected_changed_paths: Some(vec!["README.md".to_string()]),
            witness_root: None,
            gate_witness_payloads: Some(gate_payloads),
            native_required_checks: Vec::new(),
        };
        let result = decide_required_witness_request(&request);
        assert_eq!(result.decision, "reject");
        assert_eq!(result.reason_class, "verification_reject");
        assert!(
            result
                .errors
                .iter()
                .any(|err| err.contains("delta comparison mismatch"))
        );
    }

    #[test]
    fn decide_required_witness_rejects_invalid_shape() {
        let request = RequiredWitnessDecideRequest {
            witness: json!({"changedPaths": "README.md"}),
            expected_changed_paths: None,
            witness_root: None,
            gate_witness_payloads: None,
            native_required_checks: Vec::new(),
        };
        let result = decide_required_witness_request(&request);
        assert_eq!(result.decision, "reject");
        assert_eq!(result.reason_class, "invalid_witness_shape");
        assert!(
            result
                .errors
                .iter()
                .any(|err| err.contains("changedPaths must be a list"))
        );
    }
}
