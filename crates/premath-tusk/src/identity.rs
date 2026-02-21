use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Canonical intent material used for deterministic `intent_id` derivation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntentSpec {
    pub intent_kind: String,
    pub target_scope: String,
    pub requested_outcomes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Value>,
}

impl IntentSpec {
    /// Return a canonicalized copy suitable for stable hashing.
    pub fn canonicalized(&self) -> Self {
        let mut out = self.clone();
        out.requested_outcomes.sort();
        out.requested_outcomes.dedup();
        out
    }
}

/// Deterministic run identity material.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunIdentity {
    pub world_id: String,
    pub unit_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_unit_id: Option<String>,
    pub context_id: String,
    pub intent_id: String,
    pub cover_id: String,
    pub ctx_ref: String,
    pub data_head_ref: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub normalizer_id: String,
    pub policy_digest: String,
    /// Audit material by default. Optional identity material under hardening.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_strategy_digest: Option<String>,
}

/// Run ID hardening controls.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunIdOptions {
    pub include_cover_strategy_digest: bool,
}

impl RunIdentity {
    /// Deterministic run identifier derived from canonical identity material.
    pub fn compute_run_id(&self, options: RunIdOptions) -> String {
        let mut value = serde_json::to_value(self).expect("RunIdentity must serialize");
        if !options.include_cover_strategy_digest
            && let Value::Object(map) = &mut value
        {
            map.remove("coverStrategyDigest");
        }

        let bytes = canonical_json_bytes(&value);
        let hash = Sha256::digest(bytes);
        format!("run1_{}", hex_lower(&hash))
    }
}

/// Deterministic `intent_id` from canonical `IntentSpec`.
pub fn compute_intent_id(spec: &IntentSpec) -> String {
    let canonical = spec.canonicalized();
    let value = serde_json::to_value(canonical).expect("IntentSpec must serialize");
    let bytes = canonical_json_bytes(&value);
    let hash = Sha256::digest(bytes);
    format!("intent1_{}", hex_lower(&hash))
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => b"null".to_vec(),
        Value::Bool(true) => b"true".to_vec(),
        Value::Bool(false) => b"false".to_vec(),
        Value::Number(n) => n.to_string().into_bytes(),
        Value::String(_) => {
            serde_json::to_vec(value).expect("string serialization should not fail")
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
                    map.get(*key).expect("sorted key must exist in object"),
                ));
            }
            out.push(b'}');
            out
        }
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_identity() -> RunIdentity {
        RunIdentity {
            world_id: "world.dev".into(),
            unit_id: "unit.1".into(),
            parent_unit_id: Some("unit.0".into()),
            context_id: "ctx.main".into(),
            intent_id: "intent.abc".into(),
            cover_id: "cover.001".into(),
            ctx_ref: "jj:abcd".into(),
            data_head_ref: "ev:100".into(),
            adapter_id: "beads".into(),
            adapter_version: "0.1.0".into(),
            normalizer_id: "norm.v1".into(),
            policy_digest: "policy.deadbeef".into(),
            cover_strategy_digest: Some("strategy.v1".into()),
        }
    }

    #[test]
    fn intent_id_is_stable_and_order_invariant() {
        let a = IntentSpec {
            intent_kind: "plan".into(),
            target_scope: "repo".into(),
            requested_outcomes: vec!["obligations".into(), "summary".into()],
            constraints: Some(serde_json::json!({"maxDepth": 3})),
        };

        let b = IntentSpec {
            requested_outcomes: vec!["summary".into(), "obligations".into()],
            ..a.clone()
        };

        assert_eq!(compute_intent_id(&a), compute_intent_id(&b));
    }

    #[test]
    fn run_id_is_stable_for_same_identity() {
        let id = fixture_identity();
        let run_a = id.compute_run_id(RunIdOptions::default());
        let run_b = id.compute_run_id(RunIdOptions::default());
        assert_eq!(run_a, run_b);
        assert!(run_a.starts_with("run1_"));
    }

    #[test]
    fn cover_strategy_digest_is_not_identity_by_default() {
        let mut a = fixture_identity();
        let mut b = fixture_identity();
        a.cover_strategy_digest = Some("strategy.a".into());
        b.cover_strategy_digest = Some("strategy.b".into());

        let run_a = a.compute_run_id(RunIdOptions::default());
        let run_b = b.compute_run_id(RunIdOptions::default());

        assert_eq!(run_a, run_b);
    }

    #[test]
    fn cover_strategy_digest_can_be_hardened_into_identity() {
        let mut a = fixture_identity();
        let mut b = fixture_identity();
        a.cover_strategy_digest = Some("strategy.a".into());
        b.cover_strategy_digest = Some("strategy.b".into());

        let opts = RunIdOptions {
            include_cover_strategy_digest: true,
        };
        let run_a = a.compute_run_id(opts);
        let run_b = b.compute_run_id(opts);

        assert_ne!(run_a, run_b);
    }
}
