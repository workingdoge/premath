//! Event-log surface for issue memory migration/replay.
//!
//! `issue.event.v1` is the minimal expressive envelope:
//! - deterministic migration from snapshot JSONL (`issues.jsonl`)
//! - deterministic replay back to canonical `MemoryStore`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::dependency::{DepType, Dependency};
use crate::issue::Issue;
use crate::memory::MemoryStore;

pub const ISSUE_EVENT_SCHEMA: &str = "issue.event.v1";

fn default_issue_event_schema() -> String {
    ISSUE_EVENT_SCHEMA.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum IssueEventAction {
    UpsertIssue {
        issue: Issue,
    },
    AddDependency {
        depends_on_id: String,
        #[serde(rename = "type", alias = "dep_type")]
        dep_type: DepType,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        created_by: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueEvent {
    #[serde(default = "default_issue_event_schema")]
    pub schema: String,
    pub event_id: String,
    pub issue_id: String,
    pub occurred_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub actor: String,
    #[serde(flatten)]
    pub action: IssueEventAction,
}

impl IssueEvent {
    pub fn upsert(issue: Issue) -> Self {
        let issue_id = issue.id.clone();
        Self {
            schema: ISSUE_EVENT_SCHEMA.to_string(),
            event_id: format!("issue.upsert:{issue_id}"),
            issue_id,
            occurred_at: issue.updated_at,
            actor: String::new(),
            action: IssueEventAction::UpsertIssue { issue },
        }
    }

    pub fn add_dependency(
        issue_id: impl Into<String>,
        dependency: Dependency,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        let issue_id = issue_id.into();
        let target = dependency.depends_on_id.clone();
        let dep_kind = dependency.dep_type.as_str().to_string();
        Self {
            schema: ISSUE_EVENT_SCHEMA.to_string(),
            event_id: format!("issue.add_dependency:{issue_id}:{target}:{dep_kind}"),
            issue_id,
            occurred_at,
            actor: String::new(),
            action: IssueEventAction::AddDependency {
                depends_on_id: dependency.depends_on_id,
                dep_type: dependency.dep_type,
                created_by: dependency.created_by,
            },
        }
    }
}

pub fn read_events(reader: impl BufRead) -> Result<Vec<IssueEvent>, EventError> {
    let mut events = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| EventError::Io(line_no + 1, e.to_string()))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let event: IssueEvent = serde_json::from_str(trimmed)
            .map_err(|e| EventError::Parse(line_no + 1, e.to_string()))?;
        events.push(event);
    }
    Ok(events)
}

pub fn write_events(writer: &mut impl Write, events: &[IssueEvent]) -> Result<(), EventError> {
    for event in events {
        let line =
            serde_json::to_string(event).map_err(|e| EventError::Serialize(e.to_string()))?;
        writeln!(writer, "{line}").map_err(|e| EventError::Io(0, e.to_string()))?;
    }
    Ok(())
}

pub fn read_events_from_path(path: impl AsRef<Path>) -> Result<Vec<IssueEvent>, EventError> {
    let file = File::open(path.as_ref())
        .map_err(|e| EventError::Io(0, format!("{}: {e}", path.as_ref().display())))?;
    let reader = std::io::BufReader::new(file);
    read_events(reader)
}

pub fn write_events_to_path(
    path: impl AsRef<Path>,
    events: &[IssueEvent],
) -> Result<(), EventError> {
    let mut file = File::create(path.as_ref())
        .map_err(|e| EventError::Io(0, format!("{}: {e}", path.as_ref().display())))?;
    write_events(&mut file, events)
}

pub fn replay_events_from_path(path: impl AsRef<Path>) -> Result<MemoryStore, EventError> {
    let events = read_events_from_path(path)?;
    replay_events(&events)
}

pub fn migrate_store_to_events(store: &MemoryStore) -> Vec<IssueEvent> {
    let mut events = Vec::new();
    for issue in store.issues() {
        let mut issue_for_upsert = issue.clone();
        let mut dependencies = issue_for_upsert.dependencies.clone();
        sort_dependencies(&mut dependencies);
        issue_for_upsert.dependencies.clear();
        events.push(IssueEvent::upsert(issue_for_upsert));

        for dependency in dependencies {
            events.push(IssueEvent::add_dependency(
                issue.id.clone(),
                dependency,
                issue.updated_at,
            ));
        }
    }
    events
}

pub fn replay_events(events: &[IssueEvent]) -> Result<MemoryStore, EventError> {
    let mut store = MemoryStore::default();
    let mut pending_dependencies: Vec<IssueEvent> = Vec::new();

    for event in events {
        if event.schema != ISSUE_EVENT_SCHEMA {
            return Err(EventError::UnsupportedSchema(event.schema.clone()));
        }

        match &event.action {
            IssueEventAction::UpsertIssue { issue } => {
                if issue.id != event.issue_id {
                    return Err(EventError::MismatchedIssueId {
                        event_id: event.event_id.clone(),
                        envelope_issue_id: event.issue_id.clone(),
                        payload_issue_id: issue.id.clone(),
                    });
                }
                store.upsert_issue(issue.clone());
            }

            IssueEventAction::AddDependency {
                depends_on_id,
                dep_type,
                created_by,
            } => {
                let applied = try_apply_dependency(
                    &mut store,
                    &event.issue_id,
                    depends_on_id,
                    dep_type,
                    created_by,
                );
                if !applied {
                    pending_dependencies.push(event.clone());
                }
            }
        }
    }

    let mut made_progress = true;
    while made_progress && !pending_dependencies.is_empty() {
        made_progress = false;
        let mut unresolved = Vec::new();

        for event in pending_dependencies {
            match &event.action {
                IssueEventAction::AddDependency {
                    depends_on_id,
                    dep_type,
                    created_by,
                } => {
                    if try_apply_dependency(
                        &mut store,
                        &event.issue_id,
                        depends_on_id,
                        dep_type,
                        created_by,
                    ) {
                        made_progress = true;
                    } else {
                        unresolved.push(event);
                    }
                }
                IssueEventAction::UpsertIssue { .. } => {
                    return Err(EventError::InvalidPendingEvent(event.event_id.clone()));
                }
            }
        }

        pending_dependencies = unresolved;
    }

    if let Some(event) = pending_dependencies.first()
        && let IssueEventAction::AddDependency { depends_on_id, .. } = &event.action
    {
        return Err(EventError::UnresolvedDependency {
            event_id: event.event_id.clone(),
            issue_id: event.issue_id.clone(),
            depends_on_id: depends_on_id.clone(),
        });
    }

    Ok(store)
}

fn try_apply_dependency(
    store: &mut MemoryStore,
    issue_id: &str,
    depends_on_id: &str,
    dep_type: &DepType,
    created_by: &str,
) -> bool {
    if store.issue(issue_id).is_none() || store.issue(depends_on_id).is_none() {
        return false;
    }

    let issue = store
        .issue_mut(issue_id)
        .expect("issue existence checked above");
    let duplicate = issue
        .dependencies
        .iter()
        .any(|dep| dep.depends_on_id == depends_on_id && dep.dep_type == *dep_type);
    if duplicate {
        return true;
    }

    issue.dependencies.push(Dependency {
        issue_id: issue_id.to_string(),
        depends_on_id: depends_on_id.to_string(),
        dep_type: dep_type.clone(),
        created_by: created_by.to_string(),
    });
    true
}

pub fn stores_equivalent(left: &MemoryStore, right: &MemoryStore) -> bool {
    canonical_store(left) == canonical_store(right)
}

fn canonical_store(store: &MemoryStore) -> Vec<String> {
    store.issues().map(canonical_issue).collect()
}

fn canonical_issue(issue: &Issue) -> String {
    let mut snapshot = issue.clone();
    sort_dependencies(&mut snapshot.dependencies);
    serde_json::to_string(&snapshot).expect("issue snapshot should serialize")
}

fn sort_dependencies(dependencies: &mut [Dependency]) {
    dependencies.sort_by(|left, right| {
        (
            left.depends_on_id.as_str(),
            left.dep_type.as_str(),
            left.created_by.as_str(),
            left.issue_id.as_str(),
        )
            .cmp(&(
                right.depends_on_id.as_str(),
                right.dep_type.as_str(),
                right.created_by.as_str(),
                right.issue_id.as_str(),
            ))
    });
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("line {0}: I/O error: {1}")]
    Io(usize, String),

    #[error("line {0}: parse error: {1}")]
    Parse(usize, String),

    #[error("serialization error: {0}")]
    Serialize(String),

    #[error("unsupported event schema: {0}")]
    UnsupportedSchema(String),

    #[error(
        "replay issue-id mismatch for event {event_id}: envelope={envelope_issue_id}, payload={payload_issue_id}"
    )]
    MismatchedIssueId {
        event_id: String,
        envelope_issue_id: String,
        payload_issue_id: String,
    },

    #[error("replay unresolved dependency for event {event_id}: {issue_id} -> {depends_on_id}")]
    UnresolvedDependency {
        event_id: String,
        issue_id: String,
        depends_on_id: String,
    },

    #[error("replay invariant violated: pending non-dependency event {0}")]
    InvalidPendingEvent(String),
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
    fn migrate_replay_roundtrip_is_equivalent() {
        let store = MemoryStore::from_issues(vec![
            issue(
                "bd-a",
                "open",
                vec![
                    Dependency {
                        issue_id: "bd-a".to_string(),
                        depends_on_id: "bd-c".to_string(),
                        dep_type: DepType::Related,
                        created_by: "agent-b".to_string(),
                    },
                    Dependency {
                        issue_id: "bd-a".to_string(),
                        depends_on_id: "bd-b".to_string(),
                        dep_type: DepType::Blocks,
                        created_by: "agent-a".to_string(),
                    },
                ],
            ),
            issue("bd-b", "open", vec![]),
            issue("bd-c", "closed", vec![]),
        ])
        .expect("store should build");

        let events_a = migrate_store_to_events(&store);
        let events_b = migrate_store_to_events(&store);
        assert_eq!(
            serde_json::to_string(&events_a).expect("events serialize"),
            serde_json::to_string(&events_b).expect("events serialize"),
        );

        match &events_a[0].action {
            IssueEventAction::UpsertIssue { issue } => assert!(issue.dependencies.is_empty()),
            _ => panic!("first event should be issue upsert"),
        }

        let replayed = replay_events(&events_a).expect("replay should succeed");
        assert!(stores_equivalent(&store, &replayed));
    }

    #[test]
    fn read_write_roundtrip_preserves_events() {
        let store = MemoryStore::from_issues(vec![
            issue("bd-a", "open", vec![]),
            issue("bd-b", "closed", vec![]),
        ])
        .expect("store should build");
        let events = migrate_store_to_events(&store);

        let mut bytes = Vec::new();
        write_events(&mut bytes, &events).expect("event write should succeed");
        let parsed = read_events(std::io::Cursor::new(bytes)).expect("event read should succeed");

        assert_eq!(
            serde_json::to_string(&events).expect("events serialize"),
            serde_json::to_string(&parsed).expect("events serialize"),
        );
    }
}
