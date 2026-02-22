use crate::cli::HarnessFeatureStatusArg;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const HARNESS_FEATURE_LEDGER_SCHEMA: u32 = 1;
const HARNESS_FEATURE_LEDGER_KIND: &str = "premath.harness.feature_ledger.v1";
const HARNESS_FEATURE_LEDGER_CHECK_KIND: &str = "premath.harness.feature_ledger.check.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarnessFeatureLedger {
    schema: u32,
    ledger_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_ref: Option<String>,
    updated_at: String,
    #[serde(default)]
    features: Vec<HarnessFeatureRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarnessFeatureRow {
    feature_id: String,
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issue_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    instruction_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    verification_refs: Vec<String>,
    updated_at: String,
}

pub struct WriteArgs {
    pub path: String,
    pub feature_id: String,
    pub status: HarnessFeatureStatusArg,
    pub issue_id: Option<String>,
    pub summary: Option<String>,
    pub session_ref: Option<String>,
    pub instruction_refs: Vec<String>,
    pub verification_refs: Vec<String>,
    pub json: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerFinding {
    class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    feature_id: Option<String>,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerSummary {
    feature_count: usize,
    pending_count: usize,
    in_progress_count: usize,
    blocked_count: usize,
    completed_count: usize,
    closure_complete: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerCheckReport {
    check_kind: String,
    result: String,
    failure_classes: Vec<String>,
    errors: Vec<LedgerFinding>,
    summary: LedgerSummary,
    next_feature_id: Option<String>,
}

impl LedgerCheckReport {
    fn accepted(&self) -> bool {
        self.result == "accepted"
    }
}

#[derive(Debug, Clone)]
pub struct NextFeatureProjection {
    pub next_feature_id: Option<String>,
    pub closure_complete: bool,
    pub feature_count: usize,
}

pub fn run_read(path: String, json_output: bool) {
    let path_buf = PathBuf::from(path);
    let ledger = read_ledger_or_exit(&path_buf);

    if json_output {
        let payload = json!({
            "action": "harness-feature.read",
            "path": path_buf.display().to_string(),
            "ledger": ledger
        });
        print_json(&payload);
        return;
    }

    println!("premath harness-feature read");
    println!("  Path: {}", path_buf.display());
    println!("  Features: {}", ledger.features.len());
    if let Some(session_ref) = ledger.session_ref {
        println!("  Session Ref: {}", session_ref);
    }
}

pub fn run_write(args: WriteArgs) {
    let path_buf = PathBuf::from(args.path);
    let existing = read_ledger_if_exists_or_exit(&path_buf);
    let now = now_rfc3339();

    let mut ledger = existing.unwrap_or_else(|| HarnessFeatureLedger {
        schema: HARNESS_FEATURE_LEDGER_SCHEMA,
        ledger_kind: HARNESS_FEATURE_LEDGER_KIND.to_string(),
        session_ref: None,
        updated_at: now.clone(),
        features: Vec::new(),
    });

    if let Some(raw) = args.session_ref {
        ledger.session_ref = clean_optional(raw);
    }

    let feature_id = clean_required(args.feature_id, "feature-id");
    let status = status_value(&args.status).to_string();
    let issue_id_present = args.issue_id.is_some();
    let summary_present = args.summary.is_some();
    let issue_id = args.issue_id.and_then(clean_optional);
    let summary = args.summary.and_then(clean_optional);
    let instruction_refs = normalize_refs(args.instruction_refs);
    let verification_refs = normalize_refs(args.verification_refs);

    if let Some(row) = ledger
        .features
        .iter_mut()
        .find(|row| row.feature_id == feature_id)
    {
        row.status = status;
        if issue_id_present {
            row.issue_id = issue_id;
        }
        if summary_present {
            row.summary = summary;
        }
        if !instruction_refs.is_empty() {
            row.instruction_refs = instruction_refs;
        }
        if !verification_refs.is_empty() {
            row.verification_refs = verification_refs;
        }
        row.updated_at = now.clone();
    } else {
        ledger.features.push(HarnessFeatureRow {
            feature_id: feature_id.clone(),
            status,
            issue_id,
            summary,
            instruction_refs,
            verification_refs,
            updated_at: now.clone(),
        });
    }

    ledger
        .features
        .sort_by(|a, b| a.feature_id.cmp(&b.feature_id));
    ledger.updated_at = now;
    write_ledger_or_exit(&path_buf, &ledger);

    if args.json {
        let payload = json!({
            "action": "harness-feature.write",
            "path": path_buf.display().to_string(),
            "featureId": feature_id,
            "ledger": ledger
        });
        print_json(&payload);
        return;
    }

    println!("premath harness-feature write");
    println!("  Feature: {}", feature_id);
    println!("  Path: {}", path_buf.display());
}

pub fn run_check(path: String, require_closure: bool, json_output: bool) {
    let path_buf = PathBuf::from(path);
    let ledger = read_ledger_or_exit(&path_buf);
    let report = build_check_report(&ledger, require_closure);

    if json_output {
        let payload = json!({
            "action": "harness-feature.check",
            "path": path_buf.display().to_string(),
            "checkKind": report.check_kind,
            "result": report.result,
            "requireClosure": require_closure,
            "failureClasses": report.failure_classes,
            "errors": report.errors,
            "summary": report.summary,
            "nextFeatureId": report.next_feature_id
        });
        print_json(&payload);
    } else {
        println!(
            "[harness-feature] {} (features={}, closureComplete={}, nextFeature={})",
            if report.accepted() { "OK" } else { "FAIL" },
            report.summary.feature_count,
            report.summary.closure_complete,
            report.next_feature_id.as_deref().unwrap_or("none")
        );
        for finding in &report.errors {
            println!(
                "  - {}{} ({})",
                finding.class,
                finding
                    .feature_id
                    .as_ref()
                    .map(|id| format!(" feature={id}"))
                    .unwrap_or_default(),
                finding.message
            );
        }
    }

    if !report.accepted() {
        std::process::exit(1);
    }
}

pub fn run_next(path: String, json_output: bool) {
    let path_buf = PathBuf::from(path);
    let projection = project_next_feature(&path_buf).unwrap_or_else(|err| {
        emit_error(format!(
            "failed to compute next feature from {}: {err}",
            path_buf.display()
        ))
    });

    if json_output {
        let payload = json!({
            "action": "harness-feature.next",
            "path": path_buf.display().to_string(),
            "exists": projection.is_some(),
            "nextFeatureId": projection.as_ref().and_then(|row| row.next_feature_id.clone()),
            "closureComplete": projection.as_ref().map(|row| row.closure_complete),
            "featureCount": projection.as_ref().map(|row| row.feature_count)
        });
        print_json(&payload);
        return;
    }

    println!("premath harness-feature next");
    println!("  Path: {}", path_buf.display());
    if let Some(row) = projection {
        println!(
            "  Next Feature: {}",
            row.next_feature_id.unwrap_or_else(|| "none".to_string())
        );
        println!("  Closure Complete: {}", row.closure_complete);
        println!("  Feature Count: {}", row.feature_count);
    } else {
        println!("  Next Feature: none (ledger missing)");
    }
}

pub fn project_next_feature(path: &Path) -> Result<Option<NextFeatureProjection>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let ledger = read_ledger(path)?;
    let report = build_check_report(&ledger, false);
    if !report.accepted() {
        return Err(format!(
            "ledger rejected: {}",
            report.failure_classes.join(", ")
        ));
    }
    Ok(Some(NextFeatureProjection {
        next_feature_id: report.next_feature_id,
        closure_complete: report.summary.closure_complete,
        feature_count: report.summary.feature_count,
    }))
}

fn build_check_report(ledger: &HarnessFeatureLedger, require_closure: bool) -> LedgerCheckReport {
    let mut errors = Vec::new();

    if ledger.updated_at.trim().is_empty() {
        push_error(
            &mut errors,
            "harness_feature_ledger.updated_at.empty",
            None,
            "ledger.updatedAt must be non-empty",
        );
    }

    if ledger.features.is_empty() {
        push_error(
            &mut errors,
            "harness_feature_ledger.empty",
            None,
            "ledger.features must contain at least one row",
        );
    }

    let mut seen = HashSet::new();
    let mut pending_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut blocked_count = 0usize;
    let mut completed_count = 0usize;

    let mut pending_ids = Vec::new();
    let mut in_progress_ids = Vec::new();

    for row in &ledger.features {
        let feature_id = row.feature_id.trim().to_string();
        if feature_id.is_empty() {
            push_error(
                &mut errors,
                "harness_feature_ledger.feature_id.empty",
                None,
                "featureId must be non-empty",
            );
            continue;
        }

        if !seen.insert(feature_id.clone()) {
            push_error(
                &mut errors,
                "harness_feature_ledger.feature_id.duplicate",
                Some(feature_id.clone()),
                "featureId must be unique",
            );
        }

        if row.updated_at.trim().is_empty() {
            push_error(
                &mut errors,
                "harness_feature_ledger.feature.updated_at.empty",
                Some(feature_id.clone()),
                "feature.updatedAt must be non-empty",
            );
        }

        match row.status.as_str() {
            "pending" => {
                pending_count += 1;
                pending_ids.push(feature_id.clone());
            }
            "in_progress" => {
                in_progress_count += 1;
                in_progress_ids.push(feature_id.clone());
            }
            "blocked" => blocked_count += 1,
            "completed" => completed_count += 1,
            other => {
                push_error(
                    &mut errors,
                    "harness_feature_ledger.status.invalid",
                    Some(feature_id.clone()),
                    format!(
                        "feature status `{other}` is invalid (expected pending|in_progress|blocked|completed)"
                    ),
                );
            }
        }

        if row.status == "completed" && row.verification_refs.is_empty() {
            push_error(
                &mut errors,
                "harness_feature_ledger.completed_missing_verification_ref",
                Some(feature_id.clone()),
                "completed feature requires at least one verificationRef",
            );
        }

        check_ref_list(
            &mut errors,
            &feature_id,
            "harness_feature_ledger.instruction_ref.empty",
            &row.instruction_refs,
            "instructionRef entries must be non-empty",
        );
        check_ref_list(
            &mut errors,
            &feature_id,
            "harness_feature_ledger.verification_ref.empty",
            &row.verification_refs,
            "verificationRef entries must be non-empty",
        );
    }

    if in_progress_count > 1 {
        push_error(
            &mut errors,
            "harness_feature_ledger.in_progress.non_contractible",
            None,
            "at most one feature may be in_progress",
        );
    }

    in_progress_ids.sort();
    pending_ids.sort();
    let next_feature_id = if let Some(id) = in_progress_ids.first() {
        Some(id.clone())
    } else {
        pending_ids.first().cloned()
    };

    let closure_complete = errors.is_empty()
        && !ledger.features.is_empty()
        && completed_count == ledger.features.len();
    if require_closure && !closure_complete {
        push_error(
            &mut errors,
            "harness_feature_ledger.closure_incomplete",
            None,
            "require-closure set but ledger is not closed",
        );
    }

    let failure_classes = errors
        .iter()
        .map(|finding| finding.class.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let result = if errors.is_empty() {
        "accepted"
    } else {
        "rejected"
    }
    .to_string();

    LedgerCheckReport {
        check_kind: HARNESS_FEATURE_LEDGER_CHECK_KIND.to_string(),
        result,
        failure_classes,
        errors,
        summary: LedgerSummary {
            feature_count: ledger.features.len(),
            pending_count,
            in_progress_count,
            blocked_count,
            completed_count,
            closure_complete,
        },
        next_feature_id,
    }
}

fn check_ref_list(
    errors: &mut Vec<LedgerFinding>,
    feature_id: &str,
    class: &str,
    refs: &[String],
    message: &str,
) {
    for value in refs {
        if value.trim().is_empty() {
            push_error(
                errors,
                class,
                Some(feature_id.to_string()),
                message.to_string(),
            );
            break;
        }
    }
}

fn push_error(
    errors: &mut Vec<LedgerFinding>,
    class: &str,
    feature_id: Option<String>,
    message: impl Into<String>,
) {
    errors.push(LedgerFinding {
        class: class.to_string(),
        feature_id,
        message: message.into(),
    });
}

fn read_ledger_if_exists_or_exit(path: &Path) -> Option<HarnessFeatureLedger> {
    if !path.exists() {
        return None;
    }
    Some(read_ledger_or_exit(path))
}

fn read_ledger_or_exit(path: &Path) -> HarnessFeatureLedger {
    read_ledger(path).unwrap_or_else(|err| {
        emit_error(format!(
            "failed to read harness feature ledger {}: {err}",
            path.display()
        ))
    })
}

fn read_ledger(path: &Path) -> Result<HarnessFeatureLedger, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("unable to read {}: {err}", path.display()))?;
    let ledger: HarnessFeatureLedger = serde_json::from_slice(&bytes)
        .map_err(|err| format!("unable to parse {}: {err}", path.display()))?;
    validate_ledger_header(&ledger)?;
    Ok(ledger)
}

fn validate_ledger_header(ledger: &HarnessFeatureLedger) -> Result<(), String> {
    if ledger.schema != HARNESS_FEATURE_LEDGER_SCHEMA {
        return Err(format!(
            "invalid schema {} (expected {})",
            ledger.schema, HARNESS_FEATURE_LEDGER_SCHEMA
        ));
    }
    if ledger.ledger_kind != HARNESS_FEATURE_LEDGER_KIND {
        return Err(format!(
            "invalid ledger kind {} (expected {})",
            ledger.ledger_kind, HARNESS_FEATURE_LEDGER_KIND
        ));
    }
    Ok(())
}

fn write_ledger_or_exit(path: &Path, ledger: &HarnessFeatureLedger) {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).unwrap_or_else(|err| {
            emit_error(format!(
                "failed to create harness feature ledger directory {}: {err}",
                parent.display()
            ))
        });
    }
    let rendered = serde_json::to_string_pretty(ledger)
        .unwrap_or_else(|err| emit_error(format!("failed to encode feature ledger json: {err}")));
    fs::write(path, format!("{rendered}\n")).unwrap_or_else(|err| {
        emit_error(format!(
            "failed to write harness feature ledger {}: {err}",
            path.display()
        ))
    });
}

fn print_json(payload: &serde_json::Value) {
    let rendered = serde_json::to_string_pretty(payload)
        .unwrap_or_else(|err| emit_error(format!("failed to render json output: {err}")));
    println!("{rendered}");
}

fn clean_required(raw: String, field_name: &str) -> String {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        emit_error(format!("{field_name} must be non-empty"));
    }
    trimmed
}

fn clean_optional(raw: String) -> Option<String> {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
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

fn status_value(status: &HarnessFeatureStatusArg) -> &'static str {
    match status {
        HarnessFeatureStatusArg::Pending => "pending",
        HarnessFeatureStatusArg::InProgress => "in_progress",
        HarnessFeatureStatusArg::Blocked => "blocked",
        HarnessFeatureStatusArg::Completed => "completed",
    }
}

fn emit_error(message: String) -> ! {
    eprintln!("error: {message}");
    std::process::exit(1);
}
