use serde_json::{Value, json};

pub fn run(json_output: bool, repo_root: String) {
    let request = json!({
        "action": "site.current_digest",
        "payload": {
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
        let digest = response
            .get("digest")
            .and_then(Value::as_str)
            .unwrap_or("<none>");

        println!("premath site-digest");
        println!("  Result: {}", result);
        println!("  Digest: {}", digest);

        if let Some(summary) = response.get("summary") {
            let nc = summary
                .get("nodeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let ec = summary
                .get("edgeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let oc = summary
                .get("operationCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let cc = summary
                .get("coverCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let wc = summary
                .get("worldRouteBindingRowCount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            println!("  Nodes:      {}", nc);
            println!("  Edges:      {}", ec);
            println!("  Operations: {}", oc);
            println!("  Covers:     {}", cc);
            println!("  WR rows:    {}", wc);
        }

        if result == "rejected"
            && let Some(diag) = response.get("diagnostic").and_then(Value::as_str)
        {
            eprintln!("  {diag}");
        }
    }

    let exit_code = match response.get("result").and_then(Value::as_str) {
        Some("accepted") => 0,
        _ => 2,
    };
    std::process::exit(exit_code);
}
