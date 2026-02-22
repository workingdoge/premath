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
        #[arg(long, default_value = ".premath/issues.jsonl")]
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
        #[arg(long, default_value = ".premath/issues.jsonl")]
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

    /// Query Observation Surface v0 for frontend/user judgement views
    Observe {
        /// Observation surface JSON path
        #[arg(long, default_value = "artifacts/observation/latest.json")]
        surface: String,

        /// Query mode
        #[arg(long, default_value = "latest")]
        mode: ObserveModeArg,

        /// Instruction ID (required for mode=instruction)
        #[arg(long)]
        instruction_id: Option<String>,

        /// Projection digest (required for mode=projection)
        #[arg(long)]
        projection_digest: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Serve Observation Surface v0 as a tiny HTTP read API
    ObserveServe {
        /// Observation surface JSON path
        #[arg(long, default_value = "artifacts/observation/latest.json")]
        surface: String,

        /// Bind address (host:port)
        #[arg(long, default_value = "127.0.0.1:43174")]
        bind: String,
    },

    /// Serve Premath MCP tools over stdio
    McpServe {
        /// Issues JSONL path used by issue/dep tools
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Issue query backend: jsonl or surreal
        #[arg(long, default_value = "jsonl")]
        issue_query_backend: String,

        /// Optional issue-query projection path used by surreal backend mode
        #[arg(long, default_value = ".premath/surreal_issue_cache.json")]
        issue_query_projection: String,

        /// Mutation policy: open or instruction-linked (default: instruction-linked)
        #[arg(long, default_value = "instruction-linked")]
        mutation_policy: String,

        /// Observation surface JSON path used by observe tools
        #[arg(long, default_value = "artifacts/observation/latest.json")]
        surface: String,

        /// Repository root for doctrine instruction pipeline tools
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// MCP server name
        #[arg(long, default_value = "premath-mcp")]
        server_name: String,

        /// MCP server version
        #[arg(long, default_value = "0.1.0")]
        server_version: String,
    },

    /// Evaluate Premath coherence contract obligations against repository surfaces
    CoherenceCheck {
        /// Coherence contract JSON path
        #[arg(long, default_value = "specs/premath/draft/COHERENCE-CONTRACT.json")]
        contract: String,

        /// Repository root used to resolve contract-relative surfaces
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate and discharge one proposal payload through core checker semantics
    ProposalCheck {
        /// Proposal JSON path
        #[arg(long)]
        proposal: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate one instruction envelope through core checker semantics
    InstructionCheck {
        /// Instruction JSON path
        #[arg(long)]
        instruction: String,

        /// Repository root used for policy artifact resolution
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage issues in premath-bd JSONL memory
    Issue {
        #[command(subcommand)]
        command: IssueCommands,
    },

    /// Manage dependencies between issues
    Dep {
        #[command(subcommand)]
        command: DepCommands,
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

#[derive(Clone, Debug, ValueEnum)]
pub enum ObserveModeArg {
    #[value(name = "latest")]
    Latest,
    #[value(name = "needs_attention")]
    NeedsAttention,
    #[value(name = "instruction")]
    Instruction,
    #[value(name = "projection")]
    Projection,
}

#[derive(Subcommand, Clone, Debug)]
pub enum IssueCommands {
    /// Add a new issue
    Add {
        /// Issue title
        title: String,

        /// Optional explicit issue ID
        #[arg(long)]
        id: Option<String>,

        /// Issue description
        #[arg(long, default_value = "")]
        description: String,

        /// Issue status
        #[arg(long, default_value = "open")]
        status: String,

        /// Priority (0..4)
        #[arg(long, default_value_t = 2)]
        priority: i32,

        /// Issue type
        #[arg(long = "type", default_value = "task")]
        issue_type: String,

        /// Optional assignee
        #[arg(long, default_value = "")]
        assignee: String,

        /// Optional owner
        #[arg(long, default_value = "")]
        owner: String,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List issues with optional filters
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show ready open work (unblocked issues)
    Ready {
        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show non-closed issues that are currently blocked
    Blocked {
        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Update an existing issue
    Update {
        /// Issue ID
        id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// New notes
        #[arg(long)]
        notes: Option<String>,

        /// New status
        #[arg(long)]
        status: Option<String>,

        /// New priority
        #[arg(long)]
        priority: Option<i32>,

        /// New assignee
        #[arg(long)]
        assignee: Option<String>,

        /// New owner
        #[arg(long)]
        owner: Option<String>,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Atomically claim an issue for work (sets assignee + in_progress)
    Claim {
        /// Issue ID
        id: String,

        /// Assignee to claim with
        #[arg(long)]
        assignee: String,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Create discovered follow-up work linked to a parent issue
    Discover {
        /// Parent issue ID where work was discovered
        parent_issue_id: String,

        /// New issue title
        title: String,

        /// Optional explicit issue ID
        #[arg(long)]
        id: Option<String>,

        /// Issue description
        #[arg(long, default_value = "")]
        description: String,

        /// Priority (0..4)
        #[arg(long, default_value_t = 2)]
        priority: i32,

        /// Issue type
        #[arg(long = "type", default_value = "task")]
        issue_type: String,

        /// Optional assignee
        #[arg(long, default_value = "")]
        assignee: String,

        /// Optional owner
        #[arg(long, default_value = "")]
        owner: String,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Migrate issue snapshots into an issue.event.v1 JSONL log
    MigrateEvents {
        /// Path to issues JSONL snapshot
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Path to issue-event JSONL output
        #[arg(long, default_value = ".premath/memory/events.jsonl")]
        events: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Replay an issue.event.v1 log into an issues JSONL snapshot
    ReplayEvents {
        /// Path to issue-event JSONL input
        #[arg(long, default_value = ".premath/memory/events.jsonl")]
        events: String,

        /// Path to projected issues JSONL output
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Path to replay cache metadata
        #[arg(long)]
        cache: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum DepCommands {
    /// Add a dependency edge
    Add {
        /// Source issue ID
        issue_id: String,

        /// Target dependency issue ID
        depends_on_id: String,

        /// Dependency type
        #[arg(long = "type", default_value = "blocks")]
        dep_type: DepTypeArg,

        /// Optional created_by annotation
        #[arg(long, default_value = "")]
        created_by: String,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Project dependencies into one semantic view
    Project {
        /// View: execution, gtd, or groupoid
        #[arg(long, default_value = "execution")]
        view: DepViewArg,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum DepTypeArg {
    #[value(name = "blocks")]
    Blocks,
    #[value(name = "parent-child")]
    ParentChild,
    #[value(name = "conditional-blocks")]
    ConditionalBlocks,
    #[value(name = "related")]
    Related,
    #[value(name = "discovered-from")]
    DiscoveredFrom,
    #[value(name = "relates-to")]
    RelatesTo,
    #[value(name = "duplicates")]
    Duplicates,
    #[value(name = "supersedes")]
    Supersedes,
    #[value(name = "waits-for")]
    WaitsFor,
    #[value(name = "replies-to")]
    RepliesTo,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum DepViewArg {
    #[value(name = "execution")]
    Execution,
    #[value(name = "gtd")]
    Gtd,
    #[value(name = "groupoid")]
    Groupoid,
}
