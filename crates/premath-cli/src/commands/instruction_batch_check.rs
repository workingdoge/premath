use premath_coherence::{InstructionError, validate_instruction_envelope_payload};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INSTRUCTION_DIRS: [&str; 2] = ["instructions", "tests/ci/fixtures/instructions"];

fn read_instruction(path: &Path) -> Result<Value, InstructionError> {
    let bytes = fs::read(path).map_err(|err| InstructionError {
        failure_class: "instruction_envelope_invalid".to_string(),
        message: format!("failed to read instruction file {}: {err}", path.display()),
    })?;
    serde_json::from_slice::<Value>(&bytes).map_err(|err| InstructionError {
        failure_class: "instruction_envelope_invalid_json".to_string(),
        message: format!("failed to parse instruction json {}: {err}", path.display()),
    })
}

fn resolve_paths(repo_root: &Path, inputs: &[String]) -> Vec<PathBuf> {
    if !inputs.is_empty() {
        let mut resolved: Vec<PathBuf> = inputs
            .iter()
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    repo_root.join(path)
                }
            })
            .collect();
        resolved.sort();
        resolved
    } else {
        let mut resolved = Vec::new();
        for rel_dir in DEFAULT_INSTRUCTION_DIRS {
            let dir = repo_root.join(rel_dir);
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                    resolved.push(path);
                }
            }
        }
        resolved.sort();
        resolved
    }
}

pub fn run(instructions: Vec<String>, repo_root: String, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let files = resolve_paths(&repo_root, &instructions);

    if files.is_empty() {
        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "checkKind": "ci.instruction_envelope_check.v1",
                    "checked": 0,
                    "errors": ["no instruction envelopes found"],
                    "result": "rejected"
                }))
                .unwrap_or_else(|_| "{}".to_string())
            );
        } else {
            println!("[instruction-check] FAIL (no instruction envelopes found)");
        }
        std::process::exit(1);
    }

    let mut checked = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for path in files {
        checked += 1;
        if !path.exists() || !path.is_file() {
            errors.push(format!("{}: file not found", path.display()));
            continue;
        }

        let result = read_instruction(&path)
            .and_then(|raw| validate_instruction_envelope_payload(&raw, &path, &repo_root));
        if let Err(err) = result {
            errors.push(format!("{}: {}", path.display(), err));
        }
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "checkKind": "ci.instruction_envelope_check.v1",
                "checked": checked,
                "errors": errors,
                "result": if errors.is_empty() { "accepted" } else { "rejected" }
            }))
            .unwrap_or_else(|_| "{}".to_string())
        );
    } else if errors.is_empty() {
        println!("[instruction-check] OK (checked={checked})");
    } else {
        println!(
            "[instruction-check] FAIL (checked={checked}, errors={})",
            errors.len()
        );
        for err in &errors {
            println!("  - {err}");
        }
    }

    if !errors.is_empty() {
        std::process::exit(1);
    }
}
