//! Projection-only spec IR lane for relational indexing.
//!
//! This module compiles authoritative spec artifacts into typed entity/edge rows
//! without introducing semantic admissibility logic in `premath-bd`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const SPEC_IR_PROJECTION_SCHEMA: &str = "premath.spec_ir.projection.v1";
pub const SPEC_IR_AUTHORITY_MODE: &str = "projection_only";
pub const SPEC_IR_ENTITY_KIND_STATEMENT: &str = "spec.statement";
pub const SPEC_IR_EDGE_KIND_STATEMENT_BINDING: &str = "spec.statement.binding";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecIrSources {
    pub statement_index_path: String,
    pub binding_contract_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecIrEntity {
    pub entity_kind: String,
    pub statement_id: String,
    pub kcir_ref: String,
    pub digest: String,
    pub doc_path: String,
    pub anchor: String,
    pub stmt_type: String,
    pub statement_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecIrEdge {
    pub edge_kind: String,
    pub edge_id: String,
    pub source_statement_id: String,
    pub relation_kind: String,
    pub target_kind: String,
    pub target_ref: String,
    pub required: bool,
    pub statement_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecIrProjection {
    pub schema: String,
    pub authority_mode: String,
    pub source_artifacts: SpecIrSources,
    pub entities: Vec<SpecIrEntity>,
    pub edges: Vec<SpecIrEdge>,
}

#[derive(Debug, thiserror::Error)]
pub enum SpecIrProjectionError {
    #[error("I/O error at {path}: {message}")]
    Io { path: String, message: String },

    #[error("invalid JSON at {path}: {message}")]
    InvalidJson { path: String, message: String },

    #[error("{path}: root must be an object")]
    RootNotObject { path: String },

    #[error("{path}: missing field `{field}`")]
    MissingField { path: String, field: String },

    #[error("{path}: invalid field `{field}` ({message})")]
    InvalidField {
        path: String,
        field: String,
        message: String,
    },

    #[error("duplicate statement id in statement-index: {0}")]
    DuplicateStatementId(String),

    #[error("binding references unknown statement id: {0}")]
    UnknownStatementId(String),

    #[error("binding digest mismatch for {statement_id}: expected {expected}, observed {observed}")]
    BindingDigestMismatch {
        statement_id: String,
        expected: String,
        observed: String,
    },
}

pub fn load_spec_ir_projection_from_paths(
    statement_index_path: impl AsRef<Path>,
    binding_contract_path: impl AsRef<Path>,
) -> Result<SpecIrProjection, SpecIrProjectionError> {
    let statement_index_path = statement_index_path.as_ref();
    let binding_contract_path = binding_contract_path.as_ref();

    let statement_text =
        fs::read_to_string(statement_index_path).map_err(|error| SpecIrProjectionError::Io {
            path: statement_index_path.display().to_string(),
            message: error.to_string(),
        })?;
    let binding_text =
        fs::read_to_string(binding_contract_path).map_err(|error| SpecIrProjectionError::Io {
            path: binding_contract_path.display().to_string(),
            message: error.to_string(),
        })?;

    let statement_index = serde_json::from_str::<Value>(&statement_text).map_err(|error| {
        SpecIrProjectionError::InvalidJson {
            path: statement_index_path.display().to_string(),
            message: error.to_string(),
        }
    })?;
    let binding_contract = serde_json::from_str::<Value>(&binding_text).map_err(|error| {
        SpecIrProjectionError::InvalidJson {
            path: binding_contract_path.display().to_string(),
            message: error.to_string(),
        }
    })?;

    load_spec_ir_projection_from_values_with_sources(
        &statement_index,
        &binding_contract,
        SpecIrSources {
            statement_index_path: statement_index_path.display().to_string(),
            binding_contract_path: binding_contract_path.display().to_string(),
        },
    )
}

pub fn load_spec_ir_projection_from_values(
    statement_index: &Value,
    binding_contract: &Value,
) -> Result<SpecIrProjection, SpecIrProjectionError> {
    load_spec_ir_projection_from_values_with_sources(
        statement_index,
        binding_contract,
        SpecIrSources {
            statement_index_path: "<in-memory:statement-index>".to_string(),
            binding_contract_path: "<in-memory:bindings>".to_string(),
        },
    )
}

fn load_spec_ir_projection_from_values_with_sources(
    statement_index: &Value,
    binding_contract: &Value,
    source_artifacts: SpecIrSources,
) -> Result<SpecIrProjection, SpecIrProjectionError> {
    let statement_path = source_artifacts.statement_index_path.clone();
    let bindings_path = source_artifacts.binding_contract_path.clone();

    let statement_root = as_object(statement_index, &statement_path)?;
    let statement_rows = as_array(statement_root, "rows", &statement_path)?;
    let mut entities_by_statement: BTreeMap<String, SpecIrEntity> = BTreeMap::new();
    for (index, row) in statement_rows.iter().enumerate() {
        let field_prefix = format!("rows[{index}]");
        let row_object = as_inline_object(row, &field_prefix, &statement_path)?;
        let statement_id =
            required_string(row_object, "statementId", &field_prefix, &statement_path)?;
        let entity = SpecIrEntity {
            entity_kind: SPEC_IR_ENTITY_KIND_STATEMENT.to_string(),
            statement_id: statement_id.clone(),
            kcir_ref: required_string(row_object, "kcirRef", &field_prefix, &statement_path)?,
            digest: required_string(row_object, "digest", &field_prefix, &statement_path)?,
            doc_path: required_string(row_object, "docPath", &field_prefix, &statement_path)?,
            anchor: required_string(row_object, "anchor", &field_prefix, &statement_path)?,
            stmt_type: required_string(row_object, "stmtType", &field_prefix, &statement_path)?,
            statement_text: required_string(
                row_object,
                "statementText",
                &field_prefix,
                &statement_path,
            )?,
        };
        if entities_by_statement
            .insert(statement_id.clone(), entity)
            .is_some()
        {
            return Err(SpecIrProjectionError::DuplicateStatementId(statement_id));
        }
    }

    let bindings_root = as_object(binding_contract, &bindings_path)?;
    let binding_rows = as_array(bindings_root, "bindings", &bindings_path)?;
    let mut edges: Vec<SpecIrEdge> = Vec::new();
    for (index, row) in binding_rows.iter().enumerate() {
        let field_prefix = format!("bindings[{index}]");
        let row_object = as_inline_object(row, &field_prefix, &bindings_path)?;
        let statement_id =
            required_string(row_object, "statementId", &field_prefix, &bindings_path)?;
        let statement_digest =
            required_string(row_object, "statementDigest", &field_prefix, &bindings_path)?;
        let relation_kind =
            required_string(row_object, "relationKind", &field_prefix, &bindings_path)?;
        let target_kind = required_string(row_object, "targetKind", &field_prefix, &bindings_path)?;
        let target_ref = required_string(row_object, "targetRef", &field_prefix, &bindings_path)?;
        let required = optional_bool(row_object, "required", &field_prefix, &bindings_path, false)?;

        let entity = entities_by_statement
            .get(&statement_id)
            .ok_or_else(|| SpecIrProjectionError::UnknownStatementId(statement_id.clone()))?;
        if entity.digest != statement_digest {
            return Err(SpecIrProjectionError::BindingDigestMismatch {
                statement_id,
                expected: statement_digest,
                observed: entity.digest.clone(),
            });
        }

        let edge_id = binding_edge_id(
            entity.statement_id.as_str(),
            relation_kind.as_str(),
            target_kind.as_str(),
            target_ref.as_str(),
            required,
            entity.digest.as_str(),
        );
        edges.push(SpecIrEdge {
            edge_kind: SPEC_IR_EDGE_KIND_STATEMENT_BINDING.to_string(),
            edge_id,
            source_statement_id: entity.statement_id.clone(),
            relation_kind,
            target_kind,
            target_ref,
            required,
            statement_digest: entity.digest.clone(),
        });
    }
    edges.sort_by(|left, right| {
        (
            left.source_statement_id.as_str(),
            left.relation_kind.as_str(),
            left.target_kind.as_str(),
            left.target_ref.as_str(),
            left.required,
            left.statement_digest.as_str(),
        )
            .cmp(&(
                right.source_statement_id.as_str(),
                right.relation_kind.as_str(),
                right.target_kind.as_str(),
                right.target_ref.as_str(),
                right.required,
                right.statement_digest.as_str(),
            ))
    });

    Ok(SpecIrProjection {
        schema: SPEC_IR_PROJECTION_SCHEMA.to_string(),
        authority_mode: SPEC_IR_AUTHORITY_MODE.to_string(),
        source_artifacts,
        entities: entities_by_statement.into_values().collect(),
        edges,
    })
}

fn as_object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a Map<String, Value>, SpecIrProjectionError> {
    value
        .as_object()
        .ok_or_else(|| SpecIrProjectionError::RootNotObject {
            path: path.to_string(),
        })
}

fn as_array<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    path: &str,
) -> Result<&'a [Value], SpecIrProjectionError> {
    let value = object
        .get(field)
        .ok_or_else(|| SpecIrProjectionError::MissingField {
            path: path.to_string(),
            field: field.to_string(),
        })?;
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| SpecIrProjectionError::InvalidField {
            path: path.to_string(),
            field: field.to_string(),
            message: "expected array".to_string(),
        })
}

fn as_inline_object<'a>(
    value: &'a Value,
    field_prefix: &str,
    path: &str,
) -> Result<&'a Map<String, Value>, SpecIrProjectionError> {
    value
        .as_object()
        .ok_or_else(|| SpecIrProjectionError::InvalidField {
            path: path.to_string(),
            field: field_prefix.to_string(),
            message: "expected object".to_string(),
        })
}

fn required_string(
    object: &Map<String, Value>,
    field: &str,
    field_prefix: &str,
    path: &str,
) -> Result<String, SpecIrProjectionError> {
    let field_label = format!("{field_prefix}.{field}");
    let value = object
        .get(field)
        .ok_or_else(|| SpecIrProjectionError::MissingField {
            path: path.to_string(),
            field: field_label.clone(),
        })?;
    let Some(raw) = value.as_str() else {
        return Err(SpecIrProjectionError::InvalidField {
            path: path.to_string(),
            field: field_label,
            message: "expected string".to_string(),
        });
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SpecIrProjectionError::InvalidField {
            path: path.to_string(),
            field: format!("{field_prefix}.{field}"),
            message: "must be non-empty".to_string(),
        });
    }
    Ok(trimmed.to_string())
}

fn optional_bool(
    object: &Map<String, Value>,
    field: &str,
    field_prefix: &str,
    path: &str,
    default: bool,
) -> Result<bool, SpecIrProjectionError> {
    let Some(value) = object.get(field) else {
        return Ok(default);
    };
    value
        .as_bool()
        .ok_or_else(|| SpecIrProjectionError::InvalidField {
            path: path.to_string(),
            field: format!("{field_prefix}.{field}"),
            message: "expected boolean".to_string(),
        })
}

fn binding_edge_id(
    statement_id: &str,
    relation_kind: &str,
    target_kind: &str,
    target_ref: &str,
    required: bool,
    statement_digest: &str,
) -> String {
    let mut hasher = Sha256::new();
    for component in [
        statement_id,
        relation_kind,
        target_kind,
        target_ref,
        statement_digest,
    ] {
        hasher.update(component.as_bytes());
        hasher.update([0]);
    }
    hasher.update([if required { 1 } else { 0 }]);
    format!("sir1_{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projection_rows_are_deterministically_sorted() {
        let statement_index = json!({
            "rows": [
                {
                    "statementId": "KERNEL.REQ.LOCALITY.1",
                    "docPath": "specs/premath/draft/PREMATH-KERNEL.md",
                    "anchor": "6localityrestriction",
                    "stmtType": "Requirement",
                    "statementText": "locality text",
                    "digest": "digest-locality",
                    "kcirRef": "kcir1_locality"
                },
                {
                    "statementId": "KERNEL.AX.STABILITY_UNIT.1",
                    "docPath": "specs/premath/draft/PREMATH-KERNEL.md",
                    "anchor": "5stabilityreindexingcoherence",
                    "stmtType": "Axiom",
                    "statementText": "stability text",
                    "digest": "digest-stability",
                    "kcirRef": "kcir1_stability"
                }
            ]
        });
        let bindings = json!({
            "bindings": [
                {
                    "statementId": "KERNEL.REQ.LOCALITY.1",
                    "statementDigest": "digest-locality",
                    "relationKind": "operationalized_by",
                    "targetKind": "obligation",
                    "targetRef": "locality",
                    "required": true
                },
                {
                    "statementId": "KERNEL.AX.STABILITY_UNIT.1",
                    "statementDigest": "digest-stability",
                    "relationKind": "covered_by_vector",
                    "targetKind": "vector",
                    "targetRef": "tests/conformance/fixtures/foo/case.json",
                    "required": false
                }
            ]
        });

        let projection =
            load_spec_ir_projection_from_values(&statement_index, &bindings).expect("projection");
        assert_eq!(projection.schema, SPEC_IR_PROJECTION_SCHEMA);
        assert_eq!(projection.authority_mode, SPEC_IR_AUTHORITY_MODE);
        assert_eq!(projection.entities.len(), 2);
        assert_eq!(projection.edges.len(), 2);
        assert_eq!(
            projection
                .entities
                .iter()
                .map(|row| row.statement_id.as_str())
                .collect::<Vec<_>>(),
            vec!["KERNEL.AX.STABILITY_UNIT.1", "KERNEL.REQ.LOCALITY.1"]
        );
    }

    #[test]
    fn projection_rejects_unknown_statement_references() {
        let statement_index = json!({
            "rows": [
                {
                    "statementId": "KERNEL.REQ.LOCALITY.1",
                    "docPath": "specs/premath/draft/PREMATH-KERNEL.md",
                    "anchor": "6localityrestriction",
                    "stmtType": "Requirement",
                    "statementText": "locality text",
                    "digest": "digest-locality",
                    "kcirRef": "kcir1_locality"
                }
            ]
        });
        let bindings = json!({
            "bindings": [
                {
                    "statementId": "KERNEL.AX.STABILITY_UNIT.1",
                    "statementDigest": "digest-stability",
                    "relationKind": "operationalized_by",
                    "targetKind": "obligation",
                    "targetRef": "stability",
                    "required": true
                }
            ]
        });

        let error = load_spec_ir_projection_from_values(&statement_index, &bindings)
            .expect_err("unknown statement id should fail");
        assert!(matches!(
            error,
            SpecIrProjectionError::UnknownStatementId(ref id)
                if id == "KERNEL.AX.STABILITY_UNIT.1"
        ));
    }

    #[test]
    fn projection_rejects_digest_mismatch() {
        let statement_index = json!({
            "rows": [
                {
                    "statementId": "KERNEL.REQ.LOCALITY.1",
                    "docPath": "specs/premath/draft/PREMATH-KERNEL.md",
                    "anchor": "6localityrestriction",
                    "stmtType": "Requirement",
                    "statementText": "locality text",
                    "digest": "digest-locality",
                    "kcirRef": "kcir1_locality"
                }
            ]
        });
        let bindings = json!({
            "bindings": [
                {
                    "statementId": "KERNEL.REQ.LOCALITY.1",
                    "statementDigest": "digest-other",
                    "relationKind": "operationalized_by",
                    "targetKind": "obligation",
                    "targetRef": "locality",
                    "required": true
                }
            ]
        });

        let error = load_spec_ir_projection_from_values(&statement_index, &bindings)
            .expect_err("digest mismatch should fail");
        assert!(matches!(
            error,
            SpecIrProjectionError::BindingDigestMismatch {
                statement_id,
                expected,
                observed
            } if statement_id == "KERNEL.REQ.LOCALITY.1"
                && expected == "digest-other"
                && observed == "digest-locality"
        ));
    }
}
