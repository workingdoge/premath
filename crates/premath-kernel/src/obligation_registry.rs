//! Canonical obligation -> Gate failure registry.
//!
//! This module is the single semantic authority for mapping BIDIR obligations
//! to Gate failure classes/law references.

use crate::witness::{failure_class, law_ref};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObligationGateMapping {
    pub obligation_kind: &'static str,
    pub failure_class: &'static str,
    pub law_ref: &'static str,
}

/// Canonical obligation->Gate failure-class mapping.
///
/// This symbol name is intentionally stable because coherence checks parse this
/// declaration as an authority surface.
pub const OBLIGATION_TO_GATE_FAILURE: &[(&str, &str)] = &[
    ("stability", failure_class::STABILITY_FAILURE),
    ("locality", failure_class::LOCALITY_FAILURE),
    ("descent_exists", failure_class::DESCENT_FAILURE),
    ("descent_contractible", failure_class::GLUE_NON_CONTRACTIBLE),
    (
        "adjoint_triangle",
        failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
    ),
    (
        "beck_chevalley_sigma",
        failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
    ),
    (
        "beck_chevalley_pi",
        failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
    ),
    ("refinement_invariance", failure_class::STABILITY_FAILURE),
    (
        "adjoint_triple",
        failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
    ),
    ("ext_gap", failure_class::DESCENT_FAILURE),
    ("ext_ambiguous", failure_class::GLUE_NON_CONTRACTIBLE),
];

pub fn obligation_to_failure_class(kind: &str) -> Option<&'static str> {
    OBLIGATION_TO_GATE_FAILURE
        .iter()
        .find_map(|(obligation_kind, failure)| (*obligation_kind == kind).then_some(*failure))
}

pub fn failure_class_to_law_ref(class: &str) -> Option<&'static str> {
    match class {
        failure_class::STABILITY_FAILURE => Some(law_ref::STABILITY),
        failure_class::LOCALITY_FAILURE => Some(law_ref::LOCALITY),
        failure_class::DESCENT_FAILURE => Some(law_ref::DESCENT),
        failure_class::GLUE_NON_CONTRACTIBLE => Some(law_ref::UNIQUENESS),
        failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE => Some(law_ref::ADJOINT_TRIPLE),
        _ => None,
    }
}

pub fn obligation_gate_registry() -> Vec<ObligationGateMapping> {
    OBLIGATION_TO_GATE_FAILURE
        .iter()
        .map(|(obligation_kind, failure)| ObligationGateMapping {
            obligation_kind,
            failure_class: failure,
            law_ref: failure_class_to_law_ref(failure).unwrap_or(law_ref::DESCENT),
        })
        .collect()
}

pub fn obligation_gate_registry_json() -> Value {
    json!({
        "schema": 1,
        "registryKind": "premath.obligation_gate_registry.v1",
        "mappings": obligation_gate_registry(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn required_bidir_mapping_is_present_and_gate_aligned() {
        let expected = [
            (
                "stability",
                failure_class::STABILITY_FAILURE,
                law_ref::STABILITY,
            ),
            (
                "locality",
                failure_class::LOCALITY_FAILURE,
                law_ref::LOCALITY,
            ),
            (
                "descent_exists",
                failure_class::DESCENT_FAILURE,
                law_ref::DESCENT,
            ),
            (
                "descent_contractible",
                failure_class::GLUE_NON_CONTRACTIBLE,
                law_ref::UNIQUENESS,
            ),
            (
                "adjoint_triple",
                failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
                law_ref::ADJOINT_TRIPLE,
            ),
            ("ext_gap", failure_class::DESCENT_FAILURE, law_ref::DESCENT),
            (
                "ext_ambiguous",
                failure_class::GLUE_NON_CONTRACTIBLE,
                law_ref::UNIQUENESS,
            ),
        ];
        let registry = obligation_gate_registry();
        for (kind, class, law) in expected {
            let row = registry
                .iter()
                .find(|item| item.obligation_kind == kind)
                .unwrap_or_else(|| panic!("missing required obligation mapping: {kind}"));
            assert_eq!(row.failure_class, class);
            assert_eq!(row.law_ref, law);
        }
    }

    #[test]
    fn all_mapped_failure_classes_have_known_gate_law_refs() {
        let allowed_classes: BTreeSet<&str> = [
            failure_class::STABILITY_FAILURE,
            failure_class::LOCALITY_FAILURE,
            failure_class::DESCENT_FAILURE,
            failure_class::GLUE_NON_CONTRACTIBLE,
            failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
        ]
        .into_iter()
        .collect();
        let allowed_law_refs: BTreeSet<&str> = [
            law_ref::STABILITY,
            law_ref::LOCALITY,
            law_ref::DESCENT,
            law_ref::UNIQUENESS,
            law_ref::ADJOINT_TRIPLE,
        ]
        .into_iter()
        .collect();

        for row in obligation_gate_registry() {
            assert!(
                allowed_classes.contains(row.failure_class),
                "unknown Gate failure class: {}",
                row.failure_class
            );
            assert!(
                allowed_law_refs.contains(row.law_ref),
                "unknown Gate law ref: {}",
                row.law_ref
            );
            assert_eq!(
                failure_class_to_law_ref(row.failure_class),
                Some(row.law_ref),
                "failure class/law ref mismatch for {}",
                row.obligation_kind
            );
        }
    }

    #[test]
    fn registry_json_surface_is_deterministic() {
        let first = obligation_gate_registry_json();
        let second = obligation_gate_registry_json();
        assert_eq!(first, second);
        assert_eq!(
            first.get("registryKind").and_then(Value::as_str),
            Some("premath.obligation_gate_registry.v1")
        );
    }
}
