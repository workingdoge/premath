use crate::cli::IssueGraphCompactModeArg;
use crate::commands::issue_graph_compactness::{
    compactness_edge_set, evaluate_compactness_findings, print_compactness_findings,
};
use premath_bd::{
    AtomicStoreMutationError, DepType, DependencyGraphScope, MemoryStore, mutate_store_jsonl,
};
use premath_surreal::QueryCache;
use serde::Serialize;
use serde_json::json;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.issue_graph_compactness.v1";

#[derive(Debug, Clone, Serialize)]
struct SemanticSnapshot {
    #[serde(rename = "readyIds")]
    ready_ids: Vec<String>,
    #[serde(rename = "blockedIds")]
    blocked_ids: Vec<String>,
    #[serde(rename = "hasCycle")]
    has_cycle: bool,
    #[serde(rename = "cyclePath", skip_serializing_if = "Option::is_none")]
    cycle_path: Option<Vec<String>>,
}

fn resolve_issues_path(repo_root: &Path, issues: &str) -> PathBuf {
    let candidate = PathBuf::from(issues);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    }
}

fn load_store_required(path: &Path) -> MemoryStore {
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }
    MemoryStore::load_jsonl(path).unwrap_or_else(|error| {
        eprintln!("error: failed to load {}: {error}", path.display());
        std::process::exit(1);
    })
}

fn snapshot_semantics(store: &MemoryStore) -> SemanticSnapshot {
    let cache = QueryCache::hydrate(store);
    let mut ready_ids = cache.ready_open_issue_ids();
    ready_ids.sort();

    let mut blocked_ids = store
        .issues()
        .filter(|issue| issue.status != "closed")
        .filter_map(|issue| {
            let manual_blocked = issue.status == "blocked";
            let has_unresolved_blocker =
                store.blocking_dependencies_of(&issue.id).iter().any(|dep| {
                    match cache.issue(&dep.depends_on_id) {
                        Some(blocker) => blocker.status != "closed",
                        None => true,
                    }
                });
            if manual_blocked || has_unresolved_blocker {
                Some(issue.id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    blocked_ids.sort();

    let cycle_path = store.find_any_dependency_cycle_in_scope(DependencyGraphScope::Active);
    SemanticSnapshot {
        ready_ids,
        blocked_ids,
        has_cycle: cycle_path.is_some(),
        cycle_path,
    }
}

fn remove_blocks_edges(issues_path: &Path, edges: &[(String, String)]) {
    mutate_store_jsonl::<(), String, _>(issues_path, |store| {
        for (issue_id, depends_on_id) in edges {
            store
                .remove_dependency(issue_id, depends_on_id, DepType::Blocks)
                .map_err(|error| {
                    format!("failed to remove edge {issue_id}->{depends_on_id}: {error}")
                })?;
        }
        Ok(((), !edges.is_empty()))
    })
    .unwrap_or_else(|error| {
        match error {
            AtomicStoreMutationError::Mutation(message) => eprintln!("error: {message}"),
            other => eprintln!("error: {other}"),
        }
        std::process::exit(1);
    });
}

fn print_apply_summary(removed_edges: &[(String, String)]) {
    println!(
        "[issue-graph-compact] APPLY (removed={})",
        removed_edges.len()
    );
    for (issue_id, depends_on_id) in removed_edges {
        println!("  - removed blocks edge: {issue_id} -> {depends_on_id}");
    }
}

pub fn run(repo_root: String, issues: String, mode: IssueGraphCompactModeArg, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let issues_path = resolve_issues_path(&repo_root, &issues);
    let store = load_store_required(&issues_path);
    let findings = evaluate_compactness_findings(&store);

    match mode {
        IssueGraphCompactModeArg::Check => {
            let result = if findings.is_empty() {
                "accepted"
            } else {
                "rejected"
            };
            if json_output {
                let payload = json!({
                    "schema": 1,
                    "checkKind": CHECK_KIND,
                    "action": "issue_graph.compactness",
                    "mode": "check",
                    "issuesPath": issues_path.display().to_string(),
                    "findingCount": findings.len(),
                    "findings": findings,
                    "result": result,
                });
                let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
                    eprintln!("error: failed to render issue-graph-compact check payload: {error}");
                    std::process::exit(2);
                });
                println!("{rendered}");
            } else if findings.is_empty() {
                println!("[issue-graph-compact] OK (no compactness drift)");
            } else {
                print_compactness_findings(&findings, Some(&repo_root), Some(&issues_path));
            }

            if !findings.is_empty() {
                std::process::exit(1);
            }
        }
        IssueGraphCompactModeArg::Apply => {
            let edges = compactness_edge_set(&findings);
            if edges.is_empty() {
                if json_output {
                    let payload = json!({
                        "schema": 1,
                        "checkKind": CHECK_KIND,
                        "action": "issue_graph.compactness",
                        "mode": "apply",
                        "issuesPath": issues_path.display().to_string(),
                        "removedCount": 0,
                        "removedEdges": [],
                        "result": "accepted",
                    });
                    let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
                        eprintln!(
                            "error: failed to render issue-graph-compact apply payload: {error}"
                        );
                        std::process::exit(2);
                    });
                    println!("{rendered}");
                } else {
                    println!("[issue-graph-compact] APPLY (removed=0)");
                }
                return;
            }

            let before = snapshot_semantics(&store);
            remove_blocks_edges(&issues_path, &edges);
            let after_store = load_store_required(&issues_path);
            let after = snapshot_semantics(&after_store);
            let residual_findings = evaluate_compactness_findings(&after_store);

            let semantic_mismatch =
                before.ready_ids != after.ready_ids || before.blocked_ids != after.blocked_ids;
            let cycle_regression = !before.has_cycle && after.has_cycle;

            if semantic_mismatch || cycle_regression || !residual_findings.is_empty() {
                let payload = json!({
                    "schema": 1,
                    "checkKind": CHECK_KIND,
                    "action": "issue_graph.compactness",
                    "mode": "apply",
                    "issuesPath": issues_path.display().to_string(),
                    "removedEdges": edges
                        .iter()
                        .map(|(issue_id, depends_on_id)| {
                            json!({
                                "issueId": issue_id,
                                "dependsOnId": depends_on_id
                            })
                        })
                        .collect::<Vec<_>>(),
                    "before": before,
                    "after": after,
                    "residualFindings": residual_findings,
                    "result": "rejected",
                });
                if json_output {
                    let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
                        eprintln!(
                            "error: failed to render issue-graph-compact rejection payload: {error}"
                        );
                        std::process::exit(2);
                    });
                    println!("{rendered}");
                } else {
                    println!("[issue-graph-compact] FAIL (apply invariant mismatch)");
                    let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
                        eprintln!(
                            "error: failed to render issue-graph-compact rejection payload: {error}"
                        );
                        std::process::exit(2);
                    });
                    println!("{rendered}");
                }
                std::process::exit(1);
            }

            if json_output {
                let payload = json!({
                    "schema": 1,
                    "checkKind": CHECK_KIND,
                    "action": "issue_graph.compactness",
                    "mode": "apply",
                    "issuesPath": issues_path.display().to_string(),
                    "removedCount": edges.len(),
                    "removedEdges": edges
                        .iter()
                        .map(|(issue_id, depends_on_id)| {
                            json!({
                                "issueId": issue_id,
                                "dependsOnId": depends_on_id
                            })
                        })
                        .collect::<Vec<_>>(),
                    "before": before,
                    "after": after,
                    "result": "accepted",
                });
                let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
                    eprintln!("error: failed to render issue-graph-compact apply payload: {error}");
                    std::process::exit(2);
                });
                println!("{rendered}");
            } else {
                print_apply_summary(&edges);
            }
        }
    }
}
