//! Deterministic witness identifiers per WITNESS-ID spec.
//!
//! Two independent implementations given the same semantic failure
//! MUST produce identical witness IDs.
//!
//! Algorithm:
//! 1. Build canonical witness key (schema, class, lawRef, tokenPath, context)
//! 2. Serialize via RFC 8785 (JCS) — sorted keys, no whitespace, canonical numbers
//! 3. witnessId = "w1_" || base32hex_lower(SHA256(keyBytes))

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Compute a witness ID from the canonical witness key fields.
///
/// Per WITNESS-ID spec §2, only these fields contribute:
/// - schema (always 1)
/// - class
/// - lawRef
/// - tokenPath (or null)
/// - context (or null)
pub fn compute_witness_id(
    class: &str,
    law_ref: &str,
    token_path: Option<&str>,
    context: Option<&Value>,
) -> String {
    let key = canonical_witness_key(class, law_ref, token_path, context);
    let key_bytes = jcs_serialize(&key);
    let hash = Sha256::digest(&key_bytes);
    let encoded = base32hex_lower_no_pad(&hash);
    format!("w1_{encoded}")
}

/// Build the canonical witness key JSON object.
///
/// Per spec §2:
/// ```json
/// {
///   "schema": 1,
///   "class": "...",
///   "lawRef": "...",
///   "tokenPath": "..." | null,
///   "context": { ... } | null
/// }
/// ```
fn canonical_witness_key(
    class: &str,
    law_ref: &str,
    token_path: Option<&str>,
    context: Option<&Value>,
) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("schema".to_string(), Value::Number(1.into()));
    map.insert("class".to_string(), Value::String(class.to_string()));
    map.insert(
        "context".to_string(),
        context.cloned().unwrap_or(Value::Null),
    );
    map.insert("lawRef".to_string(), Value::String(law_ref.to_string()));
    map.insert(
        "tokenPath".to_string(),
        token_path
            .map(|s| Value::String(s.to_string()))
            .unwrap_or(Value::Null),
    );
    Value::Object(map)
}

/// RFC 8785 JSON Canonicalization Scheme.
///
/// Requirements:
/// - UTF-8
/// - Object keys sorted lexicographically
/// - No insignificant whitespace
/// - Canonical number formatting (no trailing zeros, no leading +, etc.)
///
/// Since serde_json::Map is a BTreeMap when the "preserve_order" feature
/// is NOT enabled, keys are already sorted. We serialize with no pretty-printing.
fn jcs_serialize(value: &Value) -> Vec<u8> {
    // serde_json without pretty-printing gives us:
    // - no whitespace
    // - canonical number formatting
    // - sorted keys (BTreeMap default)
    //
    // This is sufficient for RFC 8785 conformance for our use case
    // (integer schema, string values, null, and small context objects).
    jcs_serialize_value(value)
}

/// Recursive JCS serialization ensuring lexicographic key ordering.
fn jcs_serialize_value(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => b"null".to_vec(),
        Value::Bool(b) => {
            if *b {
                b"true".to_vec()
            } else {
                b"false".to_vec()
            }
        }
        Value::Number(n) => {
            // RFC 8785: canonical number formatting
            // For integers, just use the decimal representation
            if let Some(i) = n.as_i64() {
                format!("{i}").into_bytes()
            } else if let Some(u) = n.as_u64() {
                format!("{u}").into_bytes()
            } else if let Some(f) = n.as_f64() {
                // RFC 8785 specifies ES6-style number serialization
                format!("{f}").into_bytes()
            } else {
                n.to_string().into_bytes()
            }
        }
        Value::String(_) => {
            // RFC 8785: strings use standard JSON escaping
            serde_json::to_vec(value).unwrap()
        }
        Value::Array(arr) => {
            let mut buf = Vec::new();
            buf.push(b'[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                buf.extend_from_slice(&jcs_serialize_value(v));
            }
            buf.push(b']');
            buf
        }
        Value::Object(map) => {
            // RFC 8785: keys MUST be sorted lexicographically
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut buf = Vec::new();
            buf.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                // Serialize key as JSON string
                let key_json = serde_json::to_vec(&Value::String((*key).clone())).unwrap();
                buf.extend_from_slice(&key_json);
                buf.push(b':');
                buf.extend_from_slice(&jcs_serialize_value(&map[*key]));
            }
            buf.push(b'}');
            buf
        }
    }
}

/// RFC 4648 base32hex encoding, lowercase, without padding.
///
/// Base32hex alphabet (lowercase): 0-9 a-v
fn base32hex_lower_no_pad(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuv";

    let mut result = String::new();
    let mut bits: u64 = 0;
    let mut num_bits: u32 = 0;

    for &byte in data {
        bits = (bits << 8) | (byte as u64);
        num_bits += 8;

        while num_bits >= 5 {
            num_bits -= 5;
            let idx = ((bits >> num_bits) & 0x1f) as usize;
            result.push(ALPHABET[idx] as char);
        }
    }

    // Handle remaining bits (if any)
    if num_bits > 0 {
        let idx = ((bits << (5 - num_bits)) & 0x1f) as usize;
        result.push(ALPHABET[idx] as char);
    }

    result
}

/// A gate failure witness per GATE.md §4.1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateFailure {
    /// Deterministic witness ID per WITNESS-ID spec.
    pub witness_id: String,

    /// Failure classification.
    pub class: String,

    /// Gate law reference (e.g., "GATE-3.1").
    pub law_ref: String,

    /// Human-readable description.
    pub message: String,

    /// Serialized context key or structured map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,

    /// Affected token/definable path.
    #[serde(default)]
    pub token_path: Option<String>,

    /// Provenance records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<Value>>,

    /// Class-specific machine-readable details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl GateFailure {
    /// Create a new gate failure with computed witness ID.
    pub fn new(
        class: impl Into<String>,
        law_ref: impl Into<String>,
        message: impl Into<String>,
        token_path: Option<String>,
        context: Option<Value>,
    ) -> Self {
        let class = class.into();
        let law_ref = law_ref.into();
        let witness_id =
            compute_witness_id(&class, &law_ref, token_path.as_deref(), context.as_ref());
        Self {
            witness_id,
            class,
            law_ref,
            message: message.into(),
            context,
            token_path,
            sources: None,
            details: None,
        }
    }

    /// Ordering key per GATE.md §4.1:
    /// 1. class, 2. lawRef, 3. tokenPath, 4. context, 5. witnessId
    fn sort_key(&self) -> (&str, &str, &str, String, &str) {
        (
            &self.class,
            &self.law_ref,
            self.token_path.as_deref().unwrap_or(""),
            self.context
                .as_ref()
                .map(|c| serde_json::to_string(c).unwrap_or_default())
                .unwrap_or_default(),
            &self.witness_id,
        )
    }
}

impl PartialOrd for GateFailure {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GateFailure {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

/// The result of a Gate check per GATE.md §4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateResult {
    /// Schema version (always 1).
    pub witness_schema: u32,

    /// Conformance profile.
    pub profile: String,

    /// "accepted" or "rejected".
    pub result: String,

    /// Failure witnesses (empty if accepted).
    pub failures: Vec<GateFailure>,
}

impl GateResult {
    /// Create an accepted result.
    pub fn accepted(profile: impl Into<String>) -> Self {
        Self {
            witness_schema: 1,
            profile: profile.into(),
            result: "accepted".to_string(),
            failures: vec![],
        }
    }

    /// Create a rejected result with failures.
    ///
    /// Failures are automatically sorted per spec §4.1 ordering.
    pub fn rejected(profile: impl Into<String>, mut failures: Vec<GateFailure>) -> Self {
        failures.sort();
        Self {
            witness_schema: 1,
            profile: profile.into(),
            result: "rejected".to_string(),
            failures,
        }
    }

    /// Whether this gate check was accepted.
    pub fn is_accepted(&self) -> bool {
        self.result == "accepted"
    }
}

/// Failure class constants per GATE.md §4.
pub mod failure_class {
    pub const STABILITY_FAILURE: &str = "stability_failure";
    pub const LOCALITY_FAILURE: &str = "locality_failure";
    pub const DESCENT_FAILURE: &str = "descent_failure";
    pub const GLUE_NON_CONTRACTIBLE: &str = "glue_non_contractible";
    pub const ADJOINT_TRIPLE_COHERENCE_FAILURE: &str = "adjoint_triple_coherence_failure";
}

/// Law reference constants.
pub mod law_ref {
    pub const STABILITY: &str = "GATE-3.1";
    pub const LOCALITY: &str = "GATE-3.2";
    pub const DESCENT: &str = "GATE-3.3";
    pub const UNIQUENESS: &str = "GATE-3.4";
    pub const ADJOINT_TRIPLE: &str = "GATE-3.5";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn witness_id_determinism() {
        let id1 = compute_witness_id("stability_failure", "GATE-3.1", None, None);
        let id2 = compute_witness_id("stability_failure", "GATE-3.1", None, None);
        assert_eq!(id1, id2);
        assert!(id1.starts_with("w1_"));
    }

    #[test]
    fn witness_id_sensitivity() {
        let id1 = compute_witness_id("stability_failure", "GATE-3.1", None, None);
        let id2 = compute_witness_id("locality_failure", "GATE-3.2", None, None);
        assert_ne!(id1, id2);
    }

    #[test]
    fn witness_id_with_context() {
        let ctx = serde_json::json!({"mask": 1});
        let id1 = compute_witness_id("stability_failure", "GATE-3.1", None, Some(&ctx));
        let id2 = compute_witness_id("stability_failure", "GATE-3.1", None, Some(&ctx));
        assert_eq!(id1, id2);

        // Different context → different ID
        let ctx2 = serde_json::json!({"mask": 2});
        let id3 = compute_witness_id("stability_failure", "GATE-3.1", None, Some(&ctx2));
        assert_ne!(id1, id3);
    }

    #[test]
    fn gate_result_accepted() {
        let result = GateResult::accepted("toy");
        assert!(result.is_accepted());
        assert!(result.failures.is_empty());

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["witnessSchema"], 1);
        assert_eq!(json["result"], "accepted");
    }

    #[test]
    fn gate_result_rejected_sorted() {
        let f1 = GateFailure::new("stability_failure", "GATE-3.1", "test", None, None);
        let f2 = GateFailure::new("descent_failure", "GATE-3.3", "test", None, None);

        let result = GateResult::rejected("toy", vec![f1, f2]);

        // descent_failure should sort before stability_failure
        assert_eq!(result.failures[0].class, "descent_failure");
        assert_eq!(result.failures[1].class, "stability_failure");
    }

    #[test]
    fn base32hex_encoding() {
        // Test known value: SHA256 of empty string
        let hash = Sha256::digest(b"");
        let encoded = base32hex_lower_no_pad(&hash);
        // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        // Verify it's valid base32hex (only 0-9, a-v)
        assert!(
            encoded
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='v').contains(&c))
        );
    }

    #[test]
    fn jcs_key_ordering() {
        // Verify that our JCS serialization sorts keys lexicographically
        let key = canonical_witness_key("stability_failure", "GATE-3.1", None, None);
        let bytes = jcs_serialize(&key);
        let s = String::from_utf8(bytes).unwrap();
        // Keys should appear in order: class, context, lawRef, schema, tokenPath
        let class_pos = s.find("\"class\"").unwrap();
        let context_pos = s.find("\"context\"").unwrap();
        let law_ref_pos = s.find("\"lawRef\"").unwrap();
        let schema_pos = s.find("\"schema\"").unwrap();
        let token_path_pos = s.find("\"tokenPath\"").unwrap();
        assert!(class_pos < context_pos);
        assert!(context_pos < law_ref_pos);
        assert!(law_ref_pos < schema_pos);
        assert!(schema_pos < token_path_pos);
    }

    #[test]
    fn spec_stability_failure_witness_id() {
        // From the spec's adversarial_stability_failure_bad_stability/expect.json:
        // class: "stability_failure", lawRef: "GATE-3.1", tokenPath: null, context: {"mask": 1}
        // Expected witnessId: "w1_l1v24u75j3sudbdnhflh4l6sb29ii02vosfj2cm5m3qh0rg8ab30"
        let ctx = serde_json::json!({"mask": 1});
        let id = compute_witness_id("stability_failure", "GATE-3.1", None, Some(&ctx));
        assert_eq!(
            id, "w1_l1v24u75j3sudbdnhflh4l6sb29ii02vosfj2cm5m3qh0rg8ab30",
            "witness ID must match spec fixture"
        );
    }

    #[test]
    fn spec_locality_failure_witness_id() {
        // From adversarial_locality_failure_partial_restrict/expect.json:
        let ctx = serde_json::json!({"mask": 1});
        let id = compute_witness_id("locality_failure", "GATE-3.2", None, Some(&ctx));
        assert_eq!(
            id, "w1_v9grre901frguq5nv7g8i8dtlf3uujt05i2lkmh8i5bdfdod4q90",
            "witness ID must match spec fixture"
        );
    }

    #[test]
    fn spec_descent_failure_witness_id() {
        // v7: adversarial_descent_failure_bad_constant/expect.json
        // Now fails at glue existence (GATE-3.3) with context mask=3, not overlap mask=0
        let ctx = serde_json::json!({"mask": 3});
        let id = compute_witness_id("descent_failure", "GATE-3.3", None, Some(&ctx));
        assert_eq!(
            id, "w1_sgjilie1aln83eq8oh0gbmois0f6gtbsppfplk8j26vps7qmu2fg",
            "witness ID must match spec v7 fixture"
        );
    }

    #[test]
    fn spec_glue_non_contractible_witness_id() {
        // v7: adversarial_glue_non_contractible_non_separated/expect.json
        // Now fails at uniqueness (GATE-3.4) with context mask=3
        let ctx = serde_json::json!({"mask": 3});
        let id = compute_witness_id("glue_non_contractible", "GATE-3.4", None, Some(&ctx));
        assert_eq!(
            id, "w1_c0kubjophjuo3h5cr03elvc75606hoba3v4ksii3ktvpgr0enh9g",
            "witness ID must match spec v7 fixture"
        );
    }
}
