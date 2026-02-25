use premath_bd::{DepType, MemoryStore};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

pub const FAILURE_CLASS_CLOSED_BLOCK_EDGE: &str = "issue_graph.compactness.closed_block_edge";
pub const FAILURE_CLASS_TRANSITIVE_BLOCK_EDGE: &str =
    "issue_graph.compactness.transitive_block_edge";

const STATUS_OPEN: &str = "open";
const STATUS_IN_PROGRESS: &str = "in_progress";
const STATUS_CLOSED: &str = "closed";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompactnessFinding {
    #[serde(rename = "class")]
    pub class_name: String,
    #[serde(rename = "issueId")]
    pub issue_id: String,
    #[serde(rename = "dependsOnId")]
    pub depends_on_id: String,
    #[serde(rename = "witnessPath", skip_serializing_if = "Option::is_none")]
    pub witness_path: Option<Vec<String>>,
}

fn normalize_status(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn is_active_status(status: &str) -> bool {
    status == STATUS_OPEN || status == STATUS_IN_PROGRESS
}

fn build_blocks_adjacency(store: &MemoryStore) -> BTreeMap<String, BTreeSet<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for issue in store.issues() {
        for dep in &issue.dependencies {
            if dep.dep_type != DepType::Blocks {
                continue;
            }
            let source = if dep.issue_id.trim().is_empty() {
                issue.id.trim()
            } else {
                dep.issue_id.trim()
            };
            let target = dep.depends_on_id.trim();
            if source.is_empty() || target.is_empty() {
                continue;
            }
            adjacency
                .entry(source.to_string())
                .or_default()
                .insert(target.to_string());
        }
    }
    adjacency
}

fn find_path(
    adjacency: &BTreeMap<String, BTreeSet<String>>,
    start: &str,
    target: &str,
) -> Option<Vec<String>> {
    if start == target {
        return Some(vec![start.to_string()]);
    }

    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    queue.push_back(vec![start.to_string()]);
    visited.insert(start.to_string());

    while let Some(path) = queue.pop_front() {
        let Some(node) = path.last() else {
            continue;
        };
        let Some(next_ids) = adjacency.get(node) else {
            continue;
        };
        for next_id in next_ids {
            if next_id == target {
                let mut found = path.clone();
                found.push(next_id.clone());
                return Some(found);
            }
            if visited.insert(next_id.clone()) {
                let mut next_path = path.clone();
                next_path.push(next_id.clone());
                queue.push_back(next_path);
            }
        }
    }

    None
}

fn sort_key(finding: &CompactnessFinding) -> (String, String, String, String) {
    (
        finding.class_name.clone(),
        finding.issue_id.clone(),
        finding.depends_on_id.clone(),
        finding
            .witness_path
            .clone()
            .unwrap_or_default()
            .join(" -> "),
    )
}

pub fn evaluate_compactness_findings(store: &MemoryStore) -> Vec<CompactnessFinding> {
    let by_id: BTreeMap<String, String> = store
        .issues()
        .map(|issue| (issue.id.clone(), normalize_status(&issue.status)))
        .collect();
    let adjacency = build_blocks_adjacency(store);

    let mut findings: Vec<CompactnessFinding> = Vec::new();

    for issue_id in by_id.keys() {
        let status = by_id.get(issue_id).cloned().unwrap_or_default();
        if !is_active_status(&status) {
            continue;
        }

        let direct_targets = adjacency
            .get(issue_id)
            .map(|targets| targets.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        if direct_targets.is_empty() {
            continue;
        }

        for target_id in &direct_targets {
            let target_status = by_id.get(target_id).cloned().unwrap_or_default();
            if target_status == STATUS_CLOSED {
                findings.push(CompactnessFinding {
                    class_name: FAILURE_CLASS_CLOSED_BLOCK_EDGE.to_string(),
                    issue_id: issue_id.clone(),
                    depends_on_id: target_id.clone(),
                    witness_path: None,
                });
            }
        }

        for target_id in &direct_targets {
            let target_status = by_id.get(target_id).cloned().unwrap_or_default();
            if target_status == STATUS_CLOSED {
                continue;
            }

            for candidate_start in &direct_targets {
                if candidate_start == target_id {
                    continue;
                }
                if let Some(path) = find_path(&adjacency, candidate_start, target_id) {
                    findings.push(CompactnessFinding {
                        class_name: FAILURE_CLASS_TRANSITIVE_BLOCK_EDGE.to_string(),
                        issue_id: issue_id.clone(),
                        depends_on_id: target_id.clone(),
                        witness_path: Some(path),
                    });
                    break;
                }
            }
        }
    }

    findings.sort_by_key(sort_key);
    findings
}

pub fn compactness_edge_set(findings: &[CompactnessFinding]) -> Vec<(String, String)> {
    let mut edges = BTreeSet::new();
    for finding in findings {
        if finding.class_name == FAILURE_CLASS_CLOSED_BLOCK_EDGE
            || finding.class_name == FAILURE_CLASS_TRANSITIVE_BLOCK_EDGE
        {
            edges.insert((finding.issue_id.clone(), finding.depends_on_id.clone()));
        }
    }
    edges.into_iter().collect()
}

pub fn print_compactness_findings(
    findings: &[CompactnessFinding],
    repo_root: Option<&Path>,
    issues_path: Option<&Path>,
) {
    if findings.is_empty() {
        return;
    }
    println!(
        "[issue-graph] FAIL (compactness drift: {} finding(s))",
        findings.len()
    );
    for finding in findings {
        if let Some(witness_path) = &finding.witness_path {
            println!(
                "  - {} ({} -> {}, witness={})",
                finding.class_name,
                finding.issue_id,
                finding.depends_on_id,
                witness_path.join(" -> ")
            );
        } else {
            println!(
                "  - {} ({} -> {})",
                finding.class_name, finding.issue_id, finding.depends_on_id
            );
        }
    }
    let repo_root_arg = repo_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".".to_string());
    let issues_arg = issues_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".premath/issues.jsonl".to_string());
    println!(
        "  remediation: cargo run --package premath-cli -- issue-graph-compact --repo-root {} --issues {} --mode apply",
        repo_root_arg, issues_arg
    );
}
