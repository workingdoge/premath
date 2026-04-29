use premath_coherence::{
    RequiredGateRefFallback, RequiredGateRefRequest, RequiredGateRefResult, RequiredWitnessError,
    build_required_gate_ref,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub struct RunOptions {
    pub input: Option<String>,
    pub fallback_check_id: Option<String>,
    pub fallback_exit_code: Option<i64>,
    pub fallback_projection_digest: Option<String>,
    pub fallback_policy_digest: Option<String>,
    pub fallback_ctx_ref: Option<String>,
    pub fallback_data_head_ref: Option<String>,
    pub gate_payload_out: Option<String>,
    pub json_output: bool,
}

fn emit_error(err: RequiredWitnessError) -> ! {
    eprintln!("{err}");
    std::process::exit(2);
}

fn invalid(message: impl Into<String>) -> RequiredWitnessError {
    RequiredWitnessError {
        failure_class: "required_gate_ref_invalid".to_string(),
        message: message.into(),
    }
}

fn require_option<T>(value: Option<T>, label: &str) -> Result<T, RequiredWitnessError> {
    value.ok_or_else(|| invalid(format!("{label} is required for fallback gate ref input")))
}

fn any_fallback_arg(options: &RunOptions) -> bool {
    options.fallback_check_id.is_some()
        || options.fallback_exit_code.is_some()
        || options.fallback_projection_digest.is_some()
        || options.fallback_policy_digest.is_some()
        || options.fallback_ctx_ref.is_some()
        || options.fallback_data_head_ref.is_some()
}

fn load_request(
    options: RunOptions,
) -> Result<(RequiredGateRefRequest, Option<String>, bool), RequiredWitnessError> {
    let fallback_args_present = any_fallback_arg(&options);
    if let Some(input) = options.input {
        if fallback_args_present {
            return Err(invalid(
                "use either --input or --fallback-* arguments, not both",
            ));
        }
        let input_path = PathBuf::from(input);
        let bytes = fs::read(&input_path).map_err(|err| {
            invalid(format!(
                "failed to read required gate ref input {}: {err}",
                input_path.display()
            ))
        })?;

        let request: RequiredGateRefRequest = serde_json::from_slice(&bytes).map_err(|err| {
            invalid(format!(
                "failed to parse required gate ref input json {}: {err}",
                input_path.display()
            ))
        })?;
        return Ok((request, options.gate_payload_out, options.json_output));
    }

    let check_id = require_option(options.fallback_check_id, "--fallback-check-id")?;
    let exit_code = require_option(options.fallback_exit_code, "--fallback-exit-code")?;
    let projection_digest = require_option(
        options.fallback_projection_digest,
        "--fallback-projection-digest",
    )?;
    let policy_digest = require_option(options.fallback_policy_digest, "--fallback-policy-digest")?;
    let ctx_ref = require_option(options.fallback_ctx_ref, "--fallback-ctx-ref")?;
    let data_head_ref = require_option(options.fallback_data_head_ref, "--fallback-data-head-ref")?;
    let artifact_rel_path = format!("gates/{projection_digest}/00-{check_id}.json");
    let request = RequiredGateRefRequest {
        check_id,
        artifact_rel_path,
        source: Some("fallback".to_string()),
        gate_payload: None,
        fallback: Some(RequiredGateRefFallback {
            exit_code,
            projection_digest,
            policy_digest,
            ctx_ref,
            data_head_ref,
        }),
    };
    Ok((request, options.gate_payload_out, options.json_output))
}

fn write_gate_payload(
    gate_payload_out: String,
    result: &RequiredGateRefResult,
) -> Result<(), RequiredWitnessError> {
    let Some(gate_payload) = result.gate_payload.as_ref() else {
        return Err(invalid(
            "--gate-payload-out requires a fallback input that produces gatePayload",
        ));
    };
    let mut payload = gate_payload.clone();
    if let Value::Object(map) = &mut payload {
        map.insert(
            "witnessSource".to_string(),
            Value::String(result.gate_witness_ref.source.clone()),
        );
    }
    let out_path = PathBuf::from(gate_payload_out);
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|err| {
            invalid(format!(
                "failed to create gate payload output directory {}: {err}",
                parent.display()
            ))
        })?;
    }
    let rendered = serde_json::to_string_pretty(&payload)
        .map_err(|err| invalid(format!("failed to render gate payload json: {err}")))?;
    fs::write(&out_path, format!("{rendered}\n")).map_err(|err| {
        invalid(format!(
            "failed to write gate payload output {}: {err}",
            out_path.display()
        ))
    })?;
    Ok(())
}

pub fn run(options: RunOptions) {
    let (request, gate_payload_out, json_output) = match load_request(options) {
        Ok(value) => value,
        Err(err) => emit_error(err),
    };
    let result = match build_required_gate_ref(&request) {
        Ok(value) => value,
        Err(err) => emit_error(err),
    };
    if let Some(path) = gate_payload_out
        && let Err(err) = write_gate_payload(path, &result)
    {
        emit_error(err);
    }
    if json_output {
        let rendered = serde_json::to_string_pretty(&result).unwrap_or_else(|err| {
            emit_error(RequiredWitnessError {
                failure_class: "required_gate_ref_invalid".to_string(),
                message: format!("failed to render required gate ref json: {err}"),
            });
        });
        println!("{rendered}");
        return;
    }
    render_text(&result);
}

fn render_text(result: &RequiredGateRefResult) {
    println!("premath required-gate-ref");
    println!("  Check: {}", result.gate_witness_ref.check_id);
    println!("  Source: {}", result.gate_witness_ref.source);
    println!("  Sha256: {}", result.gate_witness_ref.sha256);
}
