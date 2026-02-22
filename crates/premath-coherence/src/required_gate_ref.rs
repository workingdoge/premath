use crate::required::{RequiredGateWitnessRef, RequiredWitnessError};
use premath_kernel::witness::compute_witness_id;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredGateRefFallback {
    pub exit_code: i64,
    pub projection_digest: String,
    pub policy_digest: String,
    pub ctx_ref: String,
    pub data_head_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredGateRefRequest {
    pub check_id: String,
    pub artifact_rel_path: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub gate_payload: Option<Value>,
    #[serde(default)]
    pub fallback: Option<RequiredGateRefFallback>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredGateRefResult {
    pub gate_witness_ref: RequiredGateWitnessRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_payload: Option<Value>,
}

fn ensure_non_empty(value: &str, label: &str) -> Result<String, RequiredWitnessError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RequiredWitnessError {
            failure_class: "required_gate_ref_invalid".to_string(),
            message: format!("{label} must be a non-empty string"),
        });
    }
    Ok(trimmed.to_string())
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => b"null".to_vec(),
        Value::Bool(true) => b"true".to_vec(),
        Value::Bool(false) => b"false".to_vec(),
        Value::Number(n) => n.to_string().into_bytes(),
        Value::String(_) => {
            serde_json::to_vec(value).expect("json string serialization should work")
        }
        Value::Array(items) => {
            let mut out = Vec::new();
            out.push(b'[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(b',');
                }
                out.extend(canonical_json_bytes(item));
            }
            out.push(b']');
            out
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = Vec::new();
            out.push(b'{');
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(b',');
                }
                let key_json =
                    serde_json::to_vec(&Value::String((*key).clone())).expect("key serialize");
                out.extend(key_json);
                out.push(b':');
                out.extend(canonical_json_bytes(
                    map.get(*key).expect("sorted key should exist"),
                ));
            }
            out.push(b'}');
            out
        }
    }
}

fn stable_sha256(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json_bytes(value));
    format!("{:x}", hasher.finalize())
}

fn sorted_unique_non_empty(values: Vec<String>) -> Vec<String> {
    let mut out = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            out.insert(trimmed.to_string());
        }
    }
    out.into_iter().collect()
}

fn extract_failure_classes(payload: &Map<String, Value>) -> Vec<String> {
    let mut classes: Vec<String> = Vec::new();
    if let Some(Value::Array(failures)) = payload.get("failures") {
        for failure in failures {
            if let Some(failure_obj) = failure.as_object()
                && let Some(class_name) = failure_obj.get("class").and_then(Value::as_str)
            {
                classes.push(class_name.to_string());
            }
        }
    }
    sorted_unique_non_empty(classes)
}

fn build_gate_ref(
    check_id: &str,
    artifact_rel_path: &str,
    source: &str,
    payload: &Map<String, Value>,
) -> RequiredGateWitnessRef {
    RequiredGateWitnessRef {
        check_id: check_id.to_string(),
        artifact_rel_path: artifact_rel_path.to_string(),
        sha256: stable_sha256(&Value::Object(payload.clone())),
        source: source.to_string(),
        run_id: payload
            .get("runId")
            .and_then(Value::as_str)
            .map(str::to_string),
        witness_kind: payload
            .get("witnessKind")
            .and_then(Value::as_str)
            .map(str::to_string),
        result: payload
            .get("result")
            .and_then(Value::as_str)
            .map(str::to_string),
        failure_classes: extract_failure_classes(payload),
    }
}

fn make_fallback_payload(check_id: &str, fallback: &RequiredGateRefFallback) -> Value {
    let intent_spec = json!({
        "intentKind": "ci_required_check",
        "targetScope": format!("check:{check_id}"),
        "requestedOutcomes": ["gate_witness_envelope"],
    });
    let intent_id = format!("intent1_{}", stable_sha256(&intent_spec));

    let identity = json!({
        "worldId": "world.ci.required",
        "unitId": format!("unit.ci.check.{check_id}"),
        "parentUnitId": format!("unit.ci.projection.{}", fallback.projection_digest),
        "contextId": format!("ctx.ci.required.{}", fallback.projection_digest),
        "intentId": intent_id,
        "coverId": format!("cover.ci.required.{}", fallback.projection_digest),
        "ctxRef": fallback.ctx_ref,
        "dataHeadRef": fallback.data_head_ref,
        "adapterId": "adapter.ci.runner",
        "adapterVersion": "1",
        "normalizerId": "normalizer.ci.required.v1",
        "policyDigest": fallback.policy_digest,
    });
    let run_id = format!("run1_{}", stable_sha256(&identity));

    let mut failures: Vec<Value> = Vec::new();
    let mut result = "accepted";
    if fallback.exit_code != 0 {
        result = "rejected";
        let token_path = format!("ci/check/{check_id}");
        let context = json!({
            "checkId": check_id,
            "exitCode": fallback.exit_code,
            "projectionDigest": fallback.projection_digest,
        });
        let witness_id = compute_witness_id(
            "descent_failure",
            "GATE-3.3",
            Some(token_path.as_str()),
            Some(&context),
        );
        failures.push(json!({
            "witnessId": witness_id,
            "class": "descent_failure",
            "lawRef": "GATE-3.3",
            "message": format!("ci required check '{check_id}' failed (exitCode={})", fallback.exit_code),
            "context": context,
            "tokenPath": token_path,
            "details": {
                "phase": "run_gate",
                "responsibleComponent": "gate_execution_plane"
            }
        }));
    }

    json!({
        "witnessSchema": 1,
        "witnessKind": "gate",
        "runId": run_id,
        "worldId": identity.get("worldId"),
        "contextId": identity.get("contextId"),
        "intentId": identity.get("intentId"),
        "adapterId": identity.get("adapterId"),
        "adapterVersion": identity.get("adapterVersion"),
        "ctxRef": identity.get("ctxRef"),
        "dataHeadRef": identity.get("dataHeadRef"),
        "normalizerId": identity.get("normalizerId"),
        "policyDigest": identity.get("policyDigest"),
        "result": result,
        "failures": failures
    })
}

pub fn build_required_gate_ref(
    request: &RequiredGateRefRequest,
) -> Result<RequiredGateRefResult, RequiredWitnessError> {
    let check_id = ensure_non_empty(&request.check_id, "checkId")?;
    let artifact_rel_path = ensure_non_empty(&request.artifact_rel_path, "artifactRelPath")?;

    let source = request
        .source
        .as_deref()
        .unwrap_or(if request.fallback.is_some() {
            "fallback"
        } else {
            "native"
        });
    if source != "native" && source != "fallback" {
        return Err(RequiredWitnessError {
            failure_class: "required_gate_ref_invalid".to_string(),
            message: format!("source must be 'native' or 'fallback' (actual={source:?})"),
        });
    }

    let has_payload = request.gate_payload.is_some();
    let has_fallback = request.fallback.is_some();
    if has_payload == has_fallback {
        return Err(RequiredWitnessError {
            failure_class: "required_gate_ref_invalid".to_string(),
            message: "exactly one of gatePayload or fallback must be provided".to_string(),
        });
    }

    if let Some(payload) = request.gate_payload.as_ref() {
        let Some(payload_obj) = payload.as_object() else {
            return Err(RequiredWitnessError {
                failure_class: "required_gate_ref_invalid".to_string(),
                message: "gatePayload must be an object".to_string(),
            });
        };
        let gate_witness_ref = build_gate_ref(&check_id, &artifact_rel_path, source, payload_obj);
        return Ok(RequiredGateRefResult {
            gate_witness_ref,
            gate_payload: None,
        });
    }

    let fallback = request
        .fallback
        .as_ref()
        .expect("fallback should be present when payload is absent");
    let _projection_digest =
        ensure_non_empty(&fallback.projection_digest, "fallback.projectionDigest")?;
    let _policy_digest = ensure_non_empty(&fallback.policy_digest, "fallback.policyDigest")?;
    let _ctx_ref = ensure_non_empty(&fallback.ctx_ref, "fallback.ctxRef")?;
    let _data_head_ref = ensure_non_empty(&fallback.data_head_ref, "fallback.dataHeadRef")?;

    if source != "fallback" {
        return Err(RequiredWitnessError {
            failure_class: "required_gate_ref_invalid".to_string(),
            message: format!("fallback mode requires source='fallback' (actual={source:?})"),
        });
    }

    let gate_payload = make_fallback_payload(&check_id, fallback);
    let payload_obj = gate_payload
        .as_object()
        .expect("fallback gate payload should be an object");
    let gate_witness_ref = build_gate_ref(&check_id, &artifact_rel_path, source, payload_obj);
    Ok(RequiredGateRefResult {
        gate_witness_ref,
        gate_payload: Some(gate_payload),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_required_gate_ref_from_native_payload() {
        let request = RequiredGateRefRequest {
            check_id: "baseline".to_string(),
            artifact_rel_path: "gates/proj1_demo/01-baseline.json".to_string(),
            source: Some("native".to_string()),
            gate_payload: Some(json!({
                "witnessKind": "gate",
                "runId": "run1_demo",
                "result": "accepted",
                "failures": []
            })),
            fallback: None,
        };

        let result = build_required_gate_ref(&request).expect("native payload should succeed");
        assert_eq!(result.gate_witness_ref.check_id, "baseline");
        assert_eq!(result.gate_witness_ref.source, "native");
        assert_eq!(result.gate_witness_ref.result.as_deref(), Some("accepted"));
        assert!(result.gate_witness_ref.failure_classes.is_empty());
        assert!(result.gate_payload.is_none());
    }

    #[test]
    fn build_required_gate_ref_from_fallback_payload() {
        let request = RequiredGateRefRequest {
            check_id: "baseline".to_string(),
            artifact_rel_path: "gates/proj1_demo/01-baseline.json".to_string(),
            source: Some("fallback".to_string()),
            gate_payload: None,
            fallback: Some(RequiredGateRefFallback {
                exit_code: 1,
                projection_digest: "proj1_demo".to_string(),
                policy_digest: "ci-topos-v0".to_string(),
                ctx_ref: "origin/main".to_string(),
                data_head_ref: "HEAD".to_string(),
            }),
        };

        let result = build_required_gate_ref(&request).expect("fallback payload should succeed");
        assert_eq!(result.gate_witness_ref.check_id, "baseline");
        assert_eq!(result.gate_witness_ref.source, "fallback");
        assert_eq!(result.gate_witness_ref.result.as_deref(), Some("rejected"));
        assert_eq!(
            result.gate_witness_ref.failure_classes,
            vec!["descent_failure".to_string()]
        );
        assert!(result.gate_payload.is_some());
    }
}
