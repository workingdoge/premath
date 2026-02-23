use crate::cli::IssueCommands;
use crate::support::{backend_status_payload, collect_backend_status};
use premath_bd::issue::{issue_type_variants, parse_issue_type};
use premath_bd::{
    AtomicStoreMutationError, ClaimNextRequest, DepType, Issue, MemoryStore,
    claim_next_issue_jsonl, event_stream_ref, migrate_store_to_events, mutate_store_jsonl,
    read_events_from_path, replay_events, replay_events_from_path, store_snapshot_ref,
    stores_equivalent, write_events_to_path,
};
use premath_surreal::QueryCache;
use serde::{Deserialize, Serialize};
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

        IssueCommands::BackendStatus {
            issues,
            repo,
            projection,
            json,
        } => run_backend_status(issues, repo, projection, json),

        IssueCommands::Ready { issues, json } => run_ready(issues, json),

        IssueCommands::Blocked { issues, json } => run_blocked(issues, json),

        IssueCommands::Check {
            issues,
            note_warn_threshold,
            json,
        } => run_check(issues, note_warn_threshold, json),

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

        IssueCommands::ClaimNext {
            assignee,
            lease_id,
            lease_ttl_seconds,
            issues,
            json,
        } => run_claim_next(assignee, lease_id, lease_ttl_seconds, issues, json),

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
            cache,
            json,
        } => run_replay_events(events, issues, cache, json),
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
    issue.issue_type = parse_issue_type(&issue_type).unwrap_or_else(|| {
        eprintln!(
            "error: invalid issue_type `{}` (expected one of: {})",
            issue_type,
            issue_type_variants().join(", ")
        );
        std::process::exit(1);
    });
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

fn run_backend_status(issues: String, repo: String, projection: String, json_output: bool) {
    let status = collect_backend_status(
        PathBuf::from(issues),
        PathBuf::from(repo),
        PathBuf::from(projection),
    );

    if json_output {
        let payload = backend_status_payload("issue.backend-status", &status, None);
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!("premath issue backend-status");
        println!("  issues: {}", status.issues_path.display());
        println!("  issues exists: {}", status.issues_exists);
        println!("  repo: {}", status.repo_root.display());
        println!("  projection: {}", status.projection_path.display());
        println!("  projection exists: {}", status.projection_exists);
        println!("  jj state: {}", status.jj_state);
        if let Some(root) = status.jj_repo_root {
            println!("  jj repo root: {}", root);
        }
        if let Some(change_id) = status.jj_head_change_id {
            println!("  jj head change: {}", change_id);
        }
        if let Some(err) = status.jj_error {
            println!("  jj error: {}", err);
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

fn run_check(issues: String, note_warn_threshold: usize, json_output: bool) {
    let (store, path) = load_store_existing_or_exit(&issues);
    let report = store.check_issue_graph(note_warn_threshold);

    if json_output {
        let payload = json!({
            "action": "issue.check",
            "issuesPath": path.display().to_string(),
            "checkKind": report.check_kind,
            "result": report.result,
            "failureClasses": report.failure_classes,
            "warningClasses": report.warning_classes,
            "errors": report.errors,
            "warnings": report.warnings,
            "summary": report.summary
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else {
        println!(
            "[issue-graph] {} (issues={}, errors={}, warnings={})",
            if report.accepted() { "OK" } else { "FAIL" },
            report.summary.issue_count,
            report.summary.error_count,
            report.summary.warning_count
        );
        for finding in &report.errors {
            println!(
                "  - {} {} ({})",
                finding.issue_id, finding.class, finding.message
            );
        }
        for finding in &report.warnings {
            println!(
                "  - WARN {} {} ({})",
                finding.issue_id, finding.class, finding.message
            );
        }
    }

    if !report.accepted() {
        std::process::exit(1);
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

    let path = PathBuf::from(issues);
    let updated = mutate_store_jsonl(&path, |store| {
        let issue = store
            .issue_mut(&id)
            .ok_or_else(|| format!("issue not found: {id}"))?;

        if issue.status == "closed" {
            return Err(format!("cannot claim closed issue: {id}"));
        }
        if !issue.assignee.is_empty() && issue.assignee != assignee {
            return Err(format!(
                "issue already claimed: {id} (assignee={})",
                issue.assignee
            ));
        }

        if issue.assignee != assignee {
            issue.assignee = assignee.clone();
        }
        if issue.status != "in_progress" {
            issue.set_status("in_progress".to_string());
        } else {
            issue.touch_updated_at();
        }
        Ok((issue.clone(), true))
    })
    .unwrap_or_else(|e| {
        match e {
            AtomicStoreMutationError::Mutation(message) => eprintln!("error: {message}"),
            other => eprintln!("error: {other}"),
        }
        std::process::exit(1);
    });

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

fn run_claim_next(
    assignee: String,
    lease_id: Option<String>,
    lease_ttl_seconds: Option<i64>,
    issues: String,
    json_output: bool,
) {
    let path = PathBuf::from(issues);
    let claim = claim_next_issue_jsonl(
        &path,
        ClaimNextRequest {
            assignee,
            lease_id,
            lease_ttl_seconds,
            now: chrono::Utc::now(),
        },
    )
    .unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    if json_output {
        let issue = claim.issue.as_ref().map(|updated| {
            json!({
                "id": updated.id,
                "title": updated.title,
                "status": updated.status,
                "priority": updated.priority,
                "issueType": updated.issue_type,
                "assignee": updated.assignee,
                "owner": updated.owner,
                "lease": updated.lease.as_ref().map(|lease| {
                    json!({
                        "leaseId": lease.lease_id,
                        "owner": lease.owner,
                        "acquiredAt": lease.acquired_at.to_rfc3339(),
                        "expiresAt": lease.expires_at.to_rfc3339(),
                        "renewedAt": lease.renewed_at.map(|item| item.to_rfc3339()),
                    })
                })
            })
        });
        let payload = json!({
            "action": "issue.claim_next",
            "issuesPath": path.display().to_string(),
            "claimed": issue.is_some(),
            "issue": issue
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else if let Some(updated) = claim.issue {
        println!(
            "premath issue claim-next\n  Claimed: {} -> {} [{}]\n  Path: {}",
            updated.id,
            updated.assignee,
            updated.status,
            path.display()
        );
    } else {
        println!(
            "premath issue claim-next\n  Claimed: none (no ready/open issue)\n  Path: {}",
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
    issue.issue_type = parse_issue_type(&issue_type).unwrap_or_else(|| {
        eprintln!(
            "error: invalid issue_type `{}` (expected one of: {})",
            issue_type,
            issue_type_variants().join(", ")
        );
        std::process::exit(1);
    });
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

const ISSUE_REPLAY_CACHE_SCHEMA: &str = "issue.replay.cache.v1";
const ISSUE_REPLAY_CACHE_BASENAME: &str = "replay-cache.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplayCacheEntry {
    schema: String,
    events_path: String,
    issues_path: String,
    event_stream_ref: String,
    snapshot_ref: String,
    event_count: usize,
    issue_count: usize,
}

fn run_replay_events(events: String, issues: String, cache: Option<String>, json_output: bool) {
    let events_path = PathBuf::from(events);
    if !events_path.exists() {
        eprintln!("error: events file not found: {}", events_path.display());
        std::process::exit(1);
    }

    let events_data = read_events_from_path(&events_path).unwrap_or_else(|e| {
        eprintln!("error: failed to load {}: {e}", events_path.display());
        std::process::exit(1);
    });
    let event_count = events_data.len();
    let event_ref = event_stream_ref(&events_data);

    let issues_path = PathBuf::from(issues);
    let cache_path = resolve_replay_cache_path(&issues_path, cache);
    let events_path_str = events_path.display().to_string();
    let issues_path_str = issues_path.display().to_string();

    let existing_store = if issues_path.exists() {
        let existing = MemoryStore::load_jsonl(&issues_path).unwrap_or_else(|e| {
            eprintln!("error: failed to load {}: {e}", issues_path.display());
            std::process::exit(1);
        });
        Some(existing)
    } else {
        None
    };

    let mut cache_hit = false;
    let issue_count: usize;
    let snapshot_ref: String;
    let equivalent_to_existing: Option<bool>;

    if let Some(existing) = existing_store.as_ref() {
        let existing_snapshot_ref = store_snapshot_ref(existing);
        if let Some(entry) = load_replay_cache_or_none(&cache_path) {
            if entry.events_path == events_path_str
                && entry.issues_path == issues_path_str
                && entry.event_stream_ref == event_ref
                && entry.snapshot_ref == existing_snapshot_ref
            {
                cache_hit = true;
                issue_count = existing.len();
                snapshot_ref = existing_snapshot_ref;
                equivalent_to_existing = Some(true);
            } else {
                let replayed = replay_events(&events_data).unwrap_or_else(|e| {
                    eprintln!("error: failed to replay {}: {e}", events_path.display());
                    std::process::exit(1);
                });
                snapshot_ref = store_snapshot_ref(&replayed);
                issue_count = replayed.len();
                let equivalent = stores_equivalent(existing, &replayed);
                if !equivalent {
                    save_store_or_exit(&replayed, &issues_path);
                }
                equivalent_to_existing = Some(equivalent);
            }
        } else {
            let replayed = replay_events(&events_data).unwrap_or_else(|e| {
                eprintln!("error: failed to replay {}: {e}", events_path.display());
                std::process::exit(1);
            });
            snapshot_ref = store_snapshot_ref(&replayed);
            issue_count = replayed.len();
            let equivalent = stores_equivalent(existing, &replayed);
            if !equivalent {
                save_store_or_exit(&replayed, &issues_path);
            }
            equivalent_to_existing = Some(equivalent);
        }
    } else {
        let replayed = replay_events(&events_data).unwrap_or_else(|e| {
            eprintln!("error: failed to replay {}: {e}", events_path.display());
            std::process::exit(1);
        });
        snapshot_ref = store_snapshot_ref(&replayed);
        issue_count = replayed.len();
        save_store_or_exit(&replayed, &issues_path);
        equivalent_to_existing = None;
    }

    if !cache_hit {
        let cache_entry = ReplayCacheEntry {
            schema: ISSUE_REPLAY_CACHE_SCHEMA.to_string(),
            events_path: events_path_str,
            issues_path: issues_path_str,
            event_stream_ref: event_ref.clone(),
            snapshot_ref: snapshot_ref.clone(),
            event_count,
            issue_count,
        };
        save_replay_cache_or_exit(&cache_path, &cache_entry);
    }

    if json_output {
        let payload = json!({
            "action": "issue.replay-events",
            "eventsPath": events_path.display().to_string(),
            "issuesPath": issues_path.display().to_string(),
            "cachePath": cache_path.display().to_string(),
            "cacheHit": cache_hit,
            "eventCount": event_count,
            "issueCount": issue_count,
            "eventStreamRef": event_ref,
            "snapshotRef": snapshot_ref,
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
            "premath issue replay-events\n  Events: {}\n  Issues: {}\n  Cache: {}\n  Cache hit: {}\n  Event count: {}\n  Issue count: {}\n  Event stream ref: {}\n  Snapshot ref: {}\n  Equivalent to existing: {}",
            events_path.display(),
            issues_path.display(),
            cache_path.display(),
            cache_hit,
            event_count,
            issue_count,
            event_ref,
            snapshot_ref,
            equivalent_label
        );
    }
}

fn resolve_replay_cache_path(issues_path: &Path, cache: Option<String>) -> PathBuf {
    if let Some(cache_path) = cache {
        return PathBuf::from(cache_path);
    }
    if let Some(parent) = issues_path.parent()
        && !parent.as_os_str().is_empty()
    {
        return parent.join(ISSUE_REPLAY_CACHE_BASENAME);
    }
    PathBuf::from(ISSUE_REPLAY_CACHE_BASENAME)
}

fn load_replay_cache_or_none(path: &Path) -> Option<ReplayCacheEntry> {
    if !path.exists() {
        return None;
    }
    let raw = fs::read_to_string(path).ok()?;
    let entry: ReplayCacheEntry = serde_json::from_str(&raw).ok()?;
    if entry.schema != ISSUE_REPLAY_CACHE_SCHEMA {
        return None;
    }
    Some(entry)
}

fn save_replay_cache_or_exit(path: &Path, entry: &ReplayCacheEntry) {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!(
                "error: failed to create replay cache directory {}: {e}",
                parent.display()
            );
            std::process::exit(1);
        });
    }
    let payload = serde_json::to_vec_pretty(entry).unwrap_or_else(|e| {
        eprintln!(
            "error: failed to serialize replay cache {}: {e}",
            path.display()
        );
        std::process::exit(1);
    });
    fs::write(path, payload).unwrap_or_else(|e| {
        eprintln!(
            "error: failed to write replay cache {}: {e}",
            path.display()
        );
        std::process::exit(1);
    });
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
