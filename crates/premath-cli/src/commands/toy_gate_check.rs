use premath_kernel::gate::{GateCheck, run_gate_check};
use premath_kernel::toy::get_world;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn emit_error(message: impl Into<String>) -> ! {
    eprintln!("{}", message.into());
    std::process::exit(2);
}

fn required_string<'a>(payload: &'a Value, key: &str, input_path: &Path) -> &'a str {
    payload.get(key).and_then(Value::as_str).unwrap_or_else(|| {
        emit_error(format!(
            "toy_gate_check_invalid: `{key}` must be a non-empty string in {}",
            input_path.display()
        ))
    })
}

pub fn run(input: String, json_output: bool) {
    let input_path = PathBuf::from(input);
    let bytes = fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(format!(
            "toy_gate_check_invalid: failed to read input {}: {err}",
            input_path.display()
        ))
    });
    let case: Value = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(format!(
            "toy_gate_check_invalid: failed to parse input json {}: {err}",
            input_path.display()
        ))
    });

    if case.get("schema").and_then(Value::as_u64) != Some(1) {
        emit_error(format!(
            "toy_gate_check_invalid: `schema` must be 1 in {}",
            input_path.display()
        ));
    }

    let world_name = required_string(&case, "world", &input_path);
    let world = get_world(world_name).unwrap_or_else(|| {
        emit_error(format!(
            "toy_gate_check_invalid: unknown toy world `{world_name}` in {}",
            input_path.display()
        ))
    });
    let check_payload = case.get("check").unwrap_or_else(|| {
        emit_error(format!(
            "toy_gate_check_invalid: missing `check` object in {}",
            input_path.display()
        ))
    });
    let check = GateCheck::from_fixture(check_payload).unwrap_or_else(|| {
        emit_error(format!(
            "toy_gate_check_invalid: failed to parse `check` in {}",
            input_path.display()
        ))
    });

    let output = run_gate_check(world.as_ref(), &check, "toy");

    if json_output {
        let rendered = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
            emit_error(format!(
                "toy_gate_check_invalid: failed to render output json: {err}"
            ))
        });
        println!("{rendered}");
        return;
    }

    println!("premath toy-gate-check");
    println!("  Input: {}", input_path.display());
    println!("  World: {world_name}");
    println!("  Result: {}", output.result);
    println!("  Failure Classes: {}", output.failures.len());
    for failure in &output.failures {
        println!("  - {}", failure.class);
    }
}
