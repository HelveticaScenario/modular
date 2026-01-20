//! Pattern operator system for the mini notation.
//!
//! Operators are transformations applied to patterns using the `$ operator(arg)` syntax.
//! Each operator can have a different argument type than the primary pattern type.

use super::{Fraction, Pattern};
use std::collections::HashMap;
use std::sync::Arc;

/// Error type for operator application.
#[derive(Debug, Clone)]
pub enum OperatorError {
    /// Unknown operator name.
    UnknownOperator(String),
    /// Invalid argument type.
    InvalidArgument(String),
    /// Missing required argument.
    MissingArgument,
    /// Parse error in argument.
    ParseError(String),
}

impl std::fmt::Display for OperatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorError::UnknownOperator(name) => write!(f, "Unknown operator: {}", name),
            OperatorError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            OperatorError::MissingArgument => write!(f, "Missing required argument"),
            OperatorError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for OperatorError {}

/// Variant for how to combine operator argument structure with primary pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperatorVariant {
    /// Default - use operator-specific default behavior.
    #[default]
    Default,
    /// Structure from primary pattern (appLeft/inner).
    In,
    /// Structure from argument pattern (appRight/outer).
    Out,
    /// Squeeze argument into primary events.
    Squeeze,
    /// Intersection structure (appBoth/mix).
    Mix,
}

impl OperatorVariant {
    /// Parse a variant name string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "in" | "inner" => Some(OperatorVariant::In),
            "out" | "outer" => Some(OperatorVariant::Out),
            "squeeze" | "sq" => Some(OperatorVariant::Squeeze),
            "mix" | "both" => Some(OperatorVariant::Mix),
            _ => None,
        }
    }
}

/// Trait for operators that transform patterns.
///
/// Each operator takes a primary pattern and an argument pattern,
/// potentially of different types, and produces an output pattern
/// of the same type as the primary.
pub trait PatternOperator<T>: Send + Sync
where
    T: Clone + Send + Sync + 'static,
{
    /// The name of the operator (for lookup).
    fn name(&self) -> &'static str;

    /// Apply the operator to a pattern with a string argument.
    ///
    /// The argument string will be parsed according to the operator's needs.
    fn apply(
        &self,
        pattern: Pattern<T>,
        argument: Option<&str>,
        variant: OperatorVariant,
    ) -> Result<Pattern<T>, OperatorError>;
}

/// Registry for looking up operators by name.
pub struct OperatorRegistry<T>
where
    T: Clone + Send + Sync + 'static,
{
    operators: HashMap<String, Arc<dyn PatternOperator<T>>>,
}

impl<T: Clone + Send + Sync + 'static> Default for OperatorRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send + Sync + 'static> OperatorRegistry<T> {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            operators: HashMap::new(),
        }
    }

    /// Register an operator.
    pub fn register<Op: PatternOperator<T> + 'static>(&mut self, op: Op) {
        self.operators.insert(op.name().to_string(), Arc::new(op));
    }

    /// Get an operator by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn PatternOperator<T>>> {
        self.operators.get(name)
    }

    /// Apply an operator by name.
    pub fn apply(
        &self,
        name: &str,
        pattern: Pattern<T>,
        argument: Option<&str>,
        variant: OperatorVariant,
    ) -> Result<Pattern<T>, OperatorError> {
        let op = self
            .get(name)
            .ok_or_else(|| OperatorError::UnknownOperator(name.to_string()))?;
        op.apply(pattern, argument, variant)
    }

    /// List all registered operator names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.operators.keys().map(|s| s.as_str())
    }
}

// ============ Built-in Operators for f64 ============

/// Fast operator - speed up pattern by factor.
pub struct FastOperator;

impl PatternOperator<f64> for FastOperator {
    fn name(&self) -> &'static str {
        "fast"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let factor: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.fast(Fraction::from(factor)))
    }
}

/// Slow operator - slow down pattern by factor.
pub struct SlowOperator;

impl PatternOperator<f64> for SlowOperator {
    fn name(&self) -> &'static str {
        "slow"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let factor: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.slow(Fraction::from(factor)))
    }
}

/// Add operator - add value to pattern.
pub struct AddOperator;

impl PatternOperator<f64> for AddOperator {
    fn name(&self) -> &'static str {
        "add"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let value: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;

        let arg_pattern = super::constructors::pure(value);

        Ok(match variant {
            OperatorVariant::Default | OperatorVariant::In => {
                pattern.app_left(&arg_pattern, |a, b| a + b)
            }
            OperatorVariant::Out => pattern.app_right(&arg_pattern, |a, b| a + b),
            OperatorVariant::Mix => pattern.app_both(&arg_pattern, |a, b| a + b),
            OperatorVariant::Squeeze => pattern.squeeze_join(move |a| super::constructors::pure(a + value)),
        })
    }
}

/// Mul operator - multiply pattern values.
pub struct MulOperator;

impl PatternOperator<f64> for MulOperator {
    fn name(&self) -> &'static str {
        "mul"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let value: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;

        let arg_pattern = super::constructors::pure(value);

        Ok(match variant {
            OperatorVariant::Default | OperatorVariant::In => {
                pattern.app_left(&arg_pattern, |a, b| a * b)
            }
            OperatorVariant::Out => pattern.app_right(&arg_pattern, |a, b| a * b),
            OperatorVariant::Mix => pattern.app_both(&arg_pattern, |a, b| a * b),
            OperatorVariant::Squeeze => pattern.squeeze_join(move |a| super::constructors::pure(a * value)),
        })
    }
}

/// Range operator - map values from 0-1 to min-max.
pub struct RangeOperator;

impl PatternOperator<f64> for RangeOperator {
    fn name(&self) -> &'static str {
        "range"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;

        // Parse "min:max" or "min max" format
        let parts: Vec<&str> = arg.split(|c| c == ':' || c == ' ').collect();
        if parts.len() != 2 {
            return Err(OperatorError::ParseError(
                "range requires 'min:max' or 'min max' format".to_string(),
            ));
        }

        let min: f64 = parts[0]
            .trim()
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", parts[0])))?;
        let max: f64 = parts[1]
            .trim()
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", parts[1])))?;

        Ok(pattern.fmap(move |v| min + v * (max - min)))
    }
}

/// Rev operator - reverse pattern per cycle.
pub struct RevOperator;

impl PatternOperator<f64> for RevOperator {
    fn name(&self) -> &'static str {
        "rev"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        _argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        Ok(pattern.rev())
    }
}

/// Early operator - shift pattern earlier in time.
pub struct EarlyOperator;

impl PatternOperator<f64> for EarlyOperator {
    fn name(&self) -> &'static str {
        "early"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let offset: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.early(Fraction::from(offset)))
    }
}

/// Late operator - shift pattern later in time.
pub struct LateOperator;

impl PatternOperator<f64> for LateOperator {
    fn name(&self) -> &'static str {
        "late"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let arg = argument.ok_or(OperatorError::MissingArgument)?;
        let offset: f64 = arg
            .parse()
            .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?;
        Ok(pattern.late(Fraction::from(offset)))
    }
}

/// Degrade operator - randomly drop events.
pub struct DegradeOperator;

impl PatternOperator<f64> for DegradeOperator {
    fn name(&self) -> &'static str {
        "degrade"
    }

    fn apply(
        &self,
        pattern: Pattern<f64>,
        argument: Option<&str>,
        _variant: OperatorVariant,
    ) -> Result<Pattern<f64>, OperatorError> {
        let prob = match argument {
            Some(arg) => arg
                .parse()
                .map_err(|_| OperatorError::ParseError(format!("Cannot parse '{}' as number", arg)))?,
            None => 0.5,
        };
        Ok(pattern.degrade_by(prob))
    }
}

/// Create a registry with all standard f64 operators.
pub fn standard_f64_registry() -> OperatorRegistry<f64> {
    let mut registry = OperatorRegistry::new();
    registry.register(FastOperator);
    registry.register(SlowOperator);
    registry.register(AddOperator);
    registry.register(MulOperator);
    registry.register(RangeOperator);
    registry.register(RevOperator);
    registry.register(EarlyOperator);
    registry.register(LateOperator);
    registry.register(DegradeOperator);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::constructors::pure;

    #[test]
    fn test_fast_operator() {
        let registry = standard_f64_registry();
        let pat = super::super::combinators::fastcat(vec![pure(1.0), pure(2.0), pure(3.0)]);

        let result = registry.apply("fast", pat, Some("2"), OperatorVariant::Default);
        assert!(result.is_ok());

        let haps = result
            .unwrap()
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        // Fast(2) should double the events
        assert_eq!(haps.len(), 6);
    }

    #[test]
    fn test_add_operator() {
        let registry = standard_f64_registry();
        let pat = pure(10.0);

        let result = registry.apply("add", pat, Some("5"), OperatorVariant::Default);
        assert!(result.is_ok());

        let haps = result
            .unwrap()
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
        assert!((haps[0].value - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_range_operator() {
        let registry = standard_f64_registry();
        let pat = pure(0.5); // Mid-range value

        let result = registry.apply("range", pat, Some("100:200"), OperatorVariant::Default);
        assert!(result.is_ok());

        let haps = result
            .unwrap()
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
        assert!((haps[0].value - 150.0).abs() < 0.001); // 100 + 0.5 * 100 = 150
    }

    #[test]
    fn test_unknown_operator() {
        let registry = standard_f64_registry();
        let pat = pure(1.0);

        let result = registry.apply("unknown", pat, None, OperatorVariant::Default);
        assert!(matches!(result, Err(OperatorError::UnknownOperator(_))));
    }

    #[test]
    fn test_operator_variant_parse() {
        assert_eq!(OperatorVariant::from_str("in"), Some(OperatorVariant::In));
        assert_eq!(OperatorVariant::from_str("out"), Some(OperatorVariant::Out));
        assert_eq!(
            OperatorVariant::from_str("squeeze"),
            Some(OperatorVariant::Squeeze)
        );
        assert_eq!(OperatorVariant::from_str("mix"), Some(OperatorVariant::Mix));
        assert_eq!(OperatorVariant::from_str("unknown"), None);
    }
}
