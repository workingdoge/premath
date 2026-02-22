use crate::required::RequiredWitnessError;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const PROJECTION_POLICY: &str = "ci-topos-v0";
const REQUIRED_WITNESS_KIND: &str = "ci.required.v1";
const CHECK_BASELINE: &str = "baseline";
const CHECK_BUILD: &str = "build";
const CHECK_TEST: &str = "test";
const CHECK_TEST_TOY: &str = "test-toy";
const CHECK_TEST_KCIR_TOY: &str = "test-kcir-toy";
const CHECK_CONFORMANCE: &str = "conformance-check";
const CHECK_CONFORMANCE_RUN: &str = "conformance-run";
const CHECK_DOCTRINE: &str = "doctrine-check";
const CHECK_ORDER: [&str; 8] = [
    CHECK_BASELINE,
    CHECK_BUILD,
    CHECK_TEST,
    CHECK_TEST_TOY,
    CHECK_TEST_KCIR_TOY,
    CHECK_CONFORMANCE,
    CHECK_CONFORMANCE_RUN,
    CHECK_DOCTRINE,
];
const DOC_FILE_NAMES: [&str; 5] = [
    "AGENTS.md",
    "COMMITMENT.md",
    "README.md",
    "RELEASE_NOTES.md",
    "LICENSE",
];
const DOC_EXTENSIONS: [&str; 6] = [".md", ".mdx", ".rst", ".txt", ".adoc", ".md"];
const SEMANTIC_BASELINE_PREFIXES: [&str; 4] = [
    ".github/workflows/",
    "tools/ci/",
    "tools/infra/terraform/",
    "infra/terraform/",
];
const SEMANTIC_BASELINE_EXACT: [&str; 3] = [".mise.toml", "hk.pkl", "pitchfork.toml"];
const RUST_PREFIXES: [&str; 1] = ["crates/"];
const RUST_EXACT: [&str; 4] = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain",
    "rust-toolchain.toml",
];
const KERNEL_PREFIX: &str = "crates/premath-kernel/";
const CONFORMANCE_PREFIXES: [&str; 6] = [
    "tests/conformance/",
    "tests/toy/fixtures/",
    "tests/kcir_toy/fixtures/",
    "tools/conformance/",
    "tools/toy/",
    "tools/kcir_toy/",
];
const RAW_DOC_TRIGGER_PREFIXES: [&str; 2] = ["specs/premath/raw/", "tests/conformance/"];
const DOCTRINE_DOC_PREFIXES: [&str; 3] = [
    "specs/premath/draft/",
    "specs/premath/raw/",
    "specs/process/",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitnessVerifyDerived {
    pub changed_paths: Vec<String>,
    pub projection_digest: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectionResult {
    changed_paths: Vec<String>,
    required_checks: Vec<String>,
    docs_only: bool,
    reasons: Vec<String>,
    projection_digest: String,
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

fn starts_with_any(path: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| path.starts_with(prefix))
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

fn is_doc_like_path(path: &str) -> bool {
    if DOC_FILE_NAMES.contains(&path) {
        return true;
    }
    if path.starts_with("docs/") || path.starts_with("specs/") {
        return true;
    }
    DOC_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

fn is_semantic_baseline_path(path: &str) -> bool {
    SEMANTIC_BASELINE_EXACT.contains(&path) || starts_with_any(path, &SEMANTIC_BASELINE_PREFIXES)
}

fn is_rust_path(path: &str) -> bool {
    RUST_EXACT.contains(&path) || starts_with_any(path, &RUST_PREFIXES)
}

fn is_conformance_path(path: &str) -> bool {
    starts_with_any(path, &CONFORMANCE_PREFIXES)
}

fn is_known_projection_surface(path: &str) -> bool {
    is_doc_like_path(path)
        || is_semantic_baseline_path(path)
        || is_rust_path(path)
        || is_conformance_path(path)
}

fn projection_digest(changed_paths: &[String], required_checks: &[String]) -> String {
    let digest = stable_sha256(&json!({
        "projectionPolicy": PROJECTION_POLICY,
        "changedPaths": changed_paths,
        "requiredChecks": required_checks,
    }));
    format!("proj1_{digest}")
}

fn project_required_checks(changed_paths: &[String]) -> ProjectionResult {
    let paths = normalize_paths(changed_paths);

    let mut reasons: BTreeSet<String> = BTreeSet::new();
    let mut checks: BTreeSet<String> = BTreeSet::new();

    if paths.is_empty() {
        reasons.insert("empty_delta_fallback_baseline".to_string());
        let ordered = vec![CHECK_BASELINE.to_string()];
        return ProjectionResult {
            changed_paths: paths.clone(),
            required_checks: ordered.clone(),
            docs_only: true,
            reasons: reasons.into_iter().collect(),
            projection_digest: projection_digest(&paths, &ordered),
        };
    }

    let docs_only = paths.iter().all(|path| is_doc_like_path(path));

    if paths.iter().any(|path| is_semantic_baseline_path(path)) {
        reasons.insert("semantic_surface_changed".to_string());
        checks.insert(CHECK_BASELINE.to_string());
    }

    if checks.contains(CHECK_BASELINE) {
        let ordered = vec![CHECK_BASELINE.to_string()];
        return ProjectionResult {
            changed_paths: paths.clone(),
            required_checks: ordered.clone(),
            docs_only,
            reasons: reasons.into_iter().collect(),
            projection_digest: projection_digest(&paths, &ordered),
        };
    }

    let rust_touched = paths.iter().any(|path| is_rust_path(path));
    if rust_touched {
        reasons.insert("rust_surface_changed".to_string());
        checks.insert(CHECK_BUILD.to_string());
        checks.insert(CHECK_TEST.to_string());
    }

    let kernel_touched = paths.iter().any(|path| path.starts_with(KERNEL_PREFIX));
    if kernel_touched {
        reasons.insert("kernel_surface_changed".to_string());
        checks.insert(CHECK_TEST_TOY.to_string());
        checks.insert(CHECK_TEST_KCIR_TOY.to_string());
    }

    let conformance_touched = paths.iter().any(|path| is_conformance_path(path));
    if conformance_touched {
        reasons.insert("conformance_surface_changed".to_string());
        checks.insert(CHECK_CONFORMANCE.to_string());
        checks.insert(CHECK_CONFORMANCE_RUN.to_string());
        checks.insert(CHECK_TEST_TOY.to_string());
        checks.insert(CHECK_TEST_KCIR_TOY.to_string());
    }

    let unknown_non_doc_paths: Vec<&String> = paths
        .iter()
        .filter(|path| !is_doc_like_path(path) && !is_known_projection_surface(path))
        .collect();
    if !unknown_non_doc_paths.is_empty() {
        reasons.insert("non_doc_unknown_surface_fallback_baseline".to_string());
        checks.insert(CHECK_BASELINE.to_string());
    }

    if docs_only {
        let raw_docs_touched = paths
            .iter()
            .any(|path| starts_with_any(path, &RAW_DOC_TRIGGER_PREFIXES));
        let doctrine_docs_touched = paths
            .iter()
            .any(|path| starts_with_any(path, &DOCTRINE_DOC_PREFIXES));
        if raw_docs_touched {
            reasons.insert("docs_only_raw_or_conformance_touched".to_string());
            checks.insert(CHECK_CONFORMANCE.to_string());
        }
        if doctrine_docs_touched {
            reasons.insert("docs_only_doctrine_surface_touched".to_string());
            checks.insert(CHECK_DOCTRINE.to_string());
        }
    }

    if checks.is_empty() && !docs_only {
        reasons.insert("non_doc_unknown_surface_fallback_baseline".to_string());
        checks.insert(CHECK_BASELINE.to_string());
    }

    let ordered: Vec<String> = if checks.contains(CHECK_BASELINE) {
        vec![CHECK_BASELINE.to_string()]
    } else {
        CHECK_ORDER
            .iter()
            .filter(|check_id| checks.contains(**check_id))
            .map(|check_id| (*check_id).to_string())
            .collect()
    };

    ProjectionResult {
        changed_paths: paths.clone(),
        required_checks: ordered.clone(),
        docs_only,
        reasons: reasons.into_iter().collect(),
        projection_digest: projection_digest(&paths, &ordered),
    }
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
    if value.is_none() {
        return None;
    }
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

    let Some(root) = witness_root else {
        return None;
    };
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
    let normalized_paths = normalize_paths(changed_paths);
    let projection = project_required_checks(&normalized_paths);
    let expected_required = projection.required_checks.clone();

    let witness_obj: Map<String, Value> = match witness.as_object() {
        Some(obj) => obj.clone(),
        None => {
            errors.push("witness must be an object".to_string());
            let derived = RequiredWitnessVerifyDerived {
                changed_paths: normalized_paths,
                projection_digest: projection.projection_digest,
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

    let witness_changed_paths = normalize_paths(&string_list(
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

    let derived = RequiredWitnessVerifyDerived {
        changed_paths: normalized_paths,
        projection_digest: projection.projection_digest,
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

    fn fixture_witness() -> (Value, Vec<String>, BTreeMap<String, Value>) {
        let changed_paths = vec!["crates/premath-bd/src/lib.rs".to_string()];
        let projection = project_required_checks(&changed_paths);
        let required_checks = projection.required_checks.clone();

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
