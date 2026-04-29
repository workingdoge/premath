use premath_kernel::{WorkTrackerCheckInput, evaluate_work_tracker_checker};
use std::fs;
use std::path::PathBuf;

fn emit_error(message: impl Into<String>) -> ! {
    eprintln!("{}", message.into());
    std::process::exit(2);
}

pub fn run(input: String, json_output: bool) {
    let input_path = PathBuf::from(input);
    let bytes = fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(format!(
            "work_tracker_check_invalid: failed to read input {}: {err}",
            input_path.display()
        ))
    });
    let request: WorkTrackerCheckInput = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(format!(
            "work_tracker_check_invalid: failed to parse input json {}: {err}",
            input_path.display()
        ))
    });
    let output = evaluate_work_tracker_checker(&request);

    if json_output {
        let rendered = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
            emit_error(format!(
                "work_tracker_check_invalid: failed to render output json: {err}"
            ))
        });
        println!("{rendered}");
        return;
    }

    println!("premath work-tracker-check");
    println!("  Input: {}", input_path.display());
    println!("  Result: {}", output.result);
    println!("  Failure Classes: {}", output.failure_classes.len());
    for class in &output.failure_classes {
        println!("  - {class}");
    }
}
