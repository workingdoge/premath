use crate::descent::{ContractibilityBasis, DescentPack, GlueMethod, GlueResult};
use crate::mapping::{TuskDiagnosticFailure, TuskFailureKind};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Deterministic v0 evaluation output for a `DescentPack`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvalOutcome {
    pub diagnostics: Vec<TuskDiagnosticFailure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glue_result: Option<GlueResult>,
}

/// Evaluate a `DescentPack` with a deterministic v0 policy.
///
/// This is intentionally minimal and conservative:
/// - enforces non-empty locals
/// - enforces overlap evidence presence for multi-local packs
/// - enforces single-proposal contractibility
/// - returns a world-owned `GlueResult` only when checks pass
pub fn evaluate_descent_pack(pack: &DescentPack) -> EvalOutcome {
    let mut diagnostics = Vec::new();

    if pack.core.mode.normalizer_id.trim().is_empty()
        || pack.core.mode.policy_digest.trim().is_empty()
    {
        diagnostics.push(TuskDiagnosticFailure {
            kind: TuskFailureKind::ModeComparisonUnavailable,
            message: "mode binding missing normalizer_id or policy_digest".to_string(),
            token_path: Some("descent.core.mode".to_string()),
            context: None,
            details: Some(json!({
                "phase": "normalize",
                "responsibleComponent": "world",
            })),
        });
    }

    if pack.core.locals.is_empty() {
        diagnostics.push(TuskDiagnosticFailure {
            kind: TuskFailureKind::MissingRequiredRestrictions,
            message: "descent core has no local states".to_string(),
            token_path: Some("descent.core.locals".to_string()),
            context: None,
            details: Some(json!({
                "phase": "restrict",
                "responsibleComponent": "adapter",
            })),
        });
    }

    if pack.core.locals.len() > 1 && pack.core.compat.is_empty() {
        diagnostics.push(TuskDiagnosticFailure {
            kind: TuskFailureKind::MissingRequiredOverlaps,
            message: "multi-local descent core missing compatibility witnesses".to_string(),
            token_path: Some("descent.core.compat".to_string()),
            context: Some(json!({
                "localCount": pack.core.locals.len(),
            })),
            details: Some(json!({
                "phase": "compat",
                "responsibleComponent": "world",
            })),
        });
    }

    let glue_result = if diagnostics.is_empty() {
        match pack.glue_proposals.as_slice() {
            [] => {
                diagnostics.push(TuskDiagnosticFailure {
                    kind: TuskFailureKind::NoValidGlueProposal,
                    message: "no glue proposals provided".to_string(),
                    token_path: Some("descent.glueProposals".to_string()),
                    context: None,
                    details: Some(json!({
                        "phase": "select_glue",
                        "responsibleComponent": "world",
                    })),
                });
                None
            }
            [only] => Some(GlueResult {
                selected: only.proposal_id.clone(),
                contractibility_basis: ContractibilityBasis {
                    mode: pack.core.mode.clone(),
                    method: GlueMethod::EquivWitness,
                    evidence_refs: Vec::new(),
                },
                normal_form_ref: None,
            }),
            _ => {
                diagnostics.push(TuskDiagnosticFailure {
                    kind: TuskFailureKind::NonContractibleSelection,
                    message: "multiple glue proposals remain under v0 mode".to_string(),
                    token_path: Some("descent.glueProposals".to_string()),
                    context: Some(json!({
                        "proposalCount": pack.glue_proposals.len(),
                    })),
                    details: Some(json!({
                        "phase": "select_glue",
                        "responsibleComponent": "world",
                    })),
                });
                None
            }
        }
    } else {
        None
    };

    EvalOutcome {
        diagnostics,
        glue_result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descent::{DescentCore, GlueProposal, ModeBinding};
    use std::collections::BTreeMap;

    fn base_pack() -> DescentPack {
        let mut locals = BTreeMap::new();
        locals.insert("part:a".to_string(), json!({"value": 1}));

        DescentPack {
            core: DescentCore {
                cover_id: "cover:demo".to_string(),
                locals,
                compat: vec![],
                mode: ModeBinding {
                    normalizer_id: "normalizer.v1".to_string(),
                    policy_digest: "policy.v1".to_string(),
                },
            },
            glue_proposals: vec![GlueProposal {
                proposal_id: "proposal:1".to_string(),
                payload: json!({"selected": true}),
            }],
        }
    }

    #[test]
    fn evaluates_single_proposal_as_glue_result() {
        let pack = base_pack();
        let outcome = evaluate_descent_pack(&pack);

        assert!(outcome.diagnostics.is_empty());
        assert_eq!(
            outcome.glue_result.as_ref().expect("glue result").selected,
            "proposal:1"
        );
    }

    #[test]
    fn rejects_multi_local_without_overlap_witnesses() {
        let mut pack = base_pack();
        pack.core
            .locals
            .insert("part:b".to_string(), json!({"value": 2}));

        let outcome = evaluate_descent_pack(&pack);
        assert!(outcome.glue_result.is_none());
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|d| d.kind == TuskFailureKind::MissingRequiredOverlaps)
        );
    }

    #[test]
    fn rejects_multiple_proposals_as_non_contractible() {
        let mut pack = base_pack();
        pack.glue_proposals.push(GlueProposal {
            proposal_id: "proposal:2".to_string(),
            payload: json!({"selected": false}),
        });

        let outcome = evaluate_descent_pack(&pack);
        assert!(outcome.glue_result.is_none());
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|d| d.kind == TuskFailureKind::NonContractibleSelection)
        );
    }
}
