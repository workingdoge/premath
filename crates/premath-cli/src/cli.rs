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

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Generate first-run evaluator artifacts (scheme + rhai + contract stub)
    EvaluatorScaffold {
        /// Directory where scaffold files are generated
        #[arg(long, default_value = ".premath/evaluator_scaffold")]
        path: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

        /// Projection lookup match mode (typed authority only by default)
        #[arg(long, default_value = "typed")]
        projection_match: ProjectionMatchArg,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build Observation Surface v0 from canonical CI witness and issue memory substrates
    ObserveBuild {
        /// Repository root used to resolve relative inputs/outputs
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// CI witness directory
        #[arg(long, default_value = "artifacts/ciwitness")]
        ciwitness_dir: String,

        /// Issue memory JSONL path
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues_path: String,

        /// Observation surface JSON output path
        #[arg(long, default_value = "artifacts/observation/latest.json")]
        out_json: String,

        /// Observation events JSONL output path
        #[arg(long, default_value = "artifacts/observation/events.jsonl")]
        out_jsonl: String,

        /// Output built surface as JSON
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

    /// Validate Observation Surface v0 semantic projection invariants
    ObservationSemanticsCheck {
        /// Repository root used to resolve relative paths
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// CI witness artifact directory
        #[arg(long, default_value = "artifacts/ciwitness")]
        ciwitness_dir: String,

        /// Observation surface JSON path
        #[arg(long, default_value = "artifacts/observation/latest.json")]
        surface: String,

        /// Issue memory JSONL path
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues_path: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

    /// Validate one or more instruction envelopes through core checker semantics
    InstructionBatchCheck {
        /// Instruction JSON path (repeatable). If omitted, checks default instruction globs.
        #[arg(long = "instruction")]
        instructions: Vec<String>,

        /// Repository root used for policy artifact resolution
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build one CI instruction witness through core checker semantics
    InstructionWitness {
        /// Instruction JSON path
        #[arg(long)]
        instruction: String,

        /// Runtime JSON path (results/timestamps/profile bindings)
        #[arg(long)]
        runtime: String,

        /// Optional pre-execution reject failure class (invalid-envelope flow)
        #[arg(long)]
        pre_execution_failure_class: Option<String>,

        /// Optional pre-execution reject reason (invalid-envelope flow)
        #[arg(long)]
        pre_execution_reason: Option<String>,

        /// Repository root used for policy artifact resolution
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build one CI required witness through core checker semantics
    RequiredWitness {
        /// Runtime JSON path (projection/results/gate refs/timestamps/profile bindings)
        #[arg(long)]
        runtime: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Project changed paths to required check IDs through core checker semantics
    RequiredProjection {
        /// Projection input JSON path (`{changedPaths:[...]}`)
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Detect git/workspace delta paths through core checker command surface
    RequiredDelta {
        /// Delta input JSON path (`{repoRoot,fromRef?,toRef?}`)
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build one required gate witness ref (+optional fallback payload) through core semantics
    RequiredGateRef {
        /// Input JSON path (`{checkId,artifactRelPath,source?,gatePayload?|fallback?}`)
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify one CI required witness against deterministic projection semantics
    RequiredWitnessVerify {
        /// Verify input JSON path (witness + changedPaths + optional gate payload map)
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Decide one CI required witness (accept/reject) through core checker semantics
    RequiredWitnessDecide {
        /// Decide input JSON path (witness + optional compare paths + optional gate payload map)
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify one CI required decision against witness/delta attestation semantics
    RequiredDecisionVerify {
        /// Verify input JSON path (decision + witness + deltaSnapshot + actual sha values)
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Evaluate governance promotion gate semantics through core command surface
    GovernancePromotionCheck {
        /// Governance gate input JSON path
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Evaluate KCIR mapping gate semantics through core command surface
    KcirMappingCheck {
        /// KCIR mapping gate input JSON path
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Evaluate one doctrine-inf vector case through core command semantics
    DoctrineInfCheck {
        /// Doctrine-inf case JSON path
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate doctrine operation registry coverage/morphism parity for MCP tools
    DoctrineMcpParityCheck {
        /// Path to mcp_serve.rs source file
        #[arg(long, default_value = "crates/premath-cli/src/commands/mcp_serve.rs")]
        mcp_source: String,

        /// Path to doctrine operation registry JSON
        #[arg(long, default_value = "specs/premath/draft/DOCTRINE-OP-REGISTRY.json")]
        registry: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Emit canonical obligation->Gate mapping registry
    ObligationRegistry {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate repository command-surface invariants (mise-only command surface)
    CommandSurfaceCheck {
        /// Repository root
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate capability fixture stub/invariance conformance vectors
    CapabilityStubInvarianceCheck {
        /// Capability fixture root directory
        #[arg(long, default_value = "tests/conformance/fixtures/capabilities")]
        fixtures: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate promoted draft spec traceability matrix integrity
    SpecTraceabilityCheck {
        /// Promoted draft spec directory
        #[arg(long, default_value = "specs/premath/draft")]
        draft_dir: String,

        /// Traceability matrix markdown path
        #[arg(long, default_value = "specs/premath/draft/SPEC-TRACEABILITY.md")]
        matrix: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Enforce deterministic drift-budget sentinels across contracts/docs/checkers
    DriftBudgetCheck {
        /// Repository root used to resolve relative paths
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Optional precomputed coherence witness JSON path
        #[arg(long)]
        coherence_json: Option<String>,

        /// Optional topology-budget contract JSON path
        #[arg(long)]
        topology_budget: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate baseline workflow wiring invariants for required gate entrypoint
    CiWiringCheck {
        /// Repository root used to resolve relative paths
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Workflow file path relative to repo root
        #[arg(long, default_value = ".github/workflows/baseline.yml")]
        workflow: String,

        /// Control-plane contract path (used to derive required pipeline entrypoint)
        #[arg(
            long,
            default_value = "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
        )]
        control_plane_contract: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate provider-neutral pipeline wrapper wiring and hook parity
    PipelineWiringCheck {
        /// Repository root used to resolve relative paths
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Control-plane contract path
        #[arg(
            long,
            default_value = "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
        )]
        control_plane_contract: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate repository hygiene guardrails for private/local-only surfaces
    RepoHygieneCheck {
        /// Repository root used for git index and `.gitignore` checks
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Optional explicit paths to evaluate instead of scanning the git index
        paths: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate tracked branch/ruleset policy against effective GitHub rule payload
    BranchPolicyCheck {
        /// Branch-policy contract JSON path
        #[arg(long, default_value = "specs/process/GITHUB-BRANCH-POLICY.json")]
        policy: String,

        /// Effective branch-rules payload JSON path (fixture/offline mode)
        #[arg(long)]
        rules_json: Option<String>,

        /// Fetch live rules from GitHub API instead of --rules-json
        #[arg(long)]
        fetch_live: bool,

        /// Repository slug owner/name override (defaults to policy.repository)
        #[arg(long)]
        repo: Option<String>,

        /// Branch override (defaults to policy.branch)
        #[arg(long)]
        branch: Option<String>,

        /// GitHub API base URL override (default: env GITHUB_API_URL or https://api.github.com)
        #[arg(long)]
        github_api_url: Option<String>,

        /// Environment variable containing API token for --fetch-live
        #[arg(long, default_value = "GITHUB_TOKEN")]
        token_env: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate issue-graph contract checks plus compactness drift invariants
    IssueGraphCheck {
        /// Repository root used to resolve relative issue-graph paths
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Issue graph JSONL path relative to --repo-root
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Warning threshold for issue note length
        #[arg(long, default_value_t = 2000)]
        note_warn_threshold: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Check/apply deterministic issue-graph compactness remediation
    IssueGraphCompact {
        /// Repository root used to resolve relative issue-graph paths
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Issue graph JSONL path relative to --repo-root
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Compactness mode: check or apply
        #[arg(long, default_value = "check")]
        mode: IssueGraphCompactModeArg,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate typed transport action registry contract
    TransportCheck {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Execute one typed transport dispatch envelope
    TransportDispatch {
        /// Transport action ID (for example: issue.claim)
        #[arg(long)]
        action: String,

        /// JSON payload string for the action envelope
        #[arg(long, default_value = "{}")]
        payload: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Execute one bounded scheme-eval host-action program
    SchemeEval {
        /// Program JSON path
        #[arg(long)]
        program: String,

        /// Control-plane contract JSON path used to derive host-action allowlist
        #[arg(
            long,
            default_value = "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
        )]
        control_plane_contract: String,

        /// Harness trajectory JSONL path where host effects are bound
        #[arg(long, default_value = ".premath/harness_trajectory.jsonl")]
        trajectory_path: String,

        /// Step-id prefix for emitted trajectory rows
        #[arg(long, default_value = "scheme_eval")]
        step_prefix: String,

        /// Maximum host calls per program
        #[arg(long, default_value_t = 32)]
        max_calls: usize,

        /// Optional default issue ID for emitted rows
        #[arg(long)]
        issue_id: Option<String>,

        /// Optional default policy digest (required for mutation actions)
        #[arg(long)]
        policy_digest: Option<String>,

        /// Optional default instruction ref (required for mutation actions)
        #[arg(long)]
        instruction_ref: Option<String>,

        /// Capability claim row (repeatable)
        #[arg(long = "capability-claim")]
        capability_claims: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Execute one bounded Rhai script over the canonical host-action ABI
    #[cfg(feature = "rhai-frontend")]
    RhaiEval {
        /// Rhai script path
        #[arg(long)]
        script: String,

        /// Control-plane contract JSON path used to derive host-action allowlist
        #[arg(
            long,
            default_value = "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
        )]
        control_plane_contract: String,

        /// Harness trajectory JSONL path where host effects are bound
        #[arg(long, default_value = ".premath/harness_trajectory.jsonl")]
        trajectory_path: String,

        /// Step-id prefix for emitted trajectory rows
        #[arg(long, default_value = "rhai_eval")]
        step_prefix: String,

        /// Maximum host calls per program
        #[arg(long, default_value_t = 32)]
        max_calls: usize,

        /// Optional default issue ID for emitted rows
        #[arg(long)]
        issue_id: Option<String>,

        /// Optional default policy digest (required for mutation actions)
        #[arg(long)]
        policy_digest: Option<String>,

        /// Optional default instruction ref (required for mutation actions)
        #[arg(long)]
        instruction_ref: Option<String>,

        /// Capability claim row (repeatable)
        #[arg(long = "capability-claim")]
        capability_claims: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate world registry + route binding consistency through kernel semantics
    WorldRegistryCheck {
        /// World registry JSON path
        #[arg(long)]
        registry: Option<String>,

        /// Doctrine site input JSON path containing worldRouteBindings declaration
        #[arg(long)]
        site_input: Option<String>,

        /// Optional operation registry/row JSON path used for morphism drift checks
        #[arg(long)]
        operations: Option<String>,

        /// Optional control-plane contract JSON used to derive required world-route families/bindings
        #[arg(long)]
        control_plane_contract: Option<String>,

        /// Required route family ID (repeatable; only used with --site-input)
        #[arg(long = "required-route-family")]
        required_route_families: Vec<String>,

        /// Required route-operation binding (repeatable; `route-family-id=operation-id`)
        #[arg(long = "required-route-binding")]
        required_route_bindings: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate world-descent contract shape through doctrine authority
    WorldDescentContractCheck {
        /// Control-plane contract JSON path
        #[arg(
            long,
            default_value = "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
        )]
        control_plane_contract: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Resolve one operation to a deterministic site/route/world binding
    SiteResolve {
        /// Site resolve request JSON path
        #[arg(long)]
        request: String,

        /// Doctrine site input JSON path
        #[arg(long, default_value = "specs/premath/draft/DOCTRINE-SITE-INPUT.json")]
        doctrine_site_input: String,

        /// Doctrine site JSON path
        #[arg(long, default_value = "specs/premath/draft/DOCTRINE-SITE.json")]
        doctrine_site: String,

        /// Doctrine operation registry JSON path
        #[arg(long, default_value = "specs/premath/draft/DOCTRINE-OP-REGISTRY.json")]
        doctrine_op_registry: String,

        /// Control-plane contract JSON path
        #[arg(
            long,
            default_value = "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
        )]
        control_plane_contract: String,

        /// Capability registry JSON path
        #[arg(long, default_value = "specs/premath/draft/CAPABILITY-REGISTRY.json")]
        capability_registry: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate runtime orchestration route/handoff bindings through core semantics
    RuntimeOrchestrationCheck {
        /// Control-plane contract JSON path
        #[arg(
            long,
            default_value = "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
        )]
        control_plane_contract: String,

        /// Doctrine operation registry JSON path
        #[arg(long, default_value = "specs/premath/draft/DOCTRINE-OP-REGISTRY.json")]
        doctrine_op_registry: String,

        /// Harness runtime contract markdown path
        #[arg(long, default_value = "specs/premath/draft/HARNESS-RUNTIME.md")]
        harness_runtime: String,

        /// Optional doctrine site input JSON path for world-route admission checks
        #[arg(long)]
        doctrine_site_input: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Run Gate checks against a real control-plane operation world
    WorldGateCheck {
        /// Operation registry/row JSON path
        #[arg(long)]
        operations: String,

        /// Gate check fixture JSON path (either `{check:{...}}` or direct check object)
        #[arg(long)]
        check: String,

        /// Gate profile label
        #[arg(long, default_value = "control-plane")]
        profile: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Reference binding profile operations (`project_ref` / `verify_ref`)
    Ref {
        #[command(subcommand)]
        command: RefCommands,
    },

    /// Manage issues in premath-bd JSONL memory
    Issue {
        #[command(subcommand)]
        command: IssueCommands,
    },

    /// Manage Tusk harness-session handoff artifacts
    HarnessSession {
        #[command(subcommand)]
        command: HarnessSessionCommands,
    },

    /// Manage Tusk harness feature-ledger artifacts
    HarnessFeature {
        #[command(subcommand)]
        command: HarnessFeatureCommands,
    },

    /// Manage append-only Tusk harness step trajectory rows
    HarnessTrajectory {
        #[command(subcommand)]
        command: HarnessTrajectoryCommands,
    },

    /// Evaluate deterministic ToolUse/JoinClosed closure state from normalized evidence
    HarnessJoinCheck {
        /// Join-check input JSON path
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

#[derive(Clone, Debug, ValueEnum)]
pub enum ProjectionMatchArg {
    #[value(name = "typed")]
    Typed,
    #[value(name = "compatibility_alias")]
    CompatibilityAlias,
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

    /// Report issue backend integration state (JSONL authority, projection cache, JJ)
    BackendStatus {
        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Repository root used for JJ discovery
        #[arg(long, default_value = ".")]
        repo: String,

        /// Path to surreal issue-query projection cache
        #[arg(long, default_value = ".premath/surreal_issue_cache.json")]
        projection: String,

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

    /// Show non-closed issues that are explicitly blocked or dependency-blocked
    Blocked {
        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Check issue-graph contract invariants
    Check {
        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Warning threshold for notes length
        #[arg(long, default_value_t = 2000)]
        note_warn_threshold: usize,

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
        #[arg(long, conflicts_with = "notes_file")]
        notes: Option<String>,

        /// Read notes from file path (`-` reads stdin)
        #[arg(long, conflicts_with = "notes")]
        notes_file: Option<String>,

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

    /// Atomically claim the next ready/open issue for work
    ClaimNext {
        /// Assignee to claim with
        #[arg(long)]
        assignee: String,

        /// Optional explicit lease identifier
        #[arg(long)]
        lease_id: Option<String>,

        /// Optional lease TTL in seconds
        #[arg(long)]
        lease_ttl_seconds: Option<i64>,

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

    /// Remove a dependency edge
    Remove {
        /// Source issue ID
        issue_id: String,

        /// Target dependency issue ID
        depends_on_id: String,

        /// Dependency type
        #[arg(long = "type", default_value = "blocks")]
        dep_type: DepTypeArg,

        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Replace one dependency edge type with another
    Replace {
        /// Source issue ID
        issue_id: String,

        /// Target dependency issue ID
        depends_on_id: String,

        /// Current dependency type
        #[arg(long = "from-type", default_value = "blocks")]
        from_dep_type: DepTypeArg,

        /// Replacement dependency type
        #[arg(long = "to-type")]
        to_dep_type: DepTypeArg,

        /// Optional created_by annotation for the updated edge
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

    /// Report dependency graph integrity diagnostics
    Diagnostics {
        /// Path to issues JSONL
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Graph scope for diagnostics (`active` excludes closed issues, `full` includes all)
        #[arg(long = "graph-scope", default_value = "active")]
        graph_scope: DepGraphScopeArg,

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

#[derive(Clone, Debug, ValueEnum)]
pub enum DepGraphScopeArg {
    #[value(name = "active")]
    Active,
    #[value(name = "full")]
    Full,
}

#[derive(Subcommand, Clone, Debug)]
pub enum HarnessSessionCommands {
    /// Read one harness-session artifact
    Read {
        /// Harness-session artifact path
        #[arg(long, default_value = ".premath/harness_session.json")]
        path: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Write/update one harness-session artifact
    Write {
        /// Harness-session artifact path
        #[arg(long, default_value = ".premath/harness_session.json")]
        path: String,

        /// Explicit session id override
        #[arg(long)]
        session_id: Option<String>,

        /// Session state
        #[arg(long, default_value = "stopped")]
        state: HarnessSessionStateArg,

        /// Current or resumed issue id
        #[arg(long)]
        issue_id: Option<String>,

        /// Compact handoff summary
        #[arg(long)]
        summary: Option<String>,

        /// Next-step recommendation for bootstrap
        #[arg(long)]
        next_step: Option<String>,

        /// Instruction witness references (repeatable)
        #[arg(long = "instruction-ref")]
        instruction_refs: Vec<String>,

        /// Gate/CI witness references (repeatable)
        #[arg(long = "witness-ref")]
        witness_refs: Vec<String>,

        /// Site lineage references (repeatable; e.g. ctx/cover/refinement refs)
        #[arg(long = "lineage-ref")]
        lineage_refs: Vec<String>,

        /// Optional issues JSONL path for snapshot reference derivation
        #[arg(long, default_value = ".premath/issues.jsonl")]
        issues: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build one bootstrap payload from a harness-session artifact
    Bootstrap {
        /// Harness-session artifact path
        #[arg(long, default_value = ".premath/harness_session.json")]
        path: String,

        /// Harness feature-ledger artifact path used for deterministic next-step projection
        #[arg(long, default_value = ".premath/harness_feature_ledger.json")]
        feature_ledger: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum HarnessSessionStateArg {
    #[value(name = "active")]
    Active,
    #[value(name = "stopped")]
    Stopped,
}

#[derive(Subcommand, Clone, Debug)]
pub enum HarnessFeatureCommands {
    /// Read one harness feature-ledger artifact
    Read {
        /// Harness feature-ledger artifact path
        #[arg(long, default_value = ".premath/harness_feature_ledger.json")]
        path: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Upsert one feature row in a harness feature-ledger artifact
    Write {
        /// Harness feature-ledger artifact path
        #[arg(long, default_value = ".premath/harness_feature_ledger.json")]
        path: String,

        /// Feature identifier
        #[arg(long)]
        feature_id: String,

        /// Feature status
        #[arg(long)]
        status: HarnessFeatureStatusArg,

        /// Optional issue ID linked to this feature row
        #[arg(long)]
        issue_id: Option<String>,

        /// Optional compact summary
        #[arg(long)]
        summary: Option<String>,

        /// Optional session reference for boot continuity
        #[arg(long)]
        session_ref: Option<String>,

        /// Instruction references (repeatable)
        #[arg(long = "instruction-ref")]
        instruction_refs: Vec<String>,

        /// Verification references (repeatable)
        #[arg(long = "verification-ref")]
        verification_refs: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate ledger shape and deterministic closure status
    Check {
        /// Harness feature-ledger artifact path
        #[arg(long, default_value = ".premath/harness_feature_ledger.json")]
        path: String,

        /// Require complete closure (all features completed with verification refs)
        #[arg(long)]
        require_closure: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Select deterministic next unfinished feature
    Next {
        /// Harness feature-ledger artifact path
        #[arg(long, default_value = ".premath/harness_feature_ledger.json")]
        path: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum HarnessFeatureStatusArg {
    #[value(name = "pending")]
    Pending,
    #[value(name = "in_progress")]
    InProgress,
    #[value(name = "blocked")]
    Blocked,
    #[value(name = "completed")]
    Completed,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Clone, Debug)]
pub enum HarnessTrajectoryCommands {
    /// Append one harness step trajectory row
    Append {
        /// Harness trajectory JSONL path
        #[arg(long, default_value = ".premath/harness_trajectory.jsonl")]
        path: String,

        /// Deterministic step identifier
        #[arg(long)]
        step_id: String,

        /// Optional issue identifier linked to this step
        #[arg(long)]
        issue_id: Option<String>,

        /// Action label for this step (e.g. apply.patch)
        #[arg(long)]
        action: String,

        /// Result class label for this step
        #[arg(long)]
        result_class: String,

        /// Optional instruction refs (repeatable)
        #[arg(long = "instruction-ref")]
        instruction_refs: Vec<String>,

        /// Witness refs (repeatable)
        #[arg(long = "witness-ref")]
        witness_refs: Vec<String>,

        /// Site lineage references (repeatable; e.g. ctx/cover/refinement refs)
        #[arg(long = "lineage-ref")]
        lineage_refs: Vec<String>,

        /// Optional started-at timestamp (RFC3339)
        #[arg(long)]
        started_at: Option<String>,

        /// Optional finished-at timestamp (RFC3339; default now)
        #[arg(long)]
        finished_at: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Query deterministic trajectory projections
    Query {
        /// Harness trajectory JSONL path
        #[arg(long, default_value = ".premath/harness_trajectory.jsonl")]
        path: String,

        /// Projection mode
        #[arg(long, default_value = "latest")]
        mode: HarnessTrajectoryModeArg,

        /// Maximum rows returned
        #[arg(long, default_value_t = 20)]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum HarnessTrajectoryModeArg {
    #[value(name = "latest")]
    Latest,
    #[value(name = "failed")]
    Failed,
    #[value(name = "retry-needed")]
    RetryNeeded,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum IssueGraphCompactModeArg {
    #[value(name = "check")]
    Check,
    #[value(name = "apply")]
    Apply,
}

#[derive(Subcommand, Clone, Debug)]
pub enum RefCommands {
    /// Compute a deterministic projected reference for `(domain, payload_bytes)`
    Project {
        /// Reference profile JSON path
        #[arg(long, default_value = "policies/ref/sha256_detached_v1.json")]
        profile: String,

        /// Domain string
        #[arg(long)]
        domain: String,

        /// Canonical payload bytes as hex
        #[arg(long = "payload-hex")]
        payload_hex: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify one provided reference against projection/evidence checks
    Verify {
        /// Reference profile JSON path
        #[arg(long, default_value = "policies/ref/sha256_detached_v1.json")]
        profile: String,

        /// Domain string for payload projection
        #[arg(long)]
        domain: String,

        /// Canonical payload bytes as hex
        #[arg(long = "payload-hex")]
        payload_hex: String,

        /// Evidence bytes as hex (empty by default)
        #[arg(long = "evidence-hex", default_value = "")]
        evidence_hex: String,

        /// Provided ref scheme ID
        #[arg(long = "ref-scheme-id")]
        ref_scheme_id: String,

        /// Provided ref params hash
        #[arg(long = "ref-params-hash")]
        ref_params_hash: String,

        /// Provided ref domain
        #[arg(long = "ref-domain")]
        ref_domain: String,

        /// Provided ref digest
        #[arg(long = "ref-digest")]
        ref_digest: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
