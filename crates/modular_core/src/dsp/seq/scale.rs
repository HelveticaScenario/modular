//! Scale root handling for the Seq module.
//!
//! This module provides:
//! - `ScaleRoot`: Represents either a fixed root note or a pattern of roots
//! - Re-exports from utilities/scale.rs for backwards compatibility

// Re-export from utilities for backwards compatibility
pub use crate::dsp::utilities::scale::{
    FixedRoot, ScaleSnapper, validate_scale_type,
};

use crate::pattern_system::Pattern;

/// Scale root - either fixed or dynamic (pattern-based).
#[derive(Clone)]
pub enum ScaleRoot {
    /// A fixed root note.
    Fixed(FixedRoot),

    /// A pattern of root notes (queried at hap.whole_begin).
    Pattern(Pattern<FixedRoot>),
}

impl ScaleRoot {
    /// Get the root at a specific time.
    /// For Fixed, returns the fixed root.
    /// For Pattern, queries the pattern at the given time.
    pub fn root_at(&self, time: f64) -> Option<FixedRoot> {
        match self {
            ScaleRoot::Fixed(root) => Some(root.clone()),
            ScaleRoot::Pattern(pat) => {
                pat.query_at_first(time).map(|hap| hap.value.clone())
            }
        }
    }
}

impl std::fmt::Debug for ScaleRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScaleRoot::Fixed(root) => write!(f, "ScaleRoot::Fixed({:?})", root),
            ScaleRoot::Pattern(_) => write!(f, "ScaleRoot::Pattern(...)"),
        }
    }
}
