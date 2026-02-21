use crate::descent::GlueSelectionFailure;
use premath_kernel::witness::{GateFailure, failure_class, law_ref};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TuskFailureKind {
    StabilityMismatch,
    MissingRequiredRestrictions,
    MissingRequiredOverlaps,
    NoValidGlueProposal,
    NonContractibleSelection,
    ModeComparisonUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateClassMapping {
    pub class: &'static str,
    pub law_ref: &'static str,
}

pub fn map_tusk_failure_kind(kind: TuskFailureKind) -> GateClassMapping {
    match kind {
        TuskFailureKind::StabilityMismatch => GateClassMapping {
            class: failure_class::STABILITY_FAILURE,
            law_ref: law_ref::STABILITY,
        },
        TuskFailureKind::MissingRequiredRestrictions | TuskFailureKind::MissingRequiredOverlaps => {
            GateClassMapping {
                class: failure_class::LOCALITY_FAILURE,
                law_ref: law_ref::LOCALITY,
            }
        }
        TuskFailureKind::NoValidGlueProposal | TuskFailureKind::ModeComparisonUnavailable => {
            GateClassMapping {
                class: failure_class::DESCENT_FAILURE,
                law_ref: law_ref::DESCENT,
            }
        }
        TuskFailureKind::NonContractibleSelection => GateClassMapping {
            class: failure_class::GLUE_NON_CONTRACTIBLE,
            law_ref: law_ref::UNIQUENESS,
        },
    }
}

pub fn map_glue_selection_failure(failure: GlueSelectionFailure) -> TuskFailureKind {
    match failure {
        GlueSelectionFailure::NoValidProposal => TuskFailureKind::NoValidGlueProposal,
        GlueSelectionFailure::NonContractibleSelection => TuskFailureKind::NonContractibleSelection,
        GlueSelectionFailure::ModeComparisonUnavailable => {
            TuskFailureKind::ModeComparisonUnavailable
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TuskDiagnosticFailure {
    pub kind: TuskFailureKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl TuskDiagnosticFailure {
    pub fn to_gate_failure(&self) -> GateFailure {
        let mapping = map_tusk_failure_kind(self.kind);
        let mut failure = GateFailure::new(
            mapping.class,
            mapping.law_ref,
            self.message.clone(),
            self.token_path.clone(),
            self.context.clone(),
        );
        failure.details = self.details.clone();
        failure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glue_selection_failure_mapping_matches_contract() {
        assert_eq!(
            map_glue_selection_failure(GlueSelectionFailure::NoValidProposal),
            TuskFailureKind::NoValidGlueProposal
        );
        assert_eq!(
            map_glue_selection_failure(GlueSelectionFailure::NonContractibleSelection),
            TuskFailureKind::NonContractibleSelection
        );
        assert_eq!(
            map_glue_selection_failure(GlueSelectionFailure::ModeComparisonUnavailable),
            TuskFailureKind::ModeComparisonUnavailable
        );
    }

    #[test]
    fn gate_class_mapping_is_spec_aligned() {
        let locality = map_tusk_failure_kind(TuskFailureKind::MissingRequiredRestrictions);
        assert_eq!(locality.class, failure_class::LOCALITY_FAILURE);
        assert_eq!(locality.law_ref, law_ref::LOCALITY);

        let descent = map_tusk_failure_kind(TuskFailureKind::ModeComparisonUnavailable);
        assert_eq!(descent.class, failure_class::DESCENT_FAILURE);
        assert_eq!(descent.law_ref, law_ref::DESCENT);

        let uniq = map_tusk_failure_kind(TuskFailureKind::NonContractibleSelection);
        assert_eq!(uniq.class, failure_class::GLUE_NON_CONTRACTIBLE);
        assert_eq!(uniq.law_ref, law_ref::UNIQUENESS);
    }
}
