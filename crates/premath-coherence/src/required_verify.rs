use crate::required::{RequiredWitnessError, compute_typed_core_projection_digest};
use crate::required_projection::{
    PROJECTION_POLICY, normalize_paths as normalize_projection_paths, project_required_checks,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const REQUIRED_WITNESS_KIND: &str = "ci.required.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitnessVerifyDerived {
    pub changed_paths: Vec<String>,
    pub projection_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typed_core_projection_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_payload_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalizer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_digest: Option<String>,
    pub required_checks: Vec<String>,
    pub executed_checks: Vec<String>,
    pub gate_witness_source_by_check: BTreeMap<String, String>,
    pub gate_semantic_failure_classes_by_check: BTreeMap<String, Vec<String>>,
    pub docs_only: bool,
    pub reasons: Vec<String>,
    pub expected_verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitnessVerifyResult {
    pub errors: Vec<String>,
    pub derived: RequiredWitnessVerifyDerived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitnessVerifyRequest {
    pub witness: Value,
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub witness_root: Option<String>,
    #[serde(default)]
    pub gate_witness_payloads: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    pub native_required_checks: Vec<String>,
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

fn stable_sha256(value: &Value) -> String {
    let mut hasher = Sha256::new();
    let rendered = serde_json::to_string(&sort_json_value(value))
        .expect("canonical json rendering should succeed");
    hasher.update(rendered.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn string_list(value: Option<&Value>, label: &str, errors: &mut Vec<String>) -> Vec<String> {
    let Some(Value::Array(items)) = value else {
        errors.push(format!("{label} must be a list"));
        return Vec::new();
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
    out
}

fn sorted_unique(values: &[String]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for value in values {
        set.insert(value.clone());
    }
    set.into_iter().collect()
}

fn optional_string_list(
    value: Option<&Value>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<Vec<String>> {
    value?;
    Some(sorted_unique(&string_list(value, label, errors)))
}

fn check_str_field(
    witness: &Map<String, Value>,
    key: &str,
    expected: &str,
    errors: &mut Vec<String>,
) {
    let value = witness.get(key).cloned().unwrap_or(Value::Null);
    if value.as_str() != Some(expected) {
        errors.push(format!(
            "{key} mismatch (expected={expected:?}, actual={:?})",
            value
        ));
    }
}

fn optional_non_empty_string_field(
    witness: &Map<String, Value>,
    key: &str,
    errors: &mut Vec<String>,
) -> Option<String> {
    let value = witness.get(key).cloned().unwrap_or(Value::Null);
    let Some(text) = value.as_str() else {
        errors.push(format!("{key} must be a non-empty string"));
        return None;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        errors.push(format!("{key} must be a non-empty string"));
        return None;
    }
    Some(trimmed.to_string())
}

fn normalize_rel_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    normalized
}

fn is_hex64_lower(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn load_gate_witness_payload(
    artifact_rel_path: &str,
    errors: &mut Vec<String>,
    witness_root: Option<&Path>,
    gate_witness_payloads: Option<&BTreeMap<String, Value>>,
) -> Option<Value> {
    if let Some(payloads) = gate_witness_payloads {
        let Some(payload) = payloads.get(artifact_rel_path) else {
            errors.push(format!(
                "gateWitnessRefs missing inline payload: {artifact_rel_path}"
            ));
            return None;
        };
        if !payload.is_object() {
            errors.push(format!(
                "gateWitness payload must be an object: {artifact_rel_path}"
            ));
            return None;
        }
        return Some(payload.clone());
    }

    let root = witness_root?;
    let target = root.join(artifact_rel_path);
    if !target.exists() || !target.is_file() {
        errors.push(format!(
            "gateWitnessRefs artifact not found: {artifact_rel_path}"
        ));
        return None;
    }

    let raw = match fs::read_to_string(&target) {
        Ok(raw) => raw,
        Err(_) => {
            errors.push(format!(
                "gateWitnessRefs artifact not found: {artifact_rel_path}"
            ));
            return None;
        }
    };
    let payload: Value = match serde_json::from_str(&raw) {
        Ok(payload) => payload,
        Err(err) => {
            errors.push(format!(
                "gateWitness artifact is not valid json ({artifact_rel_path}): {err}"
            ));
            return None;
        }
    };
    if !payload.is_object() {
        errors.push(format!(
            "gateWitness artifact root must be object: {artifact_rel_path}"
        ));
        return None;
    }
    Some(payload)
}

fn verify_gate_witness_refs(
    witness: &Map<String, Value>,
    executed_checks: &[String],
    results_by_check: &BTreeMap<String, i64>,
    errors: &mut Vec<String>,
    witness_root: Option<&Path>,
    gate_witness_payloads: Option<&BTreeMap<String, Value>>,
) -> (BTreeMap<String, String>, BTreeMap<String, Vec<String>>) {
    let mut source_by_check: BTreeMap<String, String> = BTreeMap::new();
    let mut semantic_by_check: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let Some(refs_raw) = witness.get("gateWitnessRefs") else {
        return (source_by_check, semantic_by_check);
    };
    let Some(refs) = refs_raw.as_array() else {
        errors.push("gateWitnessRefs must be a list when present".to_string());
        return (source_by_check, semantic_by_check);
    };

    if refs.len() != executed_checks.len() {
        errors.push(format!(
            "gateWitnessRefs length mismatch (expected={}, actual={})",
            executed_checks.len(),
            refs.len()
        ));
    }

    for (idx, ref_value) in refs.iter().enumerate() {
        let Some(ref_obj) = ref_value.as_object() else {
            errors.push(format!("gateWitnessRefs[{idx}] must be an object"));
            continue;
        };

        let Some(check_id_raw) = ref_obj.get("checkId").and_then(Value::as_str) else {
            errors.push(format!(
                "gateWitnessRefs[{idx}].checkId must be a non-empty string"
            ));
            continue;
        };
        let check_id = check_id_raw.trim().to_string();
        if check_id.is_empty() {
            errors.push(format!(
                "gateWitnessRefs[{idx}].checkId must be a non-empty string"
            ));
            continue;
        }

        if let Some(expected_check_id) = executed_checks.get(idx)
            && expected_check_id != &check_id
        {
            errors.push(format!(
                "gateWitnessRefs[{idx}].checkId mismatch (expected={expected_check_id:?}, actual={check_id:?})"
            ));
        }

        let Some(exit_code) = results_by_check.get(&check_id).copied() else {
            errors.push(format!(
                "gateWitnessRefs[{idx}] unknown checkId: {check_id:?}"
            ));
            continue;
        };
        let expected_gate_result = if exit_code == 0 {
            "accepted"
        } else {
            "rejected"
        };

        match ref_obj.get("source").and_then(Value::as_str) {
            Some("native") => {
                source_by_check.insert(check_id.clone(), "native".to_string());
            }
            Some("fallback") => {
                source_by_check.insert(check_id.clone(), "fallback".to_string());
            }
            source => errors.push(format!(
                "gateWitnessRefs[{idx}].source must be 'native' or 'fallback' (actual={source:?})"
            )),
        }

        let Some(artifact_rel_path_raw) = ref_obj.get("artifactRelPath").and_then(Value::as_str)
        else {
            errors.push(format!(
                "gateWitnessRefs[{idx}].artifactRelPath must be a non-empty string"
            ));
            continue;
        };
        let artifact_rel_path = normalize_rel_path(artifact_rel_path_raw);
        if artifact_rel_path.is_empty() {
            errors.push(format!(
                "gateWitnessRefs[{idx}].artifactRelPath must be a non-empty string"
            ));
            continue;
        }
        if artifact_rel_path.starts_with('/')
            || artifact_rel_path.starts_with("../")
            || artifact_rel_path.contains("/../")
            || artifact_rel_path == ".."
        {
            errors.push(format!(
                "gateWitnessRefs[{idx}].artifactRelPath must be relative"
            ));
            continue;
        }

        let Some(sha256) = ref_obj.get("sha256").and_then(Value::as_str) else {
            errors.push(format!(
                "gateWitnessRefs[{idx}].sha256 must be 64 lowercase hex chars"
            ));
            continue;
        };
        if !is_hex64_lower(sha256) {
            errors.push(format!(
                "gateWitnessRefs[{idx}].sha256 must be 64 lowercase hex chars"
            ));
            continue;
        }

        if let Some(ref_witness_kind) = ref_obj.get("witnessKind")
            && !ref_witness_kind.is_null()
            && ref_witness_kind.as_str() != Some("gate")
        {
            errors.push(format!(
                "gateWitnessRefs[{idx}].witnessKind mismatch (expected='gate', actual={ref_witness_kind:?})"
            ));
        }

        if let Some(ref_result) = ref_obj.get("result")
            && !ref_result.is_null()
            && ref_result.as_str() != Some(expected_gate_result)
        {
            errors.push(format!(
                "gateWitnessRefs[{idx}].result mismatch (expected={expected_gate_result:?}, actual={ref_result:?})"
            ));
        }

        let ref_failure_classes = optional_string_list(
            ref_obj.get("failureClasses"),
            format!("gateWitnessRefs[{idx}].failureClasses").as_str(),
            errors,
        );

        let payload = load_gate_witness_payload(
            &artifact_rel_path,
            errors,
            witness_root,
            gate_witness_payloads,
        );
        let Some(payload) = payload else {
            if let Some(ref_classes) = ref_failure_classes {
                semantic_by_check.insert(check_id.clone(), ref_classes);
            }
            continue;
        };

        let payload_digest = stable_sha256(&payload);
        if payload_digest != sha256 {
            errors.push(format!(
                "gateWitnessRefs[{idx}] digest mismatch (expected={sha256}, actual={payload_digest})"
            ));
        }

        let payload_kind = payload.get("witnessKind");
        if payload_kind.and_then(Value::as_str) != Some("gate") {
            errors.push(format!(
                "gateWitnessRefs[{idx}] payload witnessKind mismatch (expected='gate', actual={payload_kind:?})"
            ));
        }

        let payload_result = payload.get("result").and_then(Value::as_str);
        if payload_result != Some(expected_gate_result) {
            errors.push(format!(
                "gateWitnessRefs[{idx}] payload result mismatch (expected={expected_gate_result:?}, actual={payload_result:?})"
            ));
        }

        let payload_failures = payload.get("failures");
        let mut payload_failure_classes = Vec::new();
        let Some(payload_failure_list) = payload_failures.and_then(Value::as_array) else {
            errors.push(format!(
                "gateWitnessRefs[{idx}] payload failures must be a list"
            ));
            semantic_by_check.insert(check_id.clone(), Vec::new());
            continue;
        };
        for (failure_idx, failure) in payload_failure_list.iter().enumerate() {
            let Some(failure_obj) = failure.as_object() else {
                errors.push(format!(
                    "gateWitnessRefs[{idx}] payload failures[{failure_idx}] must be an object"
                ));
                continue;
            };
            let Some(class_name) = failure_obj.get("class").and_then(Value::as_str) else {
                errors.push(format!(
                    "gateWitnessRefs[{idx}] payload failures[{failure_idx}].class must be a non-empty string"
                ));
                continue;
            };
            let class_name = class_name.trim();
            if class_name.is_empty() {
                errors.push(format!(
                    "gateWitnessRefs[{idx}] payload failures[{failure_idx}].class must be a non-empty string"
                ));
                continue;
            }
            payload_failure_classes.push(class_name.to_string());
        }
        payload_failure_classes = sorted_unique(&payload_failure_classes);
        if payload_result == Some("accepted") && !payload_failure_list.is_empty() {
            errors.push(format!(
                "gateWitnessRefs[{idx}] accepted payload must have empty failures list"
            ));
        }
        if payload_result == Some("rejected") && payload_failure_classes.is_empty() {
            errors.push(format!(
                "gateWitnessRefs[{idx}] rejected payload must include failures"
            ));
        }
        semantic_by_check.insert(check_id.clone(), payload_failure_classes.clone());

        if let Some(ref_classes) = ref_failure_classes
            && ref_classes != payload_failure_classes
        {
            errors.push(format!(
                "gateWitnessRefs[{idx}].failureClasses mismatch (expected={payload_failure_classes:?}, actual={ref_classes:?})"
            ));
        }

        if let Some(ref_run_id) = ref_obj.get("runId")
            && !ref_run_id.is_null()
        {
            let ref_run_id_text = ref_run_id.as_str().unwrap_or("").trim();
            if ref_run_id_text.is_empty() {
                errors.push(format!(
                    "gateWitnessRefs[{idx}].runId must be a non-empty string"
                ));
            } else if payload.get("runId").and_then(Value::as_str) != Some(ref_run_id_text) {
                errors.push(format!(
                    "gateWitnessRefs[{idx}] runId mismatch (ref={:?}, payload={:?})",
                    ref_run_id_text,
                    payload.get("runId")
                ));
            }
        }
    }

    (source_by_check, semantic_by_check)
}

pub fn verify_required_witness_payload(
    witness: &Value,
    changed_paths: &[String],
    witness_root: Option<&Path>,
    gate_witness_payloads: Option<&BTreeMap<String, Value>>,
    native_required_checks: &[String],
) -> RequiredWitnessVerifyResult {
    let mut errors: Vec<String> = Vec::new();
    let normalized_paths = normalize_projection_paths(changed_paths);
    let projection = project_required_checks(&normalized_paths);
    let expected_required = projection.required_checks.clone();

    let witness_obj: Map<String, Value> = match witness.as_object() {
        Some(obj) => obj.clone(),
        None => {
            errors.push("witness must be an object".to_string());
            let derived = RequiredWitnessVerifyDerived {
                changed_paths: normalized_paths,
                projection_digest: projection.projection_digest,
                typed_core_projection_digest: None,
                authority_payload_digest: None,
                normalizer_id: None,
                policy_digest: None,
                required_checks: expected_required,
                executed_checks: Vec::new(),
                gate_witness_source_by_check: BTreeMap::new(),
                gate_semantic_failure_classes_by_check: BTreeMap::new(),
                docs_only: projection.docs_only,
                reasons: projection.reasons,
                expected_verdict: "accepted".to_string(),
            };
            return RequiredWitnessVerifyResult { errors, derived };
        }
    };

    if witness_obj.get("ciSchema").and_then(Value::as_i64) != Some(1) {
        errors.push(format!(
            "ciSchema must be 1 (actual={:?})",
            witness_obj.get("ciSchema").cloned().unwrap_or(Value::Null)
        ));
    }
    check_str_field(
        &witness_obj,
        "witnessKind",
        REQUIRED_WITNESS_KIND,
        &mut errors,
    );
    check_str_field(
        &witness_obj,
        "projectionPolicy",
        PROJECTION_POLICY,
        &mut errors,
    );
    check_str_field(&witness_obj, "policyDigest", PROJECTION_POLICY, &mut errors);
    let normalizer_id = optional_non_empty_string_field(&witness_obj, "normalizerId", &mut errors);
    let authority_payload_digest =
        optional_non_empty_string_field(&witness_obj, "authorityPayloadDigest", &mut errors);
    let typed_core_projection_digest =
        optional_non_empty_string_field(&witness_obj, "typedCoreProjectionDigest", &mut errors);

    let witness_changed_paths = normalize_projection_paths(&string_list(
        witness_obj.get("changedPaths"),
        "changedPaths",
        &mut errors,
    ));
    if witness_changed_paths != normalized_paths {
        errors.push(format!(
            "changedPaths mismatch (expected={normalized_paths:?}, actual={witness_changed_paths:?})"
        ));
    }

    let projection_digest = witness_obj.get("projectionDigest").and_then(Value::as_str);
    if projection_digest != Some(projection.projection_digest.as_str()) {
        errors.push(format!(
            "projectionDigest mismatch (expected={:?}, actual={projection_digest:?})",
            projection.projection_digest
        ));
    }
    if authority_payload_digest.as_deref() != Some(projection.projection_digest.as_str()) {
        errors.push(format!(
            "authorityPayloadDigest mismatch (expected={:?}, actual={:?})",
            projection.projection_digest, authority_payload_digest
        ));
    }
    let expected_typed_core_projection_digest = normalizer_id.as_deref().map(|normalizer| {
        compute_typed_core_projection_digest(
            projection.projection_digest.as_str(),
            normalizer,
            PROJECTION_POLICY,
        )
    });
    if typed_core_projection_digest.as_deref() != expected_typed_core_projection_digest.as_deref() {
        errors.push(format!(
            "typedCoreProjectionDigest mismatch (expected={:?}, actual={:?})",
            expected_typed_core_projection_digest, typed_core_projection_digest
        ));
    }

    let required_checks = string_list(
        witness_obj.get("requiredChecks"),
        "requiredChecks",
        &mut errors,
    );
    if required_checks != expected_required {
        errors.push(format!(
            "requiredChecks mismatch (expected={expected_required:?}, actual={required_checks:?})"
        ));
    }

    let executed_checks = string_list(
        witness_obj.get("executedChecks"),
        "executedChecks",
        &mut errors,
    );
    if executed_checks != required_checks {
        errors.push(format!(
            "executedChecks mismatch (expected={required_checks:?}, actual={executed_checks:?})"
        ));
    }

    let mut result_check_ids: Vec<String> = Vec::new();
    let mut results_by_check: BTreeMap<String, i64> = BTreeMap::new();
    let mut failed_count = 0usize;

    let Some(results_raw) = witness_obj.get("results").and_then(Value::as_array) else {
        errors.push("results must be a list".to_string());
        let derived = RequiredWitnessVerifyDerived {
            changed_paths: projection.changed_paths,
            projection_digest: projection.projection_digest,
            typed_core_projection_digest: None,
            authority_payload_digest: None,
            normalizer_id: None,
            policy_digest: None,
            required_checks: expected_required,
            executed_checks,
            gate_witness_source_by_check: BTreeMap::new(),
            gate_semantic_failure_classes_by_check: BTreeMap::new(),
            docs_only: projection.docs_only,
            reasons: projection.reasons,
            expected_verdict: "accepted".to_string(),
        };
        return RequiredWitnessVerifyResult { errors, derived };
    };

    for (idx, row) in results_raw.iter().enumerate() {
        let Some(row_obj) = row.as_object() else {
            errors.push(format!("results[{idx}] must be an object"));
            continue;
        };

        let Some(check_id) = row_obj.get("checkId").and_then(Value::as_str) else {
            errors.push(format!("results[{idx}].checkId must be a non-empty string"));
            continue;
        };
        if check_id.is_empty() {
            errors.push(format!("results[{idx}].checkId must be a non-empty string"));
            continue;
        }
        if results_by_check.contains_key(check_id) {
            errors.push(format!(
                "results[{idx}].checkId must be unique (duplicate={check_id:?})"
            ));
            continue;
        }

        let status = row_obj.get("status").and_then(Value::as_str);
        let exit_code = row_obj.get("exitCode").and_then(Value::as_i64);
        result_check_ids.push(check_id.to_string());

        let Some(exit_code) = exit_code else {
            errors.push(format!("results[{idx}].exitCode must be an integer"));
            continue;
        };
        results_by_check.insert(check_id.to_string(), exit_code);

        if status != Some("passed") && status != Some("failed") {
            errors.push(format!(
                "results[{idx}].status must be 'passed' or 'failed'"
            ));
        }
        let expected_status = if exit_code == 0 { "passed" } else { "failed" };
        if status != Some(expected_status) {
            errors.push(format!(
                "results[{idx}] status/exitCode mismatch (status={status:?}, exitCode={exit_code})"
            ));
        }
        if exit_code != 0 {
            failed_count += 1;
        }
    }

    if result_check_ids != executed_checks {
        errors.push(format!(
            "results checkId sequence mismatch (expected={executed_checks:?}, actual={result_check_ids:?})"
        ));
    }

    let (source_by_check, semantic_by_check) = verify_gate_witness_refs(
        &witness_obj,
        &executed_checks,
        &results_by_check,
        &mut errors,
        witness_root,
        gate_witness_payloads,
    );

    let native_required_values: Vec<Value> = native_required_checks
        .iter()
        .map(|item| Value::String(item.clone()))
        .collect();
    let native_required = string_list(
        Some(&Value::Array(native_required_values)),
        "nativeRequiredChecks",
        &mut errors,
    );
    for (idx, check_id) in native_required.iter().enumerate() {
        if !executed_checks.contains(check_id) {
            errors.push(format!(
                "nativeRequiredChecks[{idx}] not executed (checkId={check_id:?}, executed={executed_checks:?})"
            ));
            continue;
        }
        let source = source_by_check.get(check_id).cloned();
        if source.as_deref() != Some("native") {
            errors.push(format!(
                "nativeRequiredChecks[{idx}] requires native source (checkId={check_id:?}, source={source:?})"
            ));
        }
    }

    let expected_semantic_failure_classes = sorted_unique(
        &semantic_by_check
            .values()
            .flat_map(|classes| classes.iter().cloned())
            .collect::<Vec<String>>(),
    );

    let docs_only = witness_obj.get("docsOnly").and_then(Value::as_bool);
    if docs_only != Some(projection.docs_only) {
        errors.push(format!(
            "docsOnly mismatch (expected={:?}, actual={docs_only:?})",
            projection.docs_only
        ));
    }

    let reasons = string_list(witness_obj.get("reasons"), "reasons", &mut errors);
    let expected_reasons = projection.reasons.clone();
    if reasons != expected_reasons {
        errors.push(format!(
            "reasons mismatch (expected={expected_reasons:?}, actual={reasons:?})"
        ));
    }

    let expected_verdict = if failed_count == 0 {
        "accepted".to_string()
    } else {
        "rejected".to_string()
    };
    let verdict_class = witness_obj.get("verdictClass").and_then(Value::as_str);
    if verdict_class != Some(expected_verdict.as_str()) {
        errors.push(format!(
            "verdictClass mismatch (expected={expected_verdict:?}, actual={verdict_class:?})"
        ));
    }

    let mut operational_failure_classes = optional_string_list(
        witness_obj.get("operationalFailureClasses"),
        "operationalFailureClasses",
        &mut errors,
    );
    if operational_failure_classes.is_none() {
        operational_failure_classes = Some(sorted_unique(&string_list(
            witness_obj.get("failureClasses"),
            "failureClasses",
            &mut errors,
        )));
    }

    let mut semantic_failure_classes = optional_string_list(
        witness_obj.get("semanticFailureClasses"),
        "semanticFailureClasses",
        &mut errors,
    );
    if semantic_failure_classes.is_none() {
        semantic_failure_classes = Some(Vec::new());
        if !expected_semantic_failure_classes.is_empty() {
            errors.push(
                "semanticFailureClasses missing while gate witness semantic lineage is available"
                    .to_string(),
            );
        }
    }

    let expected_operational_failure_classes: Vec<String> = if failed_count == 0 {
        Vec::new()
    } else {
        vec!["check_failed".to_string()]
    };

    if operational_failure_classes
        .as_deref()
        .unwrap_or(&Vec::new())
        != expected_operational_failure_classes
    {
        errors.push(format!(
            "operationalFailureClasses mismatch (expected={expected_operational_failure_classes:?}, actual={:?})",
            operational_failure_classes.unwrap_or_default()
        ));
    }

    if semantic_failure_classes.as_deref().unwrap_or(&Vec::new())
        != expected_semantic_failure_classes
    {
        errors.push(format!(
            "semanticFailureClasses mismatch (expected={expected_semantic_failure_classes:?}, actual={:?})",
            semantic_failure_classes.unwrap_or_default()
        ));
    }

    let failure_classes = sorted_unique(&string_list(
        witness_obj.get("failureClasses"),
        "failureClasses",
        &mut errors,
    ));
    let expected_failure_classes = sorted_unique(
        &expected_operational_failure_classes
            .iter()
            .chain(expected_semantic_failure_classes.iter())
            .cloned()
            .collect::<Vec<String>>(),
    );
    if failure_classes != expected_failure_classes {
        errors.push(format!(
            "failureClasses mismatch (expected={expected_failure_classes:?}, actual={failure_classes:?})"
        ));
    }

    let derived_projection_digest = projection.projection_digest.clone();
    let derived = RequiredWitnessVerifyDerived {
        changed_paths: normalized_paths,
        projection_digest: derived_projection_digest.clone(),
        typed_core_projection_digest: expected_typed_core_projection_digest,
        authority_payload_digest: Some(derived_projection_digest),
        normalizer_id,
        policy_digest: Some(PROJECTION_POLICY.to_string()),
        required_checks: expected_required,
        executed_checks,
        gate_witness_source_by_check: source_by_check,
        gate_semantic_failure_classes_by_check: semantic_by_check,
        docs_only: projection.docs_only,
        reasons: expected_reasons,
        expected_verdict,
    };

    RequiredWitnessVerifyResult { errors, derived }
}

pub fn verify_required_witness_request(
    request: &RequiredWitnessVerifyRequest,
) -> Result<RequiredWitnessVerifyResult, RequiredWitnessError> {
    let witness_root = request.witness_root.as_ref().map(Path::new);
    Ok(verify_required_witness_payload(
        &request.witness,
        &request.changed_paths,
        witness_root,
        request.gate_witness_payloads.as_ref(),
        &request.native_required_checks,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_witness() -> (Value, Vec<String>, BTreeMap<String, Value>) {
        let changed_paths = vec!["crates/premath-bd/src/lib.rs".to_string()];
        let projection = project_required_checks(&changed_paths);
        let required_checks = projection.required_checks.clone();
        let normalizer_id = "normalizer.ci.required.v1".to_string();
        let authority_payload_digest = projection.projection_digest.clone();
        let typed_core_projection_digest = compute_typed_core_projection_digest(
            &authority_payload_digest,
            &normalizer_id,
            PROJECTION_POLICY,
        );

        let gate_build = json!({
            "witnessKind": "gate",
            "runId": "run1_fixture_build",
            "result": "accepted",
            "failures": []
        });
        let gate_test = json!({
            "witnessKind": "gate",
            "runId": "run1_fixture_test",
            "result": "rejected",
            "failures": [{
                "class": "descent_failure",
                "lawRef": "GATE-3.3",
                "message": "fixture failure"
            }]
        });
        let gate_path_build = format!("gates/{}/01-build.json", projection.projection_digest);
        let gate_path_test = format!("gates/{}/02-test.json", projection.projection_digest);
        let mut gate_payloads = BTreeMap::new();
        gate_payloads.insert(gate_path_build.clone(), gate_build.clone());
        gate_payloads.insert(gate_path_test.clone(), gate_test.clone());

        let witness = json!({
            "ciSchema": 1,
            "witnessKind": "ci.required.v1",
            "projectionPolicy": PROJECTION_POLICY,
            "projectionDigest": projection.projection_digest,
            "typedCoreProjectionDigest": typed_core_projection_digest,
            "authorityPayloadDigest": authority_payload_digest,
            "normalizerId": normalizer_id,
            "policyDigest": PROJECTION_POLICY,
            "changedPaths": changed_paths,
            "requiredChecks": required_checks,
            "executedChecks": required_checks,
            "results": [
                {"checkId": "build", "status": "passed", "exitCode": 0, "durationMs": 10},
                {"checkId": "test", "status": "failed", "exitCode": 1, "durationMs": 20}
            ],
            "gateWitnessRefs": [
                {
                    "checkId": "build",
                    "artifactRelPath": gate_path_build,
                    "sha256": stable_sha256(&gate_build),
                    "source": "native",
                    "runId": "run1_fixture_build",
                    "witnessKind": "gate",
                    "result": "accepted",
                    "failureClasses": []
                },
                {
                    "checkId": "test",
                    "artifactRelPath": gate_path_test,
                    "sha256": stable_sha256(&gate_test),
                    "source": "native",
                    "runId": "run1_fixture_test",
                    "witnessKind": "gate",
                    "result": "rejected",
                    "failureClasses": ["descent_failure"]
                }
            ],
            "verdictClass": "rejected",
            "operationalFailureClasses": ["check_failed"],
            "semanticFailureClasses": ["descent_failure"],
            "failureClasses": ["check_failed", "descent_failure"],
            "docsOnly": false,
            "reasons": ["rust_surface_changed"]
        });

        (
            witness,
            vec!["crates/premath-bd/src/lib.rs".to_string()],
            gate_payloads,
        )
    }

    #[test]
    fn verify_required_witness_payload_accepts_fixture() {
        let (witness, changed_paths, gate_payloads) = fixture_witness();
        let result = verify_required_witness_payload(
            &witness,
            &changed_paths,
            None,
            Some(&gate_payloads),
            &[],
        );
        assert!(
            result.errors.is_empty(),
            "expected no errors, got {:?}",
            result.errors
        );
        assert_eq!(result.derived.expected_verdict, "rejected");
    }

    #[test]
    fn verify_required_witness_payload_rejects_missing_semantic_union_member() {
        let (mut witness, changed_paths, gate_payloads) = fixture_witness();
        witness["failureClasses"] = json!(["check_failed"]);
        let result = verify_required_witness_payload(
            &witness,
            &changed_paths,
            None,
            Some(&gate_payloads),
            &[],
        );
        assert!(
            result
                .errors
                .iter()
                .any(|err| err.contains("failureClasses mismatch"))
        );
    }
}
