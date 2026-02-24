//! # premath-bd
//!
//! Memory layer for issue/dependency state.
//!
//! This crate provides:
//! - `Issue` and `Dependency` types (the definables)
//! - JSONL read/write (portable persistence)
//! - `MemoryStore` (canonical in-memory state)
//! - Content/structure hashing helpers for kernel-facing signatures
//!
//! It intentionally does not orchestrate versioning or query backends.
//! Those concerns live in adapter crates (`premath-jj`, `premath-surreal`).
//!
//! ## Data model
//!
//! ```text
//! JSONL (on disk, one line per issue)
//!     ↕  hydrate / flush
//! MemoryStore (deterministic in-memory projection)
//! ```

pub mod atomic_store;
pub mod claim_next;
pub mod dependency;
pub mod events;
pub mod issue;
pub mod issue_graph;
pub mod jsonl;
pub mod memory;
pub mod spec_ir;

pub use atomic_store::{AtomicStoreMutationError, issue_lock_path, mutate_store_jsonl};
pub use claim_next::{
    ClaimNextError, ClaimNextOutcome, ClaimNextRequest, DEFAULT_LEASE_TTL_SECONDS,
    MAX_LEASE_TTL_SECONDS, MIN_LEASE_TTL_SECONDS, claim_next_issue_jsonl,
};
pub use dependency::{DepType, Dependency, DependencyProjection, DependencyView};
pub use events::{
    EventError, ISSUE_EVENT_REF_PREFIX, ISSUE_EVENT_SCHEMA, ISSUE_SNAPSHOT_REF_PREFIX, IssueEvent,
    IssueEventAction, event_stream_ref, migrate_store_to_events, read_events,
    read_events_from_path, replay_events, replay_events_from_path, store_snapshot_ref,
    stores_equivalent, write_events, write_events_to_path,
};
pub use issue::{Issue, IssueLease, IssueLeaseState};
pub use issue_graph::{
    DEFAULT_NOTE_WARN_THRESHOLD, FAILURE_CLASS_ACCEPTANCE_MISSING, FAILURE_CLASS_EPIC_MISMATCH,
    FAILURE_CLASS_VERIFICATION_COMMAND_MISSING, ISSUE_GRAPH_CHECK_KIND, IssueGraphCheckReport,
    IssueGraphFinding, IssueGraphSummary, WARNING_CLASS_NOTES_LARGE, check_issue_graph,
};
pub use memory::{DependencyGraphScope, MemoryStore, MemoryStoreError};
pub use spec_ir::{
    SPEC_IR_AUTHORITY_MODE, SPEC_IR_EDGE_KIND_STATEMENT_BINDING, SPEC_IR_ENTITY_KIND_STATEMENT,
    SPEC_IR_PROJECTION_SCHEMA, SpecIrEdge, SpecIrEntity, SpecIrProjection, SpecIrProjectionError,
    SpecIrSources, load_spec_ir_projection_from_paths, load_spec_ir_projection_from_values,
};
