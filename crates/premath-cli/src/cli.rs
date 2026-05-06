use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "premath",
    about = "Premath: admissibility checks for definability by contractible descent",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
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

    /// Validate promoted draft traceability matrix and authority-class parity
    TraceabilityCheck {
        /// Draft spec directory
        #[arg(long, default_value = "specs/premath/draft")]
        draft_dir: String,

        /// Traceability matrix markdown path
        #[arg(long, default_value = "specs/premath/draft/SPEC-TRACEABILITY.md")]
        matrix: String,

        /// Machine-readable authority map JSON path
        #[arg(long, default_value = "specs/premath/AUTHORITY-MAP.json")]
        authority_map: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate repository command-surface hygiene
    CommandSurfaceCheck {
        /// Repository root to scan
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Check repository hygiene guardrails for local/private surfaces
    RepoHygieneCheck {
        /// Repository root to check
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Optional explicit paths to check; defaults to the tracked git index
        paths: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate drift-budget sentinels across specs/contracts/checkers
    DriftBudgetCheck {
        /// Repository root to check
        #[arg(long, default_value = ".")]
        repo_root: String,

        /// Optional precomputed coherence-check witness JSON
        #[arg(long)]
        coherence_json: Option<String>,

        /// Optional topology-budget contract path
        #[arg(long)]
        topology_budget: Option<String>,

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

    /// Project changed paths to local checker IDs through Premath checker semantics
    RequiredProjection {
        /// Projection input JSON path (`{changedPaths:[...]}`)
        #[arg(long)]
        input: String,

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

    /// Reference binding profile operations (`project_ref` / `verify_ref`)
    Ref {
        #[command(subcommand)]
        command: RefCommands,
    },

    /// Check one normalized work-tracker claim through Premath checker semantics
    WorkTrackerCheck {
        /// Work tracker checker input JSON path
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Run one toy Gate vector through Premath kernel semantics
    ToyGateCheck {
        /// Toy Gate case JSON path
        #[arg(long)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
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
