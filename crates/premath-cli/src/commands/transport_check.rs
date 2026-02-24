use premath_transport::transport_check;

pub fn run(json_output: bool) {
    let report = transport_check();

    if json_output {
        let rendered = serde_json::to_string_pretty(&report).unwrap_or_else(|err| {
            eprintln!("error: failed to render transport-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        println!("premath transport-check");
        println!("  Check kind: {}", report.check_kind);
        println!("  Registry kind: {}", report.registry_kind);
        println!("  Profile: {}", report.profile_id);
        println!("  Result: {}", report.result);
        println!("  Actions: {}", report.action_count);
        println!("  Issues: {}", report.issues.len());
        println!("  Semantic digest: {}", report.semantic_digest);
    }

    if report.result != "accepted" {
        std::process::exit(1);
    }
}
