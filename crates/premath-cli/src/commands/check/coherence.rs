use premath_coherence::{CoherenceWitness, run_coherence_check};
use std::path::PathBuf;

pub fn run(contract: String, repo_root: String, json_output: bool) {
    let repo_root_path = PathBuf::from(repo_root);
    let contract_path = PathBuf::from(contract);

    let witness = run_coherence_check(&repo_root_path, &contract_path).unwrap_or_else(|err| {
        eprintln!("error: coherence-check failed: {err}");
        std::process::exit(2);
    });

    if json_output {
        let rendered = serde_json::to_string_pretty(&witness).unwrap_or_else(|err| {
            eprintln!("error: failed to render coherence witness JSON: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        print_human_summary(&witness);
    }

    if witness.result != "accepted" {
        std::process::exit(1);
    }
}

fn print_human_summary(witness: &CoherenceWitness) {
    println!("premath coherence-check");
    println!("  Contract: {}", witness.contract_ref);
    println!("  Contract Digest: {}", witness.contract_digest);
    println!(
        "  Binding: ({}, {})",
        witness.binding.normalizer_id, witness.binding.policy_digest
    );
    println!("  Result: {}", witness.result);
    println!("  Obligations:");
    for obligation in &witness.obligations {
        let failure_suffix = if obligation.failure_classes.is_empty() {
            String::new()
        } else {
            format!(" [{}]", obligation.failure_classes.join(", "))
        };
        println!(
            "    - {}: {}{}",
            obligation.obligation_id, obligation.result, failure_suffix
        );
    }
    if !witness.failure_classes.is_empty() {
        println!("  Failure Classes: {}", witness.failure_classes.join(", "));
    }
}
