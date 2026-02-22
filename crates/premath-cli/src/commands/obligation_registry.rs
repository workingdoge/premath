use premath_kernel::obligation_gate_registry_json;
use serde_json::Value;

pub fn run(json_output: bool) {
    let payload = obligation_gate_registry_json();

    if json_output {
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render obligation-registry json: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    let mapping_count = payload
        .get("mappings")
        .and_then(Value::as_array)
        .map_or(0, |rows| rows.len());
    println!("premath obligation-registry");
    println!("  Schema: 1");
    println!("  Registry kind: premath.obligation_gate_registry.v1");
    println!("  Mappings: {mapping_count}");
}
