use serde_json::{Value, json};
use std::fs;

pub fn run(mutations: String, json_output: bool, repo_root: String) {
    let mutations_trimmed = mutations.trim();
    if mutations_trimmed.is_empty() {
        eprintln!("error: --mutations path is required");
        std::process::exit(2);
    }

    let mutations_json = fs::read_to_string(mutations_trimmed).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read mutations file '{}': {}",
            mutations_trimmed, err
        );
        std::process::exit(2);
    });

    let mutations_value: Value = serde_json::from_str(&mutations_json).unwrap_or_else(|err| {
        eprintln!("error: failed to parse mutations JSON: {}", err);
        std::process::exit(2);
    });

    // Accept either { "mutations": [...], "preservationClaims": [...] }
    // or a bare array of mutations.
    let (mutations_array, claims) = if let Some(arr) = mutations_value.as_array() {
        (Value::Array(arr.clone()), Vec::new())
    } else if mutations_value.is_object() {
        let arr = mutations_value
            .get("mutations")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        let claims: Vec<String> = mutations_value
            .get("preservationClaims")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        (arr, claims)
    } else {
        eprintln!(
            "error: mutations file must contain a JSON array or object with 'mutations' field"
        );
        std::process::exit(2);
    };

    let request = json!({
        "action": "site.build_change",
        "payload": {
            "mutations": mutations_array,
            "preservationClaims": claims,
            "repoRoot": repo_root,
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

        println!("premath site-build");
        println!("  Result:        {}", result);
        println!("  From digest:   {}", from_digest);
        println!("  To digest:     {}", to_digest);
        println!("  Morphism kind: {}", morphism_kind);
        println!("  Mutations:     {}", mutation_count);

        if result == "rejected"
            && let Some(diagnostics) = response.get("diagnostics").and_then(Value::as_array)
        {
            for d in diagnostics {
                let class = d.get("class").and_then(Value::as_str).unwrap_or("unknown");
                let message = d.get("message").and_then(Value::as_str).unwrap_or("");
                eprintln!("  [{class}] {message}");
            }
        }
    }

    let exit_code = match response.get("result").and_then(Value::as_str) {
        Some("accepted") => 0,
        Some("rejected") => 1,
        _ => 2,
    };
    std::process::exit(exit_code);
}
