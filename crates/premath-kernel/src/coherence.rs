//! Ambient coherence level V.
//!
//! Premath is parameterized by an ambient "sameness level" V:
//!
//! - **Set**: sameness is equality (strict hash match)
//! - **Gpd**: sameness is isomorphism (structural equivalence up to renaming)
//! - **S∞**: sameness is equivalence (full higher coherence)
//!
//! The coherence level determines how strictly overlap compatibility
//! is checked and what constitutes a valid gluing.

/// The ambient sameness level V parameterizing Premath.
///
/// This is the fundamental parameter of the kernel. Everything downstream —
/// what counts as "the same definable," when overlaps are compatible, whether
/// descent is contractible — depends on V.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CoherenceLevel {
    /// V = Set. Sameness is equality.
    ///
    /// Two definables are the same iff their content hashes match exactly.
    /// Overlap compatibility requires identical dependency types.
    /// This is the strictest level and the default for deterministic agents.
    Set,

    /// V = Gpd. Sameness is isomorphism.
    ///
    /// Two definables are the same iff they have identical structure up to
    /// renaming of identifiers and timestamps. Dependency types must agree
    /// on their blocking class (affects-ready-work) but need not match exactly.
    Gpd,

    /// V = S∞. Sameness is equivalence.
    ///
    /// Two definables are the same iff there exists an explicit equivalence
    /// witness between them. This allows agents to produce genuinely different
    /// but equivalent strategies. The most permissive level.
    SInf,
}

impl CoherenceLevel {
    /// Returns true if `self` is at least as permissive as `other`.
    ///
    /// Set < Gpd < S∞ in permissiveness.
    pub fn subsumes(self, other: Self) -> bool {
        self >= other
    }
}

impl Default for CoherenceLevel {
    fn default() -> Self {
        Self::Set
    }
}

impl std::fmt::Display for CoherenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Set => write!(f, "Set"),
            Self::Gpd => write!(f, "Gpd"),
            Self::SInf => write!(f, "S∞"),
        }
    }
}

impl std::str::FromStr for CoherenceLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "set" => Ok(Self::Set),
            "gpd" => Ok(Self::Gpd),
            "sinf" | "s_inf" | "s∞" | "infinity" => Ok(Self::SInf),
            _ => Err(format!("unknown coherence level: {s}")),
        }
    }
}

/// Trait for types that carry a coherence level.
///
/// This enables generic code to be parameterized by V without
/// threading the level through every function signature.
pub trait Coherent {
    /// The coherence level at which this value lives.
    fn coherence_level(&self) -> CoherenceLevel;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coherence_ordering() {
        assert!(CoherenceLevel::SInf.subsumes(CoherenceLevel::Gpd));
        assert!(CoherenceLevel::Gpd.subsumes(CoherenceLevel::Set));
        assert!(CoherenceLevel::SInf.subsumes(CoherenceLevel::Set));
        assert!(!CoherenceLevel::Set.subsumes(CoherenceLevel::Gpd));
    }

    #[test]
    fn coherence_parse() {
        assert_eq!(
            "set".parse::<CoherenceLevel>().unwrap(),
            CoherenceLevel::Set
        );
        assert_eq!(
            "gpd".parse::<CoherenceLevel>().unwrap(),
            CoherenceLevel::Gpd
        );
        assert_eq!(
            "sinf".parse::<CoherenceLevel>().unwrap(),
            CoherenceLevel::SInf
        );
        assert_eq!(
            "s_inf".parse::<CoherenceLevel>().unwrap(),
            CoherenceLevel::SInf
        );
    }
}
