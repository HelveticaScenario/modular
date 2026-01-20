//! Mini notation parser for Strudel-style pattern strings.
//!
//! This module provides a parser and converter for mini notation patterns.
//! Mini notation is a concise DSL for describing rhythmic and melodic patterns.
//!
//! # Syntax Overview
//!
//! ## Basic Elements
//! - `0 1 2` - Sequence (fastcat)
//! - `0, 1, 2` - Stack (simultaneous)
//! - `[0 1 2]` - Fast subsequence
//! - `<0 1 2>` - Slow subsequence (one per cycle)
//! - `~` or `-` - Rest/silence
//!
//! ## Values
//! - `42` - Number
//! - `c4` - Note (C, octave 4)
//! - `440hz` - Frequency
//! - `5v` - Voltage
//! - `m60` - MIDI note number
//! - `"sample"` - String
//!
//! ## Modifiers
//! - `0*2` - Fast by 2 (play twice per cycle)
//! - `0/2` - Slow by 2 (play half as often)
//! - `0!3` - Replicate 3 times
//! - `0?` - Degrade (50% chance)
//! - `0?0.3` - Degrade with 30% chance
//! - `0(3,8)` - Euclidean rhythm (3 hits in 8 steps)
//! - `0(3,8,2)` - Euclidean with rotation
//!
//! ## Weights
//! - `0@3 1` - First element takes 3/4 of the cycle
//!
//! ## Lists (Tails)
//! - `c:e:g` - List of values [c, e, g]
//! - `c:maj` - Two-element list for scale specs
//!
//! ## Operators
//! - `0 1 2 $ fast(2)` - Apply operator
//! - `0 1 2 $ add.squeeze(10)` - Operator with variant
//! - `0 1 2 $ fast(2) $ rev()` - Chained operators
//!
//! # Example
//!
//! ```ignore
//! use modular_core::pattern_system::mini::{parse, FromMiniAtom};
//!
//! let pat: Pattern<f64> = parse("0 1 2 3")?;
//! let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
//! assert_eq!(haps.len(), 4);
//! ```

pub mod ast;
pub mod convert;
pub mod parser;

pub use ast::{AtomValue, Located, MiniAST, OperatorCall};
pub use convert::{convert, convert_with_operators, ConvertError, FromMiniAtom};
pub use parser::{parse as parse_ast, ParseError};

use crate::pattern_system::{operators::OperatorRegistry, Pattern};

/// Parse a mini notation string and convert to a Pattern.
///
/// This is the main entry point for parsing mini notation.
///
/// # Type Parameter
/// * `T` - The target value type (must implement `FromMiniAtom`)
///
/// # Example
/// ```ignore
/// let pat: Pattern<f64> = parse("0 1 2 3")?;
/// ```
pub fn parse<T: FromMiniAtom>(input: &str) -> Result<Pattern<T>, ConvertError> {
    let ast = parse_ast(input)?;
    convert(&ast)
}

/// Parse a mini notation string with operator support.
///
/// Uses the provided operator registry to apply any operators in the pattern.
///
/// # Example
/// ```ignore
/// let registry = standard_f64_registry();
/// let pat: Pattern<f64> = parse_with_operators("0 1 2 $ fast(2)", &registry)?;
/// ```
pub fn parse_with_operators<T: FromMiniAtom>(
    input: &str,
    registry: &OperatorRegistry<T>,
) -> Result<Pattern<T>, ConvertError> {
    let ast = parse_ast(input)?;
    convert_with_operators(&ast, registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::{operators::standard_f64_registry, Fraction};

    #[test]
    fn test_parse_simple() {
        let pat: Pattern<f64> = parse("0 1 2 3").unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 4);
    }

    #[test]
    fn test_parse_with_fast_operator() {
        let registry = standard_f64_registry();
        let pat: Pattern<f64> = parse_with_operators("0 1 2 $ fast(2)", &registry).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        // fast(2) doubles the events
        assert_eq!(haps.len(), 6);
    }

    #[test]
    fn test_parse_stack() {
        let pat: Pattern<f64> = parse("0, 1").unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_parse_slowcat() {
        let pat: Pattern<f64> = parse("<0 1 2>").unwrap();

        let haps0 = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        let haps1 = pat.query_arc(Fraction::from_integer(1), Fraction::from_integer(2));

        assert_eq!(haps0[0].value, 0.0);
        assert_eq!(haps1[0].value, 1.0);
    }

    #[test]
    fn test_parse_euclidean() {
        let pat: Pattern<f64> = parse("1(3,8)").unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 3);
    }

    #[test]
    fn test_parse_nested() {
        let pat: Pattern<f64> = parse("[0 1] [2 3]").unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        // Each bracket is a fast subsequence, so [0 1] takes half the cycle
        // Total: 4 events
        assert_eq!(haps.len(), 4);
    }

    #[test]
    fn test_source_tracking() {
        let pat: Pattern<f64> = parse("0 1 2").unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Each hap should have source span info
        for hap in &haps {
            let spans = hap.context.get_all_span_tuples();
            assert!(!spans.is_empty(), "Expected source span info");
        }
    }
}
