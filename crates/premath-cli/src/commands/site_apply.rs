use serde_json::{Value, json};
use std::fs;

pub fn run(change: String, dry_run: bool, json_output: bool) {
    let change_trimmed = change.trim();
    if change_trimmed.is_empty() {
        eprintln!("error: --change path is required");
        std::process::exit(2);
    }

    let change_json = fs::read_to_string(change_trimmed).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read change request file '{}': {}",
            change_trimmed, err
        );
        std::process::exit(2);
    });

    let change_value: Value = serde_json::from_str(&change_json).unwrap_or_else(|err| {
        eprintln!("error: failed to parse change request JSON: {}", err);
        std::process::exit(2);
    });

    let request = json!({
        "action": "site.apply_change",
        "payload": {
            "changeRequest": change_value,
            "dryRun": dry_run,
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
        let change_id = response
            .get("changeId")
            .and_then(Value::as_str)
            .unwrap_or("<none>");
        let from_digest = response
            .get("fromDigest")
            .and_then(Value::as_str)
            .unwrap_or("<none>");
        let to_digest = response
            .get("toDigest")
            .and_then(Value::as_str)
            .unwrap_or("<none>");
        let commutation = response
            .get("commutationCheck")
            .and_then(Value::as_str)
            .unwrap_or("<none>");
        let is_dry_run = response
            .get("dryRun")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        println!("premath site-apply");
        println!("  Result:      {}", result);
        println!("  Change ID:   {}", change_id);
        println!("  From digest: {}", from_digest);
        println!("  To digest:   {}", to_digest);
        println!("  Commutation: {}", commutation);
        if is_dry_run {
            println!("  Dry run:     true (no artifacts written)");
        }

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
