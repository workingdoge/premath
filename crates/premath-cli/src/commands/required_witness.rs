use premath_coherence::{RequiredWitnessError, RequiredWitnessRuntime, build_required_witness};
use std::fs;
use std::path::PathBuf;

fn emit_error(err: RequiredWitnessError) -> ! {
    eprintln!("{err}");
    std::process::exit(2);
}

pub fn run(runtime: String, json_output: bool) {
    let runtime_path = PathBuf::from(runtime);

    let runtime_bytes = fs::read(&runtime_path).unwrap_or_else(|err| {
        emit_error(RequiredWitnessError {
            failure_class: "required_witness_runtime_invalid".to_string(),
            message: format!(
                "failed to read required witness runtime file {}: {err}",
                runtime_path.display()
            ),
        });
    });

    let runtime_payload: RequiredWitnessRuntime = serde_json::from_slice(&runtime_bytes)
        .unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_witness_runtime_invalid".to_string(),
                message: format!(
                    "failed to parse required witness runtime json {}: {err}",
                    runtime_path.display()
                ),
            });
        });

    let witness = build_required_witness(runtime_payload).unwrap_or_else(|err| emit_error(err));

    if json_output {
        let rendered = serde_json::to_string_pretty(&witness).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_witness_runtime_invalid".to_string(),
                message: format!("failed to render required witness json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }

    println!("premath required-witness");
    println!("  Projection Digest: {}", witness.projection_digest);
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
