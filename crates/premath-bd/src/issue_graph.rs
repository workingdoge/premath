//! Deterministic issue-graph contract checking.

use crate::dependency::DepType;
use crate::issue::{ISSUE_TYPE_EPIC, Issue, normalize_issue_type};
use crate::memory::MemoryStore;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::OnceLock;

pub const ISSUE_GRAPH_CHECK_KIND: &str = "premath.issue_graph.check.v1";
pub const DEFAULT_NOTE_WARN_THRESHOLD: usize = 2000;

const EPIC_TITLE_PREFIX: &str = "[EPIC]";
const STATUS_OPEN: &str = "open";
const STATUS_IN_PROGRESS: &str = "in_progress";

pub const FAILURE_CLASS_EPIC_MISMATCH: &str = "issue_graph.issue_type.epic_mismatch";
pub const FAILURE_CLASS_ACCEPTANCE_MISSING: &str = "issue_graph.acceptance.missing";
pub const FAILURE_CLASS_VERIFICATION_COMMAND_MISSING: &str =
    "issue_graph.verification_command.missing";
pub const FAILURE_CLASS_COMPACTNESS_CLOSED_BLOCK_EDGE: &str =
    "issue_graph.compactness.closed_block_edge";
pub const FAILURE_CLASS_COMPACTNESS_TRANSITIVE_BLOCK_EDGE: &str =
    "issue_graph.compactness.transitive_block_edge";
pub const WARNING_CLASS_NOTES_LARGE: &str = "issue_graph.notes.large";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IssueGraphFinding {
    pub issue_id: String,
    pub class: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IssueGraphSummary {
    pub issue_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub note_warn_threshold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IssueGraphCheckReport {
    pub check_kind: String,
    pub result: String,
    pub failure_classes: Vec<String>,
    pub warning_classes: Vec<String>,
    pub errors: Vec<IssueGraphFinding>,
    pub warnings: Vec<IssueGraphFinding>,
    pub summary: IssueGraphSummary,
}

impl IssueGraphCheckReport {
    pub fn accepted(&self) -> bool {
        self.result == "accepted"
    }
}

fn acceptance_section_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?im)^\s*acceptance(?:\s+criteria)?\s*:")
            .expect("acceptance regex must compile")
    })
}

fn command_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?im)^\s*(?:[-*]\s*)?(?:`)?(?:python3|cargo(?: run)?|premath|sh|nix develop -c|uv run|pytest)\b[^`\n]*(?:`)?\s*$",
        )
        .expect("command-line regex must compile")
    })
}

fn command_inline_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)`(?:python3|cargo(?: run)?|premath|sh|nix develop -c|uv run|pytest)\b[^`]*`",
        )
        .expect("command-inline regex must compile")
    })
}

fn has_acceptance_section(description: &str) -> bool {
    acceptance_section_re().is_match(description)
}

fn has_verification_command(text: &str) -> bool {
    command_line_re().is_match(text) || command_inline_re().is_match(text)
}

fn issue_status(issue: &Issue) -> String {
    issue.status.trim().to_ascii_lowercase()
}

fn collect_classes(findings: &[IssueGraphFinding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.class.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn build_blocks_adjacency(store: &MemoryStore) -> BTreeMap<String, BTreeSet<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for issue in store.issues() {
        for dep in &issue.dependencies {
            if dep.dep_type != DepType::Blocks {
                continue;
            }
            let source = if dep.issue_id.trim().is_empty() {
                issue.id.as_str()
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

    let mut queue = VecDeque::from([vec![start.to_string()]]);
    let mut visited = BTreeSet::from([start.to_string()]);
    while let Some(path) = queue.pop_front() {
        let node = path.last()?;
        for next in adjacency.get(node).into_iter().flatten() {
            if next == target {
                let mut result = path.clone();
                result.push(next.clone());
                return Some(result);
            }
            if visited.insert(next.clone()) {
                let mut next_path = path.clone();
                next_path.push(next.clone());
                queue.push_back(next_path);
            }
        }
    }

    None
}

fn append_compactness_findings(store: &MemoryStore, errors: &mut Vec<IssueGraphFinding>) {
    let adjacency = build_blocks_adjacency(store);
    let statuses = store
        .issues()
        .map(|issue| (issue.id.clone(), issue_status(issue)))
        .collect::<BTreeMap<_, _>>();

    for issue in store.issues() {
        let status = issue_status(issue);
        if status != STATUS_OPEN && status != STATUS_IN_PROGRESS {
            continue;
        }

        let Some(direct_targets) = adjacency.get(&issue.id) else {
            continue;
        };

        for target_id in direct_targets {
            if statuses
                .get(target_id)
                .is_some_and(|status| status == "closed")
            {
                errors.push(IssueGraphFinding {
                    issue_id: issue.id.clone(),
                    class: FAILURE_CLASS_COMPACTNESS_CLOSED_BLOCK_EDGE.to_string(),
                    message: format!("active blocks edge targets closed issue ({target_id})"),
                });
            }
        }

        for target_id in direct_targets {
            if statuses
                .get(target_id)
                .is_some_and(|status| status == "closed")
            {
                continue;
            }
            for candidate_start in direct_targets {
                if candidate_start == target_id {
                    continue;
                }
                let Some(path) = find_path(&adjacency, candidate_start, target_id) else {
                    continue;
                };
                errors.push(IssueGraphFinding {
                    issue_id: issue.id.clone(),
                    class: FAILURE_CLASS_COMPACTNESS_TRANSITIVE_BLOCK_EDGE.to_string(),
                    message: format!(
                        "transitive-redundant blocks edge {} -> {} (witness={})",
                        issue.id,
                        target_id,
                        path.join(" -> ")
                    ),
                });
                break;
            }
        }
    }
}

pub fn check_issue_graph(store: &MemoryStore, note_warn_threshold: usize) -> IssueGraphCheckReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for issue in store.issues() {
        let issue_id = issue.id.clone();
        let issue_type = normalize_issue_type(&issue.issue_type);
        let status = issue_status(issue);
        let is_active = status == STATUS_OPEN || status == STATUS_IN_PROGRESS;
        let description = issue.description.trim().to_string();
        let notes = issue.notes.trim().to_string();
        let combined_text = if description.is_empty() || notes.is_empty() {
            format!("{description}{notes}")
        } else {
            format!("{description}\n{notes}")
        };

        if issue.title.starts_with(EPIC_TITLE_PREFIX) && issue_type != Some(ISSUE_TYPE_EPIC) {
            errors.push(IssueGraphFinding {
                issue_id: issue_id.clone(),
                class: FAILURE_CLASS_EPIC_MISMATCH.to_string(),
                message: format!(
                    "title starts with [EPIC] but issue_type={}",
                    issue_type.unwrap_or("<invalid>")
                ),
            });
        }

        if is_active && !issue.ephemeral {
            if !has_acceptance_section(&description) {
                errors.push(IssueGraphFinding {
                    issue_id: issue_id.clone(),
                    class: FAILURE_CLASS_ACCEPTANCE_MISSING.to_string(),
                    message: "active issue must include an Acceptance section".to_string(),
                });
            }
            if !has_verification_command(&combined_text) {
                errors.push(IssueGraphFinding {
                    issue_id: issue_id.clone(),
                    class: FAILURE_CLASS_VERIFICATION_COMMAND_MISSING.to_string(),
                    message: "active issue must include at least one verification command"
                        .to_string(),
                });
            }
        }

        let notes_len = issue.notes.chars().count();
        if notes_len > note_warn_threshold {
            warnings.push(IssueGraphFinding {
                issue_id,
                class: WARNING_CLASS_NOTES_LARGE.to_string(),
                message: format!(
                    "notes payload exceeds warning threshold (len={}, threshold={})",
                    notes_len, note_warn_threshold
                ),
            });
        }
    }
    append_compactness_findings(store, &mut errors);

    let failure_classes = collect_classes(&errors);
    let warning_classes = collect_classes(&warnings);
    let result = if errors.is_empty() {
        "accepted".to_string()
    } else {
        "rejected".to_string()
    };
    let summary = IssueGraphSummary {
        issue_count: store.len(),
        error_count: errors.len(),
        warning_count: warnings.len(),
        note_warn_threshold,
    };

    IssueGraphCheckReport {
        check_kind: ISSUE_GRAPH_CHECK_KIND.to_string(),
        result,
        failure_classes,
        warning_classes,
        errors,
        warnings,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{ISSUE_TYPE_TASK, parse_issue_type};
    use chrono::Utc;

    fn issue(id: &str, title: &str, issue_type: &str, status: &str, description: &str) -> Issue {
        let now = Utc::now();
        Issue {
            id: id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            design: String::new(),
            acceptance_criteria: String::new(),
            notes: String::new(),
            status: status.to_string(),
            priority: 2,
            issue_type: parse_issue_type(issue_type).unwrap_or_else(|| ISSUE_TYPE_TASK.to_string()),
            assignee: String::new(),
            owner: String::new(),
            lease: None,
            created_at: now,
            updated_at: now,
            closed_at: None,
            ephemeral: false,
            mol_type: String::new(),
            labels: Vec::new(),
            dependencies: Vec::new(),
            metadata: None,
        }
    }

    fn blocks(source: &str, target: &str) -> crate::dependency::Dependency {
        crate::dependency::Dependency {
            issue_id: source.to_string(),
            depends_on_id: target.to_string(),
            dep_type: DepType::Blocks,
            created_by: String::new(),
        }
    }

    #[test]
    fn epic_title_requires_epic_issue_type() {
        let store = MemoryStore::from_issues(vec![issue(
            "bd-epic",
            "[EPIC] Example",
            "task",
            "open",
            "Acceptance:\n- ok\n\nVerification commands:\n- `sh tools/ci/run_task.sh baseline`",
        )])
        .expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(!report.accepted());
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == FAILURE_CLASS_EPIC_MISMATCH)
        );
    }

    #[test]
    fn active_issue_requires_acceptance_section() {
        let store = MemoryStore::from_issues(vec![issue(
            "bd-active",
            "Active issue",
            "task",
            "open",
            "No acceptance section here.\nVerification commands:\n- `sh tools/ci/run_task.sh baseline`",
        )])
        .expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == FAILURE_CLASS_ACCEPTANCE_MISSING)
        );
    }

    #[test]
    fn active_issue_requires_verification_command() {
        let store = MemoryStore::from_issues(vec![issue(
            "bd-active",
            "Active issue",
            "task",
            "open",
            "Acceptance:\n- do something",
        )])
        .expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == FAILURE_CLASS_VERIFICATION_COMMAND_MISSING)
        );
    }

    #[test]
    fn active_issue_accepts_acceptance_and_verification_command() {
        let store = MemoryStore::from_issues(vec![issue(
            "bd-active",
            "Active issue",
            "task",
            "open",
            "Acceptance:\n- complete work\n\nVerification commands:\n- `sh tools/ci/run_task.sh ci-hygiene-check`\n",
        )])
        .expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(report.accepted());
        assert_eq!(report.failure_classes, Vec::<String>::new());
        assert_eq!(report.warning_classes, Vec::<String>::new());
    }

    #[test]
    fn closed_issue_is_exempt_from_active_contract() {
        let store = MemoryStore::from_issues(vec![issue(
            "bd-closed",
            "Closed issue",
            "task",
            "closed",
            "",
        )])
        .expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(report.accepted());
    }

    #[test]
    fn note_length_emits_warning() {
        let mut row = issue("bd-note", "Note-heavy issue", "task", "closed", "");
        row.notes = "x".repeat(12);
        let store = MemoryStore::from_issues(vec![row]).expect("store should build");

        let report = check_issue_graph(&store, 10);
        assert!(report.accepted());
        assert!(
            report
                .warning_classes
                .iter()
                .any(|class| class == WARNING_CLASS_NOTES_LARGE)
        );
    }

    #[test]
    fn compactness_rejects_active_blocks_edge_to_closed_issue() {
        let mut active = issue(
            "bd-a",
            "Active issue",
            "task",
            "open",
            "Acceptance:\n- done\n\nVerification commands:\n- `sh tools/ci/run_task.sh ci-hygiene-check`",
        );
        active.dependencies.push(blocks("bd-a", "bd-b"));
        let closed = issue("bd-b", "Closed issue", "task", "closed", "");
        let store = MemoryStore::from_issues(vec![active, closed]).expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(!report.accepted());
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == FAILURE_CLASS_COMPACTNESS_CLOSED_BLOCK_EDGE)
        );
    }

    #[test]
    fn compactness_rejects_transitive_redundant_blocks_edge() {
        let description = "Acceptance:\n- done\n\nVerification commands:\n- `sh tools/ci/run_task.sh ci-hygiene-check`";
        let mut issue_a = issue("bd-a", "Issue A", "task", "open", description);
        issue_a.dependencies.push(blocks("bd-a", "bd-b"));
        issue_a.dependencies.push(blocks("bd-a", "bd-c"));
        let mut issue_b = issue("bd-b", "Issue B", "task", "open", description);
        issue_b.dependencies.push(blocks("bd-b", "bd-c"));
        let issue_c = issue("bd-c", "Issue C", "task", "open", description);
        let store =
            MemoryStore::from_issues(vec![issue_a, issue_b, issue_c]).expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(!report.accepted());
        assert!(
            report
                .failure_classes
                .iter()
                .any(|class| class == FAILURE_CLASS_COMPACTNESS_TRANSITIVE_BLOCK_EDGE)
        );
        assert!(
            report
                .errors
                .iter()
                .any(|finding| finding.message.contains("bd-a -> bd-c"))
        );
    }

    #[test]
    fn compactness_accepts_non_redundant_blocks_chain() {
        let description = "Acceptance:\n- done\n\nVerification commands:\n- `sh tools/ci/run_task.sh ci-hygiene-check`";
        let mut issue_a = issue("bd-a", "Issue A", "task", "open", description);
        issue_a.dependencies.push(blocks("bd-a", "bd-b"));
        let mut issue_b = issue("bd-b", "Issue B", "task", "open", description);
        issue_b.dependencies.push(blocks("bd-b", "bd-c"));
        let issue_c = issue("bd-c", "Issue C", "task", "open", description);
        let store =
            MemoryStore::from_issues(vec![issue_a, issue_b, issue_c]).expect("store should build");

        let report = check_issue_graph(&store, DEFAULT_NOTE_WARN_THRESHOLD);
        assert!(report.accepted());
    }
}
