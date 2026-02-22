use premath_coherence::RequiredWitnessError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const DELTA_SCHEMA: u32 = 1;
const DELTA_KIND: &str = "ci.required.delta.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequiredDeltaRequest {
    #[serde(default)]
    repo_root: Option<String>,
    #[serde(default)]
    from_ref: Option<String>,
    #[serde(default)]
    to_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequiredDeltaResult {
    schema: u32,
    delta_kind: String,
    changed_paths: Vec<String>,
    source: String,
    from_ref: Option<String>,
    to_ref: String,
}

fn emit_error(err: RequiredWitnessError) -> ! {
    eprintln!("{err}");
    std::process::exit(2);
}

fn clean(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    normalized
}

fn normalize_paths(paths: Vec<String>) -> Vec<String> {
    let mut out = BTreeSet::new();
    for path in paths {
        let normalized = normalize_path(&path);
        if !normalized.is_empty() {
            out.insert(normalized);
        }
    }
    out.into_iter().collect()
}

fn run_git(repo_root: &Path, args: &[&str]) -> Option<String> {
    let completed = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .ok()?;

    if !completed.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&completed.stdout)
            .trim()
            .to_string(),
    )
}

fn ref_exists(repo_root: &Path, reference: &str) -> bool {
    run_git(repo_root, &["rev-parse", "--verify", "--quiet", reference]).is_some()
}

fn detect_default_base_ref(repo_root: &Path) -> Option<String> {
    let env_base = clean(std::env::var("PREMATH_CI_BASE_REF").ok());
    let mut candidates: Vec<String> = Vec::new();
    if let Some(base_ref) = env_base {
        if base_ref.starts_with("origin/") {
            candidates.push(base_ref.clone());
            candidates.push(base_ref.trim_start_matches("origin/").to_string());
        } else {
            candidates.push(base_ref.clone());
            candidates.push(format!("origin/{base_ref}"));
        }
    }
    candidates.extend([
        "origin/main".to_string(),
        "main".to_string(),
        "origin/master".to_string(),
        "master".to_string(),
        "HEAD~1".to_string(),
    ]);
    candidates
        .into_iter()
        .find(|candidate| ref_exists(repo_root, candidate))
}

fn detect_default_head_ref() -> String {
    clean(std::env::var("PREMATH_CI_HEAD_REF").ok()).unwrap_or_else(|| "HEAD".to_string())
}

fn detect_changed_paths(
    repo_root: &Path,
    from_ref: Option<String>,
    to_ref: Option<String>,
) -> RequiredDeltaResult {
    let head_ref = clean(to_ref).unwrap_or_else(detect_default_head_ref);
    let base_ref = clean(from_ref).or_else(|| detect_default_base_ref(repo_root));

    let mut paths: Vec<String> = Vec::new();
    let mut source = "none".to_string();

    if let Some(base) = base_ref.as_deref() {
        let range = format!("{base}...{head_ref}");
        if let Some(output) = run_git(
            repo_root,
            &["diff", "--name-only", "--diff-filter=ACMR", &range],
        ) {
            for line in output.lines() {
                if !line.trim().is_empty() {
                    paths.push(line.trim().to_string());
                }
            }
            source = "git_diff".to_string();
        } else {
            source = "diff_failed".to_string();
        }
    }

    if let Some(staged) = run_git(
        repo_root,
        &["diff", "--name-only", "--cached", "--diff-filter=ACMR"],
    ) && !staged.is_empty()
    {
        for line in staged.lines() {
            if !line.trim().is_empty() {
                paths.push(line.trim().to_string());
            }
        }
        source = if source == "none" {
            "workspace".to_string()
        } else {
            format!("{source}+workspace")
        };
    }

    if let Some(worktree) = run_git(repo_root, &["diff", "--name-only", "--diff-filter=ACMR"])
        && !worktree.is_empty()
    {
        for line in worktree.lines() {
            if !line.trim().is_empty() {
                paths.push(line.trim().to_string());
            }
        }
        source = if source == "none" {
            "workspace".to_string()
        } else {
            format!("{source}+workspace")
        };
    }

    RequiredDeltaResult {
        schema: DELTA_SCHEMA,
        delta_kind: DELTA_KIND.to_string(),
        changed_paths: normalize_paths(paths),
        source,
        from_ref: base_ref,
        to_ref: head_ref,
    }
}

pub fn run(input: String, json_output: bool) {
    let input_path = PathBuf::from(input);
    let bytes = std::fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(RequiredWitnessError {
            failure_class: "required_delta_invalid".to_string(),
            message: format!(
                "failed to read required delta input {}: {err}",
                input_path.display()
            ),
        });
    });

    let request: RequiredDeltaRequest = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(RequiredWitnessError {
            failure_class: "required_delta_invalid".to_string(),
            message: format!(
                "failed to parse required delta input json {}: {err}",
                input_path.display()
            ),
        });
    });

    let repo_root = request
        .repo_root
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let result = detect_changed_paths(&repo_root, request.from_ref, request.to_ref);

    if json_output {
        let rendered = serde_json::to_string_pretty(&result).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_delta_invalid".to_string(),
                message: format!("failed to render required delta json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }

    println!("premath required-delta");
    println!("  Source: {}", result.source);
    println!("  From Ref: {:?}", result.from_ref);
    println!("  To Ref: {}", result.to_ref);
    println!("  Changed Paths: {}", result.changed_paths.len());
}
