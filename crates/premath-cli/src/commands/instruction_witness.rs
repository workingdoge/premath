use premath_coherence::{
    InstructionError, InstructionWitnessRuntime, build_instruction_witness,
    validate_instruction_envelope_payload,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn emit_error(err: InstructionError) -> ! {
    eprintln!("{err}");
    std::process::exit(2);
}

fn resolve_path(repo_root: &Path, raw: String) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

pub fn run(instruction: String, runtime: String, repo_root: String, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let instruction_path = resolve_path(&repo_root, instruction);
    let runtime_path = resolve_path(&repo_root, runtime);

    let instruction_bytes = fs::read(&instruction_path).unwrap_or_else(|err| {
        emit_error(InstructionError {
            failure_class: "instruction_envelope_invalid".to_string(),
            message: format!(
                "failed to read instruction file {}: {err}",
                instruction_path.display()
            ),
        });
    });
    let instruction_raw: Value = serde_json::from_slice(&instruction_bytes).unwrap_or_else(|err| {
        emit_error(InstructionError {
            failure_class: "instruction_envelope_invalid_json".to_string(),
            message: format!(
                "failed to parse instruction json {}: {err}",
                instruction_path.display()
            ),
        });
    });
    let checked =
        validate_instruction_envelope_payload(&instruction_raw, &instruction_path, &repo_root)
            .unwrap_or_else(|err| emit_error(err));

    let runtime_bytes = fs::read(&runtime_path).unwrap_or_else(|err| {
        emit_error(InstructionError {
            failure_class: "instruction_runtime_invalid".to_string(),
            message: format!(
                "failed to read runtime file {}: {err}",
                runtime_path.display()
            ),
        });
    });
    let runtime_payload: InstructionWitnessRuntime = serde_json::from_slice(&runtime_bytes)
        .unwrap_or_else(|err| {
            emit_error(InstructionError {
                failure_class: "instruction_runtime_invalid".to_string(),
                message: format!(
                    "failed to parse runtime json {}: {err}",
                    runtime_path.display()
                ),
            });
        });

    let witness =
        build_instruction_witness(&checked, runtime_payload).unwrap_or_else(|err| emit_error(err));

    if json_output {
        let rendered = serde_json::to_string_pretty(&witness).unwrap_or_else(|err| {
            emit_error(InstructionError {
                failure_class: "instruction_runtime_invalid".to_string(),
                message: format!("failed to render instruction witness json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }

    println!("premath instruction-witness");
    println!("  Instruction ID: {}", witness.instruction_id);
    println!("  Verdict: {}", witness.verdict_class);
    println!(
        "  Failure Classes: {}",
        if witness.failure_classes.is_empty() {
            "(none)".to_string()
        } else {
            witness.failure_classes.join(", ")
        }
    );
}
