use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.doctrine_mcp_parity_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "doctrine_mcp_parity_violation";
const REGISTRY_MCP_PREFIX: &str = "op/mcp.";
const MORPHISM_IDENTITY: &str = "dm.identity";
const MORPHISM_PRESENTATION: &str = "dm.presentation.projection";
const MORPHISM_EXECUTION: &str = "dm.profile.execution";

fn extract_mcp_tool_specs(mcp_source: &Path) -> Result<BTreeMap<String, bool>, String> {
    let text = fs::read_to_string(mcp_source)
        .map_err(|err| format!("failed reading {}: {err}", mcp_source.display()))?;
    let block_re = RegexBuilder::new(r"#\[mcp_tool\((.*?)\)\]")
        .dot_matches_new_line(true)
        .build()
        .map_err(|err| format!("failed compiling mcp_tool block regex: {err}"))?;
    let name_re = Regex::new(r#"name\s*=\s*"([a-z0-9_]+)""#)
        .map_err(|err| format!("failed compiling mcp tool name regex: {err}"))?;
    let read_only_re = Regex::new(r"read_only_hint\s*=\s*(true|false)")
        .map_err(|err| format!("failed compiling read_only_hint regex: {err}"))?;

    let mut specs: BTreeMap<String, bool> = BTreeMap::new();
    for block in block_re.captures_iter(&text) {
        let Some(body) = block.get(1).map(|row| row.as_str()) else {
            continue;
        };
        let Some(name_match) = name_re.captures(body).and_then(|row| row.get(1)) else {
            continue;
        };
        let name = name_match.as_str().to_string();
        let Some(read_only_match) = read_only_re.captures(body).and_then(|row| row.get(1)) else {
            return Err(format!("MCP tool {name:?} missing explicit read_only_hint"));
        };
        let read_only = read_only_match.as_str() == "true";
        if specs.contains_key(&name) {
            return Err(format!("duplicate MCP tool name: {name}"));
        }
        specs.insert(name, read_only);
    }
    Ok(specs)
}

fn extract_registry_tool_morphisms(
    registry_path: &Path,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let payload = fs::read_to_string(registry_path)
        .map_err(|err| format!("failed reading {}: {err}", registry_path.display()))?;
    let registry: Value = serde_json::from_str(&payload)
        .map_err(|err| format!("failed parsing {}: {err}", registry_path.display()))?;
    let operations = registry
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry.operations must be a list".to_string())?;

    let mut names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for raw in operations {
        let Some(operation) = raw.as_object() else {
            return Err("registry operation entries must be objects".to_string());
        };
        let Some(op_id) = operation.get("id").and_then(Value::as_str) else {
            return Err("registry operation id must be a string".to_string());
        };
        if !op_id.starts_with(REGISTRY_MCP_PREFIX) {
            continue;
        }
        let tool_name = op_id.trim_start_matches(REGISTRY_MCP_PREFIX).to_string();
        let morphisms = operation
            .get("morphisms")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("registry morphisms must be a list for {op_id}"))?;
        let mut normalized = BTreeSet::new();
        for morphism in morphisms {
            let Some(morphism_value) = morphism.as_str() else {
                return Err(format!(
                    "registry morphism entries must be strings for {op_id}"
                ));
            };
            normalized.insert(morphism_value.to_string());
        }
        if !normalized.contains(MORPHISM_IDENTITY) {
            return Err(format!(
                "registry MCP op missing {MORPHISM_IDENTITY}: {op_id}"
            ));
        }
        names.insert(tool_name, normalized);
    }
    Ok(names)
}

pub fn run(mcp_source: String, registry: String, json_output: bool) {
    let mcp_source = PathBuf::from(mcp_source);
    let registry = PathBuf::from(registry);

    if !mcp_source.exists() {
        eprintln!(
            "[doctrine-mcp-parity] ERROR: MCP source missing: {}",
            mcp_source.display()
        );
        std::process::exit(2);
    }
    if !registry.exists() {
        eprintln!(
            "[doctrine-mcp-parity] ERROR: registry missing: {}",
            registry.display()
        );
        std::process::exit(2);
    }

    let mcp_specs = extract_mcp_tool_specs(&mcp_source).unwrap_or_else(|err| {
        eprintln!("[doctrine-mcp-parity] ERROR: failed to parse inputs: {err}");
        std::process::exit(2);
    });
    let registry_morphisms = extract_registry_tool_morphisms(&registry).unwrap_or_else(|err| {
        eprintln!("[doctrine-mcp-parity] ERROR: failed to parse inputs: {err}");
        std::process::exit(2);
    });

    let mcp_names = mcp_specs.keys().cloned().collect::<BTreeSet<_>>();
    let registry_names = registry_morphisms.keys().cloned().collect::<BTreeSet<_>>();
    let missing_in_registry = mcp_names
        .difference(&registry_names)
        .cloned()
        .collect::<Vec<_>>();
    let stale_in_registry = registry_names
        .difference(&mcp_names)
        .cloned()
        .collect::<Vec<_>>();

    let mut classification_errors = Vec::new();
    for name in mcp_names.intersection(&registry_names) {
        let Some(morphisms) = registry_morphisms.get(name) else {
            continue;
        };
        let read_only = mcp_specs.get(name).copied().unwrap_or(false);
        if read_only {
            if !morphisms.contains(MORPHISM_PRESENTATION) {
                classification_errors.push(format!(
                    "read-only MCP tool {name} missing {MORPHISM_PRESENTATION}"
                ));
            }
            if morphisms.contains(MORPHISM_EXECUTION) {
                classification_errors.push(format!(
                    "read-only MCP tool {name} must not declare {MORPHISM_EXECUTION}"
                ));
            }
        } else if !morphisms.contains(MORPHISM_EXECUTION) {
            classification_errors.push(format!(
                "mutating MCP tool {name} missing {MORPHISM_EXECUTION}"
            ));
        }
    }

    let result = if missing_in_registry.is_empty()
        && stale_in_registry.is_empty()
        && classification_errors.is_empty()
    {
        "accepted"
    } else {
        "rejected"
    };
    let failure_classes: Vec<&str> = if result == "accepted" {
        Vec::new()
    } else {
        vec![FAILURE_CLASS_VIOLATION]
    };

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": result,
            "failureClasses": failure_classes,
            "mcpTools": mcp_names.len(),
            "registryMcpOps": registry_names.len(),
            "missingInRegistry": missing_in_registry,
            "staleInRegistry": stale_in_registry,
            "classificationErrors": classification_errors,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render doctrine-mcp-parity payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if result == "accepted" {
        println!(
            "[doctrine-mcp-parity] OK (mcpTools={}, registryMcpOps={})",
            mcp_names.len(),
            registry_names.len()
        );
    } else {
        println!(
            "[doctrine-mcp-parity] FAIL (mcpTools={}, registryMcpOps={})",
            mcp_names.len(),
            registry_names.len()
        );
        for name in &missing_in_registry {
            println!("  - missing registry mapping for MCP tool: {name}");
        }
        for name in &stale_in_registry {
            println!("  - stale registry mapping without MCP tool: {name}");
        }
        for err in &classification_errors {
            println!("  - {err}");
        }
    }

    if result != "accepted" {
        std::process::exit(1);
    }
}
