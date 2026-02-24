use crate::support::yes_no;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EvaluatorScaffoldOutcome {
    pub root: PathBuf,
    pub issues_path: PathBuf,
    pub contract_path: PathBuf,
    pub scheme_program_path: PathBuf,
    pub rhai_script_path: PathBuf,
    pub trajectory_path: PathBuf,
    pub created_root: bool,
    pub created_issues_file: bool,
    pub created_contract_file: bool,
    pub created_scheme_program_file: bool,
    pub created_rhai_script_file: bool,
    pub created_trajectory_file: bool,
    pub next_scheme_command: String,
    pub next_rhai_command: String,
}

pub fn scaffold_layout(path: impl AsRef<Path>) -> Result<EvaluatorScaffoldOutcome, String> {
    let root = path.as_ref().to_path_buf();

    let mut created_root = false;
    if !root.exists() {
        fs::create_dir_all(&root).map_err(|err| {
            format!(
                "failed to create scaffold directory {}: {err}",
                root.display()
            )
        })?;
        created_root = true;
    }
    if !root.is_dir() {
        return Err(format!(
            "scaffold path is not a directory: {}",
            root.display()
        ));
    }

    let issues_path = root.join("issues.jsonl");
    let contract_path = root.join("control-plane-contract.json");
    let scheme_program_path = root.join("scheme-program.json");
    let rhai_script_path = root.join("program.rhai");
    let trajectory_path = root.join("harness-trajectory.jsonl");

    let created_issues_file = write_if_absent(
        &issues_path,
        "{\"id\":\"bd-scaffold-1\",\"title\":\"Scaffold Issue\",\"status\":\"open\"}\n",
    )?;

    let contract_payload = json!({
        "schema": 1,
        "contractKind": "premath.control_plane_contract.v1",
        "controlPlaneSite": {
            "profileId": "cp.control.site.v0"
        },
        "controlPlaneKcirMappings": {
            "profileId": "cp.kcir.mapping.v0"
        },
        "ciInstructionPolicy": {
            "policyDigestPrefix": "pol1_"
        },
        "hostActionSurface": {
            "requiredActions": {
                "issue.ready": {
                    "operationId": "op/mcp.issue_ready"
                },
                "issue.claim_next": {
                    "operationId": "op/transport.issue_claim_next"
                }
            },
            "mcpOnlyHostActions": [],
            "failureClasses": {
                "bindingMismatch": "control_plane_host_action_binding_mismatch",
                "contractUnbound": "control_plane_host_action_contract_unbound"
            }
        }
    });
    let contract_body = serde_json::to_vec_pretty(&contract_payload)
        .map_err(|err| format!("failed to serialize scaffold contract payload: {err}"))?;
    let created_contract_file = write_bytes_if_absent(&contract_path, &contract_body)?;

    let program_payload = json!({
        "schema": 1,
        "programKind": "premath.scheme_eval.request.v0",
        "calls": [
            {
                "id": "call-ready",
                "action": "issue.ready",
                "args": {
                    "issuesPath": issues_path.display().to_string()
                }
            }
        ]
    });
    let program_body = serde_json::to_vec_pretty(&program_payload)
        .map_err(|err| format!("failed to serialize scaffold program payload: {err}"))?;
    let created_scheme_program_file = write_bytes_if_absent(&scheme_program_path, &program_body)?;

    let rhai_args_json = serde_json::to_string(&json!({
        "issuesPath": issues_path.display().to_string()
    }))
    .map_err(|err| format!("failed to serialize Rhai scaffold args: {err}"))?;
    let rhai_script = format!(
        "host_action(\"issue.ready\", {});\n",
        serde_json::to_string(&rhai_args_json)
            .map_err(|err| format!("failed to serialize Rhai scaffold script arg string: {err}"))?
    );
    let created_rhai_script_file = write_if_absent(&rhai_script_path, &rhai_script)?;

    let created_trajectory_file = write_if_absent(&trajectory_path, "")?;

    let next_scheme_command = format!(
        "premath scheme-eval --program {} --control-plane-contract {} --trajectory-path {} --json",
        shell_quote(scheme_program_path.display().to_string()),
        shell_quote(contract_path.display().to_string()),
        shell_quote(trajectory_path.display().to_string())
    );
    let next_rhai_command = format!(
        "premath rhai-eval --script {} --control-plane-contract {} --trajectory-path {} --json",
        shell_quote(rhai_script_path.display().to_string()),
        shell_quote(contract_path.display().to_string()),
        shell_quote(trajectory_path.display().to_string())
    );

    Ok(EvaluatorScaffoldOutcome {
        root,
        issues_path,
        contract_path,
        scheme_program_path,
        rhai_script_path,
        trajectory_path,
        created_root,
        created_issues_file,
        created_contract_file,
        created_scheme_program_file,
        created_rhai_script_file,
        created_trajectory_file,
        next_scheme_command,
        next_rhai_command,
    })
}

pub fn run(path: String, json_mode: bool) {
    let outcome = scaffold_layout(&path).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(1);
    });

    if json_mode {
        let payload = json!({
            "action": "evaluator.scaffold",
            "root": outcome.root.display().to_string(),
            "issuesPath": outcome.issues_path.display().to_string(),
            "controlPlaneContract": outcome.contract_path.display().to_string(),
            "schemeProgramPath": outcome.scheme_program_path.display().to_string(),
            "rhaiScriptPath": outcome.rhai_script_path.display().to_string(),
            "trajectoryPath": outcome.trajectory_path.display().to_string(),
            "created": {
                "root": outcome.created_root,
                "issuesFile": outcome.created_issues_file,
                "contractFile": outcome.created_contract_file,
                "schemeProgramFile": outcome.created_scheme_program_file,
                "rhaiScriptFile": outcome.created_rhai_script_file,
                "trajectoryFile": outcome.created_trajectory_file
            },
            "nextCommands": {
                "schemeEval": outcome.next_scheme_command,
                "rhaiEval": outcome.next_rhai_command
            }
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render evaluator-scaffold payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    println!("premath evaluator-scaffold {}", outcome.root.display());
    println!();
    println!("  issues path: {}", outcome.issues_path.display());
    println!(
        "  control-plane contract: {}",
        outcome.contract_path.display()
    );
    println!(
        "  scheme program: {}",
        outcome.scheme_program_path.display()
    );
    println!("  rhai script: {}", outcome.rhai_script_path.display());
    println!("  trajectory path: {}", outcome.trajectory_path.display());
    println!();
    println!("  created root: {}", yes_no(outcome.created_root));
    println!(
        "  created issues file: {}",
        yes_no(outcome.created_issues_file)
    );
    println!(
        "  created contract file: {}",
        yes_no(outcome.created_contract_file)
    );
    println!(
        "  created scheme program file: {}",
        yes_no(outcome.created_scheme_program_file)
    );
    println!(
        "  created rhai script file: {}",
        yes_no(outcome.created_rhai_script_file)
    );
    println!(
        "  created trajectory file: {}",
        yes_no(outcome.created_trajectory_file)
    );
    println!();
    println!("  next (scheme): {}", outcome.next_scheme_command);
    println!("  next (rhai): {}", outcome.next_rhai_command);
}

fn write_if_absent(path: &Path, content: &str) -> Result<bool, String> {
    write_bytes_if_absent(path, content.as_bytes())
}

fn write_bytes_if_absent(path: &Path, content: &[u8]) -> Result<bool, String> {
    if path.exists() {
        if !path.is_file() {
            return Err(format!("path exists but is not a file: {}", path.display()));
        }
        return Ok(false);
    }
    fs::write(path, content).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn shell_quote(raw: String) -> String {
    if raw
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"-._/:".contains(&byte))
    {
        return raw;
    }
    let escaped = raw.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "premath-cli-evaluator-scaffold-{prefix}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp dir should exist");
        path
    }

    #[test]
    fn scaffold_layout_creates_expected_files() {
        let root = temp_dir("create");
        let outcome = scaffold_layout(&root).expect("scaffold should succeed");
        assert!(outcome.issues_path.exists());
        assert!(outcome.contract_path.exists());
        assert!(outcome.scheme_program_path.exists());
        assert!(outcome.rhai_script_path.exists());
        assert!(outcome.trajectory_path.exists());
        assert!(outcome.created_issues_file);
        assert!(outcome.created_contract_file);
        assert!(outcome.created_scheme_program_file);
        assert!(outcome.created_rhai_script_file);
        assert!(outcome.created_trajectory_file);
    }

    #[test]
    fn scaffold_layout_is_idempotent() {
        let root = temp_dir("idempotent");
        let first = scaffold_layout(&root).expect("first scaffold should succeed");
        assert!(first.created_contract_file);

        let second = scaffold_layout(&root).expect("second scaffold should succeed");
        assert!(!second.created_contract_file);
        assert!(!second.created_scheme_program_file);
        assert!(!second.created_rhai_script_file);
        assert!(!second.created_trajectory_file);
    }
}
