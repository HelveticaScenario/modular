//! Runtime operators for SeqValue patterns.
//!
//! This module provides `CachedOperator` for runtime-applicable operators
//! on signal values during DSP processing.

use std::sync::Arc;

use super::scale::{ScaleRoot, ScaleSnapper};

/// Cached operator for runtime application to signal values.
///
/// These operators are recorded during pattern parsing when applied to
/// signal-containing patterns. They are then applied per-tick during
/// DSP processing.
#[derive(Clone, Debug)]
pub enum CachedOperator {
    /// Add a MIDI offset.
    Add(f64),

    /// Multiply MIDI value.
    Mul(f64),

    /// Snap to scale.
    Scale {
        snapper: Arc<ScaleSnapper>,
        root: ScaleRoot,
    },
}

impl CachedOperator {
    /// Apply this operator to a MIDI value.
    ///
    /// For Scale operators with dynamic roots, the time parameter is used
    /// to query the root pattern.
    pub fn apply(&self, midi: f64, time: f64) -> f64 {
        match self {
            CachedOperator::Add(offset) => midi + offset,
            CachedOperator::Mul(factor) => midi * factor,
            CachedOperator::Scale { snapper, root } => {
                // For dynamic roots, we'd need to rebuild the snapper
                // For now, use the base snapper (root pattern support TODO)
                match root {
                    ScaleRoot::Fixed(_) => snapper.snap_midi(midi),
                    ScaleRoot::Pattern(pat) => {
                        // Query root at the given time
                        if let Some(dynamic_root) = pat.query_at_first(time).map(|h| h.value.clone())
                        {
                            // Build a new snapper with the dynamic root
                            // This is expensive - in production, cache these
                            if let Some(dynamic_snapper) =
                                ScaleSnapper::new(&dynamic_root, &get_scale_name_from_snapper(snapper))
                            {
                                return dynamic_snapper.snap_midi(midi);
                            }
                        }
                        // Fallback to original snapper
                        snapper.snap_midi(midi)
                    }
                }
            }
        }
    }
}

/// Extract scale name from a ScaleSnapper (for dynamic root rebuilding).
fn get_scale_name_from_snapper(snapper: &ScaleSnapper) -> &str {
    snapper.scale_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_operator_add() {
        let op = CachedOperator::Add(12.0);
        assert!((op.apply(60.0, 0.0) - 72.0).abs() < 0.001);
    }

    #[test]
    fn test_cached_operator_mul() {
        let op = CachedOperator::Mul(0.5);
        assert!((op.apply(60.0, 0.0) - 30.0).abs() < 0.001);
    }
}
