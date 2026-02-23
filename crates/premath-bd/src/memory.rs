//! Canonical in-memory representation of issue/dependency state.
//!
//! This is the memory boundary for `premath-bd`:
//! - load/store JSONL
//! - expose deterministic issue/dependency queries
//! - avoid orchestration concerns (no JJ/Surreal coupling here)

use crate::dependency::DepType;
use crate::dependency::Dependency;
use crate::issue::{Issue, parse_issue_type};
use crate::issue_graph::{IssueGraphCheckReport, check_issue_graph};
use crate::jsonl::{JsonlError, read_issues_from_path, write_issues_to_path};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

const STATUS_OPEN: &str = "open";
const STATUS_IN_PROGRESS: &str = "in_progress";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyGraphScope {
    Active,
    Full,
}

impl DependencyGraphScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Full => "full",
        }
    }
}

fn is_active_issue_status(status: &str) -> bool {
    let normalized = status.trim().to_ascii_lowercase();
    normalized == STATUS_OPEN || normalized == STATUS_IN_PROGRESS
}

/// Errors raised while loading or querying the memory store.
#[derive(Debug, thiserror::Error)]
pub enum MemoryStoreError {
    #[error(transparent)]
    Jsonl(#[from] JsonlError),

    #[error("issue not found: {0}")]
    IssueNotFound(String),

    #[error("invalid issue_type for {issue_id}: {issue_type}")]
    InvalidIssueType {
        issue_id: String,
        issue_type: String,
    },

    #[error("dependency already exists: {issue_id} -> {depends_on_id} ({dep_type})")]
    DependencyAlreadyExists {
        issue_id: String,
        depends_on_id: String,
        dep_type: String,
    },

    #[error("dependency not found: {issue_id} -> {depends_on_id} ({dep_type})")]
    DependencyNotFound {
        issue_id: String,
        depends_on_id: String,
        dep_type: String,
    },

    #[error("dependency self-loop is not allowed: {issue_id} ({dep_type})")]
    DependencySelfLoop { issue_id: String, dep_type: String },

    #[error(
        "dependency cycle detected: {issue_id} -> {depends_on_id} ({dep_type}); path: {cycle_path}"
    )]
    DependencyCycle {
        issue_id: String,
        depends_on_id: String,
        dep_type: String,
        cycle_path: String,
    },
}

/// Canonical in-memory state for issues and typed edges.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    issues: BTreeMap<String, Issue>,
}

impl MemoryStore {
    fn issue_in_scope(&self, issue: &Issue, scope: DependencyGraphScope) -> bool {
        match scope {
            DependencyGraphScope::Full => true,
            DependencyGraphScope::Active => is_active_issue_status(&issue.status),
        }
    }

    fn issue_id_in_scope(&self, issue_id: &str, scope: DependencyGraphScope) -> bool {
        self.issue(issue_id)
            .is_some_and(|issue| self.issue_in_scope(issue, scope))
    }

    /// Build a store from fully-materialized issues.
    ///
    /// Duplicate IDs are resolved with deterministic last-write-wins semantics,
    /// matching append/overlay behavior in JSONL sync workflows.
    pub fn from_issues(issues: Vec<Issue>) -> Result<Self, MemoryStoreError> {
        let mut index = BTreeMap::new();
        for mut issue in issues {
            let issue_type = parse_issue_type(&issue.issue_type).ok_or_else(|| {
                MemoryStoreError::InvalidIssueType {
                    issue_id: issue.id.clone(),
                    issue_type: issue.issue_type.clone(),
                }
            })?;
            issue.issue_type = issue_type;
            let id = issue.id.clone();
            index.insert(id, issue);
        }
        Ok(Self { issues: index })
    }

    /// Load store state from a JSONL file.
    pub fn load_jsonl(path: impl AsRef<Path>) -> Result<Self, MemoryStoreError> {
        let issues = read_issues_from_path(path)?;
        Self::from_issues(issues)
    }

    /// Persist store state to a JSONL file.
    pub fn save_jsonl(&self, path: impl AsRef<Path>) -> Result<(), MemoryStoreError> {
        let issues: Vec<Issue> = self.issues.values().cloned().collect();
        write_issues_to_path(path, &issues)?;
        Ok(())
    }

    /// Total number of issues in memory.
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Whether the store has zero issues.
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// Lookup one issue by ID.
    pub fn issue(&self, id: &str) -> Option<&Issue> {
        self.issues.get(id)
    }

    /// Lookup one issue by ID (mutable).
    pub fn issue_mut(&mut self, id: &str) -> Option<&mut Issue> {
        self.issues.get_mut(id)
    }

    /// Insert or replace an issue by ID.
    ///
    /// Returns previous value if present.
    pub fn upsert_issue(&mut self, issue: Issue) -> Option<Issue> {
        self.issues.insert(issue.id.clone(), issue)
    }

    /// Add a typed dependency edge to an issue.
    ///
    /// Both endpoints must exist. Duplicate (issue, depends_on, type) triples
    /// are rejected deterministically.
    pub fn add_dependency(
        &mut self,
        issue_id: &str,
        depends_on_id: &str,
        dep_type: DepType,
        created_by: String,
    ) -> Result<(), MemoryStoreError> {
        if self.issue(issue_id).is_none() {
            return Err(MemoryStoreError::IssueNotFound(issue_id.to_string()));
        }
        if self.issue(depends_on_id).is_none() {
            return Err(MemoryStoreError::IssueNotFound(depends_on_id.to_string()));
        }
        if issue_id == depends_on_id {
            return Err(MemoryStoreError::DependencySelfLoop {
                issue_id: issue_id.to_string(),
                dep_type: dep_type.as_str().to_string(),
            });
        }

        let issue = self
            .issue(issue_id)
            .ok_or_else(|| MemoryStoreError::IssueNotFound(issue_id.to_string()))?;
        if issue
            .dependencies
            .iter()
            .any(|d| d.depends_on_id == depends_on_id && d.dep_type == dep_type)
        {
            return Err(MemoryStoreError::DependencyAlreadyExists {
                issue_id: issue_id.to_string(),
                depends_on_id: depends_on_id.to_string(),
                dep_type: dep_type.as_str().to_string(),
            });
        }
        if let Some(path) = self.find_dependency_path(depends_on_id, issue_id) {
            let mut cycle_path = vec![issue_id.to_string()];
            cycle_path.extend(path);
            return Err(MemoryStoreError::DependencyCycle {
                issue_id: issue_id.to_string(),
                depends_on_id: depends_on_id.to_string(),
                dep_type: dep_type.as_str().to_string(),
                cycle_path: cycle_path.join(" -> "),
            });
        }

        let issue = self
            .issue_mut(issue_id)
            .ok_or_else(|| MemoryStoreError::IssueNotFound(issue_id.to_string()))?;

        issue.dependencies.push(Dependency {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            dep_type,
            created_by,
        });
        issue.touch_updated_at();
        Ok(())
    }

    /// Remove one typed dependency edge from an issue.
    pub fn remove_dependency(
        &mut self,
        issue_id: &str,
        depends_on_id: &str,
        dep_type: DepType,
    ) -> Result<(), MemoryStoreError> {
        let issue = self
            .issue_mut(issue_id)
            .ok_or_else(|| MemoryStoreError::IssueNotFound(issue_id.to_string()))?;
        let before = issue.dependencies.len();
        issue
            .dependencies
            .retain(|dep| !(dep.depends_on_id == depends_on_id && dep.dep_type == dep_type));
        if issue.dependencies.len() == before {
            return Err(MemoryStoreError::DependencyNotFound {
                issue_id: issue_id.to_string(),
                depends_on_id: depends_on_id.to_string(),
                dep_type: dep_type.as_str().to_string(),
            });
        }
        issue.touch_updated_at();
        Ok(())
    }

    /// Replace dependency type for one existing edge.
    pub fn replace_dependency(
        &mut self,
        issue_id: &str,
        depends_on_id: &str,
        from_dep_type: DepType,
        to_dep_type: DepType,
        created_by: String,
    ) -> Result<(), MemoryStoreError> {
        if self.issue(issue_id).is_none() {
            return Err(MemoryStoreError::IssueNotFound(issue_id.to_string()));
        }
        if self.issue(depends_on_id).is_none() {
            return Err(MemoryStoreError::IssueNotFound(depends_on_id.to_string()));
        }
        if issue_id == depends_on_id {
            return Err(MemoryStoreError::DependencySelfLoop {
                issue_id: issue_id.to_string(),
                dep_type: to_dep_type.as_str().to_string(),
            });
        }
        let issue = self
            .issue_mut(issue_id)
            .ok_or_else(|| MemoryStoreError::IssueNotFound(issue_id.to_string()))?;

        let Some(index) = issue
            .dependencies
            .iter()
            .position(|dep| dep.depends_on_id == depends_on_id && dep.dep_type == from_dep_type)
        else {
            return Err(MemoryStoreError::DependencyNotFound {
                issue_id: issue_id.to_string(),
                depends_on_id: depends_on_id.to_string(),
                dep_type: from_dep_type.as_str().to_string(),
            });
        };

        if issue.dependencies.iter().enumerate().any(|(idx, dep)| {
            idx != index && dep.depends_on_id == depends_on_id && dep.dep_type == to_dep_type
        }) {
            return Err(MemoryStoreError::DependencyAlreadyExists {
                issue_id: issue_id.to_string(),
                depends_on_id: depends_on_id.to_string(),
                dep_type: to_dep_type.as_str().to_string(),
            });
        }

        issue.dependencies[index].dep_type = to_dep_type;
        if !created_by.trim().is_empty() {
            issue.dependencies[index].created_by = created_by;
        }
        issue.touch_updated_at();
        Ok(())
    }

    /// Return one deterministic dependency path `start -> ... -> target` if reachable.
    pub fn find_dependency_path(&self, start: &str, target: &str) -> Option<Vec<String>> {
        self.find_dependency_path_in_scope(start, target, DependencyGraphScope::Full)
    }

    /// Return one deterministic dependency path `start -> ... -> target` if reachable
    /// within the requested graph scope.
    pub fn find_dependency_path_in_scope(
        &self,
        start: &str,
        target: &str,
        scope: DependencyGraphScope,
    ) -> Option<Vec<String>> {
        if !self.issue_id_in_scope(start, scope) || !self.issue_id_in_scope(target, scope) {
            return None;
        }
        if self.issue(start).is_none() || self.issue(target).is_none() {
            return None;
        }
        if start == target {
            return Some(vec![start.to_string()]);
        }

        let mut queue: VecDeque<Vec<String>> = VecDeque::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        queue.push_back(vec![start.to_string()]);
        visited.insert(start.to_string());

        while let Some(path) = queue.pop_front() {
            let current = path.last()?.clone();
            let Some(issue) = self.issue(&current) else {
                continue;
            };
            let mut next_ids: Vec<String> = issue
                .dependencies
                .iter()
                .filter(|dep| self.issue_id_in_scope(&dep.depends_on_id, scope))
                .map(|dep| dep.depends_on_id.clone())
                .collect();
            next_ids.sort();
            next_ids.dedup();

            for next in next_ids {
                if next == target {
                    let mut found = path.clone();
                    found.push(next);
                    return Some(found);
                }
                if visited.insert(next.clone()) {
                    let mut next_path = path.clone();
                    next_path.push(next);
                    queue.push_back(next_path);
                }
            }
        }

        None
    }

    /// Return one deterministic cycle path if any cycle exists.
    pub fn find_any_dependency_cycle(&self) -> Option<Vec<String>> {
        self.find_any_dependency_cycle_in_scope(DependencyGraphScope::Full)
    }

    /// Return one deterministic cycle path if any cycle exists in the selected graph scope.
    pub fn find_any_dependency_cycle_in_scope(
        &self,
        scope: DependencyGraphScope,
    ) -> Option<Vec<String>> {
        for issue in self.issues() {
            if !self.issue_in_scope(issue, scope) {
                continue;
            }
            let mut deps = issue.dependencies.clone();
            deps.sort_by(|left, right| {
                (
                    left.depends_on_id.as_str(),
                    left.dep_type.as_str(),
                    left.created_by.as_str(),
                )
                    .cmp(&(
                        right.depends_on_id.as_str(),
                        right.dep_type.as_str(),
                        right.created_by.as_str(),
                    ))
            });
            for dep in deps {
                if !self.issue_id_in_scope(&dep.depends_on_id, scope) {
                    continue;
                }
                if let Some(path) =
                    self.find_dependency_path_in_scope(&dep.depends_on_id, &issue.id, scope)
                {
                    let mut cycle = vec![issue.id.clone()];
                    cycle.extend(path);
                    return Some(cycle);
                }
            }
        }
        None
    }

    /// Iterate all issues in deterministic ID order.
    pub fn issues(&self) -> impl Iterator<Item = &Issue> {
        self.issues.values()
    }

    /// Iterate dependencies declared by `issue_id`.
    pub fn dependencies_of(&self, issue_id: &str) -> impl Iterator<Item = &Dependency> {
        self.issue(issue_id)
            .into_iter()
            .flat_map(|issue| issue.dependencies.iter())
    }

    /// Return the blocking dependencies for `issue_id`.
    pub fn blocking_dependencies_of(&self, issue_id: &str) -> Vec<&Dependency> {
        self.dependencies_of(issue_id)
            .filter(|dep| dep.dep_type.is_blocking())
            .collect()
    }

    /// Compute IDs of open issues that are unblocked.
    ///
    /// Conservative rule: if a blocker issue is missing from the store,
    /// treat the dependency as unresolved and keep the issue blocked.
    pub fn ready_open_ids(&self) -> Vec<String> {
        let mut ready = Vec::new();

        for issue in self.issues() {
            if issue.status != "open" {
                continue;
            }

            let mut blocked = false;
            for dep in self.blocking_dependencies_of(&issue.id) {
                match self.issue(&dep.depends_on_id) {
                    Some(target) if target.status == "closed" => {}
                    Some(_) | None => {
                        blocked = true;
                        break;
                    }
                }
            }

            if !blocked {
                ready.push(issue.id.clone());
            }
        }

        ready
    }

    /// Run deterministic issue-graph contract checks against this store.
    pub fn check_issue_graph(&self, note_warn_threshold: usize) -> IssueGraphCheckReport {
        check_issue_graph(self, note_warn_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn issue(id: &str, status: &str, dependencies: Vec<Dependency>) -> Issue {
        let now = Utc::now();
        Issue {
            id: id.to_string(),
            title: format!("Issue {id}"),
            description: String::new(),
            design: String::new(),
            acceptance_criteria: String::new(),
            notes: String::new(),
            status: status.to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            assignee: String::new(),
            owner: String::new(),
            lease: None,
            created_at: now,
            updated_at: now,
            closed_at: None,
            ephemeral: false,
            mol_type: String::new(),
            labels: Vec::new(),
            dependencies,
            metadata: None,
        }
    }

    #[test]
    fn ready_open_ids_respects_blocking_deps() {
        let dep = Dependency {
            issue_id: "bd-a".to_string(),
            depends_on_id: "bd-b".to_string(),
            dep_type: crate::dependency::DepType::Blocks,
            created_by: String::new(),
        };

        let store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![dep]),
            issue("bd-b", "open", vec![]),
            issue("bd-c", "closed", vec![]),
        ])
        .expect("store should build");

        assert_eq!(store.ready_open_ids(), vec!["bd-b".to_string()]);
    }

    #[test]
    fn ready_open_ids_unblocks_when_blocker_closed() {
        let dep = Dependency {
            issue_id: "bd-a".to_string(),
            depends_on_id: "bd-b".to_string(),
            dep_type: crate::dependency::DepType::Blocks,
            created_by: String::new(),
        };

        let store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![dep]),
            issue("bd-b", "closed", vec![]),
        ])
        .expect("store should build");

        assert_eq!(store.ready_open_ids(), vec!["bd-a".to_string()]);
    }

    #[test]
    fn duplicate_ids_use_last_write_wins() {
        let first = issue("bd-a", "open", vec![]);
        let second = issue("bd-a", "closed", vec![]);

        let store = MemoryStore::from_issues(vec![first, second]).expect("store should build");
        assert_eq!(
            store
                .issue("bd-a")
                .expect("issue must exist after dedupe")
                .status,
            "closed"
        );
    }

    #[test]
    fn add_dependency_requires_existing_endpoints() {
        let mut store = MemoryStore::from_issues(vec![issue("bd-a", "open", vec![])])
            .expect("store should build");

        let err = store
            .add_dependency(
                "bd-a",
                "bd-missing",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect_err("missing endpoint must error");
        assert!(matches!(err, MemoryStoreError::IssueNotFound(id) if id == "bd-missing"));
    }

    #[test]
    fn add_dependency_rejects_duplicates() {
        let mut store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![]),
            issue("bd-b", "open", vec![]),
        ])
        .expect("store should build");

        store
            .add_dependency(
                "bd-a",
                "bd-b",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect("first dep should add");
        let err = store
            .add_dependency(
                "bd-a",
                "bd-b",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect_err("duplicate dep must error");
        assert!(matches!(
            err,
            MemoryStoreError::DependencyAlreadyExists {
                issue_id,
                depends_on_id,
                ..
            } if issue_id == "bd-a" && depends_on_id == "bd-b"
        ));
    }

    #[test]
    fn add_dependency_rejects_self_loop() {
        let mut store = MemoryStore::from_issues(vec![issue("bd-a", "open", vec![])])
            .expect("store should build");

        let err = store
            .add_dependency(
                "bd-a",
                "bd-a",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect_err("self loop must error");
        assert!(matches!(
            err,
            MemoryStoreError::DependencySelfLoop { issue_id, .. } if issue_id == "bd-a"
        ));
    }

    #[test]
    fn add_dependency_rejects_cycles() {
        let mut store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![]),
            issue("bd-b", "open", vec![]),
        ])
        .expect("store should build");

        store
            .add_dependency(
                "bd-a",
                "bd-b",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect("first edge should add");

        let err = store
            .add_dependency(
                "bd-b",
                "bd-a",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect_err("cycle edge must error");
        assert!(matches!(
            err,
            MemoryStoreError::DependencyCycle {
                issue_id,
                depends_on_id,
                ..
            } if issue_id == "bd-b" && depends_on_id == "bd-a"
        ));
    }

    #[test]
    fn remove_dependency_removes_edge() {
        let mut store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![]),
            issue("bd-b", "open", vec![]),
        ])
        .expect("store should build");
        store
            .add_dependency(
                "bd-a",
                "bd-b",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect("edge should add");

        store
            .remove_dependency("bd-a", "bd-b", crate::dependency::DepType::Blocks)
            .expect("edge should remove");
        assert!(
            store
                .dependencies_of("bd-a")
                .all(|dep| dep.depends_on_id != "bd-b")
        );
    }

    #[test]
    fn replace_dependency_updates_edge_type() {
        let mut store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![]),
            issue("bd-b", "open", vec![]),
        ])
        .expect("store should build");
        store
            .add_dependency(
                "bd-a",
                "bd-b",
                crate::dependency::DepType::Related,
                String::new(),
            )
            .expect("edge should add");

        store
            .replace_dependency(
                "bd-a",
                "bd-b",
                crate::dependency::DepType::Related,
                crate::dependency::DepType::Blocks,
                "codex".to_string(),
            )
            .expect("edge type should replace");

        let deps: Vec<_> = store.dependencies_of("bd-a").collect();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dep_type, crate::dependency::DepType::Blocks);
        assert_eq!(deps[0].created_by, "codex");
    }

    #[test]
    fn find_any_dependency_cycle_returns_deterministic_path() {
        let mut store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![]),
            issue("bd-b", "open", vec![]),
            issue("bd-c", "open", vec![]),
        ])
        .expect("store should build");
        store
            .add_dependency(
                "bd-a",
                "bd-b",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect("edge a->b should add");
        store
            .add_dependency(
                "bd-b",
                "bd-c",
                crate::dependency::DepType::Blocks,
                String::new(),
            )
            .expect("edge b->c should add");
        let issue_c = store.issue_mut("bd-c").expect("bd-c exists");
        issue_c.dependencies.push(Dependency {
            issue_id: "bd-c".to_string(),
            depends_on_id: "bd-a".to_string(),
            dep_type: crate::dependency::DepType::Blocks,
            created_by: String::new(),
        });

        let cycle = store
            .find_any_dependency_cycle()
            .expect("cycle should be detected");
        assert_eq!(
            cycle,
            vec![
                "bd-a".to_string(),
                "bd-b".to_string(),
                "bd-c".to_string(),
                "bd-a".to_string()
            ]
        );
    }

    #[test]
    fn scoped_cycle_diagnostics_ignore_closed_cycles_for_active_scope() {
        let dep_ab = Dependency {
            issue_id: "bd-a".to_string(),
            depends_on_id: "bd-b".to_string(),
            dep_type: crate::dependency::DepType::Blocks,
            created_by: String::new(),
        };
        let dep_ba = Dependency {
            issue_id: "bd-b".to_string(),
            depends_on_id: "bd-a".to_string(),
            dep_type: crate::dependency::DepType::Blocks,
            created_by: String::new(),
        };
        let store = MemoryStore::from_issues(vec![
            issue("bd-a", "closed", vec![dep_ab]),
            issue("bd-b", "closed", vec![dep_ba]),
            issue("bd-c", "open", vec![]),
        ])
        .expect("store should build");

        assert!(
            store
                .find_any_dependency_cycle_in_scope(DependencyGraphScope::Active)
                .is_none(),
            "closed-only cycle should be ignored in active scope"
        );
        assert_eq!(
            store.find_any_dependency_cycle_in_scope(DependencyGraphScope::Full),
            Some(vec![
                "bd-a".to_string(),
                "bd-b".to_string(),
                "bd-a".to_string()
            ])
        );
    }
}
