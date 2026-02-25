use crate::support::read_json_file_or_exit;
use premath_kernel::evaluate_runtime_orchestration;
use serde_json::Value;
use std::fs;

pub fn run(
    control_plane_contract_path: String,
    doctrine_op_registry_path: String,
    harness_runtime_path: String,
    doctrine_site_input_path: Option<String>,
    json_output: bool,
) {
    let control_plane_contract: Value =
        read_json_file_or_exit(&control_plane_contract_path, "control-plane contract");
    let doctrine_op_registry: Value =
        read_json_file_or_exit(&doctrine_op_registry_path, "doctrine operation registry");
    let harness_runtime_text = fs::read_to_string(&harness_runtime_path).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read harness runtime markdown at {}: {err}",
            harness_runtime_path
        );
        std::process::exit(1);
    });
    let doctrine_site_input = doctrine_site_input_path
        .as_deref()
        .map(|path| read_json_file_or_exit(path, "doctrine site input"));

    let report = evaluate_runtime_orchestration(
        &control_plane_contract,
        &doctrine_op_registry,
        harness_runtime_text.as_str(),
        doctrine_site_input.as_ref(),
    );

    if json_output {
        let rendered = serde_json::to_string_pretty(&report).unwrap_or_else(|err| {
            eprintln!("error: failed to render runtime-orchestration-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        println!("premath runtime-orchestration-check");
        println!("  Control-plane contract: {control_plane_contract_path}");
        println!("  Doctrine operation registry: {doctrine_op_registry_path}");
        println!("  Harness runtime: {harness_runtime_path}");
        if let Some(path) = doctrine_site_input_path.as_deref() {
            println!("  Doctrine site input: {path}");
        }
        println!("  Result: {}", report.result);
        println!("  Failure classes: {}", report.failure_classes.len());
        println!(
            "  Checked runtime routes: {}",
            report.summary.checked_routes
        );
        println!(
            "  Checked world route families: {}",
            report.summary.checked_world_route_families
        );
        println!("  Errors: {}", report.summary.errors);
    }

    if report.result != "accepted" {
        std::process::exit(1);
    }
}
