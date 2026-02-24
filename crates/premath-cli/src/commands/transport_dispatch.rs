use premath_transport::transport_dispatch_json;
use serde_json::{Value, json};

pub fn run(action: String, payload: String, json_output: bool) {
    let action_trimmed = action.trim();
    if action_trimmed.is_empty() {
        eprintln!("error: --action is required");
        std::process::exit(2);
    }

    let payload_value: Value = serde_json::from_str(&payload).unwrap_or_else(|err| {
        eprintln!("error: failed to parse --payload JSON: {err}");
        std::process::exit(2);
    });

    let request = json!({
        "action": action_trimmed,
        "payload": payload_value,
    });

    let request_json = serde_json::to_string(&request).unwrap_or_else(|err| {
        eprintln!("error: failed to serialize transport request: {err}");
        std::process::exit(2);
    });

    let response_json = transport_dispatch_json(&request_json);
    let response: Value = serde_json::from_str(&response_json).unwrap_or_else(|err| {
        eprintln!("error: failed to parse transport response: {err}");
        std::process::exit(2);
    });

    if json_output {
        let rendered = serde_json::to_string_pretty(&response).unwrap_or_else(|err| {
            eprintln!("error: failed to render transport-dispatch payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    let result = response
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let action_id = response
        .get("actionId")
        .and_then(Value::as_str)
        .unwrap_or("<none>");
    let digest = response
        .get("semanticDigest")
        .and_then(Value::as_str)
        .unwrap_or("<none>");
    println!("premath transport-dispatch");
    println!("  Action: {action_trimmed}");
    println!("  Action ID: {action_id}");
    println!("  Result: {result}");
    println!("  Semantic digest: {digest}");
}
