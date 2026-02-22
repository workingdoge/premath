use premath_coherence::{
    RequiredWitnessError, RequiredWitnessVerifyRequest, verify_required_witness_request,
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
            failure_class: "required_witness_verify_invalid".to_string(),
            message: format!(
                "failed to read required witness verify input {}: {err}",
                input_path.display()
            ),
        });
    });

    let request: RequiredWitnessVerifyRequest =
        serde_json::from_slice(&bytes).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_witness_verify_invalid".to_string(),
                message: format!(
                    "failed to parse required witness verify input json {}: {err}",
                    input_path.display()
                ),
            });
        });

    let result = verify_required_witness_request(&request).unwrap_or_else(|err| emit_error(err));

    if json_output {
        let rendered = serde_json::to_string_pretty(&result).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_witness_verify_invalid".to_string(),
                message: format!("failed to render required witness verify json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }

    if result.errors.is_empty() {
        println!(
            "premath required-witness-verify: OK (projection={}, checks={})",
            result.derived.projection_digest,
            result.derived.required_checks.len()
        );
    } else {
        println!(
            "premath required-witness-verify: FAIL (errors={})",
            result.errors.len()
        );
        for err in &result.errors {
            println!("  - {err}");
        }
    }
}
