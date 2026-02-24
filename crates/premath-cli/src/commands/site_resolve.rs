use crate::support::read_json_file_or_exit;
use premath_kernel::{SiteResolveRequest, resolve_site_request};
use serde_json::Value;

pub fn run(
    request_path: String,
    doctrine_site_input_path: String,
    doctrine_site_path: String,
    doctrine_op_registry_path: String,
    control_plane_contract_path: String,
    capability_registry_path: String,
    json_output: bool,
) {
    let request: SiteResolveRequest = read_json_file_or_exit(&request_path, "site resolve request");
    let doctrine_site_input: Value =
        read_json_file_or_exit(&doctrine_site_input_path, "doctrine site input");
    let doctrine_site: Value = read_json_file_or_exit(&doctrine_site_path, "doctrine site");
    let doctrine_op_registry: Value =
        read_json_file_or_exit(&doctrine_op_registry_path, "doctrine operation registry");
    let control_plane_contract: Value =
        read_json_file_or_exit(&control_plane_contract_path, "control-plane contract");
    let capability_registry: Value =
        read_json_file_or_exit(&capability_registry_path, "capability registry");

    let response = resolve_site_request(
        &request,
        &doctrine_site_input,
        &doctrine_site,
        &doctrine_op_registry,
        &control_plane_contract,
        &capability_registry,
    );

    if json_output {
        let rendered = serde_json::to_string_pretty(&response).unwrap_or_else(|err| {
            eprintln!("error: failed to render site-resolve payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        println!("premath site-resolve");
        println!("  Request path: {request_path}");
        println!("  Doctrine site input: {doctrine_site_input_path}");
        println!("  Doctrine site: {doctrine_site_path}");
        println!("  Doctrine op registry: {doctrine_op_registry_path}");
        println!("  Control-plane contract: {control_plane_contract_path}");
        println!("  Capability registry: {capability_registry_path}");
        println!("  Result: {}", response.result);
        println!("  Failure classes: {}", response.failure_classes.len());
        if let Some(selected) = response.selected.as_ref() {
            println!("  Selected operation: {}", selected.operation_id);
            println!("  Selected route family: {}", selected.route_family_id);
            println!("  Selected world: {}", selected.world_id);
        }
    }

    if response.result != "accepted" {
        std::process::exit(1);
    }
}
