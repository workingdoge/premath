use crate::support::read_json_file_or_exit;
use premath_doctrine::validate_world_descent_contract_projection;
use serde_json::{Value, json};

const WORLD_DESCENT_CONTRACT_CHECK_KIND: &str = "premath.world_descent_contract_check.v1";

pub fn run(control_plane_contract_path: String, json_output: bool) {
    let control_plane_contract: Value =
        read_json_file_or_exit(&control_plane_contract_path, "control-plane contract");
    let (projection, issues) = validate_world_descent_contract_projection(&control_plane_contract);
    let result = if issues.is_empty() {
        "accepted"
    } else {
        "rejected"
    };

    let mut failure_classes: Vec<String> = issues
        .iter()
        .map(|issue| issue.failure_class.clone())
        .collect();
    failure_classes.sort();
    failure_classes.dedup();

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": WORLD_DESCENT_CONTRACT_CHECK_KIND,
            "controlPlaneContractPath": control_plane_contract_path,
            "result": result,
            "failureClasses": failure_classes,
            "worldDescentContract": projection,
            "issues": issues,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render world-descent-contract-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        println!("premath world-descent-contract-check");
        println!("  Control-plane contract: {control_plane_contract_path}");
        println!("  Result: {result}");
        println!("  Failure classes: {}", failure_classes.len());
        println!("  Issues: {}", issues.len());
    }

    if result != "accepted" {
        std::process::exit(1);
    }
}
