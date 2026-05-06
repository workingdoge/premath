//! Repository hygiene checker for local/private surfaces.

use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

const FORBIDDEN_PREFIX_REASONS: &[(&str, &str)] = &[
    (".claude/", "private_agent_surface"),
    (".serena/", "private_agent_surface"),
    (".premath/cache/", "local_cache_surface"),
    (".premath/sessions/", "local_runtime_surface"),
];

const REQUIRED_GITIGNORE_ENTRIES: &[&str] = &[".claude/", ".serena/", ".premath/cache/"];

#[derive(Debug, Clone)]
struct Report {
    source: &'static str,
    scanned: usize,
    violations: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Outcome<'a> {
    schema: u8,
    check_kind: &'a str,
    result: &'a str,
    source: &'a str,
    scanned: usize,
    violations: &'a [String],
}

pub fn run(repo_root: String, paths: Vec<String>, json: bool) {
    match check(Path::new(&repo_root), &paths) {
        Ok(report) => emit(report, json),
        Err(err) => {
            let report = Report {
                source: "setup",
                scanned: 0,
                violations: vec![err],
            };
            emit(report, json);
            std::process::exit(1);
        }
    }
}

fn emit(report: Report, json: bool) {
    let accepted = report.violations.is_empty();
    if json {
        let outcome = Outcome {
            schema: 1,
            check_kind: "premath.repo_hygiene.v1",
            result: if accepted { "accepted" } else { "rejected" },
            source: report.source,
            scanned: report.scanned,
            violations: &report.violations,
        };
        println!("{}", serde_json::to_string_pretty(&outcome).unwrap());
    } else if accepted {
        println!(
            "[repo-hygiene] OK (source={}, scanned={})",
            report.source, report.scanned
        );
    } else {
        println!(
            "[repo-hygiene] FAIL (source={}, violations={})",
            report.source,
            report.violations.len()
        );
        for violation in &report.violations {
            println!("  - {violation}");
        }
    }

    if !accepted {
        std::process::exit(1);
    }
}

fn check(repo_root: &Path, paths: &[String]) -> Result<Report, String> {
    let repo_root = repo_root
        .canonicalize()
        .map_err(|err| format!("repo root not found: {} ({err})", repo_root.display()))?;

    let (scan_paths, source) = if paths.is_empty() {
        (list_tracked_paths(&repo_root)?, "git_index")
    } else {
        (
            paths
                .iter()
                .map(|path| normalize_path(path))
                .filter(|path| !path.is_empty())
                .collect(),
            "explicit_paths",
        )
    };

    let mut violations = check_paths(&scan_paths);
    let gitignore_path = repo_root.join(".gitignore");
    if !gitignore_path.exists() {
        violations.push(".gitignore: missing required file".to_string());
    } else {
        let text = fs::read_to_string(&gitignore_path)
            .map_err(|err| format!("failed to read {}: {err}", gitignore_path.display()))?;
        for entry in missing_required_gitignore_entries(&text) {
            violations.push(format!(
                ".gitignore: missing required ignore entry {entry:?}"
            ));
        }
    }

    Ok(Report {
        source,
        scanned: scan_paths.len(),
        violations,
    })
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
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
            return Some(*reason);
        }
    }
    None
}

fn parse_gitignore_entries(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn missing_required_gitignore_entries(text: &str) -> Vec<String> {
    let entries = parse_gitignore_entries(text);
    REQUIRED_GITIGNORE_ENTRIES
        .iter()
        .filter(|entry| !entries.contains(**entry))
        .map(|entry| (*entry).to_string())
        .collect()
}

fn list_tracked_paths(repo_root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["ls-files", "--cached", "-z"])
        .current_dir(repo_root)
        .output()
        .map_err(|err| format!("failed to run git ls-files: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git ls-files failed: {}", stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|item| !item.is_empty())
        .map(normalize_path)
        .collect())
}

fn check_paths(paths: &[String]) -> Vec<String> {
    let mut violations = Vec::new();
    for path in paths
        .iter()
        .map(|path| normalize_path(path))
        .collect::<BTreeSet<_>>()
    {
        if path.is_empty() {
            continue;
        }
        if let Some(reason) = classify_forbidden_path(&path) {
            violations.push(format!("{path}: {reason}"));
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_forbidden_path_prefixes() {
        assert_eq!(
            classify_forbidden_path(".claude/session.json"),
            Some("private_agent_surface")
        );
        assert_eq!(
            classify_forbidden_path(".serena/memory.md"),
            Some("private_agent_surface")
        );
        assert_eq!(
            classify_forbidden_path(".premath/cache/checker/cache.json"),
            Some("local_cache_surface")
        );
        assert_eq!(
            classify_forbidden_path("artifacts/witness/latest.json"),
            None
        );
        assert_eq!(
            classify_forbidden_path("specs/premath/draft/README.md"),
            None
        );
    }

    #[test]
    fn reports_missing_required_gitignore_entries() {
        let text = "
        .DS_Store
        .claude/
        # comment
        ";
        assert_eq!(
            missing_required_gitignore_entries(text),
            vec![".serena/".to_string(), ".premath/cache/".to_string()]
        );
    }

    #[test]
    fn reports_forbidden_entries() {
        let violations = check_paths(&[
            "specs/premath/draft/CHECKER-CLAIMS.md".to_string(),
            ".serena/memory.md".to_string(),
        ]);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("private_agent_surface"));
    }
}
