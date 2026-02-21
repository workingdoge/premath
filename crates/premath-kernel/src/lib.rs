//! # Premath Kernel
//!
//! The kernel doctrine of definability: a notion is admissible exactly when
//! it is stable under context change and glues uniquely from locally
//! compatible data.
//!
//! This crate is **ontology-agnostic**: it does not prescribe what definables
//! are (sets, groupoids, ∞-groupoids, …). It only prescribes how they must
//! behave under reindexing and descent.
//!
//! ## Architecture
//!
//! ```text
//! Coherence<V>          ← Ambient sameness level (Set, Gpd, S∞)
//!     │
//! Context               ← Objects of C, with morphisms f: Γ' → Γ
//!     │
//! Cover                 ← Families {u_i: Γ_i → Γ} ▷ Γ
//!     │
//! Definable<V>          ← Fibers Def(Γ) with reindexing f*
//!     │
//! DescentDatum<V>       ← Local definables + overlap compatibilities
//!     │
//! ContractibleDescent   ← The axiom: res_U is an equivalence
//! ```

pub mod coherence;
pub mod context;
pub mod cover;
pub mod definable;
pub mod descent;
pub mod error;
pub mod gate;
pub mod toy;
pub mod witness;

pub use coherence::CoherenceLevel;
pub use context::{Context, ContextId, Morphism};
pub use cover::Cover;
pub use definable::{ContentHash, Definable, FiberSignature};
pub use descent::{
    ContractibilityResult, DescentDatum, OverlapWitness, check_refinement_invariance,
};
pub use error::PremathError;
pub use gate::{GateCheck, World};
pub use witness::{GateFailure, GateResult};
