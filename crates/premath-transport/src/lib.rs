pub mod dispatch;
pub mod fiber;
pub mod lease;
pub mod registry;
pub mod types;

// Re-export all public items from submodules so downstream callers are unaffected.
pub use dispatch::*;
pub use lease::*;
pub use registry::*;
pub use types::*;

const DEFAULT_ISSUES_PATH: &str = ".premath/issues.jsonl";
const WORLD_ID_LEASE: &str = "world.lease.v1";
const ROUTE_FAMILY_LEASE: &str = "route.issue_claim_lease";
const MORPHISM_ROW_LEASE: &str = "wm.control.lease.mutation";
const WORLD_ID_TRANSPORT: &str = "world.transport.v1";
const ROUTE_FAMILY_TRANSPORT: &str = "route.transport.dispatch";
const MORPHISM_ROW_TRANSPORT: &str = "wm.control.transport.dispatch";
const WORLD_ID_FIBER: &str = "world.fiber.v1";
const ROUTE_FAMILY_FIBER: &str = "route.fiber.lifecycle";
const MORPHISM_ROW_FIBER: &str = "wm.control.fiber.lifecycle";
const WORLD_ID_INSTRUCTION: &str = "world.instruction.v1";
const ROUTE_FAMILY_INSTRUCTION: &str = "route.instruction_execution";
const MORPHISM_ROW_INSTRUCTION: &str = "wm.control.instruction.execution";
const WORLD_ID_SITE_CHANGE: &str = "world.site_change.v1";
const ROUTE_FAMILY_SITE_CHANGE: &str = "route.site_change";
const MORPHISM_ROW_SITE_CHANGE: &str = "wm.control.site_change.mutation";
const DOCTRINE_SITE_INPUT_JSON: &str =
    include_str!("../../../specs/premath/contracts/DOCTRINE-SITE-INPUT.json");
const DOCTRINE_SITE_JSON: &str =
    include_str!("../../../specs/premath/contracts/DOCTRINE-SITE.json");
const DOCTRINE_OP_REGISTRY_JSON: &str =
    include_str!("../../../specs/premath/contracts/DOCTRINE-OP-REGISTRY.json");
const CONTROL_PLANE_CONTRACT_JSON: &str =
    include_str!("../../../specs/premath/contracts/CONTROL-PLANE-CONTRACT.json");
const CAPABILITY_REGISTRY_JSON: &str =
    include_str!("../../../specs/premath/contracts/CAPABILITY-REGISTRY.json");

const TRANSPORT_DISPATCH_KIND: &str = "premath.transport_dispatch.v1";
const TRANSPORT_ACTION_REGISTRY_KIND: &str = "premath.transport_action_registry.v1";
const TRANSPORT_CHECK_KIND: &str = "premath.transport_check.v1";
const TRANSPORT_PROFILE_ID: &str = "transport.issue_lease.v1";
const TRANSPORT_SEMANTIC_DIGEST_PREFIX: &str = "ts1_";
const ACTION_ID_TRANSPORT_INVALID_REQUEST: &str = "transport.action.invalid_request";
const ACTION_ID_TRANSPORT_UNKNOWN: &str = "transport.action.unknown";

const FAILURE_LEASE_INVALID_ASSIGNEE: &str = "lease_invalid_assignee";
const FAILURE_LEASE_INVALID_TTL: &str = "lease_invalid_ttl";
const FAILURE_LEASE_BINDING_AMBIGUOUS: &str = "lease_binding_ambiguous";
const FAILURE_LEASE_INVALID_EXPIRES_AT: &str = "lease_invalid_expires_at";
const FAILURE_LEASE_NOT_FOUND: &str = "lease_not_found";
const FAILURE_LEASE_CLOSED: &str = "lease_issue_closed";
const FAILURE_LEASE_CONTENTION_ACTIVE: &str = "lease_contention_active";
const FAILURE_LEASE_MISSING: &str = "lease_missing";
const FAILURE_LEASE_STALE: &str = "lease_stale";
const FAILURE_LEASE_OWNER_MISMATCH: &str = "lease_owner_mismatch";
const FAILURE_LEASE_ID_MISMATCH: &str = "lease_id_mismatch";
const FAILURE_LEASE_MUTATION_LOCK_BUSY: &str = "lease_mutation_lock_busy";
const FAILURE_LEASE_MUTATION_LOCK_IO: &str = "lease_mutation_lock_io";
const FAILURE_LEASE_MUTATION_STORE_IO: &str = "lease_mutation_store_io";
const FAILURE_LEASE_INVALID_PAYLOAD: &str = "lease_invalid_payload";
const FAILURE_LEASE_UNKNOWN_ACTION: &str = "lease_unknown_action";
const FAILURE_TRANSPORT_INVALID_REQUEST: &str = "transport_invalid_request";
const FAILURE_TRANSPORT_UNKNOWN_ACTION: &str = "transport_unknown_action";
const FAILURE_TRANSPORT_REGISTRY_EMPTY_FIELD: &str = "transport_registry_empty_field";
const FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION: &str = "transport_registry_duplicate_action";
const FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION_ID: &str =
    "transport_registry_duplicate_action_id";
const FAILURE_TRANSPORT_REGISTRY_MISSING_ACTION: &str = "transport_registry_missing_action";
const FAILURE_TRANSPORT_REGISTRY_DIGEST_MISMATCH: &str = "transport_registry_digest_mismatch";
const FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE: &str = "transport_kernel_contract_unavailable";
const FAILURE_FIBER_INVALID_PAYLOAD: &str = "fiber_invalid_payload";
const FAILURE_FIBER_MISSING_FIELD: &str = "fiber_missing_field";
const FAILURE_INSTRUCTION_INVALID_PAYLOAD: &str = "instruction_invalid_payload";
const FAILURE_INSTRUCTION_EXECUTION_IO: &str = "instruction_execution_io";
const FAILURE_INSTRUCTION_RUNTIME_INVALID: &str = "instruction_runtime_invalid";
const FAILURE_SITE_CHANGE_INVALID_PAYLOAD: &str = "site_change_invalid_payload";

#[cfg(feature = "rustler_nif")]
mod nif {
    use crate::dispatch::nif_dispatch_json;

    #[rustler::nif(schedule = "DirtyIo")]
    fn dispatch(request_json: String) -> String {
        nif_dispatch_json(&request_json)
    }

    rustler::init!("Elixir.Premath.TransportNif");
}

#[cfg(test)]
mod tests {
    use super::{
        IssueClaimNextRequest, IssueClaimRequest, IssueLeaseReleaseRequest, IssueLeaseRenewRequest,
        issue_claim, issue_claim_next, issue_lease_release, issue_lease_renew,
        issue_lease_renew_json, transport_check, transport_dispatch_json, world_route_binding_json,
    };
    use crate::dispatch::nif_dispatch_json;
    use premath_bd::{Issue, MemoryStore};
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_issues_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("premath-transport-{prefix}-{unique}"));
        fs::create_dir_all(&root).expect("temp dir should be created");
        root.join("issues.jsonl")
    }

    fn seed_open_issue(path: &PathBuf, id: &str) {
        let mut issue = Issue::new(id.to_string(), format!("Issue {id}"));
        issue.set_status("open".to_string());
        let mut store = MemoryStore::default();
        store.upsert_issue(issue);
        store.save_jsonl(path).expect("store should save");
    }

    #[test]
    fn claim_renew_release_roundtrip_is_accepted() {
        let path = temp_issues_path("roundtrip");
        seed_open_issue(&path, "bd-1");

        let claim = issue_claim(IssueClaimRequest {
            id: "bd-1".to_string(),
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(claim.result, "accepted");
        assert_eq!(claim.action, "issue.claim");
        assert_eq!(claim.world_binding.world_id, "world.lease.v1");

        let lease_id = claim
            .issue
            .as_ref()
            .and_then(|item| item.lease.as_ref())
            .map(|item| item.lease_id.clone())
            .expect("claim should return lease");

        let renew = issue_lease_renew(IssueLeaseRenewRequest {
            id: "bd-1".to_string(),
            assignee: "worker-a".to_string(),
            lease_id: lease_id.clone(),
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(renew.result, "accepted");
        assert_eq!(renew.action, "issue.lease_renew");

        let release = issue_lease_release(IssueLeaseReleaseRequest {
            id: "bd-1".to_string(),
            assignee: Some("worker-a".to_string()),
            lease_id: Some(lease_id),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(release.result, "accepted");
        assert_eq!(release.action, "issue.lease_release");
        let issue = release.issue.expect("release should project issue");
        assert_eq!(issue.status, "open");
        assert!(issue.assignee.is_empty());
        assert!(issue.lease.is_none());
    }

    #[test]
    fn claim_next_accepts_and_returns_none_when_no_ready_issue() {
        let path = temp_issues_path("claim-next");
        seed_open_issue(&path, "bd-11");

        let first = issue_claim_next(IssueClaimNextRequest {
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(first.result, "accepted");
        assert_eq!(first.action, "issue.claim_next");
        assert_eq!(
            first.issue.as_ref().map(|row| row.id.as_str()),
            Some("bd-11")
        );
        assert_eq!(first.changed, Some(true));

        let second = issue_claim_next(IssueClaimNextRequest {
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(second.result, "accepted");
        assert_eq!(second.action, "issue.claim_next");
        assert!(second.issue.is_none());
        assert_eq!(second.changed, Some(false));
    }

    #[test]
    fn claim_rejects_empty_assignee() {
        let path = temp_issues_path("invalid-assignee");
        seed_open_issue(&path, "bd-2");

        let claim = issue_claim(IssueClaimRequest {
            id: "bd-2".to_string(),
            assignee: "   ".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(claim.result, "rejected");
        assert_eq!(
            claim.failure_classes,
            vec!["lease_invalid_assignee".to_string()]
        );
    }

    #[test]
    fn release_rejects_owner_mismatch() {
        let path = temp_issues_path("owner-mismatch");
        seed_open_issue(&path, "bd-3");

        let claim = issue_claim(IssueClaimRequest {
            id: "bd-3".to_string(),
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        let lease_id = claim
            .issue
            .as_ref()
            .and_then(|item| item.lease.as_ref())
            .map(|item| item.lease_id.clone())
            .expect("claim should return lease");

        let release = issue_lease_release(IssueLeaseReleaseRequest {
            id: "bd-3".to_string(),
            assignee: Some("worker-b".to_string()),
            lease_id: Some(lease_id),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(release.result, "rejected");
        assert_eq!(
            release.failure_classes,
            vec!["lease_owner_mismatch".to_string()]
        );
    }

    #[test]
    fn json_wrapper_rejects_invalid_payload() {
        let payload = issue_lease_renew_json("{\"id\":1}");
        let parsed: Value = serde_json::from_str(&payload).expect("payload should parse");
        assert_eq!(parsed["result"], "rejected");
        assert_eq!(
            parsed["failureClasses"],
            serde_json::json!(["lease_invalid_payload"])
        );
    }

    #[test]
    fn world_binding_json_reports_known_and_unknown_actions() {
        let known = world_route_binding_json("issue.lease_renew");
        let known_value: Value = serde_json::from_str(&known).expect("known payload should parse");
        assert_eq!(known_value["result"], "accepted");
        assert_eq!(known_value["binding"]["worldId"], "world.lease.v1");
        assert_eq!(
            known_value["binding"]["routeFamilyId"],
            "route.issue_claim_lease"
        );

        let unknown = world_route_binding_json("issue.not_real");
        let unknown_value: Value =
            serde_json::from_str(&unknown).expect("unknown payload should parse");
        assert_eq!(unknown_value["result"], "rejected");
        assert_eq!(
            unknown_value["failureClasses"],
            serde_json::json!(["lease_unknown_action"])
        );
    }

    #[test]
    fn transport_dispatch_claim_accepts() {
        let path = temp_issues_path("dispatch-claim");
        seed_open_issue(&path, "bd-4");
        let request = serde_json::json!({
            "action": "issue.claim",
            "payload": {
                "id": "bd-4",
                "assignee": "worker-x",
                "leaseTtlSeconds": 3600,
                "issuesPath": path.display().to_string()
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "issue.claim");
        assert_eq!(value["worldBinding"]["worldId"], "world.lease.v1");
        assert_eq!(
            value["dispatchKind"],
            serde_json::json!("premath.transport_dispatch.v1")
        );
        assert_eq!(
            value["profileId"],
            serde_json::json!("transport.issue_lease.v1")
        );
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.issue_claim")
        );
        assert!(
            value["semanticDigest"]
                .as_str()
                .map(|digest| digest.starts_with("ts1_"))
                .unwrap_or(false)
        );
    }

    #[test]
    fn transport_dispatch_claim_next_accepts() {
        let path = temp_issues_path("dispatch-claim-next");
        seed_open_issue(&path, "bd-12");
        let request = serde_json::json!({
            "action": "issue.claim_next",
            "payload": {
                "assignee": "worker-y",
                "leaseTtlSeconds": 3600,
                "issuesPath": path.display().to_string()
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "issue.claim_next");
        assert_eq!(value["worldBinding"]["worldId"], "world.lease.v1");
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.issue_claim_next")
        );
        assert_eq!(value["issue"]["id"], "bd-12");
    }

    #[test]
    fn transport_dispatch_rejects_unknown_action() {
        let request = serde_json::json!({
            "action": "issue.not_supported",
            "payload": {}
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "rejected");
        assert_eq!(
            value["failureClasses"],
            serde_json::json!(["transport_unknown_action"])
        );
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.unknown")
        );
    }

    #[test]
    fn transport_dispatch_instruction_run_rejects_invalid_payload() {
        let request = serde_json::json!({
            "action": "instruction.run",
            "payload": {
                "instructionPath": "   "
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "rejected");
        assert_eq!(
            value["failureClasses"],
            serde_json::json!(["instruction_invalid_payload"])
        );
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.instruction_run")
        );
    }

    #[test]
    fn transport_dispatch_world_route_binding_accepts() {
        let request = serde_json::json!({
            "action": "world.route_binding",
            "payload": {
                "action": "issue.claim"
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "world.route_binding");
        assert_eq!(value["operationAction"], "issue.claim");
        assert_eq!(value["binding"]["worldId"], "world.lease.v1");
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.world_route_binding")
        );
    }

    #[test]
    fn transport_dispatch_fiber_spawn_accepts() {
        let request = serde_json::json!({
            "action": "fiber.spawn",
            "payload": {
                "fiberId": "fib-alpha",
                "taskRef": "task/check-coherence",
                "scopeRef": "scope/worktree-a"
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "fiber.spawn");
        assert_eq!(value["worldBinding"]["worldId"], "world.fiber.v1");
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.fiber_spawn")
        );
        assert_eq!(value["fiberId"], "fib-alpha");
        assert_eq!(value["taskRef"], "task/check-coherence");
        assert!(
            value["fiberWitnessRef"]
                .as_str()
                .map(|item| item.starts_with("fiber://dispatch/fiber.spawn/"))
                .unwrap_or(false)
        );
    }

    #[test]
    fn transport_dispatch_fiber_join_rejects_empty_join_set() {
        let request = serde_json::json!({
            "action": "fiber.join",
            "payload": {
                "fiberId": "fib-alpha",
                "joinSet": []
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "rejected");
        assert_eq!(
            value["failureClasses"],
            serde_json::json!(["fiber_missing_field"])
        );
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.fiber_join")
        );
    }

    #[test]
    fn transport_check_reports_typed_registry() {
        let report = transport_check();
        assert_eq!(report.schema, 1);
        assert_eq!(report.check_kind, "premath.transport_check.v1");
        assert_eq!(report.registry_kind, "premath.transport_action_registry.v1");
        assert_eq!(report.profile_id, "transport.issue_lease.v1");
        assert_eq!(report.result, "accepted");
        assert!(report.failure_classes.is_empty());
        assert_eq!(report.action_count, 13);
        assert!(
            report
                .actions
                .iter()
                .any(|row| row.action == "issue.lease_renew"
                    && row.action_id == "transport.action.issue_lease_renew")
        );
        assert!(
            report
                .actions
                .iter()
                .any(|row| row.action == "issue.claim_next"
                    && row.action_id == "transport.action.issue_claim_next")
        );
        assert!(
            report.actions.iter().any(|row| row.action == "fiber.spawn"
                && row.action_id == "transport.action.fiber_spawn")
        );
        assert!(
            report
                .actions
                .iter()
                .any(|row| row.action == "instruction.run"
                    && row.action_id == "transport.action.instruction_run")
        );
        assert!(report.semantic_digest.starts_with("ts1_"));
    }

    #[test]
    fn nif_dispatch_claim_next_matches_transport_dispatch_envelope_semantics() {
        let path_transport = temp_issues_path("nif-transport-claim-next");
        let path_nif = temp_issues_path("nif-dispatch-claim-next");
        seed_open_issue(&path_transport, "bd-31");
        seed_open_issue(&path_nif, "bd-31");

        let transport_request = serde_json::json!({
            "action": "issue.claim_next",
            "payload": {
                "assignee": "worker-nif",
                "leaseTtlSeconds": 3600,
                "issuesPath": path_transport.display().to_string()
            }
        });
        let nif_request = serde_json::json!({
            "action": "issue.claim_next",
            "payload": {
                "assignee": "worker-nif",
                "leaseTtlSeconds": 3600,
                "issuesPath": path_nif.display().to_string()
            }
        });

        let transport_value: Value =
            serde_json::from_str(&transport_dispatch_json(&transport_request.to_string()))
                .expect("transport dispatch response should parse");
        let nif_value: Value = serde_json::from_str(&nif_dispatch_json(&nif_request.to_string()))
            .expect("nif dispatch response should parse");

        assert_eq!(transport_value["result"], nif_value["result"]);
        assert_eq!(transport_value["action"], nif_value["action"]);
        assert_eq!(transport_value["actionId"], nif_value["actionId"]);
        assert_eq!(transport_value["dispatchKind"], nif_value["dispatchKind"]);
        assert_eq!(transport_value["profileId"], nif_value["profileId"]);
        assert_eq!(
            transport_value["failureClasses"],
            nif_value["failureClasses"]
        );
        assert_eq!(transport_value["worldBinding"], nif_value["worldBinding"]);
        assert_eq!(transport_value["issue"]["id"], nif_value["issue"]["id"]);
        assert_eq!(
            transport_value["issue"]["lease"]["leaseId"],
            nif_value["issue"]["lease"]["leaseId"]
        );
    }
}
