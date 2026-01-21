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

/// Trait for types that support rest values.
///
/// Types implementing this trait can represent "silence" or "no value" states.
/// Operations that can produce rests (like `degrade`, `euclid`, `mask`) require
/// this trait to ensure the pattern always produces a hap when queried.
pub trait HasRest: Clone + Send + Sync + 'static {
    /// Get the rest value for this type.
    fn rest_value() -> Self;
}

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

    /// Combine a list of head atoms with a tail value.
    /// Used for distributing patterns like `c:e:[f g]` -> `[c:e:f, c:e:g]`
    /// Default implementation prepends atoms to a single-element list with the value.
    fn combine_with_head(head_atoms: &[AtomValue], tail: &Self) -> Result<Self, ConvertError>;

    /// Get the rest/silence value, if supported.
    /// 
    /// DEPRECATED: Use `HasRest` trait instead. This method is kept for
    /// backward compatibility but should not be used for new code.
    fn rest_value() -> Option<Self> {
        None
    }

    /// Returns true if this type supports rest values.
    ///
    /// Types that support rests can use operations like `degrade`, `euclid`,
    /// and the `~` (rest) syntax in mini notation. Types that don't support
    /// rests will produce parse errors when these operations are used.
    fn supports_rest() -> bool {
        false
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
    /// Operation requires rest support but pattern type doesn't support rests.
    RestNotSupported(String),
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvertError::InvalidAtom(msg) => write!(f, "Invalid atom: {}", msg),
            ConvertError::ListNotSupported => write!(f, "List syntax not supported for this type"),
            ConvertError::OperatorError(msg) => write!(f, "Operator error: {}", msg),
            ConvertError::ParseError(err) => write!(f, "Parse error: {}", err),
            ConvertError::RestNotSupported(op) => write!(
                f,
                "'{}' requires a pattern type that supports rests. This operation is only available for note/sequence patterns.",
                op
            ),
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

    fn combine_with_head(head_atoms: &[AtomValue], tail: &Self) -> Result<Self, ConvertError> {
        // For f64, combine by averaging all values
        let mut values: Vec<f64> = head_atoms
            .iter()
            .map(|a| Self::from_atom(a))
            .collect::<Result<_, _>>()?;
        values.push(*tail);
        Ok(values.iter().sum::<f64>() / values.len() as f64)
    }
    // f64 does not support rests - use default supports_rest() -> false
}

impl FromMiniAtom for f32 {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        atom.to_f64()
            .map(|f| f as f32)
            .ok_or_else(|| ConvertError::InvalidAtom("Cannot convert to f32".to_string()))
    }

    fn combine_with_head(head_atoms: &[AtomValue], tail: &Self) -> Result<Self, ConvertError> {
        let mut values: Vec<f32> = head_atoms
            .iter()
            .map(|a| Self::from_atom(a))
            .collect::<Result<_, _>>()?;
        values.push(*tail);
        Ok(values.iter().sum::<f32>() / values.len() as f32)
    }
    // f32 does not support rests - use default supports_rest() -> false
}

impl FromMiniAtom for i64 {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        atom.to_f64()
            .map(|f| f as i64)
            .ok_or_else(|| ConvertError::InvalidAtom("Cannot convert to i64".to_string()))
    }

    fn combine_with_head(_head_atoms: &[AtomValue], _tail: &Self) -> Result<Self, ConvertError> {
        Err(ConvertError::ListNotSupported)
    }
    // i64 does not support rests - use default supports_rest() -> false
}

impl FromMiniAtom for i32 {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        atom.to_f64()
            .map(|f| f as i32)
            .ok_or_else(|| ConvertError::InvalidAtom("Cannot convert to i32".to_string()))
    }

    fn combine_with_head(_head_atoms: &[AtomValue], _tail: &Self) -> Result<Self, ConvertError> {
        Err(ConvertError::ListNotSupported)
    }
    // i32 does not support rests - use default supports_rest() -> false
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

    fn combine_with_head(head_atoms: &[AtomValue], tail: &Self) -> Result<Self, ConvertError> {
        let mut strings: Vec<String> = head_atoms
            .iter()
            .map(|a| Self::from_atom(a))
            .collect::<Result<_, _>>()?;
        strings.push(tail.clone());
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

    fn combine_with_head(_head_atoms: &[AtomValue], _tail: &Self) -> Result<Self, ConvertError> {
        Err(ConvertError::ListNotSupported)
    }
    // bool does not support rests - use default supports_rest() -> false
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

        MiniAST::Rest(_span) => {
            if !T::supports_rest() {
                return Err(ConvertError::RestNotSupported("~ (rest)".to_string()));
            }
            // Safe to unwrap because supports_rest() returned true
            let val = T::rest_value().expect("supports_rest() returned true but rest_value() is None");
            Ok(pure_with_span(val, _span.clone()))
        }

        MiniAST::List(Located { node, span }) => {
            // Check if all elements are Pure (simple atoms)
            let all_pure = node.iter().all(|elem| matches!(elem, MiniAST::Pure(_)));
            
            if all_pure {
                // Extract atoms and use from_list
                let atoms: Vec<AtomValue> = node
                    .iter()
                    .filter_map(|elem| {
                        if let MiniAST::Pure(Located { node: atom, .. }) = elem {
                            Some(atom.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let value = T::from_list(&atoms)?;
                Ok(pure_with_span(value, span.clone()))
            } else {
                // Some elements are patterns - need to distribute
                // e.g., c:[e f] -> fastcat([c:e, c:f])
                // 
                // Strategy: convert each element to a pattern, then distribute
                // by applying the "list" semantics across pattern events
                
                // For now, find the first non-Pure element and distribute over it
                // This handles the common case: head:[a b c]
                
                // Separate head atoms (Pure elements before first non-Pure) from tail pattern
                let mut head_atoms: Vec<AtomValue> = Vec::new();
                let mut tail_pattern: Option<Pattern<T>> = None;
                
                for elem in node.iter() {
                    match elem {
                        MiniAST::Pure(Located { node: atom, .. }) => {
                            if tail_pattern.is_none() {
                                head_atoms.push(atom.clone());
                            } else {
                                // Atoms after a pattern - not supported yet
                                return Err(ConvertError::InvalidAtom(
                                    "Atoms after pattern in list not yet supported".to_string()
                                ));
                            }
                        }
                        _ => {
                            if tail_pattern.is_some() {
                                return Err(ConvertError::InvalidAtom(
                                    "Multiple patterns in list not yet supported".to_string()
                                ));
                            }
                            tail_pattern = Some(convert_inner(elem)?);
                        }
                    }
                }
                
                if let Some(tail) = tail_pattern {
                    // Distribute head over tail: head:[a b] -> [head:a, head:b]
                    // Use combine_with_head to merge head atoms with each tail value
                    let head = head_atoms.clone();
                    Ok(tail.fmap(move |tail_val: &T| {
                        // This unwrap is safe because we validated the atoms can convert
                        // to the target type when parsing Pure nodes
                        T::combine_with_head(&head, tail_val).unwrap_or_else(|_| tail_val.clone())
                    }))
                } else {
                    // No patterns, but we hit the !all_pure branch? Shouldn't happen
                    Err(ConvertError::InvalidAtom("Internal error in list processing".to_string()))
                }
            }
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
            if !T::supports_rest() {
                return Err(ConvertError::RestNotSupported("? (degrade)".to_string()));
            }
            let pat = convert_inner(pattern)?;
            let probability = prob.unwrap_or(0.5);
            // Safe to unwrap because supports_rest() returned true
            let rest = T::rest_value().expect("supports_rest() returned true but rest_value() is None");
            Ok(pat.degrade_by_with_rest(probability, rest))
        }

        MiniAST::Euclidean { pattern, pulses, steps, rotation } => {
            if !T::supports_rest() {
                return Err(ConvertError::RestNotSupported("euclidean rhythm (n,k)".to_string()));
            }
            let pat = convert_inner(pattern)?;
            let rot = rotation.map(|r| r as i32).unwrap_or(0);
            // Safe to unwrap because supports_rest() returned true
            let rest = T::rest_value().expect("supports_rest() returned true but rest_value() is None");
            Ok(pat.euclid_rot_with_rest(*pulses as i32, *steps, rot, rest))
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

    let result = registry
        .apply(&op.name, pattern, arg_string.as_deref(), variant)
        .map_err(|e| ConvertError::OperatorError(e.to_string()))?;

    // Add the operator's source span to all haps for editor highlighting
    Ok(result.with_modifier_span(op.span.clone()))
}

/// Convert an AST back to a string representation (for operator arguments).
fn ast_to_string(ast: &MiniAST) -> String {
    match ast {
        MiniAST::Pure(Located { node, .. }) => atom_to_string(node),
        MiniAST::List(Located { node, .. }) => {
            node.iter().map(ast_to_string).collect::<Vec<_>>().join(":")
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
    fn test_convert_nested_slowcat_in_sequence() {
        // Test that slowcat nested inside a sequence alternates correctly
        // Pattern: a <b c> should give [a, b] on even cycles and [a, c] on odd cycles
        // where a=69, b=71, c=60
        let ast = parse("a <b c>").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();

        // Cycle 0: should have a (69) and b (71)
        let haps0 = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        let values0: Vec<f64> = haps0.iter().map(|h| h.value).collect();
        assert_eq!(values0.len(), 2);
        assert_eq!(values0[0], 69.0, "First element should be 'a' (69)");
        assert_eq!(values0[1], 71.0, "Second element should be 'b' (71) on cycle 0");

        // Cycle 1: should have a (69) and c (60)
        let haps1 = pat.query_arc(Fraction::from_integer(1), Fraction::from_integer(2));
        let values1: Vec<f64> = haps1.iter().map(|h| h.value).collect();
        assert_eq!(values1.len(), 2);
        assert_eq!(values1[0], 69.0, "First element should be 'a' (69)");
        assert_eq!(values1[1], 60.0, "Second element should be 'c' (60) on cycle 1");

        // Cycle 2: should alternate back to b
        let haps2 = pat.query_arc(Fraction::from_integer(2), Fraction::from_integer(3));
        let values2: Vec<f64> = haps2.iter().map(|h| h.value).collect();
        assert_eq!(values2[1], 71.0, "Second element should be 'b' (71) on cycle 2");
    }

    #[test]
    fn test_convert_deeply_nested_slowcat() {
        // Test that deeply nested slowcat respects the inner slowcat
        // Pattern: c3 c#3 <cb <d d5>>
        // This should parse as: Sequence([c3, c#3, SlowCat([cb, SlowCat([d, d5])])])
        // 
        // Let's understand what values we expect:
        // c3 = 48, c#3 = 49, cb4 = 59, d4 = 62, d5 = 74
        // 
        // The outer slowcat has 2 elements: cb and <d d5>
        // Cycle 0: outer picks cb (index 0)
        // Cycle 1: outer picks <d d5> (index 1) - inner has its own cycle tracking
        // Cycle 2: outer picks cb (index 0)
        // Cycle 3: outer picks <d d5> (index 1) - inner advances
        //
        // The inner slowcat sees adjusted time, so it will have its own cycle counting
        let ast = parse("c3 c#3 <cb <d d5>>").unwrap();
        let pat: Pattern<f64> = convert(&ast).unwrap();

        // Verify the pattern structure by querying different cycles
        // c3 = 48, c#3 = 49, cb4 = 59 (c4 flat), d4 = 62, d5 = 74
        
        // Cycle 0: c3, c#3, cb
        let haps0 = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        let values0: Vec<f64> = haps0.iter().map(|h| h.value).collect();
        assert_eq!(values0.len(), 3, "Cycle 0 should have 3 elements");
        assert_eq!(values0[0], 48.0, "First element should be c3 (48)");
        assert_eq!(values0[1], 49.0, "Second element should be c#3 (49)");
        assert_eq!(values0[2], 59.0, "Third element should be cb (59) on cycle 0");

        // Cycle 1: c3, c#3, <d d5> - inner slowcat picks based on its time
        let haps1 = pat.query_arc(Fraction::from_integer(1), Fraction::from_integer(2));
        let values1: Vec<f64> = haps1.iter().map(|h| h.value).collect();
        assert_eq!(values1.len(), 3, "Cycle 1 should have 3 elements");
        assert_eq!(values1[0], 48.0, "First element should be c3 (48)");
        assert_eq!(values1[1], 49.0, "Second element should be c#3 (49)");
        // The inner slowcat sees cycle 0 from its perspective (first time it's queried)
        // But due to time adjustment, it might see a different cycle
        // Let's check what we actually get and verify both d and d5 appear at some point
        let cycle1_third = values1[2];
        assert!(cycle1_third == 62.0 || cycle1_third == 74.0, 
            "Third element on cycle 1 should be d (62) or d5 (74), got {}", cycle1_third);

        // Cycle 2: c3, c#3, cb
        let haps2 = pat.query_arc(Fraction::from_integer(2), Fraction::from_integer(3));
        let values2: Vec<f64> = haps2.iter().map(|h| h.value).collect();
        assert_eq!(values2[2], 59.0, "Third element should be cb (59) on cycle 2");

        // Cycle 3: c3, c#3, <d d5> - inner slowcat advances
        let haps3 = pat.query_arc(Fraction::from_integer(3), Fraction::from_integer(4));
        let values3: Vec<f64> = haps3.iter().map(|h| h.value).collect();
        let cycle3_third = values3[2];
        assert!(cycle3_third == 62.0 || cycle3_third == 74.0, 
            "Third element on cycle 3 should be d (62) or d5 (74), got {}", cycle3_third);
        
        // Key assertion: cycle 1 and cycle 3 should be DIFFERENT values from inner slowcat
        assert_ne!(cycle1_third, cycle3_third, 
            "Inner slowcat should alternate between d and d5 on cycles 1 and 3");
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
        // Euclidean should fail for f64 patterns because f64 doesn't support rests
        let ast = parse("1(3,8)").unwrap();
        let result: Result<Pattern<f64>, _> = convert(&ast);
        assert!(result.is_err(), "Euclidean should fail for f64 patterns");
        assert!(matches!(result.unwrap_err(), ConvertError::RestNotSupported(_)));
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

    #[test]
    fn test_operator_spans_captured() {
        use crate::pattern_system::operators::standard_f64_registry;

        let ast = parse("0 1 2 $ fast(2)").unwrap();
        let registry = standard_f64_registry();
        let pat: Pattern<f64> = convert_with_operators(&ast, &registry).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // fast(2) doubles the events to 6
        assert_eq!(haps.len(), 6);

        // Each hap should have the operator span in modifier_spans
        for hap in &haps {
            assert!(
                !hap.context.modifier_spans.is_empty(),
                "Expected operator span in modifier_spans"
            );
            // The operator span should cover "$ fast(2)" 
            let op_span = &hap.context.modifier_spans[0];
            assert!(op_span.start < op_span.end);
        }
    }

    #[test]
    fn test_multiple_operator_spans() {
        use crate::pattern_system::operators::standard_f64_registry;

        let ast = parse("0 1 $ fast(2) $ add(10)").unwrap();
        let registry = standard_f64_registry();
        let pat: Pattern<f64> = convert_with_operators(&ast, &registry).unwrap();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Each hap should have BOTH operator spans in modifier_spans
        for hap in &haps {
            assert!(
                hap.context.modifier_spans.len() >= 2,
                "Expected two operator spans, got {}",
                hap.context.modifier_spans.len()
            );
        }
    }

    #[test]
    fn test_convert_degrade() {
        // Degrade should fail for f64 patterns because f64 doesn't support rests
        let ast = parse("c4?").unwrap();
        let result: Result<Pattern<f64>, _> = convert(&ast);
        assert!(result.is_err(), "Degrade should fail for f64 patterns");
        assert!(matches!(result.unwrap_err(), ConvertError::RestNotSupported(_)));
    }

    #[test]
    fn test_convert_degrade_in_sequence() {
        // Degrade should fail for f64 patterns even when in a sequence
        let ast = parse("c2 c3 c4? c5").unwrap();
        let result: Result<Pattern<f64>, _> = convert(&ast);
        assert!(result.is_err(), "Degrade should fail for f64 patterns");
        assert!(matches!(result.unwrap_err(), ConvertError::RestNotSupported(_)));
    }

    #[test]
    fn test_convert_random_choice_with_rest() {
        // Rest in random choice should fail for f64 patterns
        let ast = parse("c4|~").unwrap();
        let result: Result<Pattern<f64>, _> = convert(&ast);
        assert!(result.is_err(), "Rest in random choice should fail for f64 patterns");
        assert!(matches!(result.unwrap_err(), ConvertError::RestNotSupported(_)));
    }

    #[test]
    fn test_convert_random_choice_with_rest_in_sequence() {
        // Rest in random choice should fail for f64 patterns
        let ast = parse("c2 c3 c4|~ c5").unwrap();
        let result: Result<Pattern<f64>, _> = convert(&ast);
        assert!(result.is_err(), "Rest in random choice should fail for f64 patterns");
        assert!(matches!(result.unwrap_err(), ConvertError::RestNotSupported(_)));
    }
}
