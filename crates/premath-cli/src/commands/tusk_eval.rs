use crate::support::read_json_file_or_exit;
use premath_tusk::{
    DescentPack, GateWitnessEnvelope, RunIdOptions, RunIdentity, evaluate_descent_pack,
};
use serde_json::json;

pub fn run(
    identity_path: String,
    descent_pack_path: String,
    include_cover_strategy_in_run_id: bool,
    json_output: bool,
) {
    let identity: RunIdentity = read_json_file_or_exit(&identity_path, "run identity");
    let pack: DescentPack = read_json_file_or_exit(&descent_pack_path, "descent pack");

    let outcome = evaluate_descent_pack(&pack);
    let envelope = GateWitnessEnvelope::from_diagnostics(
        &identity,
        RunIdOptions {
            include_cover_strategy_digest: include_cover_strategy_in_run_id,
        },
        outcome.diagnostics,
    );

    if json_output {
        let payload = json!({
            "envelope": envelope,
            "glueResult": outcome.glue_result,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!("premath tusk-eval");
        println!("  Run ID: {}", envelope.run_id);
        println!("  Result: {}", envelope.result);
        println!("  Failures: {}", envelope.failures.len());
        if let Some(glue) = outcome.glue_result {
            println!("  Glue selected: {}", glue.selected);
            println!(
                "  Mode: {} / {}",
                glue.contractibility_basis.mode.normalizer_id,
                glue.contractibility_basis.mode.policy_digest
            );
        }
    }
}
