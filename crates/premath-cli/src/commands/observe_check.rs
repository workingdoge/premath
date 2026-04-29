//! Observation Surface projection-invariance checker.

use premath_surreal::{OBSERVATION_KIND, OBSERVATION_SCHEMA, ObservationSurface, build_surface};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Args {
    pub repo_root: String,
    pub ciwitness_dir: String,
    pub issues_path: String,
    pub surface: String,
    pub json: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Outcome<'a> {
    schema: u8,
    check_kind: &'a str,
    result: &'a str,
    surface: &'a str,
    ciwitness_dir: &'a str,
    findings: &'a [String],
}

pub fn run(args: Args) {
    let repo_root = resolve_repo_root(&args.repo_root);
    let ciwitness_dir = resolve_rel_path(&repo_root, &args.ciwitness_dir);
    let issues_path = resolve_rel_path(&repo_root, &args.issues_path);
    let surface_path = resolve_rel_path(&repo_root, &args.surface);

    let findings = check(&repo_root, &ciwitness_dir, &issues_path, &surface_path)
        .map_or_else(|err| vec![err], |()| Vec::new());
    let accepted = findings.is_empty();

    if args.json {
        let outcome = Outcome {
            schema: 1,
            check_kind: "ci.observation.semantic_projection.v1",
            result: if accepted { "accepted" } else { "rejected" },
            surface: &surface_path.display().to_string(),
            ciwitness_dir: &ciwitness_dir.display().to_string(),
            findings: &findings,
        };
        println!("{}", serde_json::to_string_pretty(&outcome).unwrap());
    } else if accepted {
        println!(
            "[observation-semantics] OK (surface={}, ciwitness={})",
            surface_path.display(),
            ciwitness_dir.display()
        );
    } else {
        println!("[observation-semantics] FAIL (findings={})", findings.len());
        for finding in &findings {
            println!("  - {finding}");
        }
    }

    if !accepted {
        std::process::exit(1);
    }
}

fn check(
    repo_root: &Path,
    ciwitness_dir: &Path,
    issues_path: &Path,
    surface_path: &Path,
) -> Result<(), String> {
    if !surface_path.exists() {
        return Err(format!("missing surface: {}", surface_path.display()));
    }

    let actual_bytes = fs::read(surface_path)
        .map_err(|err| format!("failed to read surface {}: {err}", surface_path.display()))?;
    let actual_value: Value = serde_json::from_slice(&actual_bytes)
        .map_err(|err| format!("failed to parse surface {}: {err}", surface_path.display()))?;
    if !actual_value.is_object() {
        return Err(format!("expected object JSON: {}", surface_path.display()));
    }

    let actual_surface = serde_json::from_value::<ObservationSurface>(actual_value.clone())
        .map_err(|err| format!("invalid observation surface shape: {err}"))?;
    validate_summary(&actual_value)?;

    if actual_surface.schema != OBSERVATION_SCHEMA {
        return Err(format!(
            "surface.schema mismatch (expected={OBSERVATION_SCHEMA}, actual={})",
            actual_surface.schema
        ));
    }
    if actual_surface.surface_kind != OBSERVATION_KIND {
        return Err(format!(
            "surface.surfaceKind mismatch (expected={OBSERVATION_KIND:?}, actual={:?})",
            actual_surface.surface_kind
        ));
    }

    let expected_surface = build_surface(repo_root, ciwitness_dir, Some(issues_path))
        .map_err(|err| format!("observe-build projection failed: {err}"))?;
    let expected_value = serde_json::to_value(expected_surface)
        .map_err(|err| format!("failed to render expected observation surface: {err}"))?;

    if actual_value != expected_value {
        return Err(
            "surface payload mismatch: output is not a pure projection of current CI witness artifacts"
                .to_string(),
        );
    }

    Ok(())
}

fn validate_summary(surface: &Value) -> Result<(), String> {
    let Some(summary) = surface.get("summary").and_then(Value::as_object) else {
        return Err("surface.summary must be an object".to_string());
    };

    let state = summary
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| "summary.state must be a string".to_string())?;
    if !matches!(
        state,
        "accepted" | "rejected" | "running" | "error" | "empty"
    ) {
        return Err(format!("invalid summary.state: {state:?}"));
    }

    let needs_attention = summary
        .get("needsAttention")
        .and_then(Value::as_bool)
        .ok_or_else(|| "summary.needsAttention must be a boolean".to_string())?;

    let coherence = summary.get("coherence");
    let coherence_needs_attention = match coherence {
        None | Some(Value::Null) => false,
        Some(Value::Object(map)) => {
            if !map
                .get("attentionReasons")
                .is_some_and(|value| value.is_array())
            {
                return Err("summary.coherence.attentionReasons must be a list".to_string());
            }
            map.get("needsAttention")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        }
        Some(_) => return Err("summary.coherence must be null or an object".to_string()),
    };

    let expected_needs_attention =
        matches!(state, "rejected" | "error") || coherence_needs_attention;
    if needs_attention != expected_needs_attention {
        return Err(format!(
            "summary.needsAttention mismatch (expected={expected_needs_attention}, actual={needs_attention})"
        ));
    }

    Ok(())
}

fn resolve_repo_root(input: &str) -> PathBuf {
    let path = PathBuf::from(input.trim());
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn resolve_rel_path(root: &Path, input: &str) -> PathBuf {
    let path = PathBuf::from(input.trim());
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_summary_accepts_coherence_attention() {
        let surface = json!({
            "summary": {
                "state": "accepted",
                "needsAttention": true,
                "coherence": {
                    "needsAttention": true,
                    "attentionReasons": ["dependency_cycle"]
                }
            }
        });

        validate_summary(&surface).expect("summary should validate");
    }

    #[test]
    fn validate_summary_rejects_needs_attention_drift() {
        let surface = json!({
            "summary": {
                "state": "rejected",
                "needsAttention": false,
                "coherence": {
                    "needsAttention": false,
                    "attentionReasons": []
                }
            }
        });

        let err = validate_summary(&surface).expect_err("summary should reject");
        assert!(err.contains("summary.needsAttention mismatch"));
    }
}
