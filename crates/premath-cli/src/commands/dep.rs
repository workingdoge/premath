use crate::cli::{DepCommands, DepTypeArg};
use premath_bd::{DepType, MemoryStore};
use serde_json::json;
use std::path::PathBuf;

pub fn run(command: DepCommands) {
    match command {
        DepCommands::Add {
            issue_id,
            depends_on_id,
            dep_type,
            created_by,
            issues,
            json,
        } => run_add(
            issue_id,
            depends_on_id,
            map_dep_type(dep_type),
            created_by,
            issues,
            json,
        ),
    }
}

fn run_add(
    issue_id: String,
    depends_on_id: String,
    dep_type: DepType,
    created_by: String,
    issues: String,
    json_output: bool,
) {
    let path = PathBuf::from(issues);
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }

    let mut store = MemoryStore::load_jsonl(&path).unwrap_or_else(|e| {
        eprintln!("error: failed to load {}: {e}", path.display());
        std::process::exit(1);
    });

    store
        .add_dependency(&issue_id, &depends_on_id, dep_type.clone(), created_by)
        .unwrap_or_else(|e| {
            eprintln!("error: failed to add dependency: {e}");
            std::process::exit(1);
        });

    store.save_jsonl(&path).unwrap_or_else(|e| {
        eprintln!("error: failed to save {}: {e}", path.display());
        std::process::exit(1);
    });

    if json_output {
        let payload = json!({
            "action": "dep.add",
            "issuesPath": path.display().to_string(),
            "dependency": {
                "issueId": issue_id,
                "dependsOnId": depends_on_id,
                "type": dep_type.as_str()
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath dep add\n  Added: {} -> {} ({})\n  Path: {}",
            issue_id,
            depends_on_id,
            dep_type.as_str(),
            path.display()
        );
    }
}

fn map_dep_type(arg: DepTypeArg) -> DepType {
    match arg {
        DepTypeArg::Blocks => DepType::Blocks,
        DepTypeArg::ParentChild => DepType::ParentChild,
        DepTypeArg::ConditionalBlocks => DepType::ConditionalBlocks,
        DepTypeArg::Related => DepType::Related,
        DepTypeArg::DiscoveredFrom => DepType::DiscoveredFrom,
        DepTypeArg::RelatesTo => DepType::RelatesTo,
        DepTypeArg::Duplicates => DepType::Duplicates,
        DepTypeArg::Supersedes => DepType::Supersedes,
        DepTypeArg::WaitsFor => DepType::WaitsFor,
        DepTypeArg::RepliesTo => DepType::RepliesTo,
    }
}
