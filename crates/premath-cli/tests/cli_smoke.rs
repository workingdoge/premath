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
    let output = run_premath(["init"]);
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("premath init ."));
    assert!(stdout.contains("Creates local layout for adapter composition"));
}
