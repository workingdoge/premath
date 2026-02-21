use crate::cli::IssueCommands;
use premath_bd::{Issue, MemoryStore};
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
