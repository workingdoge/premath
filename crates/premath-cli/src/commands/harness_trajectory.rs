use crate::cli::HarnessTrajectoryModeArg;
use chrono::{SecondsFormat, Utc};
use premath_surreal::{
    HARNESS_TRAJECTORY_KIND, HARNESS_TRAJECTORY_SCHEMA, HarnessTrajectoryRow,
    TrajectoryProjectionMode, append_trajectory_row, project_trajectory, read_trajectory_rows,
};
use serde_json::json;
use std::path::PathBuf;

pub struct AppendArgs {
    pub path: String,
    pub step_id: String,
    pub issue_id: Option<String>,
    pub action: String,
    pub result_class: String,
    pub instruction_refs: Vec<String>,
    pub witness_refs: Vec<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub json: bool,
}

pub fn run_append(args: AppendArgs) {
    let path_buf = PathBuf::from(args.path);
    let row = HarnessTrajectoryRow {
        schema: HARNESS_TRAJECTORY_SCHEMA,
        step_kind: HARNESS_TRAJECTORY_KIND.to_string(),
        step_id: args.step_id,
        issue_id: args.issue_id,
        action: args.action,
        result_class: args.result_class,
        instruction_refs: args.instruction_refs,
        witness_refs: args.witness_refs,
        started_at: args.started_at,
        finished_at: args.finished_at.unwrap_or_else(now_rfc3339),
    };
    let appended = append_trajectory_row(&path_buf, row).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to append harness trajectory row at {}: {err}",
            path_buf.display()
        );
        std::process::exit(1);
    });

    if args.json {
        let payload = json!({
            "action": "harness-trajectory.append",
            "path": path_buf.display().to_string(),
            "row": appended
        });
        print_json(&payload);
        return;
    }

    println!("premath harness-trajectory append");
    println!("  Step ID: {}", appended.step_id);
    println!("  Result Class: {}", appended.result_class);
    println!("  Path: {}", path_buf.display());
}

pub fn run_query(path: String, mode: HarnessTrajectoryModeArg, limit: usize, json_output: bool) {
    let path_buf = PathBuf::from(path);
    let rows = read_trajectory_rows(&path_buf).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read harness trajectory rows from {}: {err}",
            path_buf.display()
        );
        std::process::exit(1);
    });
    let mode_value = projection_mode(mode);
    let projection = project_trajectory(&rows, mode_value, limit);

    if json_output {
        let payload = json!({
            "action": "harness-trajectory.query",
            "path": path_buf.display().to_string(),
            "projectionKind": projection.projection_kind,
            "mode": projection.mode,
            "count": projection.count,
            "totalCount": projection.total_count,
            "failedCount": projection.failed_count,
            "retryNeededCount": projection.retry_needed_count,
            "items": projection.items
        });
        print_json(&payload);
        return;
    }

    println!("premath harness-trajectory query");
    println!("  Path: {}", path_buf.display());
    println!("  Mode: {}", projection.mode);
    println!(
        "  Count: {} (total={}, failed={}, retry_needed={})",
        projection.count,
        projection.total_count,
        projection.failed_count,
        projection.retry_needed_count
    );
    for row in projection.items {
        println!(
            "  - {} [{}] {} ({})",
            row.step_id, row.result_class, row.action, row.finished_at
        );
    }
}

fn projection_mode(mode: HarnessTrajectoryModeArg) -> TrajectoryProjectionMode {
    match mode {
        HarnessTrajectoryModeArg::Latest => TrajectoryProjectionMode::Latest,
        HarnessTrajectoryModeArg::Failed => TrajectoryProjectionMode::Failed,
        HarnessTrajectoryModeArg::RetryNeeded => TrajectoryProjectionMode::RetryNeeded,
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn print_json(payload: &serde_json::Value) {
    let rendered = serde_json::to_string_pretty(payload).unwrap_or_else(|err| {
        eprintln!("error: failed to render json output: {err}");
        std::process::exit(1);
    });
    println!("{rendered}");
}
