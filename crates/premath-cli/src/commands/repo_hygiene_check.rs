use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHECK_KIND: &str = "ci.repo_hygiene_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "repo_hygiene_violation";

const FORBIDDEN_PREFIX_REASONS: [(&str, &str); 6] = [
    (".claude/", "private_agent_surface"),
    (".serena/", "private_agent_surface"),
    (".premath/cache/", "local_cache_surface"),
    (".premath/sessions/", "local_runtime_surface"),
    ("artifacts/ciwitness/", "ephemeral_ci_artifact_surface"),
    ("artifacts/observation/", "ephemeral_ci_artifact_surface"),
];

const REQUIRED_GITIGNORE_ENTRIES: [&str; 3] = [".claude/", ".serena/", ".premath/cache/"];

fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized.replacen("./", "", 1);
    }
    normalized
}

fn normalize_path_with_root(path: &str, repo_root: &Path) -> String {
    let normalized = normalize_path(path);
    let candidate = PathBuf::from(&normalized);
    if candidate.is_absolute()
        && let Ok(stripped) = candidate.strip_prefix(repo_root)
    {
        return normalize_path(&stripped.to_string_lossy());
    }
    normalized
}

fn classify_forbidden_path(path: &str) -> Option<&'static str> {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        return None;
    }
    for (prefix, reason) in FORBIDDEN_PREFIX_REASONS {
        let anchor = prefix.trim_end_matches('/');
        if normalized == anchor || normalized.starts_with(prefix) {
            return Some(reason);
        }
    }
    None
}

fn parse_gitignore_entries(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        out.insert(line.to_string());
    }
    out
}

fn missing_required_gitignore_entries(text: &str) -> Vec<String> {
    let entries = parse_gitignore_entries(text);
    REQUIRED_GITIGNORE_ENTRIES
        .iter()
        .filter(|entry| !entries.contains(**entry))
        .map(|entry| entry.to_string())
        .collect()
}

fn list_tracked_paths(repo_root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["ls-files", "--cached", "-z"])
        .current_dir(repo_root)
        .output()
        .map_err(|err| format!("failed to execute git ls-files: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed with status {}",
            output.status.code().unwrap_or(1)
        ));
    }
    let stdout = output.stdout;
    if stdout.is_empty() {
        return Ok(Vec::new());
    }
    let decoded = String::from_utf8(stdout)
        .map_err(|err| format!("git ls-files produced non-utf8 output: {err}"))?;
    Ok(decoded
        .split('\0')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(normalize_path)
        .collect())
}

pub fn run(repo_root: String, paths: Vec<String>, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let (scan_paths, source) = if paths.is_empty() {
        (
            list_tracked_paths(&repo_root).unwrap_or_else(|err| {
                eprintln!("error: {err}");
                std::process::exit(2);
            }),
            "git_index",
        )
    } else {
        (
            paths
                .iter()
                .map(|path| normalize_path_with_root(path, &repo_root))
                .filter(|path| !path.is_empty())
                .collect::<Vec<_>>(),
            "explicit_paths",
        )
    };

    let mut violations = BTreeSet::new();
    for path in &scan_paths {
        if let Some(reason) = classify_forbidden_path(path) {
            violations.insert(format!("{path}: {reason}"));
        }
    }

    let gitignore_path = repo_root.join(".gitignore");
    if !gitignore_path.exists() {
        violations.insert(".gitignore: missing required file".to_string());
    } else {
        let text = fs::read_to_string(&gitignore_path).unwrap_or_else(|err| {
            eprintln!("error: failed reading {}: {err}", gitignore_path.display());
            std::process::exit(2);
        });
        for missing in missing_required_gitignore_entries(&text) {
            violations.insert(format!(
                ".gitignore: missing required ignore entry {missing:?}"
            ));
        }
    }

    let violations = violations.into_iter().collect::<Vec<_>>();
    let result = if violations.is_empty() {
        "accepted"
    } else {
        "rejected"
    };
    let failure_classes: Vec<&str> = if violations.is_empty() {
        Vec::new()
    } else {
        vec![FAILURE_CLASS_VIOLATION]
    };

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": result,
            "failureClasses": failure_classes,
            "source": source,
            "scanned": scan_paths.len(),
            "violations": violations,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render repo-hygiene-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if violations.is_empty() {
        println!(
            "[repo-hygiene] OK (source={source}, scanned={})",
            scan_paths.len()
        );
    } else {
        println!(
            "[repo-hygiene] FAIL (source={source}, violations={})",
            violations.len()
        );
        for row in &violations {
            println!("  - {row}");
        }
    }

    if !violations.is_empty() {
        std::process::exit(1);
    }
}
