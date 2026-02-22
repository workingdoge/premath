use crate::cli::{DepCommands, DepTypeArg, DepViewArg};
use premath_bd::{DepType, Dependency, DependencyView, MemoryStore};
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
        DepCommands::Remove {
            issue_id,
            depends_on_id,
            dep_type,
            issues,
            json,
        } => run_remove(
            issue_id,
            depends_on_id,
            map_dep_type(dep_type),
            issues,
            json,
        ),
        DepCommands::Replace {
            issue_id,
            depends_on_id,
            from_dep_type,
            to_dep_type,
            created_by,
            issues,
            json,
        } => run_replace(
            issue_id,
            depends_on_id,
            map_dep_type(from_dep_type),
            map_dep_type(to_dep_type),
            created_by,
            issues,
            json,
        ),

        DepCommands::Project { view, issues, json } => {
            run_project(map_dep_view(view), issues, json)
        }
        DepCommands::Diagnostics { issues, json } => run_diagnostics(issues, json),
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
    let mut store = load_store_required(&path);

    store
        .add_dependency(&issue_id, &depends_on_id, dep_type.clone(), created_by)
        .unwrap_or_else(|e| {
            eprintln!("error: failed to add dependency: {e}");
            std::process::exit(1);
        });

    save_store_or_exit(&store, &path);

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

fn run_remove(
    issue_id: String,
    depends_on_id: String,
    dep_type: DepType,
    issues: String,
    json_output: bool,
) {
    let path = PathBuf::from(issues);
    let mut store = load_store_required(&path);
    store
        .remove_dependency(&issue_id, &depends_on_id, dep_type.clone())
        .unwrap_or_else(|e| {
            eprintln!("error: failed to remove dependency: {e}");
            std::process::exit(1);
        });
    save_store_or_exit(&store, &path);

    if json_output {
        let payload = json!({
            "action": "dep.remove",
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
            "premath dep remove\n  Removed: {} -> {} ({})\n  Path: {}",
            issue_id,
            depends_on_id,
            dep_type.as_str(),
            path.display()
        );
    }
}

fn run_replace(
    issue_id: String,
    depends_on_id: String,
    from_dep_type: DepType,
    to_dep_type: DepType,
    created_by: String,
    issues: String,
    json_output: bool,
) {
    let path = PathBuf::from(issues);
    let mut store = load_store_required(&path);
    store
        .replace_dependency(
            &issue_id,
            &depends_on_id,
            from_dep_type.clone(),
            to_dep_type.clone(),
            created_by.clone(),
        )
        .unwrap_or_else(|e| {
            eprintln!("error: failed to replace dependency: {e}");
            std::process::exit(1);
        });
    save_store_or_exit(&store, &path);

    if json_output {
        let payload = json!({
            "action": "dep.replace",
            "issuesPath": path.display().to_string(),
            "dependency": {
                "issueId": issue_id,
                "dependsOnId": depends_on_id,
                "fromType": from_dep_type.as_str(),
                "toType": to_dep_type.as_str(),
                "createdBy": created_by
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath dep replace\n  Replaced: {} -> {} ({} -> {})\n  Path: {}",
            issue_id,
            depends_on_id,
            from_dep_type.as_str(),
            to_dep_type.as_str(),
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

fn map_dep_view(arg: DepViewArg) -> DependencyView {
    match arg {
        DepViewArg::Execution => DependencyView::Execution,
        DepViewArg::Gtd => DependencyView::Gtd,
        DepViewArg::Groupoid => DependencyView::Groupoid,
    }
}

fn run_project(view: DependencyView, issues: String, json_output: bool) {
    let path = PathBuf::from(issues);
    let store = load_store_required(&path);

    let mut dependencies: Vec<Dependency> = store
        .issues()
        .flat_map(|issue| issue.dependencies.iter().cloned())
        .collect();
    dependencies.sort_by(|left, right| {
        (
            left.issue_id.as_str(),
            left.depends_on_id.as_str(),
            left.dep_type.as_str(),
            left.created_by.as_str(),
        )
            .cmp(&(
                right.issue_id.as_str(),
                right.depends_on_id.as_str(),
                right.dep_type.as_str(),
                right.created_by.as_str(),
            ))
    });

    let items = dependencies
        .iter()
        .map(|dependency| dependency.project(view))
        .collect::<Vec<_>>();
    let cycle = store.find_any_dependency_cycle();
    let integrity = json!({
        "hasCycle": cycle.is_some(),
        "cyclePath": cycle
    });

    if json_output {
        let payload = json!({
            "action": "dep.project",
            "issuesPath": path.display().to_string(),
            "view": view.as_str(),
            "count": items.len(),
            "integrity": integrity,
            "items": items.iter().map(|item| {
                json!({
                    "issueId": item.issue_id,
                    "dependsOnId": item.depends_on_id,
                    "type": item.dep_type.as_str(),
                    "view": item.view.as_str(),
                    "role": item.role,
                    "blocking": item.blocking,
                    "createdBy": item.created_by
                })
            }).collect::<Vec<_>>()
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath dep project\n  View: {}\n  Path: {}\n  Count: {}",
            view.as_str(),
            path.display(),
            items.len()
        );
        if let Some(cycle_path) = cycle {
            println!("  Integrity: cycle detected ({})", cycle_path.join(" -> "));
        } else {
            println!("  Integrity: no cycles detected");
        }
        for item in items {
            println!(
                "  - {} -> {} ({}, role={}, blocking={})",
                item.issue_id,
                item.depends_on_id,
                item.dep_type.as_str(),
                item.role,
                item.blocking
            );
        }
    }
}

fn run_diagnostics(issues: String, json_output: bool) {
    let path = PathBuf::from(issues);
    let store = load_store_required(&path);
    let cycle = store.find_any_dependency_cycle();
    if json_output {
        let payload = json!({
            "action": "dep.diagnostics",
            "issuesPath": path.display().to_string(),
            "integrity": {
                "hasCycle": cycle.is_some(),
                "cyclePath": cycle
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else if let Some(cycle_path) = cycle {
        println!(
            "premath dep diagnostics\n  Path: {}\n  Integrity: cycle detected ({})",
            path.display(),
            cycle_path.join(" -> ")
        );
    } else {
        println!(
            "premath dep diagnostics\n  Path: {}\n  Integrity: no cycles detected",
            path.display()
        );
    }
}

fn load_store_required(path: &PathBuf) -> MemoryStore {
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }
    MemoryStore::load_jsonl(path).unwrap_or_else(|e| {
        eprintln!("error: failed to load {}: {e}", path.display());
        std::process::exit(1);
    })
}

fn save_store_or_exit(store: &MemoryStore, path: &PathBuf) {
    store.save_jsonl(path).unwrap_or_else(|e| {
        eprintln!("error: failed to save {}: {e}", path.display());
        std::process::exit(1);
    });
}
