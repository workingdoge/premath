use crate::cli::MockFailureArg;
use premath_tusk::{
    GateWitnessEnvelope, IntentSpec, RunIdOptions, RunIdentity, TuskDiagnosticFailure,
    TuskFailureKind, compute_intent_id,
};
use serde_json::json;

pub struct Args {
    pub world_id: String,
    pub unit_id: String,
    pub parent_unit_id: Option<String>,
    pub context_id: String,
    pub cover_id: String,
    pub ctx_ref: String,
    pub data_head_ref: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub normalizer_id: String,
    pub policy_digest: String,
    pub cover_strategy_digest: Option<String>,
    pub intent_kind: String,
    pub target_scope: String,
    pub outcomes: Vec<String>,
    pub failures: Vec<MockFailureArg>,
    pub include_cover_strategy_in_run_id: bool,
    pub json: bool,
}

pub fn run(args: Args) {
    let intent_spec = IntentSpec {
        intent_kind: args.intent_kind,
        target_scope: args.target_scope,
        requested_outcomes: if args.outcomes.is_empty() {
            vec!["summary".to_string(), "obligations".to_string()]
        } else {
            args.outcomes
        },
        constraints: None,
    };
    let intent_id = compute_intent_id(&intent_spec);

    let identity = RunIdentity {
        world_id: args.world_id,
        unit_id: args.unit_id,
        parent_unit_id: args.parent_unit_id,
        context_id: args.context_id,
        intent_id,
        cover_id: args.cover_id,
        ctx_ref: args.ctx_ref,
        data_head_ref: args.data_head_ref,
        adapter_id: args.adapter_id,
        adapter_version: args.adapter_version,
        normalizer_id: args.normalizer_id,
        policy_digest: args.policy_digest,
        cover_strategy_digest: args.cover_strategy_digest,
    };

    let diagnostics = args
        .failures
        .iter()
        .enumerate()
        .map(|(idx, f)| {
            let (kind, message, phase, component) = mock_failure_metadata(f);
            TuskDiagnosticFailure {
                kind,
                message,
                token_path: Some(format!("mock/{}", idx + 1)),
                context: Some(json!({
                    "index": idx,
                    "kind": format!("{:?}", f),
                })),
                details: Some(json!({
                    "phase": phase,
                    "responsibleComponent": component,
                })),
            }
        })
        .collect::<Vec<_>>();

    let run_id_options = RunIdOptions {
        include_cover_strategy_digest: args.include_cover_strategy_in_run_id,
    };
    let envelope = GateWitnessEnvelope::from_diagnostics(&identity, run_id_options, diagnostics);

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).expect("json serialization")
        );
    } else {
        println!("premath mock-gate");
        println!("  Run ID: {}", envelope.run_id);
        println!("  Result: {}", envelope.result);
        println!("  Failures: {}", envelope.failures.len());
        for failure in &envelope.failures {
            println!(
                "    - [{}] {} ({})",
                failure.class, failure.message, failure.law_ref
            );
        }
    }
}

fn mock_failure_metadata(
    arg: &MockFailureArg,
) -> (TuskFailureKind, String, &'static str, &'static str) {
    match arg {
        MockFailureArg::StabilityMismatch => (
            TuskFailureKind::StabilityMismatch,
            "stability mismatch detected during mock run".to_string(),
            "compat",
            "world",
        ),
        MockFailureArg::MissingRequiredRestrictions => (
            TuskFailureKind::MissingRequiredRestrictions,
            "required local restriction missing".to_string(),
            "restrict",
            "adapter",
        ),
        MockFailureArg::MissingRequiredOverlaps => (
            TuskFailureKind::MissingRequiredOverlaps,
            "required overlap obligation missing".to_string(),
            "compat",
            "world",
        ),
        MockFailureArg::NoValidGlueProposal => (
            TuskFailureKind::NoValidGlueProposal,
            "no valid glue proposal produced".to_string(),
            "select_glue",
            "world",
        ),
        MockFailureArg::NonContractibleSelection => (
            TuskFailureKind::NonContractibleSelection,
            "multiple inequivalent glue selections remain".to_string(),
            "select_glue",
            "world",
        ),
        MockFailureArg::ModeComparisonUnavailable => (
            TuskFailureKind::ModeComparisonUnavailable,
            "comparison mode unavailable for contractibility check".to_string(),
            "normalize",
            "world",
        ),
    }
}
