use premath_surreal::build_surface;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.observation_semantics.v1";
const FAILURE_CLASS_SURFACE_MISSING: &str = "observation_surface_missing";
const FAILURE_CLASS_BUILD_FAILED: &str = "observation_surface_build_failed";
const FAILURE_CLASS_PAYLOAD_MISMATCH: &str = "observation_surface_projection_mismatch";
const FAILURE_CLASS_SCHEMA_MISMATCH: &str = "observation_surface_schema_mismatch";
const FAILURE_CLASS_KIND_MISMATCH: &str = "observation_surface_kind_mismatch";
const FAILURE_CLASS_SUMMARY_INVALID: &str = "observation_surface_summary_invalid";

const OBSERVATION_SCHEMA: i64 = 1;
const OBSERVATION_KIND: &str = "ci.observation.surface.v0";

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

fn load_json_object(path: &Path) -> Result<Map<String, Value>, String> {
    let raw =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value = serde_json::from_slice::<Value>(&raw)
        .map_err(|error| format!("failed to parse {} as JSON: {error}", path.display()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("expected object JSON: {}", path.display()))
}

fn validate_summary(summary: &Map<String, Value>) -> Result<(), String> {
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
    let mut coherence_needs_attention = false;
    if let Some(value) = coherence
        && !value.is_null()
    {
        let coherence_obj = value
            .as_object()
            .ok_or_else(|| "summary.coherence must be null or an object".to_string())?;
        let attention_reasons = coherence_obj
            .get("attentionReasons")
            .and_then(Value::as_array)
            .ok_or_else(|| "summary.coherence.attentionReasons must be a list".to_string())?;
        if !attention_reasons.iter().all(Value::is_string) {
            return Err(
                "summary.coherence.attentionReasons must contain string entries".to_string(),
            );
        }
        coherence_needs_attention = coherence_obj
            .get("needsAttention")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    }

    let expected_needs_attention =
        matches!(state, "rejected" | "error") || coherence_needs_attention;
    if needs_attention != expected_needs_attention {
        return Err(format!(
            "summary.needsAttention mismatch (expected={expected_needs_attention}, actual={needs_attention})"
        ));
    }

    Ok(())
}

fn fail(
    json_output: bool,
    failure_class: &str,
    message: String,
    surface_path: &Path,
    ciwitness_dir: &Path,
) -> ! {
    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": "rejected",
            "failureClasses": [failure_class],
            "details": {
                "surfacePath": surface_path.display().to_string(),
                "ciwitnessDir": ciwitness_dir.display().to_string(),
                "message": message
            }
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
            eprintln!("error: failed to render observation-semantics-check payload: {error}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        println!("[observation-semantics] FAIL ({message})");
    }
    std::process::exit(1);
}

pub fn run(
    repo_root: String,
    ciwitness_dir: String,
    surface: String,
    issues_path: String,
    json_output: bool,
) {
    let repo_root = resolve_repo_root(&repo_root);
    let ciwitness_dir = resolve_rel_path(&repo_root, &ciwitness_dir);
    let surface_path = resolve_rel_path(&repo_root, &surface);
    let issues_path = resolve_rel_path(&repo_root, &issues_path);

    if !surface_path.exists() {
        fail(
            json_output,
            FAILURE_CLASS_SURFACE_MISSING,
            format!("missing surface: {}", surface_path.display()),
            &surface_path,
            &ciwitness_dir,
        );
    }

    let actual_surface_obj = load_json_object(&surface_path).unwrap_or_else(|message| {
        fail(
            json_output,
            FAILURE_CLASS_SCHEMA_MISMATCH,
            message,
            &surface_path,
            &ciwitness_dir,
        )
    });
    let expected_surface = build_surface(&repo_root, &ciwitness_dir, Some(&issues_path))
        .unwrap_or_else(|error| {
            fail(
                json_output,
                FAILURE_CLASS_BUILD_FAILED,
                format!("failed to build observation surface: {error}"),
                &surface_path,
                &ciwitness_dir,
            )
        });
    let expected_surface_obj = serde_json::to_value(&expected_surface)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_else(|| {
            fail(
                json_output,
                FAILURE_CLASS_BUILD_FAILED,
                "build_surface produced non-object JSON value".to_string(),
                &surface_path,
                &ciwitness_dir,
            )
        });

    if actual_surface_obj != expected_surface_obj {
        fail(
            json_output,
            FAILURE_CLASS_PAYLOAD_MISMATCH,
            "surface payload mismatch: output is not a pure projection of current CI witness artifacts".to_string(),
            &surface_path,
            &ciwitness_dir,
        );
    }

    let actual_schema = actual_surface_obj
        .get("schema")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    if actual_schema != OBSERVATION_SCHEMA {
        fail(
            json_output,
            FAILURE_CLASS_SCHEMA_MISMATCH,
            format!(
                "surface.schema mismatch (expected={OBSERVATION_SCHEMA}, actual={actual_schema})"
            ),
            &surface_path,
            &ciwitness_dir,
        );
    }

    let actual_kind = actual_surface_obj
        .get("surfaceKind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if actual_kind != OBSERVATION_KIND {
        fail(
            json_output,
            FAILURE_CLASS_KIND_MISMATCH,
            format!(
                "surface.surfaceKind mismatch (expected={OBSERVATION_KIND:?}, actual={actual_kind:?})"
            ),
            &surface_path,
            &ciwitness_dir,
        );
    }

    let summary = actual_surface_obj
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| {
            fail(
                json_output,
                FAILURE_CLASS_SUMMARY_INVALID,
                "surface.summary must be an object".to_string(),
                &surface_path,
                &ciwitness_dir,
            )
        });

    if let Err(message) = validate_summary(&summary) {
        fail(
            json_output,
            FAILURE_CLASS_SUMMARY_INVALID,
            message,
            &surface_path,
            &ciwitness_dir,
        );
    }

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": "accepted",
            "failureClasses": [],
            "details": {
                "surfacePath": surface_path.display().to_string(),
                "ciwitnessDir": ciwitness_dir.display().to_string(),
                "issuesPath": issues_path.display().to_string()
            }
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
            eprintln!("error: failed to render observation-semantics-check payload: {error}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    println!(
        "[observation-semantics] OK (surface={}, ciwitness={})",
        surface_path.display(),
        ciwitness_dir.display()
    );
}
