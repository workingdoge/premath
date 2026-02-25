use crate::commands::scheme_eval;
use rhai::{Engine, EvalAltResult, Position};
use serde_json::{Map, Value, json};
use std::fs;
use std::sync::{Arc, Mutex};

const RHAI_PROGRAM_KIND: &str = "premath.rhai_eval.request.v0";

#[derive(Debug, Clone)]
pub struct Args {
    pub script: String,
    pub control_plane_contract: String,
    pub trajectory_path: String,
    pub step_prefix: String,
    pub max_calls: usize,
    pub issue_id: Option<String>,
    pub policy_digest: Option<String>,
    pub instruction_ref: Option<String>,
    pub capability_claims: Vec<String>,
    pub json: bool,
}

pub fn run(args: Args) {
    let source = fs::read_to_string(&args.script).unwrap_or_else(|err| {
        eprintln!(
            "error: failed to read Rhai script at {}: {err}",
            args.script
        );
        std::process::exit(2);
    });
    let calls = collect_calls(&source).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(1);
    });
    let program = build_program(calls);
    scheme_eval::run_with_program_value(
        scheme_eval::Args {
            program: args.script,
            control_plane_contract: args.control_plane_contract,
            trajectory_path: args.trajectory_path,
            step_prefix: args.step_prefix,
            max_calls: args.max_calls,
            issue_id: args.issue_id,
            policy_digest: args.policy_digest,
            instruction_ref: args.instruction_ref,
            capability_claims: args.capability_claims,
            json: args.json,
        },
        &scheme_eval::FRONTEND_RHAI,
        program,
    );
}

fn build_program(calls: Vec<Value>) -> Value {
    let mut program = Map::new();
    program.insert("schema".to_string(), json!(1));
    program.insert("programKind".to_string(), json!(RHAI_PROGRAM_KIND));
    program.insert("calls".to_string(), Value::Array(calls));
    Value::Object(program)
}

fn collect_calls(script: &str) -> Result<Vec<Value>, String> {
    let calls: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let mut engine = Engine::new();

    {
        let calls = Arc::clone(&calls);
        engine.register_fn(
            "host_action",
            move |action: &str| -> Result<(), Box<EvalAltResult>> {
                let mut lock = calls
                    .lock()
                    .map_err(|_| runtime_error("host_action lock poisoned"))?;
                let index = lock.len() + 1;
                lock.push(json!({
                    "id": format!("rhai-call-{index}"),
                    "action": action,
                    "args": {}
                }));
                Ok(())
            },
        );
    }

    {
        let calls = Arc::clone(&calls);
        engine.register_fn(
            "host_action",
            move |action: &str, args_json: &str| -> Result<(), Box<EvalAltResult>> {
                let parsed: Value = serde_json::from_str(args_json).map_err(|err| {
                    runtime_error(format!(
                        "host_action args_json must be valid JSON (action `{action}`): {err}"
                    ))
                })?;
                let mut lock = calls
                    .lock()
                    .map_err(|_| runtime_error("host_action lock poisoned"))?;
                let index = lock.len() + 1;
                lock.push(json!({
                    "id": format!("rhai-call-{index}"),
                    "action": action,
                    "args": parsed
                }));
                Ok(())
            },
        );
    }

    engine
        .eval::<()>(script)
        .map_err(|err| format!("rhai script evaluation failed: {err}"))?;
    let result = calls
        .lock()
        .map_err(|_| "host_action lock poisoned".to_string())?
        .clone();
    Ok(result)
}

fn runtime_error(message: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(message.into().into(), Position::NONE).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_host_actions_from_rhai_script() {
        let calls = collect_calls(
            r#"
            host_action("issue.ready", "{\"issuesPath\":\".premath/issues.jsonl\"}");
            host_action("dep.diagnostics");
            "#,
        )
        .expect("script should evaluate");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0]["action"], "issue.ready");
        assert_eq!(calls[1]["action"], "dep.diagnostics");
    }

    #[test]
    fn rejects_invalid_json_payload() {
        let err = collect_calls(r#"host_action("issue.ready", "{bad-json}");"#)
            .expect_err("invalid args json should fail");
        assert!(err.contains("host_action args_json"));
    }
}
