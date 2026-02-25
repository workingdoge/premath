use regex::Regex;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHECK_KIND: &str = "ci.docs_coherence_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "docs_coherence_violation";

#[derive(Debug, Clone)]
struct DocsCoherenceSummary {
    capabilities: usize,
    baseline_tasks: usize,
    projection_checks: usize,
    doctrine_checks: usize,
}

fn parse_summary(stdout: &str) -> Option<DocsCoherenceSummary> {
    let re = Regex::new(
        r"^\[docs-coherence-check\] OK \(capabilities=(\d+), baselineTasks=(\d+), projectionChecks=(\d+), doctrineChecks=(\d+)\)$",
    )
    .ok()?;
    for line in stdout.lines().rev() {
        let trimmed = line.trim();
        let Some(captures) = re.captures(trimmed) else {
            continue;
        };
        let capabilities = captures.get(1)?.as_str().parse::<usize>().ok()?;
        let baseline_tasks = captures.get(2)?.as_str().parse::<usize>().ok()?;
        let projection_checks = captures.get(3)?.as_str().parse::<usize>().ok()?;
        let doctrine_checks = captures.get(4)?.as_str().parse::<usize>().ok()?;
        return Some(DocsCoherenceSummary {
            capabilities,
            baseline_tasks,
            projection_checks,
            doctrine_checks,
        });
    }
    None
}

fn collect_error_lines(stdout: &str, stderr: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("[error] ") {
            out.push(value.to_string());
        }
    }
    if out.is_empty() {
        for line in stderr.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    if out.is_empty() {
        for line in stdout.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn normalize_nonempty_lines(input: &str) -> Vec<String> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or(crate_dir.as_path())
        .to_path_buf()
}

fn resolve_path(path: &Path, repo_root: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if path.exists() {
        return path.to_path_buf();
    }
    repo_root.join(path)
}

fn ensure_path_exists(path: &Path, label: &str) {
    if !path.exists() {
        eprintln!(
            "[docs-coherence-check] ERROR: {label} missing: {}",
            path.display()
        );
        std::process::exit(2);
    }
}

pub fn run(repo_root: String, json_output: bool) {
    let workspace_root = workspace_root();
    let script_path = resolve_path(
        &PathBuf::from("tools/conformance/check_docs_coherence.py"),
        &workspace_root,
    );
    ensure_path_exists(&script_path, "checker script");
    let repo_root = resolve_path(&PathBuf::from(repo_root), &workspace_root);
    ensure_path_exists(&repo_root, "repo root");

    let mut command = Command::new("python3");
    command.arg(&script_path);
    command.arg("--repo-root");
    command.arg(&repo_root);

    let output = command.output().unwrap_or_else(|err| {
        eprintln!("[docs-coherence-check] ERROR: failed to execute checker script: {err}");
        std::process::exit(2);
    });

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let accepted = output.status.success();
    let result = if accepted { "accepted" } else { "rejected" };
    let failure_classes: Vec<&str> = if accepted {
        Vec::new()
    } else {
        vec![FAILURE_CLASS_VIOLATION]
    };
    let errors = if accepted {
        Vec::new()
    } else {
        collect_error_lines(&stdout, &stderr)
    };
    let summary = parse_summary(&stdout);

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": result,
            "failureClasses": failure_classes,
            "capabilities": summary.as_ref().map(|row| row.capabilities),
            "baselineTasks": summary.as_ref().map(|row| row.baseline_tasks),
            "projectionChecks": summary.as_ref().map(|row| row.projection_checks),
            "doctrineChecks": summary.as_ref().map(|row| row.doctrine_checks),
            "errors": errors,
            "stdoutLines": normalize_nonempty_lines(&stdout),
            "stderrLines": normalize_nonempty_lines(&stderr),
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render docs-coherence-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        if !stdout.is_empty() {
            print!("{stdout}");
            if !stdout.ends_with('\n') {
                println!();
            }
        }
        if !stderr.is_empty() {
            eprint!("{stderr}");
            if !stderr.ends_with('\n') {
                eprintln!();
            }
        }
    }

    if !accepted {
        std::process::exit(1);
    }
}
