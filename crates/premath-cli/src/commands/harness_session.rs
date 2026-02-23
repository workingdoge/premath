use crate::cli::HarnessSessionStateArg;
use crate::commands::harness_feature;
use chrono::{SecondsFormat, Utc};
use premath_bd::{MemoryStore, store_snapshot_ref};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const HARNESS_SESSION_SCHEMA: u32 = 1;
const HARNESS_SESSION_KIND: &str = "premath.harness.session.v1";
const HARNESS_BOOTSTRAP_KIND: &str = "premath.harness.bootstrap.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarnessSession {
    schema: u32,
    session_kind: String,
    session_id: String,
    state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issue_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    next_step: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    instruction_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    witness_refs: Vec<String>,
    started_at: String,
    updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stopped_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issues_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issues_snapshot_ref: Option<String>,
}

pub struct WriteArgs {
    pub path: String,
    pub session_id: Option<String>,
    pub state: HarnessSessionStateArg,
    pub issue_id: Option<String>,
    pub summary: Option<String>,
    pub next_step: Option<String>,
    pub instruction_refs: Vec<String>,
    pub witness_refs: Vec<String>,
    pub issues: String,
    pub json: bool,
}

pub fn run_read(path: String, json_output: bool) {
    let path_buf = PathBuf::from(path);
    let session = read_session_or_exit(&path_buf);
    if json_output {
        let payload = json!({
            "action": "harness-session.read",
            "path": path_buf.display().to_string(),
            "session": session
        });
        print_json(&payload);
        return;
    }

    println!("premath harness-session read");
    println!("  Session ID: {}", session.session_id);
    println!("  State: {}", session.state);
    if let Some(issue_id) = &session.issue_id {
        println!("  Issue: {}", issue_id);
    }
    if let Some(next_step) = &session.next_step {
        println!("  Next Step: {}", next_step);
    }
    println!("  Path: {}", path_buf.display());
}

pub fn run_write(args: WriteArgs) {
    let path_buf = PathBuf::from(args.path);
    let existing = read_session_if_exists_or_exit(&path_buf);
    let now = now_rfc3339();
    let issues_path = PathBuf::from(args.issues.trim());
    let issues_snapshot_ref = derive_issue_snapshot_ref(&issues_path);

    let issue_id = match args.issue_id {
        Some(raw) => clean_optional(raw),
        None => existing
            .as_ref()
            .and_then(|session| session.issue_id.clone()),
    };

    let summary = match args.summary {
        Some(raw) => clean_optional(raw),
        None => existing
            .as_ref()
            .and_then(|session| session.summary.clone()),
    };

    let next_step = match args.next_step {
        Some(raw) => clean_optional(raw),
        None => existing
            .as_ref()
            .and_then(|session| session.next_step.clone()),
    };

    let instruction_refs = if args.instruction_refs.is_empty() {
        existing
            .as_ref()
            .map(|session| session.instruction_refs.clone())
            .unwrap_or_default()
    } else {
        normalize_refs(args.instruction_refs)
    };

    let witness_refs = if args.witness_refs.is_empty() {
        existing
            .as_ref()
            .map(|session| session.witness_refs.clone())
            .unwrap_or_default()
    } else {
        normalize_refs(args.witness_refs)
    };

    let session_id = clean_optional_from_option(args.session_id)
        .or_else(|| existing.as_ref().map(|session| session.session_id.clone()))
        .unwrap_or_else(|| generate_session_id(issue_id.as_deref()));
    let state = state_value(&args.state).to_string();

    let started_at = existing
        .as_ref()
        .map(|session| session.started_at.clone())
        .unwrap_or_else(|| now.clone());
    let stopped_at = match args.state {
        HarnessSessionStateArg::Active => None,
        HarnessSessionStateArg::Stopped => Some(now.clone()),
    };

    let session = HarnessSession {
        schema: HARNESS_SESSION_SCHEMA,
        session_kind: HARNESS_SESSION_KIND.to_string(),
        session_id,
        state,
        issue_id,
        summary,
        next_step,
        instruction_refs,
        witness_refs,
        started_at,
        updated_at: now,
        stopped_at,
        issues_path: Some(issues_path.display().to_string()),
        issues_snapshot_ref,
    };
    write_session_or_exit(&path_buf, &session);

    if args.json {
        let payload = json!({
            "action": "harness-session.write",
            "path": path_buf.display().to_string(),
            "session": session
        });
        print_json(&payload);
        return;
    }

    println!("premath harness-session write");
    println!("  Session ID: {}", session.session_id);
    println!("  State: {}", session.state);
    if let Some(issue_id) = &session.issue_id {
        println!("  Issue: {}", issue_id);
    }
    println!("  Path: {}", path_buf.display());
}

pub fn run_bootstrap(path: String, feature_ledger: String, json_output: bool) {
    let path_buf = PathBuf::from(path);
    let session = read_session_or_exit(&path_buf);
    let feature_ledger_buf = PathBuf::from(feature_ledger);
    let feature_projection = harness_feature::project_next_feature(&feature_ledger_buf)
        .unwrap_or_else(|err| {
            emit_error(format!(
                "failed to project harness feature ledger {}: {err}",
                feature_ledger_buf.display()
            ))
        });
    let next_feature_id = feature_projection
        .as_ref()
        .and_then(|row| row.next_feature_id.clone());
    let feature_closure_complete = feature_projection.as_ref().map(|row| row.closure_complete);
    let feature_count = feature_projection.as_ref().map(|row| row.feature_count);
    let payload = json!({
        "action": "harness-session.bootstrap",
        "bootstrapKind": HARNESS_BOOTSTRAP_KIND,
        "sessionRef": path_buf.display().to_string(),
        "sessionId": session.session_id,
        "mode": if session.state == "stopped" { "resume" } else { "attach" },
        "resumeIssueId": session.issue_id,
        "nextStep": session.next_step,
        "summary": session.summary,
        "instructionRefs": session.instruction_refs,
        "witnessRefs": session.witness_refs,
        "issuesPath": session.issues_path,
        "issuesSnapshotRef": session.issues_snapshot_ref,
        "featureLedgerRef": feature_ledger_buf.display().to_string(),
        "nextFeatureId": next_feature_id,
        "featureClosureComplete": feature_closure_complete,
        "featureCount": feature_count,
        "startedAt": session.started_at,
        "updatedAt": session.updated_at,
        "stoppedAt": session.stopped_at
    });

    if json_output {
        print_json(&payload);
        return;
    }

    println!("premath harness-session bootstrap");
    println!("  Session Ref: {}", path_buf.display());
    println!(
        "  Mode: {}",
        if session.state == "stopped" {
            "resume"
        } else {
            "attach"
        }
    );
    if let Some(issue_id) = session.issue_id {
        println!("  Resume Issue: {}", issue_id);
    }
    if let Some(next_step) = session.next_step {
        println!("  Next Step: {}", next_step);
    }
    if let Some(next_feature_id) = payload["nextFeatureId"].as_str() {
        println!("  Next Feature: {}", next_feature_id);
    }
}

fn read_session_if_exists_or_exit(path: &Path) -> Option<HarnessSession> {
    if !path.exists() {
        return None;
    }
    Some(read_session_or_exit(path))
}

fn read_session_or_exit(path: &Path) -> HarnessSession {
    let bytes = fs::read(path).unwrap_or_else(|err| {
        emit_error(format!(
            "failed to read harness session artifact {}: {err}",
            path.display()
        ))
    });
    let session: HarnessSession = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(format!(
            "failed to parse harness session artifact {}: {err}",
            path.display()
        ))
    });
    validate_session(&session);
    session
}

fn validate_session(session: &HarnessSession) {
    if session.schema != HARNESS_SESSION_SCHEMA {
        emit_error(format!(
            "invalid harness session schema {} (expected {})",
            session.schema, HARNESS_SESSION_SCHEMA
        ));
    }
    if session.session_kind != HARNESS_SESSION_KIND {
        emit_error(format!(
            "invalid harness session kind {} (expected {})",
            session.session_kind, HARNESS_SESSION_KIND
        ));
    }
    if session.state != "active" && session.state != "stopped" {
        emit_error(format!(
            "invalid harness session state {} (expected active|stopped)",
            session.state
        ));
    }
    if session.session_id.trim().is_empty() {
        emit_error("invalid harness session id (empty)".to_string());
    }
}

fn write_session_or_exit(path: &Path, session: &HarnessSession) {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).unwrap_or_else(|err| {
            emit_error(format!(
                "failed to create harness session directory {}: {err}",
                parent.display()
            ))
        });
    }
    let rendered = serde_json::to_string_pretty(session)
        .unwrap_or_else(|err| emit_error(format!("failed to encode harness session json: {err}")));
    fs::write(path, format!("{rendered}\n")).unwrap_or_else(|err| {
        emit_error(format!(
            "failed to write harness session artifact {}: {err}",
            path.display()
        ))
    });
}

fn print_json(payload: &serde_json::Value) {
    let rendered = serde_json::to_string_pretty(payload)
        .unwrap_or_else(|err| emit_error(format!("failed to render json output: {err}")));
    println!("{rendered}");
}

fn derive_issue_snapshot_ref(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    let store = MemoryStore::load_jsonl(path).ok()?;
    Some(store_snapshot_ref(&store))
}

fn clean_optional(raw: String) -> Option<String> {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn clean_optional_from_option(raw: Option<String>) -> Option<String> {
    raw.and_then(clean_optional)
}

fn normalize_refs(raw_refs: Vec<String>) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for raw in raw_refs {
        if let Some(cleaned) = clean_optional(raw) {
            refs.insert(cleaned);
        }
    }
    refs.into_iter().collect()
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn session_token(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "session".to_string()
    } else {
        trimmed.to_string()
    }
}

fn generate_session_id(issue_id: Option<&str>) -> String {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    match issue_id {
        Some(issue_id) => format!("hs1_{}_{}", timestamp, session_token(issue_id)),
        None => format!("hs1_{}", timestamp),
    }
}

fn state_value(state: &HarnessSessionStateArg) -> &'static str {
    match state {
        HarnessSessionStateArg::Active => "active",
        HarnessSessionStateArg::Stopped => "stopped",
    }
}

fn emit_error(message: String) -> ! {
    eprintln!("error: {message}");
    std::process::exit(1);
}
