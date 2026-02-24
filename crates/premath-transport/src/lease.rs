use crate::registry::validate_transport_action_binding_with_kernel;
use crate::types::*;
use crate::*;
use chrono::Utc;
use premath_bd::{
    ClaimNextRequest, IssueLease, IssueLeaseState, MemoryStore, claim_next_issue_jsonl,
    mutate_store_jsonl,
};

pub fn issue_claim_next(request: IssueClaimNextRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::ClaimNext;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }

    let assignee = request.assignee.trim().to_string();
    if assignee.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        );
    }

    let now = Utc::now();
    let outcome = match claim_next_issue_jsonl(
        &issues_path,
        ClaimNextRequest {
            assignee,
            lease_id: request.lease_id.clone(),
            lease_ttl_seconds: request.lease_ttl_seconds,
            now,
        },
    ) {
        Ok(value) => value,
        Err(err) => {
            let mapped = map_claim_next_error(err);
            return rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic);
        }
    };

    let store = match MemoryStore::load_jsonl(&issues_path) {
        Ok(store) => store,
        Err(source) => {
            return rejected_envelope(
                kind,
                issues_path,
                FAILURE_LEASE_MUTATION_STORE_IO,
                source.to_string(),
            );
        }
    };
    let issue = outcome.issue.as_ref().map(|item| issue_summary(item, now));
    accepted_envelope_optional(
        kind,
        issues_path,
        issue,
        outcome.issue.is_some(),
        compute_lease_projection(&store, now),
    )
}

pub fn issue_claim(request: IssueClaimRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::Claim;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }
    let assignee = request.assignee.trim().to_string();
    if assignee.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        );
    }

    let now = Utc::now();
    let lease_expires_at = match parse_lease_expiry(
        request.lease_ttl_seconds,
        request.lease_expires_at.clone(),
        now,
    ) {
        Ok(value) => value,
        Err(err) => {
            return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
        }
    };
    let requested_lease_id = request.lease_id.clone();

    let mutation = mutate_store_jsonl(&issues_path, |store| {
        let issue = store.issue_mut(&request.id).ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", request.id),
            )
        })?;

        if issue.status == "closed" {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CLOSED,
                format!("cannot claim closed issue: {}", request.id),
            ));
        }

        let mut changed = false;
        let mut status_changed = false;

        if issue.lease_state_at(now) == IssueLeaseState::Stale {
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

        if let Some(active_lease) = issue.lease.as_ref().filter(|lease| lease.expires_at > now)
            && active_lease.owner != assignee
        {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CONTENTION_ACTIVE,
                format!(
                    "issue already leased: {} (owner={}, lease_id={})",
                    request.id, active_lease.owner, active_lease.lease_id
                ),
            ));
        }

        if issue.lease.is_none() && !issue.assignee.is_empty() && issue.assignee != assignee {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CONTENTION_ACTIVE,
                format!(
                    "issue already claimed: {} (assignee={})",
                    request.id, issue.assignee
                ),
            ));
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

        let lease_id = issue
            .lease
            .as_ref()
            .filter(|existing| existing.expires_at > now && existing.owner == assignee)
            .map(|existing| existing.lease_id.clone())
            .unwrap_or_else(|| {
                resolve_lease_id(requested_lease_id.clone(), &request.id, &assignee)
            });

        let next_lease = match issue.lease.as_ref() {
            Some(existing) if existing.owner == assignee && existing.lease_id == lease_id => {
                IssueLease {
                    lease_id: lease_id.clone(),
                    owner: assignee.clone(),
                    acquired_at: existing.acquired_at,
                    expires_at: lease_expires_at,
                    renewed_at: Some(now),
                }
            }
            _ => IssueLease {
                lease_id: lease_id.clone(),
                owner: assignee.clone(),
                acquired_at: now,
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

        let updated = issue.clone();
        Ok(((updated, changed, store.clone()), changed))
    });

    match mutation {
        Ok((updated, changed, store)) => accepted_envelope(
            kind,
            issues_path,
            issue_summary(&updated, now),
            changed,
            compute_lease_projection(&store, now),
        ),
        Err(err) => {
            let mapped = map_atomic_store_error(err);
            rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic)
        }
    }
}

pub fn issue_lease_renew(request: IssueLeaseRenewRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::Renew;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }
    let assignee = request.assignee.trim().to_string();
    if assignee.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        );
    }
    let lease_id = request.lease_id.trim().to_string();
    if lease_id.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_ID_MISMATCH,
            "lease_id is required",
        );
    }

    let now = Utc::now();
    let lease_expires_at = match parse_lease_expiry(
        request.lease_ttl_seconds,
        request.lease_expires_at.clone(),
        now,
    ) {
        Ok(value) => value,
        Err(err) => {
            return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
        }
    };

    let mutation = mutate_store_jsonl(&issues_path, |store| {
        let issue = store.issue_mut(&request.id).ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", request.id),
            )
        })?;

        if issue.status == "closed" {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CLOSED,
                format!("cannot renew lease on closed issue: {}", request.id),
            ));
        }

        let current = issue.lease.clone().ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_MISSING,
                format!("issue has no lease: {}", request.id),
            )
        })?;

        if current.expires_at <= now {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_STALE,
                format!("lease is stale and must be reclaimed: {}", request.id),
            ));
        }
        if current.owner != assignee {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_OWNER_MISMATCH,
                format!(
                    "lease owner mismatch for {} (expected={}, got={})",
                    request.id, current.owner, assignee
                ),
            ));
        }
        if current.lease_id != lease_id {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_ID_MISMATCH,
                format!(
                    "lease_id mismatch for {} (expected={}, got={})",
                    request.id, current.lease_id, lease_id
                ),
            ));
        }

        let mut changed = false;
        let mut status_changed = false;
        if issue.assignee != assignee {
            issue.assignee = assignee.clone();
            changed = true;
        }
        if issue.status != "in_progress" {
            issue.set_status("in_progress".to_string());
            changed = true;
            status_changed = true;
        }

        let renewed = IssueLease {
            lease_id,
            owner: assignee,
            acquired_at: current.acquired_at,
            expires_at: lease_expires_at,
            renewed_at: Some(now),
        };
        if issue.lease.as_ref() != Some(&renewed) {
            issue.lease = Some(renewed);
            changed = true;
        }

        if changed && !status_changed {
            issue.touch_updated_at();
        }

        let updated = issue.clone();
        Ok(((updated, changed, store.clone()), changed))
    });

    match mutation {
        Ok((updated, changed, store)) => accepted_envelope(
            kind,
            issues_path,
            issue_summary(&updated, now),
            changed,
            compute_lease_projection(&store, now),
        ),
        Err(err) => {
            let mapped = map_atomic_store_error(err);
            rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic)
        }
    }
}

pub fn issue_lease_release(request: IssueLeaseReleaseRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::Release;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }
    let expected_assignee = non_empty(request.assignee.clone());
    let expected_lease_id = non_empty(request.lease_id.clone());
    let now = Utc::now();

    let mutation = mutate_store_jsonl(&issues_path, |store| {
        let issue = store.issue_mut(&request.id).ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", request.id),
            )
        })?;

        let mut changed = false;
        let mut status_changed = false;

        match issue.lease.as_ref() {
            None => {
                if expected_assignee.is_some() || expected_lease_id.is_some() {
                    return Err(LeaseMutationError::new(
                        FAILURE_LEASE_MISSING,
                        format!("issue has no lease: {}", request.id),
                    ));
                }
            }
            Some(current) => {
                if let Some(expected) = expected_assignee.as_ref()
                    && current.owner != *expected
                {
                    return Err(LeaseMutationError::new(
                        FAILURE_LEASE_OWNER_MISMATCH,
                        format!(
                            "lease owner mismatch for {} (expected={}, got={})",
                            request.id, current.owner, expected
                        ),
                    ));
                }
                if let Some(expected) = expected_lease_id.as_ref()
                    && current.lease_id != *expected
                {
                    return Err(LeaseMutationError::new(
                        FAILURE_LEASE_ID_MISMATCH,
                        format!(
                            "lease_id mismatch for {} (expected={}, got={})",
                            request.id, current.lease_id, expected
                        ),
                    ));
                }
                issue.lease = None;
                changed = true;
            }
        }

        if changed {
            if !issue.assignee.is_empty() {
                issue.assignee.clear();
            }
            if issue.status == "in_progress" {
                issue.set_status("open".to_string());
                status_changed = true;
            }
            if !status_changed {
                issue.touch_updated_at();
            }
        }

        let updated = issue.clone();
        Ok(((updated, changed, store.clone()), changed))
    });

    match mutation {
        Ok((updated, changed, store)) => accepted_envelope(
            kind,
            issues_path,
            issue_summary(&updated, now),
            changed,
            compute_lease_projection(&store, now),
        ),
        Err(err) => {
            let mapped = map_atomic_store_error(err);
            rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic)
        }
    }
}

pub fn issue_claim_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueClaimRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_claim(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::Claim,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid claim payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease claim envelope should serialize")
}

pub fn issue_claim_next_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueClaimNextRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_claim_next(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::ClaimNext,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid claim-next payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease claim-next envelope should serialize")
}

pub fn issue_lease_renew_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueLeaseRenewRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_lease_renew(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::Renew,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid renew payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease renew envelope should serialize")
}

pub fn issue_lease_release_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueLeaseReleaseRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_lease_release(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::Release,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid release payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease release envelope should serialize")
}
