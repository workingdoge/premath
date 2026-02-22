use premath_coherence::{
    RequiredGateRefRequest, RequiredGateRefResult, RequiredWitnessError, build_required_gate_ref,
};
use std::fs;
use std::path::PathBuf;

fn emit_error(err: RequiredWitnessError) -> ! {
    eprintln!("{err}");
    std::process::exit(2);
}

pub fn run(input: String, json_output: bool) {
    let input_path = PathBuf::from(input);
    let bytes = fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(RequiredWitnessError {
            failure_class: "required_gate_ref_invalid".to_string(),
            message: format!(
                "failed to read required gate ref input {}: {err}",
                input_path.display()
            ),
        });
    });

    let request: RequiredGateRefRequest = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(RequiredWitnessError {
            failure_class: "required_gate_ref_invalid".to_string(),
            message: format!(
                "failed to parse required gate ref input json {}: {err}",
                input_path.display()
            ),
        });
    });

    let result = match build_required_gate_ref(&request) {
        Ok(value) => value,
        Err(err) => emit_error(err),
    };
    if json_output {
        let rendered = serde_json::to_string_pretty(&result).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_gate_ref_invalid".to_string(),
                message: format!("failed to render required gate ref json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }
    render_text(&result);
}

fn render_text(result: &RequiredGateRefResult) {
    println!("premath required-gate-ref");
    println!("  Check: {}", result.gate_witness_ref.check_id);
    println!("  Source: {}", result.gate_witness_ref.source);
    println!("  Sha256: {}", result.gate_witness_ref.sha256);
}
