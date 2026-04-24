//! Compatibility re-exports for Premath identity contracts.
//!
//! Generic identity semantics live in `premath-identity`; `premath-tusk`
//! re-exports them so existing runtime callers can migrate gradually.

pub use premath_identity::{IntentSpec, RunIdOptions, RunIdentity, compute_intent_id};
