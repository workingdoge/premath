use serde_json::Value;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "premath-cli-{prefix}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_premath<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let bin = env!("CARGO_BIN_EXE_premath");
    Command::new(bin)
        .args(args)
        .output()
        .expect("premath command should execute")
}

fn assert_success(output: &Output) {
    if !output.status.success() {
        panic!(
            "command failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

fn parse_json_stdout(output: &Output) -> Value {
    serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "expected valid JSON stdout, got error: {e}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn write_sample_issues(path: &Path) {
    let lines = [
        r#"{"id":"bd-a","title":"Issue A","status":"open"}"#,
        r#"{"id":"bd-b","title":"Issue B","status":"closed"}"#,
    ];
    fs::write(path, format!("{}\n", lines.join("\n"))).expect("sample issues should be written");
}

fn write_tusk_eval_inputs(dir: &Path) -> (PathBuf, PathBuf) {
    let identity = serde_json::json!({
        "worldId": "world.dev",
        "unitId": "unit.test",
        "contextId": "ctx.main",
        "intentId": "intent.test",
        "coverId": "cover.test",
        "ctxRef": "ctx:head",
        "dataHeadRef": "data:head",
        "adapterId": "adapter.test",
        "adapterVersion": "0.1.0",
        "normalizerId": "normalizer.test.v1",
        "policyDigest": "policy.test.v1"
    });
    let pack = serde_json::json!({
        "core": {
            "coverId": "cover.test",
            "locals": {
                "part:a": {"value": 1}
            },
            "compat": [],
            "mode": {
                "normalizerId": "normalizer.test.v1",
                "policyDigest": "policy.test.v1"
            }
        },
        "glueProposals": [
            {
                "proposalId": "proposal:1",
                "payload": {"selected": true}
            }
        ]
    });

    let identity_path = dir.join("identity.json");
    let pack_path = dir.join("descent-pack.json");
    fs::write(
        &identity_path,
        serde_json::to_vec_pretty(&identity).expect("identity should serialize"),
    )
    .expect("identity json should be written");
    fs::write(
        &pack_path,
        serde_json::to_vec_pretty(&pack).expect("descent pack should serialize"),
    )
    .expect("descent pack json should be written");

    (identity_path, pack_path)
}

fn write_observation_surface(path: &Path) {
    let payload = serde_json::json!({
        "schema": 1,
        "surfaceKind": "ci.observation.surface.v0",
        "summary": {
            "state": "accepted",
            "needsAttention": false,
            "topFailureClass": "verified_accept",
            "latestProjectionDigest": "proj1_alpha",
            "latestInstructionId": "20260221T010000Z-ci-wiring-golden",
            "requiredCheckCount": 1,
            "executedCheckCount": 1,
            "changedPathCount": 2
        },
        "latest": {
            "delta": {
                "ref": "artifacts/ciwitness/latest-delta.json",
                "projectionPolicy": "ci-topos-v0",
                "projectionDigest": "proj1_alpha",
                "deltaSource": "explicit",
                "fromRef": "origin/main",
                "toRef": "HEAD",
                "changedPaths": ["README.md", "tools/ci/README.md"],
                "changedPathCount": 2
            },
            "required": {
                "ref": "artifacts/ciwitness/latest-required.json",
                "witnessKind": "ci.required.v1",
                "projectionPolicy": "ci-topos-v0",
                "projectionDigest": "proj1_alpha",
                "verdictClass": "accepted",
                "requiredChecks": ["baseline"],
                "executedChecks": ["baseline"],
                "failureClasses": []
            },
            "decision": {
                "ref": "artifacts/ciwitness/latest-decision.json",
                "decisionKind": "ci.required.decision.v1",
                "projectionDigest": "proj1_alpha",
                "decision": "accept",
                "reasonClass": "verified_accept",
                "witnessPath": "artifacts/ciwitness/latest-required.json",
                "deltaSnapshotPath": "artifacts/ciwitness/latest-delta.json",
                "requiredChecks": ["baseline"]
            }
        },
        "instructions": [{
            "ref": "artifacts/ciwitness/20260221T010000Z-ci-wiring-golden.json",
            "witnessKind": "ci.instruction.v1",
            "instructionId": "20260221T010000Z-ci-wiring-golden",
            "instructionDigest": "instr1_alpha",
            "instructionClassification": {"state": "typed", "kind": "ci.gate.check"},
            "intent": "verify ci wiring",
            "scope": {"kind": "repo"},
            "policyDigest": "policy.ci.v1",
            "verdictClass": "accepted",
            "requiredChecks": ["ci-wiring-check"],
            "executedChecks": ["ci-wiring-check"],
            "failureClasses": []
        }]
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&payload).expect("surface should serialize"),
    )
    .expect("surface should be written");
}

#[test]
fn check_json_smoke() {
    let tmp = TempDirGuard::new("check-json");
    let issues = tmp.path().join("issues.jsonl");
    write_sample_issues(&issues);

    let output = run_premath([
        OsString::from("check"),
        OsString::from("all"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["scope"], "all");
    assert_eq!(payload["issue_count"], 2);
    assert!(payload["contractible"].is_boolean());
    assert!(payload["coherence_level"].is_string());
}

#[test]
fn verify_json_smoke() {
    let tmp = TempDirGuard::new("verify-json");
    let issues = tmp.path().join("issues.jsonl");
    write_sample_issues(&issues);

    let output = run_premath([
        OsString::from("verify"),
        OsString::from("all"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["scope"], "all");
    assert_eq!(payload["issue_count"], 2);
    assert!(payload["axioms"]["locality"].is_boolean());
    assert!(payload["axioms"]["gluing"].is_boolean());
    assert!(payload["axioms"]["uniqueness"].is_boolean());
    assert!(payload["violations"]["descent_conflict_count"].is_number());
}

#[test]
fn mock_gate_json_smoke() {
    let output = run_premath(["mock-gate", "--json"]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["witnessKind"], "gate");
    assert_eq!(payload["result"], "accepted");
    assert_eq!(
        payload["failures"]
            .as_array()
            .expect("failures should be an array")
            .len(),
        0
    );
}

#[test]
fn tusk_eval_json_smoke() {
    let tmp = TempDirGuard::new("tusk-eval-json");
    let (identity, descent_pack) = write_tusk_eval_inputs(tmp.path());

    let output = run_premath([
        OsString::from("tusk-eval"),
        OsString::from("--identity"),
        identity.as_os_str().to_os_string(),
        OsString::from("--descent-pack"),
        descent_pack.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["envelope"]["result"], "accepted");
    assert_eq!(payload["glueResult"]["selected"], "proposal:1");
}

#[test]
fn init_text_smoke() {
    let tmp = TempDirGuard::new("init");
    let repo_root = tmp.path().join("repo");

    let output = run_premath([OsString::from("init"), repo_root.as_os_str().to_os_string()]);
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("premath init"));
    assert!(stdout.contains("premath dir:"));
    assert!(stdout.contains("issues path:"));
    assert!(repo_root.join(".premath/issues.jsonl").exists());
}

#[test]
fn observe_latest_json_smoke() {
    let tmp = TempDirGuard::new("observe-latest-json");
    let surface = tmp.path().join("surface.json");
    write_observation_surface(&surface);

    let output = run_premath([
        OsString::from("observe"),
        OsString::from("--surface"),
        surface.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("latest"),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["summary"]["state"], "accepted");
    assert_eq!(payload["summary"]["latestProjectionDigest"], "proj1_alpha");
    assert_eq!(
        payload["latest"]["required"]["requiredChecks"][0],
        "baseline"
    );
}

#[test]
fn observe_instruction_json_smoke() {
    let tmp = TempDirGuard::new("observe-instruction-json");
    let surface = tmp.path().join("surface.json");
    write_observation_surface(&surface);

    let output = run_premath([
        OsString::from("observe"),
        OsString::from("--surface"),
        surface.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("instruction"),
        OsString::from("--instruction-id"),
        OsString::from("20260221T010000Z-ci-wiring-golden"),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(
        payload["instructionId"],
        "20260221T010000Z-ci-wiring-golden"
    );
    assert_eq!(payload["verdictClass"], "accepted");
}

#[test]
fn issue_add_dep_ready_json_smoke() {
    let tmp = TempDirGuard::new("issue-add-ready");
    let issues = tmp.path().join("issues.jsonl");

    let out_add_root = run_premath([
        OsString::from("issue"),
        OsString::from("add"),
        OsString::from("Root issue"),
        OsString::from("--id"),
        OsString::from("bd-root"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_add_root);

    let out_add_child = run_premath([
        OsString::from("issue"),
        OsString::from("add"),
        OsString::from("Child issue"),
        OsString::from("--id"),
        OsString::from("bd-child"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_add_child);

    let out_dep = run_premath([
        OsString::from("dep"),
        OsString::from("add"),
        OsString::from("bd-child"),
        OsString::from("bd-root"),
        OsString::from("--type"),
        OsString::from("blocks"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_dep);

    let out_blocked = run_premath([
        OsString::from("issue"),
        OsString::from("blocked"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_blocked);
    let blocked = parse_json_stdout(&out_blocked);
    assert_eq!(blocked["count"], 1);
    assert_eq!(blocked["items"][0]["id"], "bd-child");
    assert_eq!(blocked["items"][0]["blockers"][0]["dependsOnId"], "bd-root");
    assert_eq!(blocked["items"][0]["blockers"][0]["type"], "blocks");

    let out_ready = run_premath([
        OsString::from("issue"),
        OsString::from("ready"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_ready);
    let payload = parse_json_stdout(&out_ready);
    assert_eq!(payload["count"], 1);
    assert_eq!(payload["items"][0]["id"], "bd-root");
}

#[test]
fn issue_update_and_list_json_smoke() {
    let tmp = TempDirGuard::new("issue-update-list");
    let issues = tmp.path().join("issues.jsonl");
    write_sample_issues(&issues);

    let out_update = run_premath([
        OsString::from("issue"),
        OsString::from("update"),
        OsString::from("bd-a"),
        OsString::from("--status"),
        OsString::from("in_progress"),
        OsString::from("--assignee"),
        OsString::from("agent"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_update);
    let updated = parse_json_stdout(&out_update);
    assert_eq!(updated["issue"]["id"], "bd-a");
    assert_eq!(updated["issue"]["status"], "in_progress");
    assert_eq!(updated["issue"]["assignee"], "agent");

    let out_list = run_premath([
        OsString::from("issue"),
        OsString::from("list"),
        OsString::from("--status"),
        OsString::from("in_progress"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_list);
    let listed = parse_json_stdout(&out_list);
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["items"][0]["id"], "bd-a");
}

#[test]
fn issue_migrate_events_json_smoke() {
    let tmp = TempDirGuard::new("issue-migrate-events");
    let issues = tmp.path().join("issues.jsonl");
    let events = tmp.path().join("events.jsonl");

    let out_add_root = run_premath([
        OsString::from("issue"),
        OsString::from("add"),
        OsString::from("Root issue"),
        OsString::from("--id"),
        OsString::from("bd-root"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_add_root);

    let out_add_child = run_premath([
        OsString::from("issue"),
        OsString::from("add"),
        OsString::from("Child issue"),
        OsString::from("--id"),
        OsString::from("bd-child"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_add_child);

    let out_dep = run_premath([
        OsString::from("dep"),
        OsString::from("add"),
        OsString::from("bd-child"),
        OsString::from("bd-root"),
        OsString::from("--type"),
        OsString::from("blocks"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_dep);

    let out_migrate = run_premath([
        OsString::from("issue"),
        OsString::from("migrate-events"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--events"),
        events.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_migrate);

    let payload = parse_json_stdout(&out_migrate);
    assert_eq!(payload["action"], "issue.migrate-events");
    assert_eq!(payload["issueCount"], 2);
    assert_eq!(payload["eventCount"], 3);
    assert_eq!(payload["equivalent"], true);

    let event_lines = fs::read_to_string(&events).expect("events jsonl should exist");
    assert_eq!(event_lines.lines().count(), 3);
    let first_event: Value =
        serde_json::from_str(event_lines.lines().next().expect("at least one event line"))
            .expect("first event should parse");
    assert_eq!(first_event["schema"], "issue.event.v1");
    assert_eq!(first_event["action"], "upsert_issue");
}

#[test]
fn dep_project_views_json_smoke() {
    let tmp = TempDirGuard::new("dep-project-views");
    let issues = tmp.path().join("issues.jsonl");

    let out_add_root = run_premath([
        OsString::from("issue"),
        OsString::from("add"),
        OsString::from("Root issue"),
        OsString::from("--id"),
        OsString::from("bd-root"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_add_root);

    let out_add_child = run_premath([
        OsString::from("issue"),
        OsString::from("add"),
        OsString::from("Child issue"),
        OsString::from("--id"),
        OsString::from("bd-child"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_add_child);

    let out_dep = run_premath([
        OsString::from("dep"),
        OsString::from("add"),
        OsString::from("bd-child"),
        OsString::from("bd-root"),
        OsString::from("--type"),
        OsString::from("blocks"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_dep);

    let out_gtd = run_premath([
        OsString::from("dep"),
        OsString::from("project"),
        OsString::from("--view"),
        OsString::from("gtd"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_gtd);
    let gtd = parse_json_stdout(&out_gtd);
    assert_eq!(gtd["action"], "dep.project");
    assert_eq!(gtd["view"], "gtd");
    assert_eq!(gtd["count"], 1);
    assert_eq!(gtd["items"][0]["type"], "blocks");
    assert_eq!(gtd["items"][0]["role"], "next-action");
    assert_eq!(gtd["items"][0]["blocking"], true);

    let out_groupoid = run_premath([
        OsString::from("dep"),
        OsString::from("project"),
        OsString::from("--view"),
        OsString::from("groupoid"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_groupoid);
    let groupoid = parse_json_stdout(&out_groupoid);
    assert_eq!(groupoid["view"], "groupoid");
    assert_eq!(groupoid["items"][0]["role"], "constraint");
}

#[test]
fn coherence_check_json_smoke() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should be two levels above crate dir")
        .to_path_buf();
    let contract = repo_root.join("specs/premath/draft/COHERENCE-CONTRACT.json");

    let output = run_premath([
        OsString::from("coherence-check"),
        OsString::from("--contract"),
        contract.as_os_str().to_os_string(),
        OsString::from("--repo-root"),
        repo_root.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);
    let payload = parse_json_stdout(&output);
    assert_eq!(payload["witnessKind"], "premath.coherence.v1");
    assert_eq!(
        payload["result"].as_str().expect("result should be string"),
        "accepted"
    );
}
