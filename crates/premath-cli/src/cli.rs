use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "premath",
    about = "Premath: generic kernel checks over pluggable memory/query/version adapters",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Check contractibility over one selected cover scope
    Check {
        /// Scope root issue ID (or `all` for full dataset)
        id: String,

        /// Coherence level: set, gpd, or s_inf
        #[arg(long, default_value = "set")]
        level: String,

        /// Path to issues JSONL
        #[arg(long, default_value = ".beads/issues.jsonl")]
        issues: String,

        /// Repository path used for optional JJ snapshot metadata
        #[arg(long, default_value = ".")]
        repo: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify locality/gluing/uniqueness/refinement over selected scope
    Verify {
        /// Scope root issue ID (or `all` for full dataset)
        id: String,

        /// Coherence level
        #[arg(long, default_value = "set")]
        level: String,

        /// Path to issues JSONL
        #[arg(long, default_value = ".beads/issues.jsonl")]
        issues: String,

        /// Repository path used for optional JJ snapshot metadata
        #[arg(long, default_value = ".")]
        repo: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Emit a mock Tusk GateWitnessEnvelope from synthetic diagnostics
    MockGate {
        /// World identifier
        #[arg(long, default_value = "world.dev")]
        world_id: String,

        /// Unit identifier
        #[arg(long, default_value = "unit.mock")]
        unit_id: String,

        /// Optional parent unit identifier
        #[arg(long)]
        parent_unit_id: Option<String>,

        /// Context object identifier
        #[arg(long, default_value = "ctx.main")]
        context_id: String,

        /// Cover identifier
        #[arg(long, default_value = "cover.default")]
        cover_id: String,

        /// Context lineage reference
        #[arg(long, default_value = "ctx:head")]
        ctx_ref: String,

        /// EventStore data head reference
        #[arg(long, default_value = "data:head")]
        data_head_ref: String,

        /// Adapter identifier
        #[arg(long, default_value = "adapter.mock")]
        adapter_id: String,

        /// Adapter version
        #[arg(long, default_value = "0.1.0")]
        adapter_version: String,

        /// Normalizer identifier
        #[arg(long, default_value = "normalizer.mock.v1")]
        normalizer_id: String,

        /// Policy digest value
        #[arg(long, default_value = "policy.mock.v1")]
        policy_digest: String,

        /// Optional cover-strategy digest (audit by default)
        #[arg(long)]
        cover_strategy_digest: Option<String>,

        /// Intent kind used to derive deterministic intent_id
        #[arg(long, default_value = "mock_gate")]
        intent_kind: String,

        /// Intent target scope used to derive deterministic intent_id
        #[arg(long, default_value = "cli")]
        target_scope: String,

        /// Requested outcome(s) for intent derivation (repeatable)
        #[arg(long = "outcome")]
        outcomes: Vec<String>,

        /// Synthetic failure kinds (repeatable). Omit for an accepted envelope.
        #[arg(long = "failure", value_enum)]
        failures: Vec<MockFailureArg>,

        /// Include cover_strategy_digest in run_id material (hardened mode)
        #[arg(long)]
        include_cover_strategy_in_run_id: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Evaluate a DescentPack JSON and emit a GateWitnessEnvelope
    TuskEval {
        /// Path to RunIdentity JSON
        #[arg(long)]
        identity: String,

        /// Path to DescentPack JSON
        #[arg(long)]
        descent_pack: String,

        /// Include cover_strategy_digest in run_id material
        #[arg(long)]
        include_cover_strategy_in_run_id: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Initialize a premath-tracked repository layout
    Init {
        /// Directory to initialize
        #[arg(default_value = ".")]
        path: String,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum MockFailureArg {
    #[value(name = "stability_mismatch")]
    StabilityMismatch,
    #[value(name = "missing_required_restrictions")]
    MissingRequiredRestrictions,
    #[value(name = "missing_required_overlaps")]
    MissingRequiredOverlaps,
    #[value(name = "no_valid_glue_proposal")]
    NoValidGlueProposal,
    #[value(name = "non_contractible_selection")]
    NonContractibleSelection,
    #[value(name = "mode_comparison_unavailable")]
    ModeComparisonUnavailable,
}
