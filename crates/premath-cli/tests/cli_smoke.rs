use serde_json::Value;
use sha2::{Digest, Sha256};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Barrier};
use std::thread;
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

fn write_claim_next_issues(path: &Path) {
    let lines = [
        r#"{"id":"bd-1","title":"Issue 1","status":"open"}"#,
        r#"{"id":"bd-2","title":"Issue 2","status":"open"}"#,
    ];
    fs::write(path, format!("{}\n", lines.join("\n")))
        .expect("claim-next issues should be written");
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

fn write_proposal_input(dir: &Path) -> PathBuf {
    let proposal = serde_json::json!({
        "proposalKind": "value",
        "targetCtxRef": "ctx:repo.main",
        "targetJudgment": {
            "kind": "obj",
            "shape": "ObjNF:site"
        },
        "candidateRefs": ["ref:alpha"],
        "binding": {
            "normalizerId": "normalizer.ci.v1",
            "policyDigest": "pol1_demo"
        }
    });
    let proposal_path = dir.join("proposal.json");
    fs::write(
        &proposal_path,
        serde_json::to_vec_pretty(&proposal).expect("proposal should serialize"),
    )
    .expect("proposal should be written");
    proposal_path
}

fn write_instruction_runtime_input(dir: &Path, instruction_ref: &str, failed: bool) -> PathBuf {
    let runtime = serde_json::json!({
        "instructionId": "20260221T010000Z-ci-wiring-golden",
        "instructionRef": instruction_ref,
        "instructionDigest": "instr1_demo",
        "squeakSiteProfile": "local",
        "runStartedAt": "2026-02-22T00:00:00Z",
        "runFinishedAt": "2026-02-22T00:00:01Z",
        "runDurationMs": 1000,
        "results": [{
            "checkId": "ci-wiring-check",
            "status": if failed { "failed" } else { "passed" },
            "exitCode": if failed { 1 } else { 0 },
            "durationMs": 25
        }]
    });
    let runtime_path = dir.join("instruction-runtime.json");
    fs::write(
        &runtime_path,
        serde_json::to_vec_pretty(&runtime).expect("runtime should serialize"),
    )
    .expect("runtime should be written");
    runtime_path
}

fn write_required_runtime_input(dir: &Path, failed: bool) -> PathBuf {
    let normalizer_id = "normalizer.ci.required.v1";
    let projection_digest = "proj1_demo";
    let typed_core_projection_digest = format!(
        "ev1_{:x}",
        Sha256::digest(format!(
            "{projection_digest}\0{normalizer_id}\0{}\0",
            "ci-topos-v0"
        ))
    );
    let runtime = serde_json::json!({
        "projectionPolicy": "ci-topos-v0",
        "projectionDigest": projection_digest,
        "changedPaths": ["README.md"],
        "requiredChecks": ["baseline"],
        "results": [{
            "checkId": "baseline",
            "status": if failed { "failed" } else { "passed" },
            "exitCode": if failed { 1 } else { 0 },
            "durationMs": 25
        }],
        "gateWitnessRefs": [{
            "checkId": "baseline",
            "artifactRelPath": "gates/proj1_demo/01-baseline.json",
            "sha256": "abc123",
            "source": "native",
            "runId": "run1_demo",
            "witnessKind": "gate",
            "result": if failed { "rejected" } else { "accepted" },
            "failureClasses": if failed { vec!["descent_failure"] } else { Vec::<&str>::new() }
        }],
        "docsOnly": false,
        "reasons": ["kernel_or_ci_or_governance_change"],
        "deltaSource": "explicit",
        "fromRef": "origin/main",
        "toRef": "HEAD",
        "normalizerId": normalizer_id,
        "policyDigest": "ci-topos-v0",
        "typedCoreProjectionDigest": typed_core_projection_digest,
        "authorityPayloadDigest": projection_digest,
        "squeakSiteProfile": "local",
        "runStartedAt": "2026-02-22T00:00:00Z",
        "runFinishedAt": "2026-02-22T00:00:01Z",
        "runDurationMs": 1000
    });
    let runtime_path = dir.join("required-runtime.json");
    fs::write(
        &runtime_path,
        serde_json::to_vec_pretty(&runtime).expect("runtime should serialize"),
    )
    .expect("required runtime should be written");
    runtime_path
}

fn write_required_projection_input(dir: &Path, changed_paths: Vec<&str>) -> PathBuf {
    let input = serde_json::json!({
        "changedPaths": changed_paths
    });
    let input_path = dir.join("required-projection-input.json");
    fs::write(
        &input_path,
        serde_json::to_vec_pretty(&input).expect("projection input should serialize"),
    )
    .expect("required projection input should be written");
    input_path
}

fn write_required_delta_input(
    dir: &Path,
    repo_root: &Path,
    from_ref: Option<&str>,
    to_ref: Option<&str>,
) -> PathBuf {
    let mut input = serde_json::Map::new();
    input.insert(
        "repoRoot".to_string(),
        Value::String(repo_root.to_string_lossy().to_string()),
    );
    if let Some(value) = from_ref {
        input.insert("fromRef".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = to_ref {
        input.insert("toRef".to_string(), Value::String(value.to_string()));
    }
    let input_path = dir.join("required-delta-input.json");
    fs::write(
        &input_path,
        serde_json::to_vec_pretty(&Value::Object(input)).expect("delta input should serialize"),
    )
    .expect("required delta input should be written");
    input_path
}

fn run_git<I, S>(repo_root: &Path, args: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .expect("git command should execute");
    if !output.status.success() {
        panic!(
            "git command failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

fn write_required_verify_input(dir: &Path, failed: bool) -> PathBuf {
    let runtime = write_required_runtime_input(dir, failed);
    let witness_output = run_premath([
        OsString::from("required-witness"),
        OsString::from("--runtime"),
        runtime.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&witness_output);
    let witness = parse_json_stdout(&witness_output);
    let input = serde_json::json!({
        "witness": witness,
        "changedPaths": ["README.md"],
        "nativeRequiredChecks": []
    });
    let input_path = dir.join("required-verify-input.json");
    fs::write(
        &input_path,
        serde_json::to_vec_pretty(&input).expect("verify input should serialize"),
    )
    .expect("required verify input should be written");
    input_path
}

fn write_required_decide_input(dir: &Path, failed: bool) -> PathBuf {
    let runtime = write_required_runtime_input(dir, failed);
    let witness_output = run_premath([
        OsString::from("required-witness"),
        OsString::from("--runtime"),
        runtime.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&witness_output);
    let witness = parse_json_stdout(&witness_output);
    let input = serde_json::json!({
        "witness": witness,
        "expectedChangedPaths": ["README.md"],
        "nativeRequiredChecks": []
    });
    let input_path = dir.join("required-decide-input.json");
    fs::write(
        &input_path,
        serde_json::to_vec_pretty(&input).expect("decide input should serialize"),
    )
    .expect("required decide input should be written");
    input_path
}

fn write_required_decision_verify_input(dir: &Path) -> PathBuf {
    let typed_core_projection_digest = "ev1_demo";
    let authority_payload_digest = "proj1_demo";
    let normalizer_id = "normalizer.ci.required.v1";
    let policy_digest = "ci-topos-v0";
    let decision = serde_json::json!({
        "decisionKind": "ci.required.decision.v1",
        "decision": "accept",
        "projectionDigest": "proj1_demo",
        "typedCoreProjectionDigest": typed_core_projection_digest,
        "authorityPayloadDigest": authority_payload_digest,
        "normalizerId": normalizer_id,
        "policyDigest": policy_digest,
        "requiredChecks": ["baseline"],
        "witnessSha256": "witness_hash",
        "deltaSha256": "delta_hash"
    });
    let witness = serde_json::json!({
        "typedCoreProjectionDigest": typed_core_projection_digest,
        "authorityPayloadDigest": authority_payload_digest,
        "normalizerId": normalizer_id,
        "policyDigest": policy_digest,
        "projectionDigest": "proj1_demo",
        "requiredChecks": ["baseline"]
    });
    let delta = serde_json::json!({
        "typedCoreProjectionDigest": typed_core_projection_digest,
        "authorityPayloadDigest": authority_payload_digest,
        "normalizerId": normalizer_id,
        "policyDigest": policy_digest,
        "projectionDigest": "proj1_demo",
        "requiredChecks": ["baseline"]
    });
    let input = serde_json::json!({
        "decision": decision,
        "witness": witness,
        "deltaSnapshot": delta,
        "actualWitnessSha256": "witness_hash",
        "actualDeltaSha256": "delta_hash"
    });
    let input_path = dir.join("required-decision-verify-input.json");
    fs::write(
        &input_path,
        serde_json::to_vec_pretty(&input).expect("decision verify input should serialize"),
    )
    .expect("required decision verify input should be written");
    input_path
}

fn write_required_gate_ref_input(dir: &Path) -> PathBuf {
    let input = serde_json::json!({
        "checkId": "baseline",
        "artifactRelPath": "gates/proj1_demo/01-baseline.json",
        "source": "fallback",
        "fallback": {
            "exitCode": 1,
            "projectionDigest": "proj1_demo",
            "policyDigest": "ci-topos-v0",
            "ctxRef": "origin/main",
            "dataHeadRef": "HEAD"
        }
    });
    let input_path = dir.join("required-gate-ref-input.json");
    fs::write(
        &input_path,
        serde_json::to_vec_pretty(&input).expect("gate ref input should serialize"),
    )
    .expect("required gate ref input should be written");
    input_path
}

fn write_observation_surface(path: &Path) {
    let payload = serde_json::json!({
        "schema": 1,
        "surfaceKind": "ci.observation.surface.v0",
        "summary": {
            "state": "accepted",
            "needsAttention": false,
            "topFailureClass": "verified_accept",
            "latestProjectionDigest": "ev1_alpha",
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
                "typedCoreProjectionDigest": "ev1_alpha",
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
                "typedCoreProjectionDigest": "ev1_alpha",
                "verdictClass": "accepted",
                "requiredChecks": ["baseline"],
                "executedChecks": ["baseline"],
                "failureClasses": []
            },
            "decision": {
                "ref": "artifacts/ciwitness/latest-decision.json",
                "decisionKind": "ci.required.decision.v1",
                "projectionDigest": "proj1_alpha",
                "typedCoreProjectionDigest": "ev1_alpha",
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
fn issue_claim_next_json_smoke() {
    let tmp = TempDirGuard::new("issue-claim-next");
    let issues = tmp.path().join("issues.jsonl");
    write_claim_next_issues(&issues);

    let first = run_premath([
        OsString::from("issue"),
        OsString::from("claim-next"),
        OsString::from("--assignee"),
        OsString::from("worker-a"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&first);
    let first_payload = parse_json_stdout(&first);
    assert_eq!(first_payload["action"], "issue.claim_next");
    assert_eq!(first_payload["claimed"], true);
    assert_eq!(first_payload["issue"]["id"], "bd-1");
    assert_eq!(first_payload["issue"]["status"], "in_progress");
    assert_eq!(first_payload["issue"]["assignee"], "worker-a");
    assert_eq!(
        first_payload["issue"]["lease"]["leaseId"],
        serde_json::json!("lease1_bd-1_worker-a")
    );

    let second = run_premath([
        OsString::from("issue"),
        OsString::from("claim-next"),
        OsString::from("--assignee"),
        OsString::from("worker-a"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&second);
    let second_payload = parse_json_stdout(&second);
    assert_eq!(second_payload["claimed"], true);
    assert_eq!(second_payload["issue"]["id"], "bd-2");

    let third = run_premath([
        OsString::from("issue"),
        OsString::from("claim-next"),
        OsString::from("--assignee"),
        OsString::from("worker-a"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&third);
    let third_payload = parse_json_stdout(&third);
    assert_eq!(third_payload["claimed"], false);
    assert_eq!(third_payload["issue"], Value::Null);
}

#[test]
fn issue_claim_next_json_contention_smoke() {
    let tmp = TempDirGuard::new("issue-claim-next-contention");
    let issues = tmp.path().join("issues.jsonl");
    let lines = [
        r#"{"id":"bd-1","title":"Issue 1","status":"open"}"#,
        r#"{"id":"bd-2","title":"Issue 2","status":"open"}"#,
        r#"{"id":"bd-3","title":"Issue 3","status":"open"}"#,
        r#"{"id":"bd-4","title":"Issue 4","status":"open"}"#,
    ];
    fs::write(&issues, format!("{}\n", lines.join("\n")))
        .expect("contention issues should be written");

    let workers = 4;
    let barrier = Arc::new(Barrier::new(workers + 1));
    let mut handles = Vec::new();
    for idx in 0..workers {
        let issues = issues.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let assignee = format!("worker-{idx}");
            barrier.wait();
            let output = run_premath([
                OsString::from("issue"),
                OsString::from("claim-next"),
                OsString::from("--assignee"),
                OsString::from(assignee),
                OsString::from("--issues"),
                issues.as_os_str().to_os_string(),
                OsString::from("--json"),
            ]);
            assert_success(&output);
            let payload = parse_json_stdout(&output);
            assert_eq!(payload["claimed"], true);
            payload["issue"]["id"]
                .as_str()
                .expect("claimed issue id should be present")
                .to_string()
        }));
    }
    barrier.wait();

    let mut claimed_ids = handles
        .into_iter()
        .map(|handle| handle.join().expect("worker should join"))
        .collect::<Vec<_>>();
    claimed_ids.sort();
    claimed_ids.dedup();
    assert_eq!(
        claimed_ids,
        vec![
            "bd-1".to_string(),
            "bd-2".to_string(),
            "bd-3".to_string(),
            "bd-4".to_string()
        ]
    );
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
fn proposal_check_json_smoke() {
    let tmp = TempDirGuard::new("proposal-check-json");
    let proposal = write_proposal_input(tmp.path());

    let output = run_premath([
        OsString::from("proposal-check"),
        OsString::from("--proposal"),
        proposal.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["canonical"]["proposalKind"], "value");
    assert_eq!(payload["discharge"]["outcome"], "accepted");
    assert_eq!(
        payload["discharge"]["failureClasses"],
        serde_json::json!([])
    );
    assert!(
        payload["kcirRef"]
            .as_str()
            .expect("kcirRef should be string")
            .starts_with("kcir1_")
    );
}

#[test]
fn instruction_check_json_smoke() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should be two levels above crate dir")
        .to_path_buf();
    let instruction = repo_root
        .join("tests")
        .join("ci")
        .join("fixtures")
        .join("instructions")
        .join("20260221T010000Z-ci-wiring-golden.json");

    let output = run_premath([
        OsString::from("instruction-check"),
        OsString::from("--instruction"),
        instruction.as_os_str().to_os_string(),
        OsString::from("--repo-root"),
        repo_root.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["normalizerId"], "normalizer.ci.v1");
    assert!(
        payload["policyDigest"]
            .as_str()
            .expect("policyDigest should be string")
            .starts_with("pol1_")
    );
    assert_eq!(
        payload["requestedChecks"],
        serde_json::json!(["ci-wiring-check"])
    );
}

#[test]
fn instruction_witness_json_smoke() {
    let tmp = TempDirGuard::new("instruction-witness-json");
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should be two levels above crate dir")
        .to_path_buf();
    let instruction = repo_root
        .join("tests")
        .join("ci")
        .join("fixtures")
        .join("instructions")
        .join("20260221T010000Z-ci-wiring-golden.json");
    let runtime = write_instruction_runtime_input(
        tmp.path(),
        "tests/ci/fixtures/instructions/20260221T010000Z-ci-wiring-golden.json",
        false,
    );

    let output = run_premath([
        OsString::from("instruction-witness"),
        OsString::from("--instruction"),
        instruction.as_os_str().to_os_string(),
        OsString::from("--runtime"),
        runtime.as_os_str().to_os_string(),
        OsString::from("--repo-root"),
        repo_root.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["witnessKind"], "ci.instruction.v1");
    assert_eq!(
        payload["instructionId"],
        "20260221T010000Z-ci-wiring-golden"
    );
    assert_eq!(payload["verdictClass"], "accepted");
    assert_eq!(payload["failureClasses"], serde_json::json!([]));
}

#[test]
fn required_witness_json_smoke() {
    let tmp = TempDirGuard::new("required-witness-json");
    let runtime = write_required_runtime_input(tmp.path(), true);

    let output = run_premath([
        OsString::from("required-witness"),
        OsString::from("--runtime"),
        runtime.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["witnessKind"], "ci.required.v1");
    assert_eq!(payload["projectionDigest"], "proj1_demo");
    assert_eq!(payload["verdictClass"], "rejected");
    assert_eq!(
        payload["failureClasses"],
        serde_json::json!(["check_failed", "descent_failure"])
    );
}

#[test]
fn required_projection_json_smoke() {
    let tmp = TempDirGuard::new("required-projection-json");
    let input =
        write_required_projection_input(tmp.path(), vec!["crates/premath-kernel/src/lib.rs"]);

    let output = run_premath([
        OsString::from("required-projection"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["schema"], 1);
    assert_eq!(payload["projectionPolicy"], "ci-topos-v0");
    assert_eq!(
        payload["changedPaths"],
        serde_json::json!(["crates/premath-kernel/src/lib.rs"])
    );
    assert_eq!(
        payload["requiredChecks"],
        serde_json::json!(["build", "test", "test-toy", "test-kcir-toy"])
    );
    assert_eq!(payload["docsOnly"], false);
}

#[test]
fn required_delta_json_smoke() {
    let tmp = TempDirGuard::new("required-delta-json");
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(&repo_root).expect("repo root should be created");

    run_git(&repo_root, ["init", "--quiet"]);
    let readme = repo_root.join("README.md");
    fs::write(&readme, "first line\n").expect("initial readme should be written");
    run_git(&repo_root, ["add", "README.md"]);
    run_git(
        &repo_root,
        [
            "-c",
            "user.name=Premath Test",
            "-c",
            "user.email=premath@example.com",
            "commit",
            "-m",
            "init",
            "--quiet",
        ],
    );
    fs::write(&readme, "first line\nsecond line\n").expect("readme should be updated");

    let input = write_required_delta_input(tmp.path(), &repo_root, Some("HEAD"), Some("HEAD"));
    let output = run_premath([
        OsString::from("required-delta"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["schema"], 1);
    assert_eq!(payload["deltaKind"], "ci.required.delta.v1");
    assert_eq!(payload["fromRef"], "HEAD");
    assert_eq!(payload["toRef"], "HEAD");
    assert!(
        payload["source"]
            .as_str()
            .expect("source should be a string")
            .contains("workspace")
    );
    assert_eq!(payload["changedPaths"], serde_json::json!(["README.md"]));
}

#[test]
fn required_witness_verify_json_smoke() {
    let tmp = TempDirGuard::new("required-witness-verify-json");
    let input = write_required_verify_input(tmp.path(), false);

    let output = run_premath([
        OsString::from("required-witness-verify"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert!(
        payload["errors"]
            .as_array()
            .expect("errors should be an array")
            .iter()
            .any(|row| row
                .as_str()
                .unwrap_or_default()
                .contains("projectionDigest mismatch"))
    );
    assert!(
        payload["derived"]["projectionDigest"]
            .as_str()
            .expect("projectionDigest should be string")
            .starts_with("proj1_")
    );
    assert_eq!(payload["derived"]["expectedVerdict"], "accepted");
}

#[test]
fn required_witness_decide_json_smoke() {
    let tmp = TempDirGuard::new("required-witness-decide-json");
    let input = write_required_decide_input(tmp.path(), false);

    let output = run_premath([
        OsString::from("required-witness-decide"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["decisionKind"], "ci.required.decision.v1");
    assert_eq!(payload["decision"], "reject");
    assert_eq!(payload["reasonClass"], "verification_reject");
    assert!(
        payload["errors"]
            .as_array()
            .expect("errors should be an array")
            .iter()
            .any(|row| row
                .as_str()
                .unwrap_or_default()
                .contains("projectionDigest mismatch"))
    );
}

#[test]
fn required_decision_verify_json_smoke() {
    let tmp = TempDirGuard::new("required-decision-verify-json");
    let input = write_required_decision_verify_input(tmp.path());

    let output = run_premath([
        OsString::from("required-decision-verify"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["errors"], serde_json::json!([]));
    assert_eq!(payload["derived"]["decision"], "accept");
    assert_eq!(payload["derived"]["projectionDigest"], "proj1_demo");
    assert_eq!(
        payload["derived"]["requiredChecks"],
        serde_json::json!(["baseline"])
    );
}

#[test]
fn required_gate_ref_json_smoke() {
    let tmp = TempDirGuard::new("required-gate-ref-json");
    let input = write_required_gate_ref_input(tmp.path());

    let output = run_premath([
        OsString::from("required-gate-ref"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["gateWitnessRef"]["checkId"], "baseline");
    assert_eq!(payload["gateWitnessRef"]["source"], "fallback");
    assert_eq!(payload["gateWitnessRef"]["result"], "rejected");
    assert_eq!(
        payload["gateWitnessRef"]["failureClasses"],
        serde_json::json!(["descent_failure"])
    );
    assert_eq!(payload["gatePayload"]["result"], "rejected");
}

#[test]
fn obligation_registry_json_smoke() {
    let output = run_premath(["obligation-registry", "--json"]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["schema"], 1);
    assert_eq!(
        payload["registryKind"],
        serde_json::json!("premath.obligation_gate_registry.v1")
    );
    let mappings = payload["mappings"]
        .as_array()
        .expect("mappings should be an array");
    assert!(
        mappings
            .iter()
            .any(|row| row["obligationKind"] == "stability"
                && row["failureClass"] == "stability_failure"
                && row["lawRef"] == "GATE-3.1")
    );
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
    assert_eq!(payload["summary"]["latestProjectionDigest"], "ev1_alpha");
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
fn observe_projection_uses_typed_default() {
    let tmp = TempDirGuard::new("observe-projection-typed-default");
    let surface = tmp.path().join("surface.json");
    write_observation_surface(&surface);

    let output = run_premath([
        OsString::from("observe"),
        OsString::from("--surface"),
        surface.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("projection"),
        OsString::from("--projection-digest"),
        OsString::from("ev1_alpha"),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["projectionDigest"], "ev1_alpha");
    assert_eq!(
        payload["required"]["typedCoreProjectionDigest"],
        "ev1_alpha"
    );
}

#[test]
fn observe_projection_alias_requires_compatibility_mode() {
    let tmp = TempDirGuard::new("observe-projection-alias-mode");
    let surface = tmp.path().join("surface.json");
    write_observation_surface(&surface);

    let default_mode = run_premath([
        OsString::from("observe"),
        OsString::from("--surface"),
        surface.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("projection"),
        OsString::from("--projection-digest"),
        OsString::from("proj1_alpha"),
        OsString::from("--json"),
    ]);
    assert!(
        !default_mode.status.success(),
        "alias lookup should fail in typed default mode"
    );

    let compat_mode = run_premath([
        OsString::from("observe"),
        OsString::from("--surface"),
        surface.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("projection"),
        OsString::from("--projection-digest"),
        OsString::from("proj1_alpha"),
        OsString::from("--projection-match"),
        OsString::from("compatibility_alias"),
        OsString::from("--json"),
    ]);
    assert_success(&compat_mode);
    let payload = parse_json_stdout(&compat_mode);
    assert_eq!(payload["projectionDigest"], "proj1_alpha");
    assert_eq!(
        payload["required"]["typedCoreProjectionDigest"],
        "ev1_alpha"
    );
}

#[test]
fn observe_build_json_smoke() {
    let tmp = TempDirGuard::new("observe-build-json");
    let repo_root = tmp.path();
    let ciwitness = repo_root.join("artifacts/ciwitness");
    let issues = repo_root.join(".premath/issues.jsonl");
    fs::create_dir_all(&ciwitness).expect("ciwitness dir should be created");
    fs::create_dir_all(issues.parent().expect("issues parent should exist"))
        .expect("issues parent should be created");

    fs::write(
        ciwitness.join("latest-delta.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "projectionPolicy": "ci-topos-v0",
            "projectionDigest": "proj1_alpha",
            "typedCoreProjectionDigest": "ev1_alpha",
            "deltaSource": "explicit",
            "changedPaths": ["README.md"]
        }))
        .expect("delta should serialize"),
    )
    .expect("delta should write");
    fs::write(
        ciwitness.join("latest-required.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "witnessKind": "ci.required.v1",
            "projectionPolicy": "ci-topos-v0",
            "projectionDigest": "proj1_alpha",
            "typedCoreProjectionDigest": "ev1_alpha",
            "verdictClass": "accepted",
            "requiredChecks": ["baseline"],
            "executedChecks": ["baseline"],
            "failureClasses": []
        }))
        .expect("required should serialize"),
    )
    .expect("required should write");
    fs::write(
        ciwitness.join("latest-decision.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "decisionKind": "ci.required.decision.v1",
            "projectionDigest": "proj1_alpha",
            "typedCoreProjectionDigest": "ev1_alpha",
            "decision": "accept",
            "reasonClass": "verified_accept"
        }))
        .expect("decision should serialize"),
    )
    .expect("decision should write");
    fs::write(
        ciwitness.join("20260222T010000Z-ci.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "witnessKind": "ci.instruction.v1",
            "instructionId": "20260222T010000Z-ci",
            "instructionDigest": "instr1_alpha",
            "policyDigest": "pol1_alpha",
            "verdictClass": "accepted",
            "requiredChecks": ["baseline"],
            "executedChecks": ["baseline"],
            "failureClasses": [],
            "runFinishedAt": "2026-02-22T01:00:00Z"
        }))
        .expect("instruction should serialize"),
    )
    .expect("instruction should write");
    fs::write(
        &issues,
        "{\"id\":\"bd-root\",\"title\":\"Root\",\"status\":\"open\"}\n",
    )
    .expect("issues should write");

    let out_json = repo_root.join("artifacts/observation/latest.json");
    let out_jsonl = repo_root.join("artifacts/observation/events.jsonl");
    let output = run_premath([
        OsString::from("observe-build"),
        OsString::from("--repo-root"),
        repo_root.as_os_str().to_os_string(),
        OsString::from("--ciwitness-dir"),
        OsString::from("artifacts/ciwitness"),
        OsString::from("--issues-path"),
        OsString::from(".premath/issues.jsonl"),
        OsString::from("--out-json"),
        OsString::from("artifacts/observation/latest.json"),
        OsString::from("--out-jsonl"),
        OsString::from("artifacts/observation/events.jsonl"),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["summary"]["state"], "accepted");
    assert_eq!(payload["summary"]["latestProjectionDigest"], "ev1_alpha");
    assert_eq!(payload["summary"]["requiredCheckCount"], 1);
    assert!(payload["summary"]["coherence"].is_object());
    assert!(out_json.exists());
    assert!(out_jsonl.exists());

    let events_raw = fs::read_to_string(out_jsonl).expect("events jsonl should read");
    assert!(events_raw.contains("\"kind\":\"ci.required.v1.summary\""));
    assert!(events_raw.contains("\"kind\":\"ci.observation.surface.v0.summary\""));
}

#[test]
fn ref_project_json_smoke() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should be two levels above crate dir")
        .to_path_buf();
    let profile = repo_root
        .join("policies")
        .join("ref")
        .join("sha256_detached_v1.json");

    let output = run_premath([
        OsString::from("ref"),
        OsString::from("project"),
        OsString::from("--profile"),
        profile.as_os_str().to_os_string(),
        OsString::from("--domain"),
        OsString::from("kcir.node"),
        OsString::from("--payload-hex"),
        OsString::from("deadbeef"),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["schema"], 1);
    assert_eq!(payload["profileId"], "ref.sha256.detached.v1");
    assert_eq!(payload["ref"]["schemeId"], "ref.sha256.detached.v1");
    assert_eq!(payload["ref"]["paramsHash"], "sha256.detached.params.v1");
    assert_eq!(payload["ref"]["domain"], "kcir.node");
    assert_eq!(
        payload["ref"]["digest"],
        "c461b57a070b9629fbfb7ebb028bc18855b01fad8f8ce5221eb2ddd95ca5fdda"
    );
}

#[test]
fn ref_verify_json_smoke() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should be two levels above crate dir")
        .to_path_buf();
    let profile = repo_root
        .join("policies")
        .join("ref")
        .join("sha256_detached_v1.json");

    let output = run_premath([
        OsString::from("ref"),
        OsString::from("verify"),
        OsString::from("--profile"),
        profile.as_os_str().to_os_string(),
        OsString::from("--domain"),
        OsString::from("kcir.node"),
        OsString::from("--payload-hex"),
        OsString::from("deadbeef"),
        OsString::from("--evidence-hex"),
        OsString::from(""),
        OsString::from("--ref-scheme-id"),
        OsString::from("ref.sha256.detached.v1"),
        OsString::from("--ref-params-hash"),
        OsString::from("sha256.detached.params.v1"),
        OsString::from("--ref-domain"),
        OsString::from("kcir.node"),
        OsString::from("--ref-digest"),
        OsString::from("c461b57a070b9629fbfb7ebb028bc18855b01fad8f8ce5221eb2ddd95ca5fdda"),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["schema"], 1);
    assert_eq!(payload["result"], "accepted");
    assert_eq!(payload["failureClasses"], serde_json::json!([]));
    assert_eq!(
        payload["projectedRef"]["schemeId"],
        "ref.sha256.detached.v1"
    );
    assert_eq!(
        payload["projectedRef"]["paramsHash"],
        "sha256.detached.params.v1"
    );
    assert_eq!(payload["projectedRef"]["domain"], "kcir.node");
    assert_eq!(
        payload["projectedRef"]["digest"],
        "c461b57a070b9629fbfb7ebb028bc18855b01fad8f8ce5221eb2ddd95ca5fdda"
    );
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
fn issue_check_json_smoke() {
    let tmp = TempDirGuard::new("issue-check");
    let issues_ok = tmp.path().join("issues-ok.jsonl");
    let issues_bad = tmp.path().join("issues-bad.jsonl");

    fs::write(
        &issues_ok,
        concat!(
            "{\"id\":\"bd-ok\",\"title\":\"Issue ok\",\"status\":\"open\",\"issue_type\":\"task\",",
            "\"description\":\"Acceptance:\\n- complete work\\n\\nVerification commands:\\n- `mise run baseline`\"}\n"
        ),
    )
    .expect("valid issues should be written");

    let out_ok = run_premath([
        OsString::from("issue"),
        OsString::from("check"),
        OsString::from("--issues"),
        issues_ok.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_ok);
    let ok_payload = parse_json_stdout(&out_ok);
    assert_eq!(ok_payload["action"], "issue.check");
    assert_eq!(ok_payload["checkKind"], "premath.issue_graph.check.v1");
    assert_eq!(ok_payload["result"], "accepted");
    assert_eq!(ok_payload["summary"]["errorCount"], 0);

    fs::write(
        &issues_bad,
        concat!(
            "{\"id\":\"bd-epic\",\"title\":\"[EPIC] Broken\",\"status\":\"open\",\"issue_type\":\"task\",",
            "\"description\":\"Acceptance:\\n- done\\n\\nVerification commands:\\n- `mise run baseline`\"}\n"
        ),
    )
    .expect("invalid issues should be written");

    let out_bad = run_premath([
        OsString::from("issue"),
        OsString::from("check"),
        OsString::from("--issues"),
        issues_bad.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert!(
        !out_bad.status.success(),
        "expected issue check to fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out_bad.stdout),
        String::from_utf8_lossy(&out_bad.stderr)
    );
    let bad_payload = parse_json_stdout(&out_bad);
    assert_eq!(bad_payload["action"], "issue.check");
    assert_eq!(bad_payload["result"], "rejected");
    assert_eq!(
        bad_payload["failureClasses"],
        serde_json::json!(["issue_graph.issue_type.epic_mismatch"])
    );
}

#[test]
fn issue_backend_status_json_smoke() {
    let tmp = TempDirGuard::new("issue-backend-status");
    let issues = tmp.path().join("issues.jsonl");
    let projection = tmp.path().join("surreal_issue_cache.json");

    write_sample_issues(&issues);
    fs::write(&projection, "{}").expect("projection cache should write");

    let out_status = run_premath([
        OsString::from("issue"),
        OsString::from("backend-status"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--repo"),
        tmp.path().as_os_str().to_os_string(),
        OsString::from("--projection"),
        projection.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_status);

    let payload = parse_json_stdout(&out_status);
    assert_eq!(payload["action"], "issue.backend-status");
    assert_eq!(payload["canonicalMemory"]["kind"], "jsonl");
    assert_eq!(payload["canonicalMemory"]["exists"], true);
    assert_eq!(
        payload["queryProjection"]["kind"],
        "premath.surreal.issue_projection.v0"
    );
    assert_eq!(payload["queryProjection"]["exists"], true);
    assert_eq!(payload["queryProjection"]["state"], "invalid");
    assert!(payload["queryProjection"]["error"].is_string());
    assert!(payload["jj"]["available"].is_boolean());
    let jj_state = payload["jj"]["state"]
        .as_str()
        .expect("jj.state should be a string");
    assert!(jj_state == "ready" || jj_state == "error" || jj_state == "unavailable");
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
fn issue_replay_events_json_smoke() {
    let tmp = TempDirGuard::new("issue-replay-events");
    let issues = tmp.path().join("issues.jsonl");
    let replayed_issues = tmp.path().join("replayed-issues.jsonl");
    let events = tmp.path().join("events.jsonl");
    let replay_cache = tmp.path().join("replay-cache.json");

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

    let out_replay_first = run_premath([
        OsString::from("issue"),
        OsString::from("replay-events"),
        OsString::from("--events"),
        events.as_os_str().to_os_string(),
        OsString::from("--issues"),
        replayed_issues.as_os_str().to_os_string(),
        OsString::from("--cache"),
        replay_cache.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_replay_first);
    let replay_first = parse_json_stdout(&out_replay_first);
    assert_eq!(replay_first["action"], "issue.replay-events");
    assert_eq!(replay_first["cacheHit"], false);
    assert_eq!(replay_first["eventCount"], 3);
    assert_eq!(replay_first["issueCount"], 2);
    assert_eq!(replay_first["equivalentToExisting"], Value::Null);
    assert_eq!(
        replay_first["cachePath"],
        replay_cache.display().to_string()
    );
    let event_ref_first = replay_first["eventStreamRef"]
        .as_str()
        .expect("eventStreamRef should be a string")
        .to_string();
    let snapshot_ref_first = replay_first["snapshotRef"]
        .as_str()
        .expect("snapshotRef should be a string")
        .to_string();
    assert!(event_ref_first.starts_with("ev1_"));
    assert!(snapshot_ref_first.starts_with("iss1_"));
    let replay_cache_payload = serde_json::from_str::<Value>(
        &fs::read_to_string(&replay_cache).expect("replay cache should exist"),
    )
    .expect("replay cache should parse");
    assert_eq!(replay_cache_payload["schema"], "issue.replay.cache.v1");
    assert_eq!(replay_cache_payload["eventStreamRef"], event_ref_first);
    assert_eq!(replay_cache_payload["snapshotRef"], snapshot_ref_first);

    let out_replay_second = run_premath([
        OsString::from("issue"),
        OsString::from("replay-events"),
        OsString::from("--events"),
        events.as_os_str().to_os_string(),
        OsString::from("--issues"),
        replayed_issues.as_os_str().to_os_string(),
        OsString::from("--cache"),
        replay_cache.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_replay_second);
    let replay_second = parse_json_stdout(&out_replay_second);
    assert_eq!(replay_second["cacheHit"], true);
    assert_eq!(replay_second["equivalentToExisting"], true);
    assert_eq!(replay_second["eventStreamRef"], event_ref_first);
    assert_eq!(replay_second["snapshotRef"], snapshot_ref_first);

    let out_ready = run_premath([
        OsString::from("issue"),
        OsString::from("ready"),
        OsString::from("--issues"),
        replayed_issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_ready);
    let ready = parse_json_stdout(&out_ready);
    assert_eq!(ready["count"], 1);
    assert_eq!(ready["items"][0]["id"], "bd-root");
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
fn dep_remove_replace_and_diagnostics_json_smoke() {
    let tmp = TempDirGuard::new("dep-remove-replace");
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

    let out_dep_add = run_premath([
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
    assert_success(&out_dep_add);

    let out_dep_replace = run_premath([
        OsString::from("dep"),
        OsString::from("replace"),
        OsString::from("bd-child"),
        OsString::from("bd-root"),
        OsString::from("--from-type"),
        OsString::from("blocks"),
        OsString::from("--to-type"),
        OsString::from("related"),
        OsString::from("--created-by"),
        OsString::from("codex"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_dep_replace);
    let replace_payload = parse_json_stdout(&out_dep_replace);
    assert_eq!(replace_payload["action"], "dep.replace");
    assert_eq!(replace_payload["dependency"]["fromType"], "blocks");
    assert_eq!(replace_payload["dependency"]["toType"], "related");

    let out_dep_remove = run_premath([
        OsString::from("dep"),
        OsString::from("remove"),
        OsString::from("bd-child"),
        OsString::from("bd-root"),
        OsString::from("--type"),
        OsString::from("related"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_dep_remove);
    let remove_payload = parse_json_stdout(&out_dep_remove);
    assert_eq!(remove_payload["action"], "dep.remove");
    assert_eq!(remove_payload["dependency"]["type"], "related");

    let out_diag = run_premath([
        OsString::from("dep"),
        OsString::from("diagnostics"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_diag);
    let diag_payload = parse_json_stdout(&out_diag);
    assert_eq!(diag_payload["action"], "dep.diagnostics");
    assert_eq!(diag_payload["graphScope"], "active");
    assert_eq!(diag_payload["integrity"]["hasCycle"], false);
    assert_eq!(diag_payload["integrity"]["cyclePath"], Value::Null);

    let out_dep_root_child = run_premath([
        OsString::from("dep"),
        OsString::from("add"),
        OsString::from("bd-root"),
        OsString::from("bd-child"),
        OsString::from("--type"),
        OsString::from("blocks"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_dep_root_child);

    let out_cycle = run_premath([
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
    assert!(
        !out_cycle.status.success(),
        "expected cycle add to fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out_cycle.stdout),
        String::from_utf8_lossy(&out_cycle.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out_cycle.stderr).contains("dependency cycle detected"),
        "expected cycle diagnostic in stderr, got:\n{}",
        String::from_utf8_lossy(&out_cycle.stderr)
    );
}

#[test]
fn dep_diagnostics_scope_filters_closed_cycle_noise() {
    let tmp = TempDirGuard::new("dep-diagnostics-scope");
    let issues = tmp.path().join("issues.jsonl");
    fs::write(
        &issues,
        concat!(
            r#"{"id":"bd-a","title":"A","status":"closed","dependencies":[{"issue_id":"bd-a","depends_on_id":"bd-b","type":"blocks"}]}"#,
            "\n",
            r#"{"id":"bd-b","title":"B","status":"closed","dependencies":[{"issue_id":"bd-b","depends_on_id":"bd-a","type":"blocks"}]}"#,
            "\n",
            r#"{"id":"bd-c","title":"C","status":"open"}"#,
            "\n"
        ),
    )
    .expect("issues fixture should write");

    let out_active = run_premath([
        OsString::from("dep"),
        OsString::from("diagnostics"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_active);
    let active_payload = parse_json_stdout(&out_active);
    assert_eq!(active_payload["graphScope"], "active");
    assert_eq!(active_payload["integrity"]["hasCycle"], false);
    assert_eq!(active_payload["integrity"]["cyclePath"], Value::Null);

    let out_full = run_premath([
        OsString::from("dep"),
        OsString::from("diagnostics"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--graph-scope"),
        OsString::from("full"),
        OsString::from("--json"),
    ]);
    assert_success(&out_full);
    let full_payload = parse_json_stdout(&out_full);
    assert_eq!(full_payload["graphScope"], "full");
    assert_eq!(full_payload["integrity"]["hasCycle"], true);
    assert_eq!(
        full_payload["integrity"]["cyclePath"],
        serde_json::json!(["bd-a", "bd-b", "bd-a"])
    );
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

#[test]
fn coherence_check_rejects_on_coherence_spec_obligation_drift() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should be two levels above crate dir")
        .to_path_buf();
    let contract_path = repo_root.join("specs/premath/draft/COHERENCE-CONTRACT.json");
    let coherence_spec_path = repo_root.join("specs/premath/draft/PREMATH-COHERENCE.md");

    let temp = TempDirGuard::new("coherence-obligation-drift");
    let mutated_spec_path = temp.path().join("PREMATH-COHERENCE.drift.md");
    let mutated_contract_path = temp.path().join("COHERENCE-CONTRACT.drift.json");

    let coherence_spec =
        fs::read_to_string(&coherence_spec_path).expect("coherence spec should be readable");
    let mutated_coherence_spec =
        coherence_spec.replacen("`cwf_comprehension_eta`", "`cwf_comprehension_eta_typo`", 1);
    assert_ne!(
        coherence_spec, mutated_coherence_spec,
        "expected coherence spec mutation to change content"
    );
    fs::write(&mutated_spec_path, mutated_coherence_spec)
        .expect("mutated coherence spec should be writable");

    let mut contract: Value = serde_json::from_slice(
        &fs::read(&contract_path).expect("coherence contract should be readable"),
    )
    .expect("coherence contract should parse");
    let surfaces = contract
        .get_mut("surfaces")
        .and_then(Value::as_object_mut)
        .expect("contract surfaces should be object");
    surfaces.insert(
        "coherenceSpecPath".to_string(),
        Value::String(mutated_spec_path.to_string_lossy().to_string()),
    );
    fs::write(
        &mutated_contract_path,
        serde_json::to_vec_pretty(&contract).expect("mutated contract should serialize"),
    )
    .expect("mutated coherence contract should be writable");

    let output = run_premath([
        OsString::from("coherence-check"),
        OsString::from("--contract"),
        mutated_contract_path.as_os_str().to_os_string(),
        OsString::from("--repo-root"),
        repo_root.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "coherence-check should return non-zero on rejected witness"
    );

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["witnessKind"], "premath.coherence.v1");
    assert_eq!(
        payload["result"]
            .as_str()
            .expect("coherence-check result should be string"),
        "rejected"
    );

    let failure_classes = payload["failureClasses"]
        .as_array()
        .expect("failureClasses should be array");
    assert!(
        failure_classes.iter().any(|item| {
            item.as_str()
                == Some("coherence.scope_noncontradiction.coherence_spec_missing_obligation")
        }),
        "expected missing-obligation failure class in top-level union"
    );
    assert!(
        failure_classes.iter().any(|item| {
            item.as_str()
                == Some("coherence.scope_noncontradiction.coherence_spec_unknown_obligation")
        }),
        "expected unknown-obligation failure class in top-level union"
    );
}

#[test]
fn harness_session_write_read_bootstrap_json_smoke() {
    let tmp = TempDirGuard::new("harness-session");
    let session_path = tmp.path().join("harness-session.json");
    let issues = tmp.path().join("issues.jsonl");
    write_sample_issues(&issues);

    let out_write_stopped = run_premath([
        OsString::from("harness-session"),
        OsString::from("write"),
        OsString::from("--path"),
        session_path.as_os_str().to_os_string(),
        OsString::from("--state"),
        OsString::from("stopped"),
        OsString::from("--issue-id"),
        OsString::from("bd-a"),
        OsString::from("--summary"),
        OsString::from("finished slice"),
        OsString::from("--next-step"),
        OsString::from("run ci-hygiene-check"),
        OsString::from("--instruction-ref"),
        OsString::from("instructions/i-2.json"),
        OsString::from("--instruction-ref"),
        OsString::from("instructions/i-1.json"),
        OsString::from("--instruction-ref"),
        OsString::from("instructions/i-2.json"),
        OsString::from("--witness-ref"),
        OsString::from("artifacts/ciwitness/w-2.json"),
        OsString::from("--witness-ref"),
        OsString::from("artifacts/ciwitness/w-1.json"),
        OsString::from("--lineage-ref"),
        OsString::from("refinement://worker-loop/bd-a/worker.1/ref-a"),
        OsString::from("--lineage-ref"),
        OsString::from("ctx://issue/bd-a/ctx-a"),
        OsString::from("--lineage-ref"),
        OsString::from("refinement://worker-loop/bd-a/worker.1/ref-a"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_write_stopped);
    let write_stopped = parse_json_stdout(&out_write_stopped);
    assert_eq!(write_stopped["action"], "harness-session.write");
    let session = &write_stopped["session"];
    assert_eq!(session["schema"], 1);
    assert_eq!(session["sessionKind"], "premath.harness.session.v1");
    assert_eq!(session["state"], "stopped");
    assert_eq!(session["issueId"], "bd-a");
    assert_eq!(session["summary"], "finished slice");
    assert_eq!(session["nextStep"], "run ci-hygiene-check");
    assert_eq!(
        session["instructionRefs"],
        serde_json::json!(["instructions/i-1.json", "instructions/i-2.json"])
    );
    assert_eq!(
        session["witnessRefs"],
        serde_json::json!([
            "artifacts/ciwitness/w-1.json",
            "artifacts/ciwitness/w-2.json"
        ])
    );
    assert_eq!(
        session["lineageRefs"],
        serde_json::json!([
            "ctx://issue/bd-a/ctx-a",
            "refinement://worker-loop/bd-a/worker.1/ref-a"
        ])
    );
    assert_eq!(session["issuesPath"], issues.display().to_string());
    let session_id = session["sessionId"]
        .as_str()
        .expect("sessionId should be string")
        .to_string();
    assert!(session_id.starts_with("hs1_"));
    assert!(
        session["issuesSnapshotRef"]
            .as_str()
            .expect("issuesSnapshotRef should be string")
            .starts_with("iss1_")
    );

    let out_bootstrap_resume = run_premath([
        OsString::from("harness-session"),
        OsString::from("bootstrap"),
        OsString::from("--path"),
        session_path.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_bootstrap_resume);
    let bootstrap_resume = parse_json_stdout(&out_bootstrap_resume);
    assert_eq!(bootstrap_resume["action"], "harness-session.bootstrap");
    assert_eq!(
        bootstrap_resume["bootstrapKind"],
        "premath.harness.bootstrap.v1"
    );
    assert_eq!(bootstrap_resume["mode"], "resume");
    assert_eq!(bootstrap_resume["resumeIssueId"], "bd-a");
    assert_eq!(bootstrap_resume["sessionId"], session_id);
    assert_eq!(
        bootstrap_resume["lineageRefs"],
        serde_json::json!([
            "ctx://issue/bd-a/ctx-a",
            "refinement://worker-loop/bd-a/worker.1/ref-a"
        ])
    );
    assert_eq!(
        bootstrap_resume["sessionRef"],
        session_path.display().to_string()
    );

    let out_write_active = run_premath([
        OsString::from("harness-session"),
        OsString::from("write"),
        OsString::from("--path"),
        session_path.as_os_str().to_os_string(),
        OsString::from("--state"),
        OsString::from("active"),
        OsString::from("--summary"),
        OsString::from("continue work"),
        OsString::from("--issues"),
        issues.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_write_active);
    let write_active = parse_json_stdout(&out_write_active);
    assert_eq!(write_active["session"]["sessionId"], session_id);
    assert_eq!(write_active["session"]["state"], "active");
    assert_eq!(write_active["session"]["stoppedAt"], Value::Null);
    assert_eq!(write_active["session"]["summary"], "continue work");
    assert_eq!(
        write_active["session"]["lineageRefs"],
        serde_json::json!([
            "ctx://issue/bd-a/ctx-a",
            "refinement://worker-loop/bd-a/worker.1/ref-a"
        ])
    );

    let out_read = run_premath([
        OsString::from("harness-session"),
        OsString::from("read"),
        OsString::from("--path"),
        session_path.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_read);
    let read = parse_json_stdout(&out_read);
    assert_eq!(read["action"], "harness-session.read");
    assert_eq!(read["session"]["sessionId"], session_id);
    assert_eq!(read["session"]["state"], "active");
    assert_eq!(read["session"]["issueId"], "bd-a");
    assert_eq!(read["session"]["nextStep"], "run ci-hygiene-check");
    assert_eq!(
        read["session"]["lineageRefs"],
        serde_json::json!([
            "ctx://issue/bd-a/ctx-a",
            "refinement://worker-loop/bd-a/worker.1/ref-a"
        ])
    );

    let out_bootstrap_attach = run_premath([
        OsString::from("harness-session"),
        OsString::from("bootstrap"),
        OsString::from("--path"),
        session_path.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_bootstrap_attach);
    let bootstrap_attach = parse_json_stdout(&out_bootstrap_attach);
    assert_eq!(bootstrap_attach["mode"], "attach");
    assert_eq!(bootstrap_attach["sessionId"], session_id);
}

#[test]
fn harness_feature_ledger_incomplete_rejects_when_require_closure() {
    let tmp = TempDirGuard::new("harness-feature-incomplete");
    let ledger_path = tmp.path().join("feature-ledger.json");

    let out_write = run_premath([
        OsString::from("harness-feature"),
        OsString::from("write"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--feature-id"),
        OsString::from("feature.alpha"),
        OsString::from("--status"),
        OsString::from("pending"),
        OsString::from("--json"),
    ]);
    assert_success(&out_write);

    let out_check = run_premath([
        OsString::from("harness-feature"),
        OsString::from("check"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--require-closure"),
        OsString::from("--json"),
    ]);
    assert!(
        !out_check.status.success(),
        "expected closure check rejection, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out_check.stdout),
        String::from_utf8_lossy(&out_check.stderr)
    );
    let payload = parse_json_stdout(&out_check);
    assert_eq!(payload["action"], "harness-feature.check");
    assert_eq!(payload["result"], "rejected");
    assert_eq!(payload["nextFeatureId"], "feature.alpha");
    assert!(
        payload["failureClasses"]
            .as_array()
            .expect("failureClasses should be an array")
            .iter()
            .any(|row| row.as_str() == Some("harness_feature_ledger.closure_incomplete"))
    );
}

#[test]
fn harness_feature_ledger_malformed_rejects() {
    let tmp = TempDirGuard::new("harness-feature-malformed");
    let ledger_path = tmp.path().join("feature-ledger.json");
    fs::write(
        &ledger_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "ledgerKind": "premath.harness.feature_ledger.v1",
            "updatedAt": "2026-02-22T00:00:00Z",
            "features": [{
                "featureId": "feature.alpha",
                "status": "completed",
                "updatedAt": "2026-02-22T00:00:00Z",
                "verificationRefs": []
            }]
        }))
        .expect("malformed ledger should serialize"),
    )
    .expect("malformed ledger should write");

    let out_check = run_premath([
        OsString::from("harness-feature"),
        OsString::from("check"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert!(
        !out_check.status.success(),
        "expected malformed ledger rejection, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out_check.stdout),
        String::from_utf8_lossy(&out_check.stderr)
    );
    let payload = parse_json_stdout(&out_check);
    assert_eq!(payload["result"], "rejected");
    assert!(
        payload["failureClasses"]
            .as_array()
            .expect("failureClasses should be an array")
            .iter()
            .any(|row| {
                row.as_str() == Some("harness_feature_ledger.completed_missing_verification_ref")
            })
    );
}

#[test]
fn harness_feature_ledger_complete_and_next_json_smoke() {
    let tmp = TempDirGuard::new("harness-feature-complete");
    let ledger_path = tmp.path().join("feature-ledger.json");

    let out_write_pending = run_premath([
        OsString::from("harness-feature"),
        OsString::from("write"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--feature-id"),
        OsString::from("feature.beta"),
        OsString::from("--status"),
        OsString::from("pending"),
        OsString::from("--json"),
    ]);
    assert_success(&out_write_pending);

    let out_write_progress = run_premath([
        OsString::from("harness-feature"),
        OsString::from("write"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--feature-id"),
        OsString::from("feature.alpha"),
        OsString::from("--status"),
        OsString::from("in_progress"),
        OsString::from("--session-ref"),
        OsString::from(".premath/harness_session.json"),
        OsString::from("--json"),
    ]);
    assert_success(&out_write_progress);

    let out_next_open = run_premath([
        OsString::from("harness-feature"),
        OsString::from("next"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_next_open);
    let next_open = parse_json_stdout(&out_next_open);
    assert_eq!(next_open["action"], "harness-feature.next");
    assert_eq!(next_open["exists"], true);
    assert_eq!(next_open["nextFeatureId"], "feature.alpha");
    assert_eq!(next_open["closureComplete"], false);

    let out_complete_alpha = run_premath([
        OsString::from("harness-feature"),
        OsString::from("write"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--feature-id"),
        OsString::from("feature.alpha"),
        OsString::from("--status"),
        OsString::from("completed"),
        OsString::from("--verification-ref"),
        OsString::from("artifacts/ciwitness/alpha.json"),
        OsString::from("--json"),
    ]);
    assert_success(&out_complete_alpha);

    let out_complete_beta = run_premath([
        OsString::from("harness-feature"),
        OsString::from("write"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--feature-id"),
        OsString::from("feature.beta"),
        OsString::from("--status"),
        OsString::from("completed"),
        OsString::from("--verification-ref"),
        OsString::from("artifacts/ciwitness/beta.json"),
        OsString::from("--json"),
    ]);
    assert_success(&out_complete_beta);

    let out_check_closed = run_premath([
        OsString::from("harness-feature"),
        OsString::from("check"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--require-closure"),
        OsString::from("--json"),
    ]);
    assert_success(&out_check_closed);
    let check_closed = parse_json_stdout(&out_check_closed);
    assert_eq!(check_closed["result"], "accepted");
    assert_eq!(check_closed["summary"]["closureComplete"], true);
    assert_eq!(check_closed["summary"]["completedCount"], 2);
    assert_eq!(check_closed["nextFeatureId"], Value::Null);

    let out_next_closed = run_premath([
        OsString::from("harness-feature"),
        OsString::from("next"),
        OsString::from("--path"),
        ledger_path.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&out_next_closed);
    let next_closed = parse_json_stdout(&out_next_closed);
    assert_eq!(next_closed["nextFeatureId"], Value::Null);
    assert_eq!(next_closed["closureComplete"], true);
    assert_eq!(next_closed["featureCount"], 2);
}

#[test]
fn harness_trajectory_append_and_query_json_smoke() {
    let tmp = TempDirGuard::new("harness-trajectory");
    let path = tmp.path().join("harness-trajectory.jsonl");

    let out_append_1 = run_premath([
        OsString::from("harness-trajectory"),
        OsString::from("append"),
        OsString::from("--path"),
        path.as_os_str().to_os_string(),
        OsString::from("--step-id"),
        OsString::from("step-1"),
        OsString::from("--issue-id"),
        OsString::from("bd-1"),
        OsString::from("--action"),
        OsString::from("run.check"),
        OsString::from("--result-class"),
        OsString::from("transient_failure"),
        OsString::from("--witness-ref"),
        OsString::from("artifacts/ciwitness/w1.json"),
        OsString::from("--witness-ref"),
        OsString::from("artifacts/ciwitness/w1.json"),
        OsString::from("--lineage-ref"),
        OsString::from("refinement://worker-loop/bd-1/worker.1/ref-a"),
        OsString::from("--lineage-ref"),
        OsString::from("ctx://issue/bd-1/ctx-a"),
        OsString::from("--lineage-ref"),
        OsString::from("ctx://issue/bd-1/ctx-a"),
        OsString::from("--finished-at"),
        OsString::from("2026-02-22T00:01:00Z"),
        OsString::from("--json"),
    ]);
    assert_success(&out_append_1);
    let append_1 = parse_json_stdout(&out_append_1);
    assert_eq!(append_1["action"], "harness-trajectory.append");
    assert_eq!(append_1["row"]["stepKind"], "premath.harness.step.v1");
    assert_eq!(
        append_1["row"]["witnessRefs"],
        serde_json::json!(["artifacts/ciwitness/w1.json"])
    );
    assert_eq!(
        append_1["row"]["lineageRefs"],
        serde_json::json!([
            "ctx://issue/bd-1/ctx-a",
            "refinement://worker-loop/bd-1/worker.1/ref-a"
        ])
    );

    let out_append_2 = run_premath([
        OsString::from("harness-trajectory"),
        OsString::from("append"),
        OsString::from("--path"),
        path.as_os_str().to_os_string(),
        OsString::from("--step-id"),
        OsString::from("step-2"),
        OsString::from("--issue-id"),
        OsString::from("bd-2"),
        OsString::from("--action"),
        OsString::from("run.check"),
        OsString::from("--result-class"),
        OsString::from("policy_reject"),
        OsString::from("--witness-ref"),
        OsString::from("artifacts/ciwitness/w2.json"),
        OsString::from("--finished-at"),
        OsString::from("2026-02-22T00:02:00Z"),
        OsString::from("--json"),
    ]);
    assert_success(&out_append_2);

    let out_append_3 = run_premath([
        OsString::from("harness-trajectory"),
        OsString::from("append"),
        OsString::from("--path"),
        path.as_os_str().to_os_string(),
        OsString::from("--step-id"),
        OsString::from("step-3"),
        OsString::from("--issue-id"),
        OsString::from("bd-3"),
        OsString::from("--action"),
        OsString::from("apply.patch"),
        OsString::from("--result-class"),
        OsString::from("accepted"),
        OsString::from("--witness-ref"),
        OsString::from("artifacts/ciwitness/w3.json"),
        OsString::from("--finished-at"),
        OsString::from("2026-02-22T00:03:00Z"),
        OsString::from("--json"),
    ]);
    assert_success(&out_append_3);

    let out_latest = run_premath([
        OsString::from("harness-trajectory"),
        OsString::from("query"),
        OsString::from("--path"),
        path.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("latest"),
        OsString::from("--limit"),
        OsString::from("2"),
        OsString::from("--json"),
    ]);
    assert_success(&out_latest);
    let latest = parse_json_stdout(&out_latest);
    assert_eq!(latest["action"], "harness-trajectory.query");
    assert_eq!(
        latest["projectionKind"],
        "premath.harness.trajectory.projection.v1"
    );
    assert_eq!(latest["mode"], "latest");
    assert_eq!(latest["count"], 2);
    assert_eq!(latest["totalCount"], 3);
    assert_eq!(latest["failedCount"], 2);
    assert_eq!(latest["retryNeededCount"], 1);
    assert_eq!(latest["items"][0]["stepId"], "step-3");
    assert_eq!(latest["items"][1]["stepId"], "step-2");

    let out_failed = run_premath([
        OsString::from("harness-trajectory"),
        OsString::from("query"),
        OsString::from("--path"),
        path.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("failed"),
        OsString::from("--limit"),
        OsString::from("10"),
        OsString::from("--json"),
    ]);
    assert_success(&out_failed);
    let failed = parse_json_stdout(&out_failed);
    assert_eq!(failed["mode"], "failed");
    assert_eq!(failed["count"], 2);
    assert_eq!(failed["items"][0]["stepId"], "step-2");
    assert_eq!(failed["items"][1]["stepId"], "step-1");

    let out_retry = run_premath([
        OsString::from("harness-trajectory"),
        OsString::from("query"),
        OsString::from("--path"),
        path.as_os_str().to_os_string(),
        OsString::from("--mode"),
        OsString::from("retry-needed"),
        OsString::from("--limit"),
        OsString::from("10"),
        OsString::from("--json"),
    ]);
    assert_success(&out_retry);
    let retry = parse_json_stdout(&out_retry);
    assert_eq!(retry["mode"], "retry_needed");
    assert_eq!(retry["count"], 1);
    assert_eq!(retry["items"][0]["stepId"], "step-1");
}
