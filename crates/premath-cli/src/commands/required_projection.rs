use premath_coherence::{RequiredProjectionRequest, RequiredWitnessError, project_required_checks};
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
            failure_class: "required_projection_invalid".to_string(),
            message: format!(
                "failed to read required projection input {}: {err}",
                input_path.display()
            ),
        });
    });

    let request: RequiredProjectionRequest = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(RequiredWitnessError {
            failure_class: "required_projection_invalid".to_string(),
            message: format!(
                "failed to parse required projection input json {}: {err}",
                input_path.display()
            ),
        });
    });

    let result = project_required_checks(&request.changed_paths);

    if json_output {
        let rendered = serde_json::to_string_pretty(&result).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_projection_invalid".to_string(),
                message: format!("failed to render required projection json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }

    println!("premath required-projection");
    println!("  Projection Policy: {}", result.projection_policy);
    println!("  Projection Digest: {}", result.projection_digest);
    println!(
        "  Required Checks: {}",
        if result.required_checks.is_empty() {
            "(none)".to_string()
        } else {
            result.required_checks.join(", ")
        }
    );
}
