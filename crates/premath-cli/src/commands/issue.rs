use crate::cli::IssueCommands;
use crate::support::{backend_status_payload, collect_backend_status};
use premath_bd::issue::{issue_type_variants, parse_issue_type};
use premath_bd::{
    AtomicStoreMutationError, DepType, Issue, MemoryStore, event_stream_ref,
    migrate_store_to_events, mutate_store_jsonl, read_events_from_path, replay_events,
    replay_events_from_path, store_snapshot_ref, stores_equivalent, write_events_to_path,
};
use premath_surreal::QueryCache;
use premath_transport::{
    IssueClaimNextRequest as TransportIssueClaimNextRequest,
    IssueClaimRequest as TransportIssueClaimRequest, IssueSummary as TransportIssueSummary,
    LeaseActionEnvelope, issue_claim as transport_issue_claim,
    issue_claim_next as transport_issue_claim_next,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

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
            notes_file,
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
            notes_file,
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
    let path = PathBuf::from(issues);
    let resolved_issue_type = parse_issue_type(&issue_type).unwrap_or_else(|| {
        eprintln!(
            "error: invalid issue_type `{}` (expected one of: {})",
            issue_type,
            issue_type_variants().join(", ")
        );
        std::process::exit(1);
    });
    let persisted = mutate_store_jsonl(&path, |store| {
        let issue_id = id.clone().unwrap_or_else(|| next_issue_id(store));

        if store.issue(&issue_id).is_some() {
            return Err(format!("issue already exists: {issue_id}"));
        }

        let mut issue = Issue::new(issue_id, title);
        issue.description = description;
        issue.priority = priority;
        issue.issue_type = resolved_issue_type;
        issue.assignee = assignee;
        issue.owner = owner;
        issue.set_status(status);
        store.upsert_issue(issue.clone());
        Ok((issue, true))
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
    manual_blocked: bool,
    blockers: Vec<BlockedDependency>,
}

fn run_blocked(issues: String, json_output: bool) {
    let (store, path) = load_store_existing_or_exit(&issues);
    let cache = QueryCache::hydrate(&store);

    let rows = store
        .issues()
        .filter(|issue| issue.status != "closed")
        .filter_map(|issue| {
            let manual_blocked = issue.status == "blocked";
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

            if blockers.is_empty() && !manual_blocked {
                return None;
            }

            Some(BlockedIssueRow {
                id: issue.id.clone(),
                title: issue.title.clone(),
                status: issue.status.clone(),
                priority: issue.priority,
                manual_blocked,
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
                    "manualBlocked": row.manual_blocked,
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
                "  - {} [{} p{} manual_blocked={}] {}",
                row.id, row.status, row.priority, row.manual_blocked, row.title
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
    notes_file: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
    assignee: Option<String>,
    owner: Option<String>,
    issues: String,
    json_output: bool,
) {
    let path = PathBuf::from(issues);
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }
    let resolved_notes = resolve_update_notes(notes, notes_file).unwrap_or_else(|message| {
        eprintln!("error: {message}");
        std::process::exit(1);
    });
    let updated = mutate_store_jsonl(&path, |store| {
        let issue = store
            .issue_mut(&id)
            .ok_or_else(|| format!("issue not found: {id}"))?;

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
        if let Some(next) = resolved_notes.clone() {
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
            return Err("no update fields provided".to_string());
        }

        if !status_changed {
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

fn resolve_update_notes(
    notes: Option<String>,
    notes_file: Option<String>,
) -> Result<Option<String>, String> {
    match (notes, notes_file) {
        (Some(_), Some(_)) => {
            Err("`--notes` and `--notes-file` are mutually exclusive".to_string())
        }
        (Some(value), None) => Ok(Some(value)),
        (None, Some(path)) => {
            if path == "-" {
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .map_err(|error| format!("failed to read notes from stdin: {error}"))?;
                return Ok(Some(buffer));
            }
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read notes file `{path}`: {error}"))?;
            Ok(Some(content))
        }
        (None, None) => Ok(None),
    }
}

fn run_claim(id: String, assignee: String, issues: String, json_output: bool) {
    let assignee = assignee.trim().to_string();
    if assignee.is_empty() {
        eprintln!("error: assignee is required");
        std::process::exit(1);
    }
    let path = PathBuf::from(issues);
    let envelope = transport_issue_claim(TransportIssueClaimRequest {
        id: id.clone(),
        assignee,
        lease_id: None,
        lease_ttl_seconds: None,
        lease_expires_at: None,
        issues_path: Some(path.display().to_string()),
    });
    let updated = require_transport_claim_success(envelope);

    if json_output {
        let payload = json!({
            "action": "issue.claim",
            "issuesPath": path.display().to_string(),
            "issue": transport_issue_summary_json(&updated)
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
    let assignee = assignee.trim().to_string();
    if assignee.is_empty() {
        eprintln!("error: assignee is required");
        std::process::exit(1);
    }
    let path = PathBuf::from(issues);
    let issues_path = path.display().to_string();
    const MAX_ATTEMPTS: usize = 64;
    let mut claimed_issue: Option<TransportIssueSummary> = None;
    let mut terminal_error: Option<LeaseActionEnvelope> = None;
    for _ in 0..MAX_ATTEMPTS {
        let envelope = transport_issue_claim_next(TransportIssueClaimNextRequest {
            assignee: assignee.clone(),
            lease_id: lease_id.clone(),
            lease_ttl_seconds,
            issues_path: Some(issues_path.clone()),
        });
        if envelope.result == "accepted" {
            claimed_issue = envelope.issue;
            break;
        }
        if is_retryable_claim_next_rejection(&envelope) {
            thread::sleep(Duration::from_millis(5));
            continue;
        }
        terminal_error = Some(envelope);
        break;
    }
    if let Some(envelope) = terminal_error {
        fail_transport_claim(envelope);
    }

    if json_output {
        let issue = claimed_issue.as_ref().map(transport_issue_summary_json);
        let payload = json!({
            "action": "issue.claim_next",
            "issuesPath": issues_path,
            "claimed": issue.is_some(),
            "issue": issue
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization")
        );
    } else if let Some(updated) = claimed_issue {
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

fn transport_issue_summary_json(issue: &TransportIssueSummary) -> Value {
    json!({
        "id": issue.id,
        "title": issue.title,
        "status": issue.status,
        "priority": issue.priority,
        "issueType": issue.issue_type,
        "assignee": issue.assignee,
        "owner": issue.owner,
        "lease": issue.lease.as_ref().map(|lease| {
            json!({
                "leaseId": lease.lease_id,
                "owner": lease.owner,
                "acquiredAt": lease.acquired_at,
                "expiresAt": lease.expires_at,
                "renewedAt": lease.renewed_at,
            })
        })
    })
}

fn first_failure_class(envelope: &LeaseActionEnvelope) -> String {
    envelope
        .failure_classes
        .first()
        .cloned()
        .unwrap_or_else(|| "transport_claim_rejected".to_string())
}

fn fail_transport_claim(envelope: LeaseActionEnvelope) -> ! {
    let failure_class = first_failure_class(&envelope);
    let diagnostic = envelope
        .diagnostic
        .as_deref()
        .unwrap_or("transport claim rejected");
    eprintln!("error: {diagnostic} ({failure_class})");
    std::process::exit(1);
}

fn require_transport_claim_success(envelope: LeaseActionEnvelope) -> TransportIssueSummary {
    if envelope.result == "accepted" {
        return envelope.issue.unwrap_or_else(|| {
            eprintln!("error: transport claim accepted without issue payload");
            std::process::exit(1);
        });
    }
    fail_transport_claim(envelope);
}

fn is_retryable_claim_next_rejection(envelope: &LeaseActionEnvelope) -> bool {
    envelope
        .failure_classes
        .iter()
        .any(|class| class == "lease_mutation_lock_busy")
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
    let path = PathBuf::from(issues);
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }
    let resolved_issue_type = parse_issue_type(&issue_type).unwrap_or_else(|| {
        eprintln!(
            "error: invalid issue_type `{}` (expected one of: {})",
            issue_type,
            issue_type_variants().join(", ")
        );
        std::process::exit(1);
    });
    let persisted = mutate_store_jsonl(&path, |store| {
        if store.issue(&parent_issue_id).is_none() {
            return Err(format!("parent issue not found: {parent_issue_id}"));
        }

        let issue_id = id.clone().unwrap_or_else(|| next_issue_id(store));
        if store.issue(&issue_id).is_some() {
            return Err(format!("issue already exists: {issue_id}"));
        }

        let mut issue = Issue::new(issue_id.clone(), title);
        issue.description = description;
        issue.priority = priority;
        issue.issue_type = resolved_issue_type;
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
            .map_err(|e| format!("failed to add discovered-from dependency: {e}"))?;
        let persisted = store
            .issue(&issue_id)
            .cloned()
            .ok_or_else(|| format!("discovered issue missing after mutation: {issue_id}"))?;

        Ok((persisted, true))
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
            "action": "issue.discover",
            "issuesPath": path.display().to_string(),
            "issue": {
                "id": persisted.id.clone(),
                "title": persisted.title,
                "status": persisted.status,
                "priority": persisted.priority,
                "issueType": persisted.issue_type,
                "assignee": persisted.assignee,
                "owner": persisted.owner
            },
            "dependency": {
                "issueId": persisted.id.clone(),
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
            persisted.id.as_str(),
            persisted.id.as_str(),
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

#[cfg(test)]
mod tests {
    use super::resolve_update_notes;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn resolve_update_notes_uses_literal_notes_when_provided() {
        let notes = resolve_update_notes(Some("hello".to_string()), None)
            .expect("notes should resolve")
            .expect("notes value");
        assert_eq!(notes, "hello");
    }

    #[test]
    fn resolve_update_notes_reads_notes_file() {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix epoch should be valid")
            .as_nanos();
        path.push(format!(
            "premath-issue-notes-test-{}-{}.txt",
            std::process::id(),
            unique
        ));
        write_file(&path, "line 1\nline 2\n");

        let notes = resolve_update_notes(None, Some(path.display().to_string()))
            .expect("notes file should resolve")
            .expect("notes value");
        assert_eq!(notes, "line 1\nline 2\n");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn resolve_update_notes_rejects_dual_sources() {
        let error = resolve_update_notes(Some("inline".to_string()), Some("notes.txt".to_string()))
            .expect_err("dual source must fail");
        assert!(error.contains("mutually exclusive"));
    }

    fn write_file(path: &PathBuf, content: &str) {
        fs::write(path, content).expect("failed to write test notes file");
    }
}
