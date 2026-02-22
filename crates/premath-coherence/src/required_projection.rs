use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const PROJECTION_SCHEMA: u32 = 1;
pub const PROJECTION_POLICY: &str = "ci-topos-v0";

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
const DOC_EXTENSIONS: [&str; 5] = [".md", ".mdx", ".rst", ".txt", ".adoc"];
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
pub struct RequiredProjectionRequest {
    #[serde(default)]
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredProjectionResult {
    pub schema: u32,
    pub projection_policy: String,
    pub projection_digest: String,
    pub changed_paths: Vec<String>,
    pub required_checks: Vec<String>,
    pub docs_only: bool,
    pub reasons: Vec<String>,
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

pub fn normalize_paths(paths: &[String]) -> Vec<String> {
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

pub fn project_required_checks(changed_paths: &[String]) -> RequiredProjectionResult {
    let paths = normalize_paths(changed_paths);

    let mut reasons: BTreeSet<String> = BTreeSet::new();
    let mut checks: BTreeSet<String> = BTreeSet::new();

    if paths.is_empty() {
        reasons.insert("empty_delta_fallback_baseline".to_string());
        let ordered = vec![CHECK_BASELINE.to_string()];
        return RequiredProjectionResult {
            schema: PROJECTION_SCHEMA,
            projection_policy: PROJECTION_POLICY.to_string(),
            projection_digest: projection_digest(&paths, &ordered),
            changed_paths: paths,
            required_checks: ordered,
            docs_only: true,
            reasons: reasons.into_iter().collect(),
        };
    }

    let docs_only = paths.iter().all(|path| is_doc_like_path(path));

    if paths.iter().any(|path| is_semantic_baseline_path(path)) {
        reasons.insert("semantic_surface_changed".to_string());
        checks.insert(CHECK_BASELINE.to_string());
    }

    if checks.contains(CHECK_BASELINE) {
        let ordered = vec![CHECK_BASELINE.to_string()];
        return RequiredProjectionResult {
            schema: PROJECTION_SCHEMA,
            projection_policy: PROJECTION_POLICY.to_string(),
            projection_digest: projection_digest(&paths, &ordered),
            changed_paths: paths,
            required_checks: ordered,
            docs_only,
            reasons: reasons.into_iter().collect(),
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

    RequiredProjectionResult {
        schema: PROJECTION_SCHEMA,
        projection_policy: PROJECTION_POLICY.to_string(),
        projection_digest: projection_digest(&paths, &ordered),
        changed_paths: paths,
        required_checks: ordered,
        docs_only,
        reasons: reasons.into_iter().collect(),
    }
}

pub fn projection_plan_payload(
    projection: &RequiredProjectionResult,
    source: &str,
    from_ref: Option<&str>,
    to_ref: &str,
) -> Value {
    json!({
        "schema": projection.schema,
        "projectionPolicy": projection.projection_policy,
        "projectionDigest": projection.projection_digest,
        "changedPaths": projection.changed_paths,
        "requiredChecks": projection.required_checks,
        "docsOnly": projection.docs_only,
        "reasons": projection.reasons,
        "deltaSource": source,
        "fromRef": from_ref,
        "toRef": to_ref,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_required_checks_empty_delta_fallbacks_to_baseline() {
        let result = project_required_checks(&Vec::new());
        assert_eq!(result.required_checks, vec!["baseline".to_string()]);
        assert!(result.docs_only);
        assert!(
            result
                .reasons
                .contains(&"empty_delta_fallback_baseline".to_string())
        );
        assert!(result.projection_digest.starts_with("proj1_"));
    }

    #[test]
    fn project_required_checks_kernel_touch_includes_toys() {
        let result = project_required_checks(&["crates/premath-kernel/src/lib.rs".to_string()]);
        assert_eq!(
            result.required_checks,
            vec![
                "build".to_string(),
                "test".to_string(),
                "test-toy".to_string(),
                "test-kcir-toy".to_string()
            ]
        );
        assert!(
            result
                .reasons
                .contains(&"kernel_surface_changed".to_string())
        );
    }

    #[test]
    fn project_required_checks_docs_doctrine_surface_includes_doctrine_check() {
        let result = project_required_checks(&["specs/premath/draft/BIDIR-DESCENT.md".to_string()]);
        assert_eq!(result.required_checks, vec!["doctrine-check".to_string()]);
        assert!(result.docs_only);
        assert!(
            result
                .reasons
                .contains(&"docs_only_doctrine_surface_touched".to_string())
        );
    }
}
