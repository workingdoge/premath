use crate::cli::IssueCommands;
use premath_bd::{
    DepType, Issue, MemoryStore, migrate_store_to_events, read_events_from_path,
    replay_events_from_path, stores_equivalent, write_events_to_path,
};
use premath_surreal::QueryCache;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(command: IssueCommands) {
    match command {
        IssueCommands::Add {
            title,
            id,
            description,
            status,
            priority,
            issue_type,
            assignee,
            owner,
            issues,
            json,
        } => run_add(
            title,
            id,
            description,
            status,
            priority,
            issue_type,
            assignee,
            owner,
            issues,
            json,
        ),

        IssueCommands::List {
            status,
            assignee,
            issues,
            json,
        } => run_list(status, assignee, issues, json),

        IssueCommands::Ready { issues, json } => run_ready(issues, json),

        IssueCommands::Blocked { issues, json } => run_blocked(issues, json),

        IssueCommands::Update {
            id,
            title,
            description,
            notes,
            status,
            priority,
            assignee,
            owner,
            issues,
            json,
        } => run_update(
            id,
            title,
            description,
            notes,
            status,
            priority,
            assignee,
            owner,
            issues,
            json,
        ),

        IssueCommands::Claim {
            id,
            assignee,
            issues,
            json,
        } => run_claim(id, assignee, issues, json),

        IssueCommands::Discover {
            parent_issue_id,
            title,
            id,
            description,
            priority,
            issue_type,
            assignee,
            owner,
            issues,
            json,
        } => run_discover(
            parent_issue_id,
            title,
            id,
            description,
            priority,
            issue_type,
            assignee,
            owner,
            issues,
            json,
        ),

        IssueCommands::MigrateEvents {
            issues,
            events,
            json,
        } => run_migrate_events(issues, events, json),

        IssueCommands::ReplayEvents {
            events,
            issues,
            json,
        } => run_replay_events(events, issues, json),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_add(
    title: String,
    id: Option<String>,
    description: String,
    status: String,
    priority: i32,
    issue_type: String,
    assignee: String,
    owner: String,
    issues: String,
    json_output: bool,
) {
    let (mut store, path) = load_store_or_empty_or_exit(&issues);
    let issue_id = id.unwrap_or_else(|| next_issue_id(&store));

    if store.issue(&issue_id).is_some() {
        eprintln!("error: issue already exists: {issue_id}");
        std::process::exit(1);
    }

    let mut issue = Issue::new(issue_id.clone(), title);
    issue.description = description;
    issue.priority = priority;
    issue.issue_type = issue_type;
    issue.assignee = assignee;
    issue.owner = owner;
    issue.set_status(status);
    let persisted = issue.clone();

    store.upsert_issue(issue);
    save_store_or_exit(&store, &path);

    if json_output {
        let payload = json!({
            "action": "issue.add",
            "issuesPath": path.display().to_string(),
            "issue": {
                "id": persisted.id,
                "title": persisted.title,
                "status": persisted.status,
                "priority": persisted.priority,
                "issueType": persisted.issue_type,
                "assignee": persisted.assignee,
                "owner": persisted.owner
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue add\n  Added: {} [{}]\n  Path: {}",
            persisted.id,
            persisted.status,
            path.display()
        );
    }
}

fn run_list(status: Option<String>, assignee: Option<String>, issues: String, json_output: bool) {
    let (store, path) = load_store_existing_or_exit(&issues);

    let rows: Vec<&Issue> = store
        .issues()
        .filter(|issue| status.as_ref().is_none_or(|s| issue.status == *s))
        .filter(|issue| assignee.as_ref().is_none_or(|a| issue.assignee == *a))
        .collect();

    if json_output {
        let items = rows
            .iter()
            .map(|issue| {
                json!({
                    "id": issue.id,
                    "title": issue.title,
                    "status": issue.status,
                    "priority": issue.priority,
                    "issueType": issue.issue_type,
                    "assignee": issue.assignee
                })
            })
            .collect::<Vec<_>>();
        let payload = json!({
            "action": "issue.list",
            "issuesPath": path.display().to_string(),
            "count": items.len(),
            "items": items
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue list\n  Path: {}\n  Count: {}",
            path.display(),
            rows.len()
        );
        for issue in rows {
            println!(
                "  - {} [{} p{}] {}",
                issue.id, issue.status, issue.priority, issue.title
            );
        }
    }
}

fn run_ready(issues: String, json_output: bool) {
    let (store, path) = load_store_existing_or_exit(&issues);
    let cache = QueryCache::hydrate(&store);
    let ids = cache.ready_open_issue_ids();

    if json_output {
        let items = ids
            .iter()
            .filter_map(|id| cache.issue(id))
            .map(|issue| {
                json!({
                    "id": issue.id,
                    "title": issue.title,
                    "status": issue.status,
                    "priority": issue.priority
                })
            })
            .collect::<Vec<_>>();
        let payload = json!({
            "action": "issue.ready",
            "issuesPath": path.display().to_string(),
            "count": items.len(),
            "items": items
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue ready\n  Path: {}\n  Count: {}",
            path.display(),
            ids.len()
        );
        for id in ids {
            if let Some(issue) = cache.issue(&id) {
                println!("  - {} [p{}] {}", issue.id, issue.priority, issue.title);
            }
        }
    }
}

#[derive(Debug)]
struct BlockedDependency {
    issue_id: String,
    depends_on_id: String,
    dep_type: String,
    created_by: String,
    blocker_status: Option<String>,
    blocker_missing: bool,
}

#[derive(Debug)]
struct BlockedIssueRow {
    id: String,
    title: String,
    status: String,
    priority: i32,
    blockers: Vec<BlockedDependency>,
}

fn run_blocked(issues: String, json_output: bool) {
    let (store, path) = load_store_existing_or_exit(&issues);
    let cache = QueryCache::hydrate(&store);

    let rows = store
        .issues()
        .filter(|issue| issue.status != "closed")
        .filter_map(|issue| {
            let blockers = store
                .blocking_dependencies_of(&issue.id)
                .into_iter()
                .filter_map(|dep| {
                    let blocker = cache.issue(&dep.depends_on_id);
                    let unresolved = blocker.is_none_or(|b| b.status != "closed");
                    if !unresolved {
                        return None;
                    }

                    Some(BlockedDependency {
                        issue_id: dep.issue_id.clone(),
                        depends_on_id: dep.depends_on_id.clone(),
                        dep_type: dep.dep_type.as_str().to_string(),
                        created_by: dep.created_by.clone(),
                        blocker_status: blocker.map(|b| b.status.clone()),
                        blocker_missing: blocker.is_none(),
                    })
                })
                .collect::<Vec<_>>();

            if blockers.is_empty() {
                return None;
            }

            Some(BlockedIssueRow {
                id: issue.id.clone(),
                title: issue.title.clone(),
                status: issue.status.clone(),
                priority: issue.priority,
                blockers,
            })
        })
        .collect::<Vec<_>>();

    if json_output {
        let items = rows
            .iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "title": row.title,
                    "status": row.status,
                    "priority": row.priority,
                    "blockers": row.blockers.iter().map(|blocker| {
                        json!({
                            "issueId": blocker.issue_id,
                            "dependsOnId": blocker.depends_on_id,
                            "type": blocker.dep_type,
                            "createdBy": blocker.created_by,
                            "blockerStatus": blocker.blocker_status,
                            "blockerMissing": blocker.blocker_missing
                        })
                    }).collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();

        let payload = json!({
            "action": "issue.blocked",
            "issuesPath": path.display().to_string(),
            "count": items.len(),
            "items": items
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue blocked\n  Path: {}\n  Count: {}",
            path.display(),
            rows.len()
        );
        for row in rows {
            println!(
                "  - {} [{} p{}] {}",
                row.id, row.status, row.priority, row.title
            );
            for blocker in row.blockers {
                let status = blocker
                    .blocker_status
                    .unwrap_or_else(|| "missing".to_string());
                println!(
                    "    blocker: {} (type={}, status={}, created_by={})",
                    blocker.depends_on_id, blocker.dep_type, status, blocker.created_by
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_update(
    id: String,
    title: Option<String>,
    description: Option<String>,
    notes: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
    assignee: Option<String>,
    owner: Option<String>,
    issues: String,
    json_output: bool,
) {
    let (mut store, path) = load_store_existing_or_exit(&issues);
    let updated = {
        let issue = store.issue_mut(&id).unwrap_or_else(|| {
            eprintln!("error: issue not found: {id}");
            std::process::exit(1);
        });

        let mut changed = false;
        let mut status_changed = false;

        if let Some(next) = title {
            issue.title = next;
            changed = true;
        }
        if let Some(next) = description {
            issue.description = next;
            changed = true;
        }
        if let Some(next) = notes {
            issue.notes = next;
            changed = true;
        }
        if let Some(next) = priority {
            issue.priority = next;
            changed = true;
        }
        if let Some(next) = assignee {
            issue.assignee = next;
            changed = true;
        }
        if let Some(next) = owner {
            issue.owner = next;
            changed = true;
        }
        if let Some(next) = status {
            issue.set_status(next);
            changed = true;
            status_changed = true;
        }

        if !changed {
            eprintln!("error: no update fields provided");
            std::process::exit(1);
        }

        if !status_changed {
            issue.touch_updated_at();
        }

        issue.clone()
    };

    save_store_or_exit(&store, &path);

    if json_output {
        let payload = json!({
            "action": "issue.update",
            "issuesPath": path.display().to_string(),
            "issue": {
                "id": updated.id,
                "title": updated.title,
                "status": updated.status,
                "priority": updated.priority,
                "issueType": updated.issue_type,
                "assignee": updated.assignee,
                "owner": updated.owner
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue update\n  Updated: {} [{}]\n  Path: {}",
            updated.id,
            updated.status,
            path.display()
        );
    }
}

fn run_claim(id: String, assignee: String, issues: String, json_output: bool) {
    let assignee = assignee.trim().to_string();
    if assignee.is_empty() {
        eprintln!("error: assignee is required");
        std::process::exit(1);
    }

    let (mut store, path) = load_store_existing_or_exit(&issues);
    let updated = {
        let issue = store.issue_mut(&id).unwrap_or_else(|| {
            eprintln!("error: issue not found: {id}");
            std::process::exit(1);
        });

        if issue.status == "closed" {
            eprintln!("error: cannot claim closed issue: {id}");
            std::process::exit(1);
        }
        if !issue.assignee.is_empty() && issue.assignee != assignee {
            eprintln!(
                "error: issue already claimed: {id} (assignee={})",
                issue.assignee
            );
            std::process::exit(1);
        }

        if issue.assignee != assignee {
            issue.assignee = assignee.clone();
        }
        if issue.status != "in_progress" {
            issue.set_status("in_progress".to_string());
        } else {
            issue.touch_updated_at();
        }
        issue.clone()
    };

    save_store_or_exit(&store, &path);

    if json_output {
        let payload = json!({
            "action": "issue.claim",
            "issuesPath": path.display().to_string(),
            "issue": {
                "id": updated.id,
                "title": updated.title,
                "status": updated.status,
                "priority": updated.priority,
                "issueType": updated.issue_type,
                "assignee": updated.assignee,
                "owner": updated.owner
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue claim\n  Claimed: {} -> {} [{}]\n  Path: {}",
            updated.id,
            updated.assignee,
            updated.status,
            path.display()
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn run_discover(
    parent_issue_id: String,
    title: String,
    id: Option<String>,
    description: String,
    priority: i32,
    issue_type: String,
    assignee: String,
    owner: String,
    issues: String,
    json_output: bool,
) {
    let (mut store, path) = load_store_existing_or_exit(&issues);
    if store.issue(&parent_issue_id).is_none() {
        eprintln!("error: parent issue not found: {parent_issue_id}");
        std::process::exit(1);
    }

    let issue_id = id.unwrap_or_else(|| next_issue_id(&store));
    if store.issue(&issue_id).is_some() {
        eprintln!("error: issue already exists: {issue_id}");
        std::process::exit(1);
    }

    let mut issue = Issue::new(issue_id.clone(), title);
    issue.description = description;
    issue.priority = priority;
    issue.issue_type = issue_type;
    issue.assignee = assignee;
    issue.owner = owner;
    issue.set_status("open".to_string());

    store.upsert_issue(issue);
    store
        .add_dependency(
            &issue_id,
            &parent_issue_id,
            DepType::DiscoveredFrom,
            String::new(),
        )
        .unwrap_or_else(|e| {
            eprintln!("error: failed to add discovered-from dependency: {e}");
            std::process::exit(1);
        });

    let persisted = store.issue(&issue_id).expect("discovered issue must exist");
    save_store_or_exit(&store, &path);

    if json_output {
        let payload = json!({
            "action": "issue.discover",
            "issuesPath": path.display().to_string(),
            "issue": {
                "id": persisted.id,
                "title": persisted.title,
                "status": persisted.status,
                "priority": persisted.priority,
                "issueType": persisted.issue_type,
                "assignee": persisted.assignee,
                "owner": persisted.owner
            },
            "dependency": {
                "issueId": issue_id,
                "dependsOnId": parent_issue_id,
                "type": "discovered-from"
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue discover\n  Added: {} [open]\n  Linked: {} -> {} (discovered-from)\n  Path: {}",
            persisted.id,
            persisted.id,
            parent_issue_id,
            path.display()
        );
    }
}

fn run_migrate_events(issues: String, events: String, json_output: bool) {
    let (store, issues_path) = load_store_existing_or_exit(&issues);
    let events_path = PathBuf::from(events);

    if let Some(parent) = events_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!(
                "error: failed to create events directory {}: {e}",
                parent.display()
            );
            std::process::exit(1);
        });
    }

    let events = migrate_store_to_events(&store);
    write_events_to_path(&events_path, &events).unwrap_or_else(|e| {
        eprintln!("error: failed to save {}: {e}", events_path.display());
        std::process::exit(1);
    });

    let replayed = replay_events_from_path(&events_path).unwrap_or_else(|e| {
        eprintln!("error: failed to replay {}: {e}", events_path.display());
        std::process::exit(1);
    });

    let equivalent = stores_equivalent(&store, &replayed);
    if !equivalent {
        eprintln!(
            "error: migration replay mismatch between {} and {}",
            issues_path.display(),
            events_path.display()
        );
        std::process::exit(1);
    }

    if json_output {
        let payload = json!({
            "action": "issue.migrate-events",
            "issuesPath": issues_path.display().to_string(),
            "eventsPath": events_path.display().to_string(),
            "issueCount": store.len(),
            "eventCount": events.len(),
            "equivalent": equivalent
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "premath issue migrate-events\n  Issues: {}\n  Events: {}\n  Issue count: {}\n  Event count: {}\n  Equivalent replay: {}",
            issues_path.display(),
            events_path.display(),
            store.len(),
            events.len(),
            equivalent
        );
    }
}

fn run_replay_events(events: String, issues: String, json_output: bool) {
    let events_path = PathBuf::from(events);
    if !events_path.exists() {
        eprintln!("error: events file not found: {}", events_path.display());
        std::process::exit(1);
    }

    let event_count = read_events_from_path(&events_path)
        .unwrap_or_else(|e| {
            eprintln!("error: failed to load {}: {e}", events_path.display());
            std::process::exit(1);
        })
        .len();
    let replayed = replay_events_from_path(&events_path).unwrap_or_else(|e| {
        eprintln!("error: failed to replay {}: {e}", events_path.display());
        std::process::exit(1);
    });

    let issues_path = PathBuf::from(issues);
    let equivalent_to_existing = if issues_path.exists() {
        let existing = MemoryStore::load_jsonl(&issues_path).unwrap_or_else(|e| {
            eprintln!("error: failed to load {}: {e}", issues_path.display());
            std::process::exit(1);
        });
        Some(stores_equivalent(&existing, &replayed))
    } else {
        None
    };

    save_store_or_exit(&replayed, &issues_path);

    if json_output {
        let payload = json!({
            "action": "issue.replay-events",
            "eventsPath": events_path.display().to_string(),
            "issuesPath": issues_path.display().to_string(),
            "eventCount": event_count,
            "issueCount": replayed.len(),
            "equivalentToExisting": equivalent_to_existing
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        let equivalent_label = equivalent_to_existing
            .map(|value| value.to_string())
            .unwrap_or_else(|| "n/a".to_string());
        println!(
            "premath issue replay-events\n  Events: {}\n  Issues: {}\n  Event count: {}\n  Issue count: {}\n  Equivalent to existing: {}",
            events_path.display(),
            issues_path.display(),
            event_count,
            replayed.len(),
            equivalent_label
        );
    }
}

fn load_store_existing_or_exit(issues: &str) -> (MemoryStore, PathBuf) {
    let path = PathBuf::from(issues);
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }
    let store = MemoryStore::load_jsonl(&path).unwrap_or_else(|e| {
        eprintln!("error: failed to load {}: {e}", path.display());
        std::process::exit(1);
    });
    (store, path)
}

fn load_store_or_empty_or_exit(issues: &str) -> (MemoryStore, PathBuf) {
    let path = PathBuf::from(issues);
    if path.exists() {
        let store = MemoryStore::load_jsonl(&path).unwrap_or_else(|e| {
            eprintln!("error: failed to load {}: {e}", path.display());
            std::process::exit(1);
        });
        return (store, path);
    }
    (MemoryStore::default(), path)
}

fn save_store_or_exit(store: &MemoryStore, path: &Path) {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!(
                "error: failed to create issues directory {}: {e}",
                parent.display()
            );
            std::process::exit(1);
        });
    }

    store.save_jsonl(path).unwrap_or_else(|e| {
        eprintln!("error: failed to save {}: {e}", path.display());
        std::process::exit(1);
    });
}

fn next_issue_id(store: &MemoryStore) -> String {
    let mut seq = 1usize;
    loop {
        let candidate = format!("bd-{seq}");
        if store.issue(&candidate).is_none() {
            return candidate;
        }
        seq += 1;
    }
}
