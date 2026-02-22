use premath_coherence::{InstructionError, validate_instruction_envelope_payload};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

fn emit_error(err: InstructionError) -> ! {
    eprintln!("{err}");
    std::process::exit(2);
}

pub fn run(instruction: String, repo_root: String, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let instruction_path = {
        let path = PathBuf::from(instruction);
        if path.is_absolute() {
            path
        } else {
            repo_root.join(path)
        }
    };

    let bytes = fs::read(&instruction_path).unwrap_or_else(|err| {
        emit_error(InstructionError {
            failure_class: "instruction_envelope_invalid".to_string(),
            message: format!(
                "failed to read instruction file {}: {err}",
                instruction_path.display()
            ),
        });
    });

    let raw: Value = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(InstructionError {
            failure_class: "instruction_envelope_invalid_json".to_string(),
            message: format!(
                "failed to parse instruction json {}: {err}",
                instruction_path.display()
            ),
        });
    });

    let checked = validate_instruction_envelope_payload(&raw, &instruction_path, &repo_root)
        .unwrap_or_else(|err| emit_error(err));

    if json_output {
        let rendered = serde_json::to_string_pretty(&checked).unwrap_or_else(|err| {
            emit_error(InstructionError {
                failure_class: "instruction_envelope_invalid".to_string(),
                message: format!("failed to render instruction-check json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }

    let summary = json!({
        "instruction": instruction_path.display().to_string(),
        "requestedChecks": checked.requested_checks,
        "policyDigest": checked.policy_digest,
        "normalizerId": checked.normalizer_id,
    });
    println!("premath instruction-check");
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string())
    );
}
