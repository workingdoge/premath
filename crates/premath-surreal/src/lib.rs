//! Query/index layer for issue graphs.
//!
//! This crate models the SurrealDB-facing responsibility as a query cache:
//! fast graph lookups over issue/dependency state loaded from `premath-bd`.
//!
//! It does not own canonical storage (that's `premath-bd`) and does not own
//! versioning/snapshots (that's `premath-jj`).

mod observation;
mod trajectory;

pub use observation::{
    DecisionSummary, DeltaSummary, INSTRUCTION_EVENT_KIND, INSTRUCTION_WITNESS_KIND,
    InstructionSummary, LatestObservation, OBSERVATION_KIND, OBSERVATION_SCHEMA, ObservationError,
    ObservationEvent, ObservationIndex, ObservationSummary, ObservationSurface,
    ProjectionMatchMode, ProjectionView, REQUIRED_DECISION_EVENT_KIND, REQUIRED_DECISION_KIND,
    REQUIRED_EVENT_KIND, REQUIRED_WITNESS_KIND, RequiredSummary, build_events, build_surface,
};
pub use trajectory::{
    HARNESS_TRAJECTORY_KIND, HARNESS_TRAJECTORY_SCHEMA, HarnessTrajectoryProjection,
    HarnessTrajectoryRow, TRAJECTORY_PROJECTION_KIND, TrajectoryError, TrajectoryProjectionMode,
    append_trajectory_row, project_trajectory, read_trajectory_rows,
};

use premath_bd::{Dependency, Issue, MemoryStore};
use std::collections::BTreeMap;

/// In-memory query cache shaped like a graph database projection.
#[derive(Debug, Clone, Default)]
pub struct QueryCache {
    issues: BTreeMap<String, Issue>,
    outgoing: BTreeMap<String, Vec<Dependency>>,
    incoming: BTreeMap<String, Vec<Dependency>>,
}

impl QueryCache {
    /// Hydrate query indices from canonical memory state.
    pub fn hydrate(store: &MemoryStore) -> Self {
        let mut issues = BTreeMap::new();
        for issue in store.issues() {
            issues.insert(issue.id.clone(), issue.clone());
        }

        let mut outgoing: BTreeMap<String, Vec<Dependency>> = BTreeMap::new();
        let mut incoming: BTreeMap<String, Vec<Dependency>> = BTreeMap::new();

        for issue in issues.values() {
            for dep in &issue.dependencies {
                outgoing
                    .entry(dep.issue_id.clone())
                    .or_default()
                    .push(dep.clone());
                incoming
                    .entry(dep.depends_on_id.clone())
                    .or_default()
                    .push(dep.clone());
            }
        }

        Self {
            issues,
            outgoing,
            incoming,
        }
    }

    /// Lookup one issue by ID.
    pub fn issue(&self, id: &str) -> Option<&Issue> {
        self.issues.get(id)
    }

    /// Return all issue IDs in deterministic order.
    pub fn issue_ids(&self) -> Vec<String> {
        self.issues.keys().cloned().collect()
    }

    /// Outgoing dependencies from `issue_id`.
    pub fn dependencies_of(&self, issue_id: &str) -> Vec<&Dependency> {
        self.outgoing
            .get(issue_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Incoming dependents pointing at `issue_id`.
    pub fn dependents_of(&self, issue_id: &str) -> Vec<&Dependency> {
        self.incoming
            .get(issue_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// IDs of open issues that are currently unblocked.
    pub fn ready_open_issue_ids(&self) -> Vec<String> {
        let mut ready = Vec::new();

        for issue in self.issues.values() {
            if issue.status != "open" {
                continue;
            }

            let mut blocked = false;
            for dep in self.dependencies_of(&issue.id) {
                if !dep.dep_type.is_blocking() {
                    continue;
                }

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

        ready.sort();
        ready
    }

    /// Return blocker issues for a given issue.
    pub fn blockers_of(&self, issue_id: &str) -> Vec<&Issue> {
        self.dependencies_of(issue_id)
            .into_iter()
            .filter(|dep| dep.dep_type.is_blocking())
            .filter_map(|dep| self.issue(&dep.depends_on_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use premath_bd::{DepType, MemoryStore};

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
    fn ready_query_uses_blocking_semantics() {
        let dep = Dependency {
            issue_id: "bd-a".to_string(),
            depends_on_id: "bd-b".to_string(),
            dep_type: DepType::Blocks,
            created_by: String::new(),
        };

        let store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![dep]),
            issue("bd-b", "open", vec![]),
            issue("bd-c", "closed", vec![]),
        ])
        .expect("store should build");

        let cache = QueryCache::hydrate(&store);
        assert_eq!(cache.ready_open_issue_ids(), vec!["bd-b".to_string()]);
    }
}
