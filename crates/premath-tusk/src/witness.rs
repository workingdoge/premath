use crate::identity::{RunIdOptions, RunIdentity};
use crate::mapping::TuskDiagnosticFailure;
use premath_kernel::witness::GateFailure;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateWitnessEnvelope {
    pub witness_schema: u32,
    pub witness_kind: String,
    pub run_id: String,
    pub world_id: String,
    pub context_id: String,
    pub intent_id: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub ctx_ref: String,
    pub data_head_ref: String,
    pub normalizer_id: String,
    pub policy_digest: String,
    pub result: String,
    pub failures: Vec<GateFailure>,
}

impl GateWitnessEnvelope {
    pub fn accepted(identity: &RunIdentity, run_id_options: RunIdOptions) -> Self {
        Self {
            witness_schema: 1,
            witness_kind: "gate".to_string(),
            run_id: identity.compute_run_id(run_id_options),
            world_id: identity.world_id.clone(),
            context_id: identity.context_id.clone(),
            intent_id: identity.intent_id.clone(),
            adapter_id: identity.adapter_id.clone(),
            adapter_version: identity.adapter_version.clone(),
            ctx_ref: identity.ctx_ref.clone(),
            data_head_ref: identity.data_head_ref.clone(),
            normalizer_id: identity.normalizer_id.clone(),
            policy_digest: identity.policy_digest.clone(),
            result: "accepted".to_string(),
            failures: vec![],
        }
    }

    pub fn rejected(
        identity: &RunIdentity,
        run_id_options: RunIdOptions,
        mut failures: Vec<GateFailure>,
    ) -> Self {
        failures.sort();
        Self {
            witness_schema: 1,
            witness_kind: "gate".to_string(),
            run_id: identity.compute_run_id(run_id_options),
            world_id: identity.world_id.clone(),
            context_id: identity.context_id.clone(),
            intent_id: identity.intent_id.clone(),
            adapter_id: identity.adapter_id.clone(),
            adapter_version: identity.adapter_version.clone(),
            ctx_ref: identity.ctx_ref.clone(),
            data_head_ref: identity.data_head_ref.clone(),
            normalizer_id: identity.normalizer_id.clone(),
            policy_digest: identity.policy_digest.clone(),
            result: "rejected".to_string(),
            failures,
        }
    }

    pub fn from_diagnostics(
        identity: &RunIdentity,
        run_id_options: RunIdOptions,
        diagnostics: Vec<TuskDiagnosticFailure>,
    ) -> Self {
        if diagnostics.is_empty() {
            return Self::accepted(identity, run_id_options);
        }

        let failures = diagnostics
            .into_iter()
            .map(|d| d.to_gate_failure())
            .collect::<Vec<_>>();

        Self::rejected(identity, run_id_options, failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::TuskFailureKind;

    fn fixture_identity() -> RunIdentity {
        RunIdentity {
            world_id: "world.dev".into(),
            unit_id: "unit.1".into(),
            parent_unit_id: None,
            context_id: "ctx.main".into(),
            intent_id: "intent.abc".into(),
            cover_id: "cover.001".into(),
            ctx_ref: "jj:abcd".into(),
            data_head_ref: "ev:100".into(),
            adapter_id: "beads".into(),
            adapter_version: "0.1.0".into(),
            normalizer_id: "norm.v1".into(),
            policy_digest: "policy.deadbeef".into(),
            cover_strategy_digest: None,
        }
    }

    #[test]
    fn accepted_envelope_for_empty_diagnostics() {
        let id = fixture_identity();
        let env = GateWitnessEnvelope::from_diagnostics(&id, RunIdOptions::default(), vec![]);

        assert_eq!(env.result, "accepted");
        assert!(env.failures.is_empty());
        assert_eq!(env.witness_kind, "gate");
        assert_eq!(env.witness_schema, 1);
    }

    #[test]
    fn rejected_envelope_is_order_invariant() {
        let id = fixture_identity();

        let a = vec![
            TuskDiagnosticFailure {
                kind: TuskFailureKind::NoValidGlueProposal,
                message: "no glue".into(),
                token_path: None,
                context: Some(serde_json::json!({"part": 2})),
                details: None,
            },
            TuskDiagnosticFailure {
                kind: TuskFailureKind::MissingRequiredOverlaps,
                message: "missing overlap".into(),
                token_path: None,
                context: Some(serde_json::json!({"part": 1})),
                details: None,
            },
        ];

        let b = vec![a[1].clone(), a[0].clone()];

        let env_a = GateWitnessEnvelope::from_diagnostics(&id, RunIdOptions::default(), a);
        let env_b = GateWitnessEnvelope::from_diagnostics(&id, RunIdOptions::default(), b);

        assert_eq!(env_a.result, "rejected");
        assert_eq!(env_a.run_id, env_b.run_id);

        // Deterministic failure order and witness IDs after sorting.
        assert_eq!(env_a.failures, env_b.failures);
    }
}
