use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.ci_wiring_check.v1";
const FAILURE_CLASS_UNBOUND: &str = "ci_wiring_unbound";
const FAILURE_CLASS_PARITY_DRIFT: &str = "ci_wiring_parity_drift";

const DEFAULT_REQUIRED_ENTRYPOINT: &[&str] = &["python3", "tools/ci/pipeline_required.py"];
const FORBIDDEN_PATTERNS: [(&str, &str); 5] = [
    (
        "legacy split-step required gate command",
        r"^\s*run:\s*mise run ci-required\s*$",
    ),
    (
        "legacy split-step strict verification command",
        r"^\s*run:\s*mise run ci-verify-required-strict\s*$",
    ),
    (
        "legacy split-step decision command",
        r"^\s*run:\s*mise run ci-decide-required\s*$",
    ),
    (
        "legacy split-step decision verification command",
        r"^\s*run:\s*mise run ci-verify-decision\s*$",
    ),
    (
        "legacy attested task workflow command",
        r"^\s*run:\s*mise run ci-required-attested\s*$",
    ),
];

fn token_pattern(token: &str) -> String {
    if token.starts_with('$') {
        format!("\"?{}\"?", regex::escape(token))
    } else {
        regex::escape(token)
    }
}

fn entrypoint_pattern(tokens: &[String]) -> Result<Regex, String> {
    let rendered = tokens
        .iter()
        .map(|token| token_pattern(token))
        .collect::<Vec<_>>()
        .join(r"\s+");
    RegexBuilder::new(&format!(r"^\s*run:\s*{}\s*$", rendered))
        .multi_line(true)
        .build()
        .map_err(|err| format!("failed compiling entrypoint regex: {err}"))
}

fn render_entrypoint(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| {
            if token.starts_with('$') {
                format!("\"{}\"", token)
            } else {
                token.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_required_entrypoint(contract_path: &Path) -> Vec<String> {
    let bytes = match fs::read(contract_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return DEFAULT_REQUIRED_ENTRYPOINT
                .iter()
                .map(|s| s.to_string())
                .collect();
        }
    };
    let payload: Value = match serde_json::from_slice(&bytes) {
        Ok(payload) => payload,
        Err(_) => {
            return DEFAULT_REQUIRED_ENTRYPOINT
                .iter()
                .map(|s| s.to_string())
                .collect();
        }
    };
    payload
        .get("pipelineWrapperSurface")
        .and_then(|row| row.get("requiredPipelineEntrypoint"))
        .and_then(Value::as_array)
        .map(|tokens| {
            tokens
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(|token| token.to_string())
                .collect::<Vec<_>>()
        })
        .filter(|tokens| !tokens.is_empty())
        .unwrap_or_else(|| {
            DEFAULT_REQUIRED_ENTRYPOINT
                .iter()
                .map(|s| s.to_string())
                .collect()
        })
}

pub fn run(repo_root: String, workflow: String, control_plane_contract: String, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let workflow_path = {
        let path = PathBuf::from(workflow);
        if path.is_absolute() {
            path
        } else {
            repo_root.join(path)
        }
    };
    let contract_path = {
        let path = PathBuf::from(control_plane_contract);
        if path.is_absolute() {
            path
        } else {
            repo_root.join(path)
        }
    };

    let mut errors: Vec<String> = Vec::new();
    let mut failure_classes: Vec<String> = Vec::new();

    if !workflow_path.exists() {
        errors.push(format!(
            "workflow file not found: {}",
            workflow_path.display()
        ));
        failure_classes.push(FAILURE_CLASS_UNBOUND.to_string());
    } else {
        let text = fs::read_to_string(&workflow_path).unwrap_or_else(|err| {
            eprintln!(
                "error: failed to read workflow {}: {err}",
                workflow_path.display()
            );
            std::process::exit(2);
        });

        let required_entrypoint = parse_required_entrypoint(&contract_path);
        let required_pattern = entrypoint_pattern(&required_entrypoint).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            std::process::exit(2);
        });
        let required_command = render_entrypoint(&required_entrypoint);
        let required_count = required_pattern.find_iter(&text).count();
        if required_count == 0 {
            errors.push(format!(
                "missing canonical gate command in workflow: `{required_command}`"
            ));
            failure_classes.push(FAILURE_CLASS_UNBOUND.to_string());
        } else if required_count > 1 {
            errors.push(format!(
                "expected exactly one canonical gate command `{required_command}`, found {required_count}"
            ));
            failure_classes.push(FAILURE_CLASS_PARITY_DRIFT.to_string());
        }

        for (label, pattern) in FORBIDDEN_PATTERNS {
            let re = RegexBuilder::new(pattern)
                .multi_line(true)
                .build()
                .unwrap_or_else(|err| {
                    eprintln!("error: failed compiling forbidden pattern `{label}`: {err}");
                    std::process::exit(2);
                });
            if re.is_match(&text) {
                errors.push(format!("forbidden {label} found"));
                failure_classes.push(FAILURE_CLASS_PARITY_DRIFT.to_string());
            }
        }
    }

    failure_classes.sort();
    failure_classes.dedup();
    let result = if errors.is_empty() {
        "accepted"
    } else {
        "rejected"
    };

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": result,
            "failureClasses": failure_classes,
            "workflow": workflow_path.display().to_string(),
            "errors": errors,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render ci-wiring-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if errors.is_empty() {
        let required_entrypoint = parse_required_entrypoint(&contract_path);
        let required_command = render_entrypoint(&required_entrypoint);
        println!(
            "[ci-wiring] OK (workflow={}, command={})",
            workflow_path.display(),
            required_command
        );
    } else {
        println!("[ci-wiring] FAIL ({})", workflow_path.display());
        for err in &errors {
            println!("  - {err}");
        }
    }

    if !errors.is_empty() {
        std::process::exit(1);
    }
}
