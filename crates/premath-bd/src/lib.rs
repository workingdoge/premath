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

pub mod dependency;
pub mod issue;
pub mod jsonl;
pub mod memory;

pub use dependency::{DepType, Dependency};
pub use issue::Issue;
pub use memory::{MemoryStore, MemoryStoreError};
