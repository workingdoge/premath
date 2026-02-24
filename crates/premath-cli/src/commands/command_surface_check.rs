use regex::{Regex, RegexBuilder};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHECK_KIND: &str = "ci.command_surface_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "command_surface_violation";

fn list_repo_files(repo_root: &Path) -> Result<Vec<PathBuf>, String> {
    let primary = Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .current_dir(repo_root)
        .output()
        .map_err(|err| format!("failed to execute git ls-files: {err}"))?;

    let output = if primary.status.success() {
        primary
    } else {
        Command::new("git")
            .args([
                "--git-dir=.git",
                "--work-tree=.",
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
            ])
            .current_dir(repo_root)
            .output()
            .map_err(|err| format!("failed to execute fallback git ls-files: {err}"))?
    };

    if !output.status.success() {
        return Err(format!(
            "git ls-files failed with status {}",
            output.status.code().unwrap_or(1)
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("git ls-files produced non-utf8 output: {err}"))?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| repo_root.join(line))
        .collect())
}

fn should_skip_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.ends_with("crates/premath-cli/src/commands/command_surface_check.rs")
}

fn collect_violations(repo_root: &Path) -> Result<Vec<String>, String> {
    let inline_just_re = Regex::new(r"`just\s+[^`]+`")
        .map_err(|err| format!("failed compiling inline just regex: {err}"))?;
    let nix_just_re = Regex::new(r"\bnix\s+develop\s+-c\s+just\b")
        .map_err(|err| format!("failed compiling nix just regex: {err}"))?;
    let run_just_re = Regex::new(r"\brun:\s*just\s+\S+")
        .map_err(|err| format!("failed compiling run just regex: {err}"))?;
    let justfile_word_re = RegexBuilder::new(r"\bjustfile\b")
        .case_insensitive(true)
        .build()
        .map_err(|err| format!("failed compiling justfile regex: {err}"))?;

    let mut violations = Vec::new();

    let justfile = repo_root.join("justfile");
    if justfile.exists() {
        violations.push(format!(
            "{}: expected removed (mise-only command surface)",
            justfile.display()
        ));
    }

    for path in list_repo_files(repo_root)? {
        if should_skip_path(&path) || !path.is_file() {
            continue;
        }

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::InvalidData {
                    continue;
                }
                return Err(format!("failed reading {}: {err}", path.display()));
            }
        };

        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(path.as_path())
            .display()
            .to_string();
        for (idx, line) in text.lines().enumerate() {
            let stripped = line.trim_start();
            let reason = if stripped.starts_with("just ") || stripped.starts_with("$ just ") {
                Some("command-style `just ...` usage")
            } else if inline_just_re.is_match(line) {
                Some("inline backtick `just ...` usage")
            } else if nix_just_re.is_match(line) {
                Some("`nix develop -c just ...` usage")
            } else if run_just_re.is_match(line) {
                Some("workflow/task `run: just ...` usage")
            } else if justfile_word_re.is_match(line) {
                Some("`justfile` reference")
            } else {
                None
            };
            if let Some(reason) = reason {
                violations.push(format!("{}:{}: {}", rel, idx + 1, reason));
            }
        }
    }

    Ok(violations)
}

pub fn run(repo_root: String, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let violations = collect_violations(&repo_root).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(2);
    });
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
            "violations": violations,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render command-surface-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if violations.is_empty() {
        println!("[command-surface] OK (mise-only)");
    } else {
        println!("[command-surface] FAIL (violations={})", violations.len());
        for row in &violations {
            println!("  - {row}");
        }
    }

    if !violations.is_empty() {
        std::process::exit(1);
    }
}
