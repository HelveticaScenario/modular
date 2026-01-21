//! Operators for SeqValue patterns.
//!
//! This module provides:
//! - `CachedOperator`: Runtime-applicable operators for signal values
//! - `seq_value_registry`: Registry of all operators for SeqValue patterns
//! - Pattern operators: Fast, Slow, Rev, Early, Late, Degrade, Add, Mul, Scale

use std::sync::Arc;

use crate::pattern_system::{
    operators::{OperatorError, OperatorRegistry, OperatorVariant, PatternOperator},
    Fraction, Pattern,
};

use super::scale::{validate_scale_type, FixedRoot, ScaleRoot, ScaleSnapper};
use super::seq_value::SeqValue;

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

// ============ Structural Operators ============

/// Fast operator - speed up pattern by factor.
pub struct FastOperator;

impl PatternOperator<SeqValue> for FastOperator {
    fn name(&self) -> &'static str {
        "fast"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let factor: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.fast(Fraction::from(factor)))
    }
}

/// Slow operator - slow down pattern by factor.
pub struct SlowOperator;

impl PatternOperator<SeqValue> for SlowOperator {
    fn name(&self) -> &'static str {
        "slow"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let factor: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.slow(Fraction::from(factor)))
    }
}

/// Rev operator - reverse pattern per cycle.
pub struct RevOperator;

impl PatternOperator<SeqValue> for RevOperator {
    fn name(&self) -> &'static str {
        "rev"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        _argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        Ok(pattern.rev())
    }
}

/// Early operator - shift pattern earlier in time.
pub struct EarlyOperator;

impl PatternOperator<SeqValue> for EarlyOperator {
    fn name(&self) -> &'static str {
        "early"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let offset: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.early(Fraction::from(offset)))
    }
}

/// Late operator - shift pattern later in time.
pub struct LateOperator;

impl PatternOperator<SeqValue> for LateOperator {
    fn name(&self) -> &'static str {
        "late"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let offset: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.late(Fraction::from(offset)))
    }
}

/// Degrade operator - randomly drop events.
pub struct DegradeOperator;

impl PatternOperator<SeqValue> for DegradeOperator {
    fn name(&self) -> &'static str {
        "degrade"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let prob = match argument {
            Some(arg) => arg.parse().map_err(|_| {
                OperatorError::ParseError(format!("Cannot parse '{}' as number", arg))
            })?,
            None => 0.5,
        };
        Ok(pattern.degrade_by(prob))
    }
}

// ============ Value Operators ============

/// Add operator - add MIDI offset to values.
///
/// For static values (Midi, Note), applies immediately via fmap.
/// For Signal values, the operator is recorded for runtime application.
pub struct AddOperator;

impl PatternOperator<SeqValue> for AddOperator {
    fn name(&self) -> &'static str {
        "add"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let offset: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;

        // Apply immediately to static values
        Ok(pattern.fmap(move |v| v.add_midi(offset)))
    }
}

/// Mul operator - multiply MIDI values.
///
/// For static values (Midi, Note), applies immediately via fmap.
/// For Signal values, the operator is recorded for runtime application.
pub struct MulOperator;

impl PatternOperator<SeqValue> for MulOperator {
    fn name(&self) -> &'static str {
        "mul"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let factor: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;

        // Apply immediately to static values
        Ok(pattern.fmap(move |v| v.mul_midi(factor)))
    }
}

/// Scale operator - snap values to a musical scale.
///
/// Argument format: "root:scale_type" (e.g., "c:major", "d:dorian")
/// Or as a list: root and scale type as separate elements.
///
/// For static values (Midi, Note), applies immediately via fmap.
/// For Signal values, the operator is recorded for runtime application.
pub struct ScaleOperator;

/// Parse a single scale spec like "c:major" or "a:minor"
fn parse_scale_spec(spec: &str) -> Result<(FixedRoot, String), OperatorError> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 2 {
        return Err(OperatorError::ParseError(format!(
            "Invalid scale spec '{}', expected 'root:scale_type' format (e.g., 'c:major')",
            spec
        )));
    }

    let root_str = parts[0].trim();
    let scale_type = parts[1].trim();

    // Validate scale type
    if !validate_scale_type(scale_type) {
        return Err(OperatorError::ParseError(format!(
            "Unknown scale type '{}'",
            scale_type
        )));
    }

    // Parse root
    let root = FixedRoot::parse(root_str).ok_or_else(|| {
        OperatorError::ParseError(format!("Invalid root note '{}'", root_str))
    })?;

    Ok((root, scale_type.to_string()))
}

impl PatternOperator<SeqValue> for ScaleOperator {
    fn name(&self) -> &'static str {
        "scale"
    }

    fn apply(
        &self,
        pattern: Pattern<SeqValue>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<SeqValue>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;

        // Check if this is a pattern of scales (contains spaces) or a single scale
        let scale_specs: Vec<&str> = arg.split_whitespace().collect();

        if scale_specs.len() == 1 {
            // Single scale - apply statically
            let (root, scale_type) = parse_scale_spec(scale_specs[0])?;

            // Build snapper
            let snapper = Arc::new(ScaleSnapper::new(&root, &scale_type).ok_or_else(|| {
                OperatorError::ParseError(format!(
                    "Failed to build scale '{} {}'",
                    root.letter, scale_type
                ))
            })?);

            // Apply immediately to static values
            Ok(pattern.fmap(move |v| {
                match v {
                    SeqValue::Midi(m) => SeqValue::Midi(snapper.snap_midi(*m)),
                    SeqValue::Note { .. } => {
                        if let Some(midi) = v.to_midi() {
                            SeqValue::Midi(snapper.snap_midi(midi))
                        } else {
                            v.clone()
                        }
                    }
                    // Signal values pass through - they'll be scaled at runtime
                    SeqValue::Signal { .. } => v.clone(),
                    SeqValue::Rest => v.clone(),
                }
            }))
        } else {
            // Pattern of scales - build snappers for each
            let mut snappers: Vec<Arc<ScaleSnapper>> = Vec::new();

            for spec in &scale_specs {
                let (root, scale_type) = parse_scale_spec(spec)?;
                let snapper = Arc::new(ScaleSnapper::new(&root, &scale_type).ok_or_else(|| {
                    OperatorError::ParseError(format!(
                        "Failed to build scale '{} {}'",
                        root.letter, scale_type
                    ))
                })?);
                snappers.push(snapper);
            }

            // Create a pattern that cycles through scales based on the hap's cycle position
            // We use app_left to combine the note pattern with a scale selection pattern
            let scale_pattern = crate::pattern_system::combinators::fastcat(
                snappers
                    .into_iter()
                    .map(|s| crate::pattern_system::constructors::pure(s))
                    .collect(),
            );

            // Apply scales using applicative - the scale changes per step
            Ok(pattern.app_left(&scale_pattern, |v, snapper| {
                match v {
                    SeqValue::Midi(m) => SeqValue::Midi(snapper.snap_midi(*m)),
                    SeqValue::Note { .. } => {
                        if let Some(midi) = v.to_midi() {
                            SeqValue::Midi(snapper.snap_midi(midi))
                        } else {
                            v.clone()
                        }
                    }
                    SeqValue::Signal { .. } => v.clone(),
                    SeqValue::Rest => v.clone(),
                }
            }))
        }
    }
}

/// Create the standard operator registry for SeqValue patterns.
pub fn seq_value_registry() -> OperatorRegistry<SeqValue> {
    let mut registry = OperatorRegistry::new();

    // Structural operators
    registry.register(FastOperator);
    registry.register(SlowOperator);
    registry.register(RevOperator);
    registry.register(EarlyOperator);
    registry.register(LateOperator);
    registry.register(DegradeOperator);

    // Value operators
    registry.register(AddOperator);
    registry.register(MulOperator);
    registry.register(ScaleOperator);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::constructors::pure;

    #[test]
    fn test_add_operator() {
        let registry = seq_value_registry();
        let pat = pure(SeqValue::Midi(60.0));

        let result = registry.apply("add", pat, Some("12"), OperatorVariant::Default);
        assert!(result.is_ok());

        let haps = result
            .unwrap()
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);

        if let SeqValue::Midi(m) = &haps[0].value {
            assert!((m - 72.0).abs() < 0.001);
        } else {
            panic!("Expected Midi value");
        }
    }

    #[test]
    fn test_scale_operator() {
        let registry = seq_value_registry();
        let pat = pure(SeqValue::Midi(61.0)); // C#4, should snap to C or D in C major

        let result = registry.apply("scale", pat, Some("c:major"), OperatorVariant::Default);
        assert!(result.is_ok());

        let haps = result
            .unwrap()
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);

        if let SeqValue::Midi(m) = &haps[0].value {
            // Should snap to 60 (C) or 62 (D)
            assert!(*m == 60.0 || *m == 62.0);
        } else {
            panic!("Expected Midi value");
        }
    }

    #[test]
    fn test_scale_operator_invalid_scale() {
        let registry = seq_value_registry();
        let pat = pure(SeqValue::Midi(60.0));

        let result = registry.apply("scale", pat, Some("c:foobar"), OperatorVariant::Default);
        assert!(matches!(result, Err(OperatorError::ParseError(_))));
    }

    #[test]
    fn test_scale_operator_pattern() {
        let registry = seq_value_registry();
        // Two notes that will be scaled by alternating scales
        let pat = crate::pattern_system::combinators::fastcat(vec![
            pure(SeqValue::Midi(61.0)), // C#4 - will be snapped by a:minor
            pure(SeqValue::Midi(61.0)), // C#4 - will be snapped by a:major
        ]);

        let result = registry.apply(
            "scale",
            pat,
            Some("a:minor a:major"),
            OperatorVariant::Default,
        );
        assert!(result.is_ok(), "Expected scale pattern to parse successfully");

        let haps = result
            .unwrap()
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have 2 haps
        assert_eq!(haps.len(), 2);

        // Both should be snapped MIDI values
        for hap in &haps {
            assert!(matches!(hap.value, SeqValue::Midi(_)));
        }
    }

    #[test]
    fn test_fast_operator() {
        let registry = seq_value_registry();
        let pat = crate::pattern_system::combinators::fastcat(vec![
            pure(SeqValue::Midi(60.0)),
            pure(SeqValue::Midi(62.0)),
        ]);

        let result = registry.apply("fast", pat, Some("2"), OperatorVariant::Default);
        assert!(result.is_ok());

        let haps = result
            .unwrap()
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        // fast(2) doubles the events
        assert_eq!(haps.len(), 4);
    }

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

    #[test]
    fn test_scale_operator_slowcat_syntax() {
        // Test that scale(<a:minor a:major>) parses correctly
        // The operator receives the raw string argument, so slowcat syntax
        // needs to be handled differently - it comes as pattern tokens
        let registry = seq_value_registry();
        let pat = pure(SeqValue::Midi(61.0));

        // This tests with space-separated scales which creates fastcat behavior
        let result = registry.apply(
            "scale",
            pat.clone(),
            Some("a:minor a:major"),
            OperatorVariant::Default,
        );
        assert!(result.is_ok(), "Space-separated scales should work");
    }
}
