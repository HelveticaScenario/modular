//! Convert mini notation AST to Pattern.
//!
//! The conversion is parameterized by the target atom type through
//! the `FromMiniAtom` trait.

use super::ast::{AtomValue, Located, MiniAST, OperatorCall};
use super::parser::ParseError;
use crate::pattern_system::{
    combinators::{fastcat, slowcat, stack, timecat},
    constructors::{pure, pure_with_span, silence},
    operators::{OperatorRegistry, OperatorVariant},
    random::choose,
    Fraction, Pattern,
};

/// Trait for types that can be created from mini notation atoms.
pub trait FromMiniAtom: Clone + Send + Sync + 'static {
    /// Convert a single atom value to this type.
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError>;

    /// Convert a list of atoms to this type.
    /// Override for types that support list syntax (e.g., scale specs).
    fn from_list(atoms: &[AtomValue]) -> Result<Self, ConvertError> {
        if atoms.len() == 1 {
            Self::from_atom(&atoms[0])
        } else {
            Err(ConvertError::ListNotSupported)
        }
    }

    /// Get the rest/silence value, if supported.
    fn rest_value() -> Option<Self> {
        None
    }
}

/// Error type for conversion failures.
#[derive(Debug, Clone)]
pub enum ConvertError {
    /// Cannot convert atom to target type.
    InvalidAtom(String),
    /// Lists are not supported for this type.
    ListNotSupported,
    /// Operator error.
    OperatorError(String),
    /// Parse error.
    ParseError(ParseError),
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvertError::InvalidAtom(msg) => write!(f, "Invalid atom: {}", msg),
            ConvertError::ListNotSupported => write!(f, "List syntax not supported for this type"),
            ConvertError::OperatorError(msg) => write!(f, "Operator error: {}", msg),
            ConvertError::ParseError(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl std::error::Error for ConvertError {}

impl From<ParseError> for ConvertError {
    fn from(err: ParseError) -> Self {
        ConvertError::ParseError(err)
    }
}

// Implement FromMiniAtom for common types

impl FromMiniAtom for f64 {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        atom.to_f64()
            .ok_or_else(|| ConvertError::InvalidAtom("Cannot convert to f64".to_string()))
    }

    fn from_list(atoms: &[AtomValue]) -> Result<Self, ConvertError> {
        if atoms.len() == 1 {
            Self::from_atom(&atoms[0])
        } else {
            // For f64, try averaging the list values
            let values: Vec<f64> = atoms
                .iter()
                .map(|a| Self::from_atom(a))
                .collect::<Result<_, _>>()?;
            Ok(values.iter().sum::<f64>() / values.len() as f64)
        }
    }

    fn rest_value() -> Option<Self> {
        Some(0.0)
    }
}

impl FromMiniAtom for f32 {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        atom.to_f64()
            .map(|f| f as f32)
            .ok_or_else(|| ConvertError::InvalidAtom("Cannot convert to f32".to_string()))
    }

    fn rest_value() -> Option<Self> {
        Some(0.0)
    }
}

impl FromMiniAtom for i64 {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        atom.to_f64()
            .map(|f| f as i64)
            .ok_or_else(|| ConvertError::InvalidAtom("Cannot convert to i64".to_string()))
    }

    fn rest_value() -> Option<Self> {
        Some(0)
    }
}

impl FromMiniAtom for i32 {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        atom.to_f64()
            .map(|f| f as i32)
            .ok_or_else(|| ConvertError::InvalidAtom("Cannot convert to i32".to_string()))
    }

    fn rest_value() -> Option<Self> {
        Some(0)
    }
}

impl FromMiniAtom for String {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        match atom {
            AtomValue::Identifier(s) => Ok(s.clone()),
            AtomValue::String(s) => Ok(s.clone()),
            AtomValue::Number(n) => Ok(n.to_string()),
            _ => Err(ConvertError::InvalidAtom("Cannot convert to String".to_string())),
        }
    }

    fn from_list(atoms: &[AtomValue]) -> Result<Self, ConvertError> {
        // Join with colons for string lists
        let strings: Vec<String> = atoms
            .iter()
            .map(|a| Self::from_atom(a))
            .collect::<Result<_, _>>()?;
        Ok(strings.join(":"))
    }
}

impl FromMiniAtom for bool {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        match atom {
            AtomValue::Number(n) => Ok(*n != 0.0),
            AtomValue::Identifier(s) => match s.to_lowercase().as_str() {
                "true" | "t" | "1" | "yes" => Ok(true),
                "false" | "f" | "0" | "no" => Ok(false),
                _ => Err(ConvertError::InvalidAtom(format!("Cannot convert '{}' to bool", s))),
            },
            _ => Err(ConvertError::InvalidAtom("Cannot convert to bool".to_string())),
        }
    }

    fn rest_value() -> Option<Self> {
        Some(false)
    }
}

/// Convert an AST to a Pattern.
pub fn convert<T: FromMiniAtom>(ast: &MiniAST) -> Result<Pattern<T>, ConvertError> {
    convert_inner(ast)
}

/// Convert with operator registry for applying operator chains.
pub fn convert_with_operators<T: FromMiniAtom>(
    ast: &MiniAST,
    registry: &OperatorRegistry<T>,
) -> Result<Pattern<T>, ConvertError> {
    match ast {
        MiniAST::WithOperators { base, operators } => {
            let mut pattern = convert_inner(base)?;

            for op in operators {
                pattern = apply_operator(pattern, op, registry)?;
            }

            Ok(pattern)
        }
        _ => convert_inner(ast),
    }
}

fn convert_inner<T: FromMiniAtom>(ast: &MiniAST) -> Result<Pattern<T>, ConvertError> {
    match ast {
        MiniAST::Pure(Located { node, span }) => {
            let value = T::from_atom(node)?;
            Ok(pure_with_span(value, span.clone()))
        }

        MiniAST::Rest(span) => {
            match T::rest_value() {
                Some(val) => Ok(pure_with_span(val, span.clone())),
                None => Ok(silence()), // No rest value, return silence
            }
        }

        MiniAST::List(Located { node, span }) => {
            let value = T::from_list(node)?;
            Ok(pure_with_span(value, span.clone()))
        }

        MiniAST::Sequence(elements) => {
            let mut patterns = Vec::new();
            let mut weights = Vec::new();
            let mut has_weights = false;

            for (ast, weight) in elements {
                patterns.push(convert_inner(ast)?);
                if let Some(w) = weight {
                    weights.push(Fraction::from(*w));
                    has_weights = true;
                } else {
                    weights.push(Fraction::from_integer(1));
                }
            }

            if has_weights {
                // timecat expects (Fraction, Pattern) pairs
                Ok(timecat(weights.into_iter().zip(patterns.into_iter()).collect()))
            } else {
                Ok(fastcat(patterns))
            }
        }

        MiniAST::Stack(patterns) => {
            let converted: Vec<Pattern<T>> = patterns
                .iter()
                .map(convert_inner)
                .collect::<Result<_, _>>()?;
            Ok(stack(converted))
        }

        MiniAST::SlowCat(patterns) => {
            let converted: Vec<Pattern<T>> = patterns
                .iter()
                .map(convert_inner)
                .collect::<Result<_, _>>()?;
            Ok(slowcat(converted))
        }

        MiniAST::RandomChoice(patterns) => {
            let converted: Vec<Pattern<T>> = patterns
                .iter()
                .map(convert_inner)
                .collect::<Result<_, _>>()?;
            // Use choose for random selection
            Ok(choose(converted).bind(|p| p.clone()))
        }

        MiniAST::Range(start, end) => {
            // Generate a sequence of integers in the range
            let values: Vec<i64> = (*start..=*end).collect();
            let patterns: Result<Vec<Pattern<T>>, _> = values
                .iter()
                .map(|n| {
                    let atom = AtomValue::Number(*n as f64);
                    T::from_atom(&atom).map(pure)
                })
                .collect();
            Ok(fastcat(patterns?))
        }

        MiniAST::PolyMeter(sequences) => {
            // Polymeter: stack patterns with different lengths
            let converted: Vec<Pattern<T>> = sequences
                .iter()
                .map(convert_inner)
                .collect::<Result<_, _>>()?;
            Ok(stack(converted))
        }

        MiniAST::Fast(pattern, factor) => {
            let pat = convert_inner(pattern)?;
            let factor_pat: Pattern<f64> = convert_inner(factor)?;
            // For now, query factor at cycle start
            let factor_haps = factor_pat.query_arc(Fraction::from_integer(0), Fraction::new(1, 1000));
            let factor_val = factor_haps.first().map(|h| h.value).unwrap_or(1.0);
            Ok(pat.fast(Fraction::from(factor_val)))
        }

        MiniAST::Slow(pattern, factor) => {
            let pat = convert_inner(pattern)?;
            let factor_pat: Pattern<f64> = convert_inner(factor)?;
            let factor_haps = factor_pat.query_arc(Fraction::from_integer(0), Fraction::new(1, 1000));
            let factor_val = factor_haps.first().map(|h| h.value).unwrap_or(1.0);
            Ok(pat.slow(Fraction::from(factor_val)))
        }

        MiniAST::Replicate(pattern, count) => {
            let pat = convert_inner(pattern)?;
            let pats: Vec<Pattern<T>> = (0..*count).map(|_| pat.clone()).collect();
            Ok(fastcat(pats))
        }

        MiniAST::Degrade(pattern, prob) => {
            let pat = convert_inner(pattern)?;
            let probability = prob.unwrap_or(0.5);
            Ok(pat.degrade_by(probability))
        }

        MiniAST::Euclidean { pattern, pulses, steps, rotation } => {
            let pat = convert_inner(pattern)?;
            let rot = rotation.map(|r| r as i32).unwrap_or(0);
            Ok(pat.euclid_rot(*pulses as i32, *steps, rot))
        }

        MiniAST::WithOperators { base, .. } => {
            // Without a registry, just convert the base
            convert_inner(base)
        }
    }
}

fn apply_operator<T: FromMiniAtom>(
    pattern: Pattern<T>,
    op: &OperatorCall,
    registry: &OperatorRegistry<T>,
) -> Result<Pattern<T>, ConvertError> {
    // Get the argument as a string for the operator
    let arg_string = op.argument.as_ref().map(|arg| ast_to_string(arg));

    // Parse variant
    let variant = op
        .variant
        .as_ref()
        .and_then(|v| OperatorVariant::from_str(v))
        .unwrap_or_default();

    registry
        .apply(&op.name, pattern, arg_string.as_deref(), variant)
        .map_err(|e| ConvertError::OperatorError(e.to_string()))
}

/// Convert an AST back to a string representation (for operator arguments).
fn ast_to_string(ast: &MiniAST) -> String {
    match ast {
        MiniAST::Pure(Located { node, .. }) => atom_to_string(node),
        MiniAST::List(Located { node, .. }) => {
            node.iter().map(atom_to_string).collect::<Vec<_>>().join(":")
        }
        MiniAST::Sequence(elements) => {
            elements
                .iter()
                .map(|(a, w)| {
                    let s = ast_to_string(a);
                    match w {
                        Some(weight) => format!("{}@{}", s, weight),
                        None => s,
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        MiniAST::Stack(patterns) => patterns
            .iter()
            .map(ast_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        MiniAST::SlowCat(patterns) => {
            format!(
                "<{}>",
                patterns
                    .iter()
                    .map(ast_to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        }
        MiniAST::Range(start, end) => format!("{}..{}", start, end),
        _ => String::new(),
    }
}

fn atom_to_string(atom: &AtomValue) -> String {
    match atom {
        AtomValue::Number(n) => n.to_string(),
        AtomValue::Midi(m) => format!("m{}", m),
        AtomValue::Hz(h) => format!("{}hz", h),
        AtomValue::Volts(v) => format!("{}v", v),
        AtomValue::Note { letter, accidental, octave } => {
            let mut s = letter.to_string();
            if let Some(acc) = accidental {
                s.push(*acc);
            }
            if let Some(oct) = octave {
                s.push_str(&oct.to_string());
            }
            s
        }
        AtomValue::Identifier(s) => s.clone(),
        AtomValue::String(s) => format!("\"{}\"", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::mini::parser::parse;

    #[test]
    fn test_convert_number() {
        let ast = parse("42").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 42.0);
    }

    #[test]
    fn test_convert_sequence() {
        let ast = parse("0 1 2").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 3);
    }

    #[test]
    fn test_convert_stack() {
        let ast = parse("0, 1").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_convert_slowcat() {
        let ast = parse("<0 1 2>").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();

        // Each cycle should have one value
        let haps0 = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        let haps1 = pat.query_arc(Fraction::from_integer(1), Fraction::from_integer(2));
        let haps2 = pat.query_arc(Fraction::from_integer(2), Fraction::from_integer(3));

        assert_eq!(haps0[0].value, 0.0);
        assert_eq!(haps1[0].value, 1.0);
        assert_eq!(haps2[0].value, 2.0);
    }

    #[test]
    fn test_convert_fast() {
        let ast = parse("0*2").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_convert_replicate() {
        let ast = parse("0!3").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 3);
    }

    #[test]
    fn test_convert_range() {
        let ast = parse("0..4").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 5); // 0, 1, 2, 3, 4
    }

    #[test]
    fn test_convert_weighted() {
        let ast = parse("0@3 1").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // First element should take 3/4, second 1/4
        assert_eq!(haps.len(), 2);
        let first_duration = haps[0].whole.as_ref().unwrap().duration();
        let second_duration = haps[1].whole.as_ref().unwrap().duration();
        assert_eq!(first_duration, Fraction::new(3, 4));
        assert_eq!(second_duration, Fraction::new(1, 4));
    }

    #[test]
    fn test_convert_euclidean() {
        let ast = parse("1(3,8)").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        // Euclidean(3,8) has 3 hits
        assert_eq!(haps.len(), 3);
    }

    #[test]
    fn test_source_spans() {
        let ast = parse("0 1 2").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Each hap should have a source span
        for hap in &haps {
            assert!(hap.context.source_span.is_some());
        }
    }
}
