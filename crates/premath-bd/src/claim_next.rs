//! Atomic claim-next primitive over JSONL issue memory.
//!
//! This module provides one canonical lock-scoped mutation path that:
//! - selects the next ready/open issue deterministically
//! - claims it for an assignee
//! - writes deterministic lease metadata

use crate::issue::{IssueLease, IssueLeaseState};
use crate::{AtomicStoreMutationError, Issue, mutate_store_jsonl};
use chrono::{DateTime, Duration, Utc};
use std::path::Path;

pub const DEFAULT_LEASE_TTL_SECONDS: i64 = 3600;
pub const MIN_LEASE_TTL_SECONDS: i64 = 30;
pub const MAX_LEASE_TTL_SECONDS: i64 = 86_400;

#[derive(Debug, Clone)]
pub struct ClaimNextRequest {
    pub assignee: String,
    pub lease_id: Option<String>,
    pub lease_ttl_seconds: Option<i64>,
    pub now: DateTime<Utc>,
}

impl ClaimNextRequest {
    pub fn new(assignee: impl Into<String>) -> Self {
        Self {
            assignee: assignee.into(),
            lease_id: None,
            lease_ttl_seconds: None,
            now: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClaimNextOutcome {
    pub issue: Option<Issue>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimNextError {
    #[error("assignee is required")]
    InvalidAssignee,

    #[error(
        "lease_ttl_seconds must be in range [{min}, {max}] (got {actual})",
        min = MIN_LEASE_TTL_SECONDS,
        max = MAX_LEASE_TTL_SECONDS
    )]
    InvalidLeaseTtl { actual: i64 },

    #[error("lease_ttl_seconds overflowed timestamp range")]
    LeaseTtlOverflow,

    #[error(transparent)]
    Atomic(#[from] AtomicStoreMutationError<std::convert::Infallible>),
}

/// Atomically claim the next ready/open issue from a JSONL memory file.
///
/// Deterministic selection order follows `MemoryStore::ready_open_ids()`.
/// When no claimable ready issue exists, returns `issue: None`.
pub fn claim_next_issue_jsonl(
    path: impl AsRef<Path>,
    request: ClaimNextRequest,
) -> Result<ClaimNextOutcome, ClaimNextError> {
    let path = path.as_ref().to_path_buf();
    let assignee = request.assignee.trim().to_string();
    if assignee.is_empty() {
        return Err(ClaimNextError::InvalidAssignee);
    }

    let ttl = request
        .lease_ttl_seconds
        .unwrap_or(DEFAULT_LEASE_TTL_SECONDS);
    if !(MIN_LEASE_TTL_SECONDS..=MAX_LEASE_TTL_SECONDS).contains(&ttl) {
        return Err(ClaimNextError::InvalidLeaseTtl { actual: ttl });
    }

    let lease_expires_at = request
        .now
        .checked_add_signed(Duration::seconds(ttl))
        .ok_or(ClaimNextError::LeaseTtlOverflow)?;

    let claimed = mutate_store_jsonl(&path, |store| {
        let mut claimed: Option<Issue> = None;
        let ready_ids = store.ready_open_ids();
        let mut changed_any = false;

        for candidate_id in ready_ids {
            let mut status_changed = false;
            let mut changed = false;

            let candidate = {
                let issue = match store.issue_mut(&candidate_id) {
                    Some(issue) => issue,
                    None => continue,
                };

                if issue.status == "closed" {
                    continue;
                }

                if issue.lease_state_at(request.now) == IssueLeaseState::Stale {
                    issue.lease = None;
                    changed = true;

                    if issue.status == "in_progress" {
                        issue.set_status("open".to_string());
                        status_changed = true;
                    }
                    if !issue.assignee.is_empty() && issue.assignee != assignee {
                        issue.assignee.clear();
                        changed = true;
                    }
                }

                if let Some(active_lease) = issue
                    .lease
                    .as_ref()
                    .filter(|lease| lease.expires_at > request.now)
                    && active_lease.owner != assignee
                {
                    continue;
                }
                if issue.lease.is_none() && !issue.assignee.is_empty() && issue.assignee != assignee
                {
                    continue;
                }

                if issue.assignee != assignee {
                    issue.assignee = assignee.clone();
                    changed = true;
                }
                if issue.status != "in_progress" {
                    issue.set_status("in_progress".to_string());
                    changed = true;
                    status_changed = true;
                }

                let lease_id = resolve_lease_id(
                    request.lease_id.clone(),
                    issue.id.as_str(),
                    assignee.as_str(),
                );
                let next_lease = match issue.lease.as_ref() {
                    Some(existing)
                        if existing.owner == assignee && existing.lease_id == lease_id =>
                    {
                        IssueLease {
                            lease_id: lease_id.clone(),
                            owner: assignee.clone(),
                            acquired_at: existing.acquired_at,
                            expires_at: lease_expires_at,
                            renewed_at: Some(request.now),
                        }
                    }
                    _ => IssueLease {
                        lease_id,
                        owner: assignee.clone(),
                        acquired_at: request.now,
                        expires_at: lease_expires_at,
                        renewed_at: None,
                    },
                };
                if issue.lease.as_ref() != Some(&next_lease) {
                    issue.lease = Some(next_lease);
                    changed = true;
                }

                if changed && !status_changed {
                    issue.touch_updated_at();
                }
                (issue.clone(), changed)
            };

            if candidate.0.assignee == assignee && candidate.0.status == "in_progress" {
                changed_any = changed_any || candidate.1;
                claimed = Some(candidate.0);
                break;
            }
        }

        Ok((claimed, changed_any))
    })?;

    Ok(ClaimNextOutcome { issue: claimed })
}

fn resolve_lease_id(raw_lease_id: Option<String>, issue_id: &str, assignee: &str) -> String {
    let explicit = raw_lease_id.map(|value| value.trim().to_string());
    match explicit {
        Some(value) if !value.is_empty() => value,
        _ => format!("lease1_{}_{}", lease_token(issue_id), lease_token(assignee)),
    }
}

fn lease_token(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "anon".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryStore;
    use crate::issue::Issue;
    use crate::issue_lock_path;
    use chrono::TimeZone;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn issue(id: &str, status: &str) -> Issue {
        let mut item = Issue::new(id, format!("Issue {id}"));
        item.set_status(status.to_string());
        item
    }

    fn temp_issues_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("premath-bd-{prefix}-{unique}"));
        fs::create_dir_all(&root).expect("temp dir should be created");
        root.join("issues.jsonl")
    }

    #[test]
    fn claim_next_claims_first_ready_issue_deterministically() {
        let path = temp_issues_path("claim-next-first");
        let store = MemoryStore::from_issues(vec![issue("bd-2", "open"), issue("bd-1", "open")])
            .expect("store should build");
        store.save_jsonl(&path).expect("store should save");

        let now = Utc
            .with_ymd_and_hms(2026, 2, 23, 0, 0, 0)
            .single()
            .expect("fixed time");
        let outcome = claim_next_issue_jsonl(
            &path,
            ClaimNextRequest {
                assignee: "worker-a".to_string(),
                lease_id: None,
                lease_ttl_seconds: Some(3600),
                now,
            },
        )
        .expect("claim should succeed");

        let claimed = outcome.issue.expect("one issue should be claimed");
        assert_eq!(claimed.id, "bd-1");
        assert_eq!(claimed.status, "in_progress");
        assert_eq!(claimed.assignee, "worker-a");
        let lease = claimed.lease.expect("lease should be present");
        assert_eq!(lease.lease_id, "lease1_bd-1_worker-a");
        assert_eq!(lease.owner, "worker-a");
        assert_eq!(lease.acquired_at, now);
        assert_eq!(
            lease.expires_at,
            now.checked_add_signed(Duration::seconds(3600))
                .expect("expiry should compute")
        );
        assert_eq!(lease.renewed_at, None);

        let refreshed = MemoryStore::load_jsonl(&path).expect("store should reload");
        assert_eq!(
            refreshed
                .issue("bd-1")
                .expect("claimed issue should exist")
                .status,
            "in_progress"
        );
        assert_eq!(
            refreshed
                .issue("bd-2")
                .expect("other issue should exist")
                .status,
            "open"
        );
    }

    #[test]
    fn claim_next_skips_active_other_owner_and_claims_next() {
        let path = temp_issues_path("claim-next-skip-active");
        let now = Utc
            .with_ymd_and_hms(2026, 2, 23, 0, 0, 0)
            .single()
            .expect("fixed time");

        let mut first = issue("bd-1", "open");
        first.assignee = "worker-b".to_string();
        first.lease = Some(IssueLease {
            lease_id: "lease1_bd-1_worker-b".to_string(),
            owner: "worker-b".to_string(),
            acquired_at: now,
            expires_at: now
                .checked_add_signed(Duration::seconds(3600))
                .expect("expiry should compute"),
            renewed_at: None,
        });
        let store = MemoryStore::from_issues(vec![first, issue("bd-2", "open")])
            .expect("store should build");
        store.save_jsonl(&path).expect("store should save");

        let outcome = claim_next_issue_jsonl(
            &path,
            ClaimNextRequest {
                assignee: "worker-a".to_string(),
                lease_id: None,
                lease_ttl_seconds: Some(3600),
                now,
            },
        )
        .expect("claim should succeed");

        let claimed = outcome.issue.expect("one issue should be claimed");
        assert_eq!(claimed.id, "bd-2");
        assert_eq!(claimed.assignee, "worker-a");
        assert_eq!(claimed.status, "in_progress");
    }

    #[test]
    fn claim_next_returns_none_when_no_ready_issue_exists() {
        let path = temp_issues_path("claim-next-none");
        let store = MemoryStore::from_issues(vec![issue("bd-1", "closed")]).expect("store build");
        store.save_jsonl(&path).expect("store should save");

        let outcome = claim_next_issue_jsonl(
            &path,
            ClaimNextRequest {
                assignee: "worker-a".to_string(),
                lease_id: None,
                lease_ttl_seconds: None,
                now: Utc::now(),
            },
        )
        .expect("claim should succeed");

        assert!(outcome.issue.is_none());
    }

    #[test]
    fn claim_next_rejects_when_lock_already_exists() {
        let path = temp_issues_path("claim-next-lock");
        let store = MemoryStore::from_issues(vec![issue("bd-1", "open")]).expect("store build");
        store.save_jsonl(&path).expect("store should save");

        let lock_path = issue_lock_path(&path);
        fs::write(&lock_path, "busy\n").expect("lock should be created");
        let result = claim_next_issue_jsonl(
            &path,
            ClaimNextRequest {
                assignee: "worker-a".to_string(),
                lease_id: None,
                lease_ttl_seconds: None,
                now: Utc::now(),
            },
        );

        match result {
            Err(ClaimNextError::Atomic(AtomicStoreMutationError::LockBusy {
                lock_path: reported,
            })) => {
                assert_eq!(reported, lock_path.display().to_string());
            }
            other => panic!("expected lock busy error, got {other:?}"),
        }
        let _ = fs::remove_file(lock_path);
    }
}
