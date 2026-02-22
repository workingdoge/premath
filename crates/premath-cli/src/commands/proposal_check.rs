use premath_coherence::{
    compile_proposal_obligations, discharge_proposal_obligations, validate_proposal_payload,
};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

pub fn run(proposal: String, json_output: bool) {
    let proposal_path = PathBuf::from(proposal);
    let bytes = fs::read(&proposal_path).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read proposal file {}: {err}",
            proposal_path.display()
        );
        std::process::exit(2);
    });
    let raw: Value = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to parse proposal json {}: {err}",
            proposal_path.display()
        );
        std::process::exit(2);
    });

    let validated = validate_proposal_payload(&raw).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(2);
    });
    let obligations = compile_proposal_obligations(&validated.canonical);
    let discharge = discharge_proposal_obligations(&validated.canonical, &obligations);

    let payload = json!({
        "canonical": validated.canonical,
        "digest": validated.digest,
        "kcirRef": validated.kcir_ref,
        "obligations": obligations,
        "discharge": discharge,
    });

    if json_output {
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render proposal-check json: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    let outcome = payload
        .get("discharge")
        .and_then(|item| item.get("outcome"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    println!("premath proposal-check");
    println!("  Proposal: {}", proposal_path.display());
    println!("  Outcome: {}", outcome);
}
