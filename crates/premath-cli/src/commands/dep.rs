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

        DepCommands::Project { view, issues, json } => {
            run_project(map_dep_view(view), issues, json)
        }
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

fn map_dep_view(arg: DepViewArg) -> DependencyView {
    match arg {
        DepViewArg::Execution => DependencyView::Execution,
        DepViewArg::Gtd => DependencyView::Gtd,
        DepViewArg::Groupoid => DependencyView::Groupoid,
    }
}

fn run_project(view: DependencyView, issues: String, json_output: bool) {
    let path = PathBuf::from(issues);
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }

    let store = MemoryStore::load_jsonl(&path).unwrap_or_else(|e| {
        eprintln!("error: failed to load {}: {e}", path.display());
        std::process::exit(1);
    });

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

    if json_output {
        let payload = json!({
            "action": "dep.project",
            "issuesPath": path.display().to_string(),
            "view": view.as_str(),
            "count": items.len(),
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
