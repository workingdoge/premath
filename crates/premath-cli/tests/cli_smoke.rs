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
    let runtime = serde_json::json!({
        "projectionPolicy": "ci-topos-v0",
        "projectionDigest": "proj1_demo",
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
        "policyDigest": "ci-topos-v0",
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
