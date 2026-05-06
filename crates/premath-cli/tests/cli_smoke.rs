use serde_json::Value;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root should be two levels above crate dir")
        .to_path_buf()
}

fn run_premath<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_premath"))
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
    serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "expected valid JSON stdout, got error: {error}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn write_temp_json(name: &str, payload: &Value) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "premath-cli-smoke-{}-{nonce}-{name}.json",
        std::process::id()
    ));
    let bytes = serde_json::to_vec(payload).expect("temp input payload should serialize");
    fs::write(&path, bytes).expect("temp input payload should be writable");
    path
}

#[test]
fn work_tracker_check_json_smoke() {
    let case_path = workspace_root()
        .join("tests/checker/fixtures/work-tracker-checker/golden/simple_claim_accept/case.json");
    let case = serde_json::from_slice::<Value>(
        &fs::read(case_path).expect("work tracker checker fixture should be readable"),
    )
    .expect("work tracker checker fixture should parse");
    let input_payload = case
        .get("input")
        .expect("work tracker checker fixture should contain input");
    let input = write_temp_json("work-tracker-check", input_payload);

    let output = run_premath([
        OsString::from("work-tracker-check"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    let _ = fs::remove_file(&input);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["checkKind"], "premath.work_tracker_checker.raw.v1");
    assert_eq!(payload["result"], "accepted");
    assert_eq!(payload["failureClasses"], serde_json::json!([]));
}

#[test]
fn toy_gate_check_json_smoke() {
    let input = workspace_root().join("tests/toy/fixtures/golden_stability_sheaf_bits/case.json");

    let output = run_premath([
        OsString::from("toy-gate-check"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--json"),
    ]);
    assert_success(&output);

    let payload = parse_json_stdout(&output);
    assert_eq!(payload["witnessSchema"], 1);
    assert_eq!(payload["profile"], "toy");
    assert_eq!(payload["result"], "accepted");
    assert_eq!(payload["failures"], serde_json::json!([]));
}

#[test]
fn ref_project_json_smoke() {
    let profile = workspace_root().join("policies/ref/sha256_detached_v1.json");

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
    assert_eq!(payload["ref"]["domain"], "kcir.node");
}
