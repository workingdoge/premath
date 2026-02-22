//! Canonical in-memory representation of issue/dependency state.
//!
//! This is the memory boundary for `premath-bd`:
//! - load/store JSONL
//! - expose deterministic issue/dependency queries
//! - avoid orchestration concerns (no JJ/Surreal coupling here)

use crate::dependency::DepType;
use crate::dependency::Dependency;
use crate::issue::Issue;
use crate::jsonl::{JsonlError, read_issues_from_path, write_issues_to_path};
use std::collections::BTreeMap;
use std::path::Path;

/// Errors raised while loading or querying the memory store.
#[derive(Debug, thiserror::Error)]
pub enum MemoryStoreError {
    #[error(transparent)]
    Jsonl(#[from] JsonlError),

    #[error("issue not found: {0}")]
    IssueNotFound(String),

    #[error("dependency already exists: {issue_id} -> {depends_on_id} ({dep_type})")]
    DependencyAlreadyExists {
        issue_id: String,
        depends_on_id: String,
        dep_type: String,
    },
}

/// Canonical in-memory state for issues and typed edges.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    issues: BTreeMap<String, Issue>,
}

impl MemoryStore {
    /// Build a store from fully-materialized issues.
    ///
    /// Duplicate IDs are resolved with deterministic last-write-wins semantics,
    /// matching append/overlay behavior in JSONL sync workflows.
    pub fn from_issues(issues: Vec<Issue>) -> Result<Self, MemoryStoreError> {
        let mut index = BTreeMap::new();
        for issue in issues {
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

        let issue = self
            .issue_mut(issue_id)
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

        issue.dependencies.push(Dependency {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            dep_type,
            created_by,
        });
        issue.touch_updated_at();
        Ok(())
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
}
