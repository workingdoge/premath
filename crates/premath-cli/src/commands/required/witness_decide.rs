use premath_coherence::{
    RequiredWitnessDecideRequest, RequiredWitnessDecideResult, RequiredWitnessError,
    decide_required_witness_request,
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
            failure_class: "required_witness_decide_invalid".to_string(),
            message: format!(
                "failed to read required witness decide input {}: {err}",
                input_path.display()
            ),
        });
    });

    let request: RequiredWitnessDecideRequest =
        serde_json::from_slice(&bytes).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_witness_decide_invalid".to_string(),
                message: format!(
                    "failed to parse required witness decide input json {}: {err}",
                    input_path.display()
                ),
            });
        });

    let result = decide_required_witness_request(&request);

    if json_output {
        let rendered = serde_json::to_string_pretty(&result).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_witness_decide_invalid".to_string(),
                message: format!("failed to render required witness decide json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }

    render_text(&result);
}

fn render_text(result: &RequiredWitnessDecideResult) {
    if result.errors.is_empty() {
        println!(
            "premath required-witness-decide: ACCEPT (projection={})",
            result
                .projection_digest
                .as_deref()
                .unwrap_or("(missing-projection-digest)")
        );
        return;
    }

    println!(
        "premath required-witness-decide: REJECT (reason={}, errors={})",
        result.reason_class,
        result.errors.len()
    );
    for err in &result.errors {
        println!("  - {err}");
    }
}
