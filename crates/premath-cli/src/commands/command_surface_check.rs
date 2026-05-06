//! Repository command-surface hygiene checker.

use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

const MISE_SURFACE_PATHS: &[&str] = &[".envrc", "flake.nix"];

const SELF_PATH: &str = "crates/premath-cli/src/commands/command_surface_check.rs";

#[derive(Serialize)]
struct Outcome<'a> {
    schema: u8,
    check_kind: &'a str,
    result: &'a str,
    checked_files: usize,
    violations: &'a [String],
}

pub fn run(repo_root: String, json: bool) {
    match check(Path::new(&repo_root)) {
        Ok((checked_files, violations)) => emit(checked_files, violations, json),
        Err(err) => {
            let violations = vec![err];
            emit(0, violations, json);
            std::process::exit(1);
        }
    }
}

fn emit(checked_files: usize, violations: Vec<String>, json: bool) {
    let accepted = violations.is_empty();
    if json {
        let outcome = Outcome {
            schema: 1,
            check_kind: "premath.command_surface.v1",
            result: if accepted { "accepted" } else { "rejected" },
            checked_files,
            violations: &violations,
        };
        println!("{}", serde_json::to_string_pretty(&outcome).unwrap());
    } else if accepted {
        println!("[command-surface] OK (direct scripts/Nix)");
    } else {
        println!("[command-surface] FAIL (violations={})", violations.len());
        for violation in &violations {
            println!("  - {violation}");
        }
    }

    if !accepted {
        std::process::exit(1);
    }
}

fn check(repo_root: &Path) -> Result<(usize, Vec<String>), String> {
    let repo_root = repo_root
        .canonicalize()
        .map_err(|err| format!("repo root not found: {} ({err})", repo_root.display()))?;
    let mut violations = Vec::new();

    let justfile = repo_root.join("justfile");
    if justfile.exists() {
        violations.push(format!(
            "{}: expected removed (direct command surface)",
            justfile.display()
        ));
    }

    let mise_toml = repo_root.join(".mise.toml");
    if mise_toml.exists() {
        violations.push(format!(
            "{}: expected removed (direct command surface)",
            mise_toml.display()
        ));
    }

    let mut checked_files = 0usize;
    for rel in list_repo_files(&repo_root)? {
        if rel == SELF_PATH {
            continue;
        }
        let path = repo_root.join(&rel);
        if !path.is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        checked_files += 1;

        if MISE_SURFACE_PATHS.contains(&rel.as_str()) && has_mise_reference(&text) {
            violations.push(format!("{rel}: retired mise command surface reference"));
        }

        for (line_no, reason) in find_line_violations(&text) {
            violations.push(format!("{rel}:{line_no}: {reason}"));
        }
    }

    Ok((checked_files, violations))
}

fn list_repo_files(repo_root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .current_dir(repo_root)
        .output()
        .map_err(|err| format!("failed to run git ls-files: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn find_line_violations(text: &str) -> Vec<(usize, &'static str)> {
    let mut violations = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let stripped = line.trim_start();
        if stripped.starts_with("just ") || stripped.starts_with("$ just ") {
            violations.push((line_no, "command-style `just ...` usage"));
            continue;
        }
        if line.contains("`just ") {
            violations.push((line_no, "inline backtick `just ...` usage"));
            continue;
        }
        if has_nix_develop_just(line) {
            violations.push((line_no, "`nix develop -c just ...` usage"));
            continue;
        }
        if is_run_just(stripped) {
            violations.push((line_no, "workflow/task `run: just ...` usage"));
            continue;
        }
        if line.to_ascii_lowercase().contains("justfile") {
            violations.push((line_no, "`justfile` reference"));
        }
    }
    violations
}

fn has_nix_develop_just(line: &str) -> bool {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    tokens
        .windows(4)
        .any(|window| window == ["nix", "develop", "-c", "just"])
}

fn is_run_just(stripped: &str) -> bool {
    let Some(rest) = stripped.strip_prefix("run:") else {
        return false;
    };
    rest.trim_start().starts_with("just ")
}

fn has_mise_reference(text: &str) -> bool {
    text.contains("jdx/mise-action") || text.contains(".mise.toml") || contains_word(text, "mise")
}

fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(idx, _)| {
        let before = text[..idx].chars().next_back();
        let after = text[idx + word.len()..].chars().next();
        !is_word_char(before) && !is_word_char(after)
    })
}

fn is_word_char(ch: Option<char>) -> bool {
    ch.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}
