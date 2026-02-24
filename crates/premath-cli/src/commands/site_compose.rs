use serde_json::{Value, json};
use std::fs;

pub fn run(request1: String, request2: String, json_output: bool) {
    let r1_trimmed = request1.trim();
    let r2_trimmed = request2.trim();
    if r1_trimmed.is_empty() || r2_trimmed.is_empty() {
        eprintln!("error: --request1 and --request2 paths are required");
        std::process::exit(2);
    }

    let r1_json = fs::read_to_string(r1_trimmed).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read request1 file '{}': {}",
            r1_trimmed, err
        );
        std::process::exit(2);
    });
    let r2_json = fs::read_to_string(r2_trimmed).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read request2 file '{}': {}",
            r2_trimmed, err
        );
        std::process::exit(2);
    });

    let r1_value: Value = serde_json::from_str(&r1_json).unwrap_or_else(|err| {
        eprintln!("error: failed to parse request1 JSON: {}", err);
        std::process::exit(2);
    });
    let r2_value: Value = serde_json::from_str(&r2_json).unwrap_or_else(|err| {
        eprintln!("error: failed to parse request2 JSON: {}", err);
        std::process::exit(2);
    });

    let request = json!({
        "action": "site.compose_changes",
        "payload": {
            "request1": r1_value,
            "request2": r2_value,
        }
    });

    let request_json = serde_json::to_string(&request).unwrap_or_else(|err| {
        eprintln!("error: failed to serialize transport request: {}", err);
        std::process::exit(2);
    });

    let response_json = premath_transport::transport_dispatch_json(&request_json);
    let response: Value = serde_json::from_str(&response_json).unwrap_or_else(|err| {
        eprintln!("error: failed to parse transport response: {}", err);
        std::process::exit(2);
    });

    if json_output {
        let rendered = serde_json::to_string_pretty(&response).unwrap_or_else(|err| {
            eprintln!("error: failed to render JSON: {}", err);
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        let result = response
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let from_digest = response
            .get("fromDigest")
            .and_then(Value::as_str)
            .unwrap_or("<none>");
        let to_digest = response
            .get("toDigest")
            .and_then(Value::as_str)
            .unwrap_or("<none>");
        let morphism_kind = response
            .get("morphismKind")
            .and_then(Value::as_str)
            .unwrap_or("<none>");
        let mutation_count = response
            .get("mutationCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);

        println!("premath site-compose");
        println!("  Result:        {}", result);
        println!("  From digest:   {}", from_digest);
        println!("  To digest:     {}", to_digest);
        println!("  Morphism kind: {}", morphism_kind);
        println!("  Mutations:     {}", mutation_count);

        if result == "rejected"
            && let Some(diag) = response.get("diagnostic").and_then(Value::as_str)
        {
            eprintln!("  {diag}");
        }
    }

    let exit_code = match response.get("result").and_then(Value::as_str) {
        Some("accepted") => 0,
        Some("rejected") => 1,
        _ => 2,
    };
    std::process::exit(exit_code);
}
