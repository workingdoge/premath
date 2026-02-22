use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use thiserror::Error;

pub const HARNESS_TRAJECTORY_SCHEMA: u64 = 1;
pub const HARNESS_TRAJECTORY_KIND: &str = "premath.harness.step.v1";
pub const TRAJECTORY_PROJECTION_KIND: &str = "premath.harness.trajectory.projection.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarnessTrajectoryRow {
    pub schema: u64,
    pub step_kind: String,
    pub step_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
    pub action: String,
    pub result_class: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instruction_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub witness_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    pub finished_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrajectoryProjectionMode {
    Latest,
    Failed,
    RetryNeeded,
}

impl TrajectoryProjectionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Latest => "latest",
            Self::Failed => "failed",
            Self::RetryNeeded => "retry_needed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarnessTrajectoryProjection {
    pub projection_kind: String,
    pub mode: String,
    pub count: u64,
    pub total_count: u64,
    pub failed_count: u64,
    pub retry_needed_count: u64,
    pub items: Vec<HarnessTrajectoryRow>,
}

#[derive(Debug, Error)]
pub enum TrajectoryError {
    #[error("failed to read/write trajectory: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse trajectory row: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid trajectory row: {0}")]
    Invalid(String),
}

pub fn read_trajectory_rows(path: &Path) -> Result<Vec<HarnessTrajectoryRow>, TrajectoryError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    if !path.is_file() {
        return Err(TrajectoryError::Invalid(format!(
            "trajectory path is not a file: {}",
            path.display()
        )));
    }

    let contents = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<HarnessTrajectoryRow>(line)?;
        let normalized = normalize_row(row).map_err(|message| {
            TrajectoryError::Invalid(format!(
                "{message} (path={}, line={})",
                path.display(),
                line_index + 1
            ))
        })?;
        rows.push(normalized);
    }
    Ok(rows)
}

pub fn append_trajectory_row(
    path: &Path,
    row: HarnessTrajectoryRow,
) -> Result<HarnessTrajectoryRow, TrajectoryError> {
    let normalized = normalize_row(row).map_err(TrajectoryError::Invalid)?;

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(&normalized)?;
    writeln!(file, "{line}")?;
    Ok(normalized)
}

pub fn project_trajectory(
    rows: &[HarnessTrajectoryRow],
    mode: TrajectoryProjectionMode,
    limit: usize,
) -> HarnessTrajectoryProjection {
    let mut normalized_rows = rows
        .iter()
        .filter_map(|row| normalize_row(row.clone()).ok())
        .collect::<Vec<_>>();
    sort_rows_desc(&mut normalized_rows);

    let total_count = normalized_rows.len() as u64;
    let failed_count = normalized_rows
        .iter()
        .filter(|row| is_failed_class(&row.result_class))
        .count() as u64;
    let retry_needed_count = normalized_rows
        .iter()
        .filter(|row| is_retry_needed_class(&row.result_class))
        .count() as u64;

    let filtered = match mode {
        TrajectoryProjectionMode::Latest => normalized_rows,
        TrajectoryProjectionMode::Failed => normalized_rows
            .into_iter()
            .filter(|row| is_failed_class(&row.result_class))
            .collect(),
        TrajectoryProjectionMode::RetryNeeded => normalized_rows
            .into_iter()
            .filter(|row| is_retry_needed_class(&row.result_class))
            .collect(),
    };

    let items = filtered.into_iter().take(limit).collect::<Vec<_>>();
    HarnessTrajectoryProjection {
        projection_kind: TRAJECTORY_PROJECTION_KIND.to_string(),
        mode: mode.as_str().to_string(),
        count: items.len() as u64,
        total_count,
        failed_count,
        retry_needed_count,
        items,
    }
}

fn normalize_row(mut row: HarnessTrajectoryRow) -> Result<HarnessTrajectoryRow, String> {
    if row.schema != HARNESS_TRAJECTORY_SCHEMA {
        return Err(format!(
            "schema mismatch (expected={HARNESS_TRAJECTORY_SCHEMA}, actual={})",
            row.schema
        ));
    }
    if row.step_kind.trim() != HARNESS_TRAJECTORY_KIND {
        return Err(format!(
            "stepKind mismatch (expected={HARNESS_TRAJECTORY_KIND}, actual={})",
            row.step_kind
        ));
    }
    row.step_kind = HARNESS_TRAJECTORY_KIND.to_string();
    row.step_id = clean_required(row.step_id, "stepId")?;
    row.issue_id = row.issue_id.and_then(clean_optional);
    row.action = clean_required(row.action, "action")?;
    row.result_class = clean_required(row.result_class, "resultClass")?;
    row.instruction_refs = normalize_refs(row.instruction_refs);
    row.witness_refs = normalize_refs(row.witness_refs);
    row.started_at = row.started_at.and_then(clean_optional);
    row.finished_at = clean_required(row.finished_at, "finishedAt")?;
    parse_rfc3339(&row.finished_at)
        .ok_or_else(|| format!("finishedAt must be RFC3339: {}", row.finished_at))?;
    if let Some(started_at) = row.started_at.as_deref() {
        parse_rfc3339(started_at)
            .ok_or_else(|| format!("startedAt must be RFC3339: {started_at}"))?;
    }
    Ok(row)
}

fn clean_required(value: String, name: &str) -> Result<String, String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(format!("{name} must be non-empty"));
    }
    Ok(trimmed)
}

fn clean_optional(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_refs(values: Vec<String>) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for value in values {
        if let Some(cleaned) = clean_optional(value) {
            refs.insert(cleaned);
        }
    }
    refs.into_iter().collect()
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn sort_rows_desc(rows: &mut [HarnessTrajectoryRow]) {
    rows.sort_by(|left, right| compare_rows_desc(left, right));
}

fn compare_rows_desc(left: &HarnessTrajectoryRow, right: &HarnessTrajectoryRow) -> Ordering {
    let left_time = parse_rfc3339(&left.finished_at);
    let right_time = parse_rfc3339(&right.finished_at);
    match (left_time, right_time) {
        (Some(l), Some(r)) => r
            .cmp(&l)
            .then_with(|| left.step_id.cmp(&right.step_id))
            .then_with(|| left.action.cmp(&right.action)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => right
            .finished_at
            .cmp(&left.finished_at)
            .then_with(|| left.step_id.cmp(&right.step_id))
            .then_with(|| left.action.cmp(&right.action)),
    }
}

fn normalize_result_class(value: &str) -> String {
    value.trim().to_lowercase().replace('-', "_")
}

fn is_failed_class(value: &str) -> bool {
    !matches!(
        normalize_result_class(value).as_str(),
        "accepted" | "verified_accept" | "completed" | "success" | "ok" | "passed"
    )
}

fn is_retry_needed_class(value: &str) -> bool {
    let normalized = normalize_result_class(value);
    normalized.contains("retry")
        || normalized.contains("transient")
        || normalized.contains("timeout")
        || normalized.contains("network")
        || normalized.contains("rate_limit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn row(step_id: &str, result_class: &str, finished_at: &str) -> HarnessTrajectoryRow {
        HarnessTrajectoryRow {
            schema: HARNESS_TRAJECTORY_SCHEMA,
            step_kind: HARNESS_TRAJECTORY_KIND.to_string(),
            step_id: step_id.to_string(),
            issue_id: Some("bd-1".to_string()),
            action: "apply.patch".to_string(),
            result_class: result_class.to_string(),
            instruction_refs: vec![
                "instructions/a.json".to_string(),
                "instructions/a.json".to_string(),
            ],
            witness_refs: vec!["artifacts/ciwitness/w1.json".to_string()],
            started_at: Some("2026-02-22T01:00:00Z".to_string()),
            finished_at: finished_at.to_string(),
        }
    }

    #[test]
    fn append_read_roundtrip_normalizes() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "premath-surreal-trajectory-{unique}-{}",
            std::process::id()
        ));
        let path = root.join(".premath/harness_trajectory.jsonl");

        let appended = append_trajectory_row(
            &path,
            HarnessTrajectoryRow {
                schema: HARNESS_TRAJECTORY_SCHEMA,
                step_kind: HARNESS_TRAJECTORY_KIND.to_string(),
                step_id: "step-1".to_string(),
                issue_id: Some(" bd-1 ".to_string()),
                action: " apply.patch ".to_string(),
                result_class: " accepted ".to_string(),
                instruction_refs: vec!["instructions/a.json".to_string(), "".to_string()],
                witness_refs: vec![
                    "artifacts/ciwitness/w1.json".to_string(),
                    "artifacts/ciwitness/w1.json".to_string(),
                ],
                started_at: Some("2026-02-22T01:00:00Z".to_string()),
                finished_at: "2026-02-22T01:01:00Z".to_string(),
            },
        )
        .expect("row should append");

        assert_eq!(appended.action, "apply.patch");
        assert_eq!(appended.result_class, "accepted");
        assert_eq!(appended.issue_id.as_deref(), Some("bd-1"));
        assert_eq!(appended.witness_refs.len(), 1);
        assert_eq!(appended.instruction_refs.len(), 1);

        let rows = read_trajectory_rows(&path).expect("rows should load");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], appended);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn projection_modes_are_deterministic() {
        let rows = vec![
            row("step-a", "accepted", "2026-02-22T01:01:00Z"),
            row("step-b", "transient_failure", "2026-02-22T01:03:00Z"),
            row("step-c", "policy_reject", "2026-02-22T01:02:00Z"),
        ];

        let latest = project_trajectory(&rows, TrajectoryProjectionMode::Latest, 2);
        assert_eq!(latest.count, 2);
        assert_eq!(latest.total_count, 3);
        assert_eq!(latest.failed_count, 2);
        assert_eq!(latest.retry_needed_count, 1);
        assert_eq!(latest.items[0].step_id, "step-b");
        assert_eq!(latest.items[1].step_id, "step-c");

        let failed = project_trajectory(&rows, TrajectoryProjectionMode::Failed, 10);
        assert_eq!(failed.count, 2);
        assert_eq!(failed.items[0].step_id, "step-b");
        assert_eq!(failed.items[1].step_id, "step-c");

        let retry = project_trajectory(&rows, TrajectoryProjectionMode::RetryNeeded, 10);
        assert_eq!(retry.count, 1);
        assert_eq!(retry.items[0].step_id, "step-b");
    }

    #[test]
    fn invalid_row_rejected() {
        let err = append_trajectory_row(
            Path::new("/tmp/ignore"),
            HarnessTrajectoryRow {
                schema: HARNESS_TRAJECTORY_SCHEMA,
                step_kind: HARNESS_TRAJECTORY_KIND.to_string(),
                step_id: String::new(),
                issue_id: None,
                action: "apply.patch".to_string(),
                result_class: "accepted".to_string(),
                instruction_refs: Vec::new(),
                witness_refs: Vec::new(),
                started_at: None,
                finished_at: "bad-time".to_string(),
            },
        )
        .expect_err("row should fail");
        assert!(matches!(err, TrajectoryError::Invalid(_)));
    }
}
