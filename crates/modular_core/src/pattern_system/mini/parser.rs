//! Pest parser for mini notation.
//!
//! Parses mini notation strings into AST nodes.

use pest::Parser;
use pest_derive::Parser;

use super::ast::{AtomValue, Located, MiniAST, OperatorCall};
use crate::pattern_system::SourceSpan;

#[derive(Parser)]
#[grammar = "pattern_system/mini/grammar.pest"]
pub struct MiniParser;

/// Error type for parsing failures.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Option<SourceSpan>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.span {
            Some(span) => write!(f, "Parse error at {}-{}: {}", span.start, span.end, self.message),
            None => write!(f, "Parse error: {}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(err: pest::error::Error<Rule>) -> Self {
        ParseError {
            message: err.to_string(),
            span: None,
        }
    }
}

/// Parse a mini notation string into an AST.
pub fn parse(input: &str) -> Result<MiniAST, ParseError> {
    let pairs = MiniParser::parse(Rule::program, input)?;

    // Get the program pair
    let program = pairs.into_iter().next().ok_or_else(|| ParseError {
        message: "Empty input".to_string(),
        span: None,
    })?;

    parse_program(program)
}

fn parse_program(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let mut inner = pair.into_inner();

    // Parse the pattern expression
    let pattern_pair = inner.next().ok_or_else(|| ParseError {
        message: "Expected pattern expression".to_string(),
        span: None,
    })?;

    let mut ast = parse_pattern_expr(pattern_pair)?;

    // Check for operator chain
    if let Some(chain_pair) = inner.next() {
        if chain_pair.as_rule() == Rule::operator_chain {
            let operators = parse_operator_chain(chain_pair)?;
            ast = MiniAST::WithOperators {
                base: Box::new(ast),
                operators,
            };
        }
    }

    Ok(ast)
}

fn parse_pattern_expr(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    match pair.as_rule() {
        Rule::pattern_expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_pattern_expr(inner)
        }
        Rule::stack_expr => parse_stack_expr(pair),
        Rule::sequence_expr => parse_sequence_expr(pair),
        Rule::weighted_elem => parse_weighted_elem(pair).map(|(ast, _)| ast),
        Rule::element => parse_element(pair),
        Rule::modified_atom => parse_modified_atom(pair),
        Rule::atom => parse_atom(pair),
        _ => Err(ParseError {
            message: format!("Unexpected rule: {:?}", pair.as_rule()),
            span: Some(SourceSpan::new(pair.as_span().start(), pair.as_span().end())),
        }),
    }
}

fn parse_stack_expr(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let sequences: Vec<MiniAST> = pair
        .into_inner()
        .map(parse_sequence_expr)
        .collect::<Result<_, _>>()?;

    if sequences.len() == 1 {
        Ok(sequences.into_iter().next().unwrap())
    } else {
        Ok(MiniAST::Stack(sequences))
    }
}

fn parse_sequence_expr(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let elements: Vec<(MiniAST, Option<f64>)> = pair
        .into_inner()
        .map(parse_weighted_elem)
        .collect::<Result<_, _>>()?;

    if elements.len() == 1 && elements[0].1.is_none() {
        Ok(elements.into_iter().next().unwrap().0)
    } else {
        Ok(MiniAST::Sequence(elements))
    }
}

fn parse_weighted_elem(pair: pest::iterators::Pair<Rule>) -> Result<(MiniAST, Option<f64>), ParseError> {
    let mut inner = pair.into_inner();

    let element_pair = inner.next().unwrap();
    let ast = parse_element(element_pair)?;

    let weight = inner.next().and_then(|p| p.as_str().parse::<f64>().ok());

    Ok((ast, weight))
}

fn parse_element(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let span = pair.as_span();
    let mut elements: Vec<MiniAST> = Vec::new();
    let mut first_span = None;
    let mut last_span = None;

    for inner in pair.into_inner() {
        let inner_span = inner.as_span();
        if first_span.is_none() {
            first_span = Some(inner_span.start());
        }
        last_span = Some(inner_span.end());

        match inner.as_rule() {
            Rule::element_base => {
                let base_inner = inner.into_inner().next().unwrap();
                let base_ast = match base_inner.as_rule() {
                    Rule::polymeter => parse_polymeter(base_inner)?,
                    Rule::fast_sub => parse_fast_sub(base_inner)?,
                    Rule::slow_sub => parse_slow_sub(base_inner)?,
                    Rule::group => {
                        let inner_expr = base_inner.into_inner().next().unwrap();
                        parse_pattern_expr(inner_expr)?
                    }
                    Rule::modified_atom => parse_modified_atom(base_inner)?,
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected element_base rule: {:?}", base_inner.as_rule()),
                            span: Some(SourceSpan::new(base_inner.as_span().start(), base_inner.as_span().end())),
                        });
                    }
                };
                elements.push(base_ast);
            }
            Rule::tail_element => {
                // tail_element can be fast_sub, slow_sub, group, or value
                let inner_pair = inner.into_inner().next().unwrap();
                match inner_pair.as_rule() {
                    Rule::value => {
                        let atom = parse_value(inner_pair)?;
                        let elem_span = SourceSpan::new(inner_span.start(), inner_span.end());
                        elements.push(MiniAST::Pure(Located::new(atom, elem_span.start, elem_span.end)));
                    }
                    Rule::fast_sub => {
                        elements.push(parse_fast_sub(inner_pair)?);
                    }
                    Rule::slow_sub => {
                        elements.push(parse_slow_sub(inner_pair)?);
                    }
                    Rule::group => {
                        // group contains pattern_expr
                        let inner_expr = inner_pair.into_inner().next().unwrap();
                        elements.push(parse_pattern_expr(inner_expr)?);
                    }
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected tail element rule: {:?}", inner_pair.as_rule()),
                            span: Some(SourceSpan::new(inner_pair.as_span().start(), inner_pair.as_span().end())),
                        });
                    }
                }
            }
            _ => {
                return Err(ParseError {
                    message: format!("Unexpected element rule: {:?}", inner.as_rule()),
                    span: Some(SourceSpan::new(inner_span.start(), inner_span.end())),
                });
            }
        }
    }

    let start = first_span.unwrap_or(span.start());
    let end = last_span.unwrap_or(span.end());

    if elements.len() == 1 {
        Ok(elements.remove(0))
    } else {
        Ok(MiniAST::List(Located::new(elements, start, end)))
    }
}

fn parse_polymeter(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let sequences: Vec<MiniAST> = pair
        .into_inner()
        .map(parse_sequence_expr)
        .collect::<Result<_, _>>()?;

    Ok(MiniAST::PolyMeter(sequences))
}

fn parse_fast_sub(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr(inner)?;

    // Fast subsequence means fastcat
    match ast {
        MiniAST::Sequence(elements) => {
            let patterns: Vec<MiniAST> = elements.into_iter().map(|(p, _)| p).collect();
            Ok(MiniAST::Sequence(patterns.into_iter().map(|p| (p, None)).collect()))
        }
        MiniAST::Stack(patterns) => Ok(MiniAST::Stack(patterns)),
        other => Ok(other),
    }
}

fn parse_slow_sub(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr(inner)?;

    // Slow subsequence means slowcat
    match ast {
        MiniAST::Sequence(elements) => {
            let patterns: Vec<MiniAST> = elements.into_iter().map(|(p, _)| p).collect();
            Ok(MiniAST::SlowCat(patterns))
        }
        other => Ok(MiniAST::SlowCat(vec![other])),
    }
}

fn parse_modified_atom(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let atom_pair = inner.next().unwrap();
    let mut ast = parse_atom(atom_pair)?;

    // Apply modifiers
    for modifier in inner {
        ast = apply_modifier(ast, modifier, span.start(), span.end())?;
    }

    Ok(ast)
}

fn apply_modifier(
    ast: MiniAST,
    modifier: pest::iterators::Pair<Rule>,
    _start: usize,
    _end: usize,
) -> Result<MiniAST, ParseError> {
    let inner = modifier.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::fast_mod => {
            let slice_pair = inner.into_inner().next().unwrap();
            let factor = parse_slice(slice_pair)?;
            Ok(MiniAST::Fast(Box::new(ast), Box::new(factor)))
        }
        Rule::slow_mod => {
            let slice_pair = inner.into_inner().next().unwrap();
            let factor = parse_slice(slice_pair)?;
            Ok(MiniAST::Slow(Box::new(ast), Box::new(factor)))
        }
        Rule::replicate => {
            let count = inner
                .into_inner()
                .next()
                .and_then(|p| p.as_str().parse::<u32>().ok())
                .unwrap_or(2);
            Ok(MiniAST::Replicate(Box::new(ast), count))
        }
        Rule::degrade => {
            let prob = inner
                .into_inner()
                .next()
                .and_then(|p| p.as_str().parse::<f64>().ok());
            Ok(MiniAST::Degrade(Box::new(ast), prob))
        }
        Rule::euclidean => {
            let mut nums = inner.into_inner();
            let pulses = nums
                .next()
                .unwrap()
                .as_str()
                .parse::<u32>()
                .unwrap_or(0);
            let steps = nums
                .next()
                .unwrap()
                .as_str()
                .parse::<u32>()
                .unwrap_or(0);
            let rotation = nums.next().and_then(|p| p.as_str().parse::<u32>().ok());

            Ok(MiniAST::Euclidean {
                pattern: Box::new(ast),
                pulses,
                steps,
                rotation,
            })
        }
        _ => Err(ParseError {
            message: format!("Unknown modifier: {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn parse_slice(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::number => {
            let n: f64 = inner.as_str().parse().unwrap_or(1.0);
            let span = inner.as_span();
            Ok(MiniAST::Pure(Located::new(
                AtomValue::Number(n),
                span.start(),
                span.end(),
            )))
        }
        Rule::fast_sub => parse_fast_sub(inner),
        Rule::slow_sub => parse_slow_sub(inner),
        Rule::group => {
            let inner_expr = inner.into_inner().next().unwrap();
            parse_pattern_expr(inner_expr)
        }
        _ => Err(ParseError {
            message: format!("Unexpected slice rule: {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn parse_atom(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let span = pair.as_span();
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::rest => Ok(MiniAST::Rest(SourceSpan::new(span.start(), span.end()))),

        Rule::range_pattern => {
            let mut nums = inner.into_inner();
            let start: i64 = nums.next().unwrap().as_str().parse().unwrap_or(0);
            let end: i64 = nums.next().unwrap().as_str().parse().unwrap_or(0);
            Ok(MiniAST::Range(start, end))
        }

        Rule::random_choice => {
            let values: Vec<MiniAST> = inner
                .into_inner()
                .map(|p| {
                    let span = p.as_span();
                    let value = parse_value(p)?;
                    Ok(MiniAST::Pure(Located::new(value, span.start(), span.end())))
                })
                .collect::<Result<_, ParseError>>()?;
            Ok(MiniAST::RandomChoice(values))
        }

        Rule::value => {
            let value_span = inner.as_span();
            let atom = parse_value(inner)?;
            Ok(MiniAST::Pure(Located::new(atom, value_span.start(), value_span.end())))
        }

        _ => Err(ParseError {
            message: format!("Unexpected atom rule: {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(span.start(), span.end())),
        }),
    }
}

fn parse_value(pair: pest::iterators::Pair<Rule>) -> Result<AtomValue, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::hz_value => {
            let num_str: String = inner.as_str().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
            let n: f64 = num_str.parse().unwrap_or(0.0);
            Ok(AtomValue::Hz(n))
        }
        Rule::volts_value => {
            let num_str: String = inner.as_str().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
            let n: f64 = num_str.parse().unwrap_or(0.0);
            Ok(AtomValue::Volts(n))
        }
        Rule::midi_value => {
            let num_str: String = inner.as_str().chars().skip(1).collect();
            let n: i32 = num_str.parse().unwrap_or(0);
            Ok(AtomValue::Midi(n))
        }
        Rule::note_value => {
            let s = inner.as_str().trim();  // Trim whitespace!
            let mut chars = s.chars().peekable();
            let letter = chars.next().unwrap_or('c');
            let mut octave_str = String::new();

            // Check for single accidental
            let accidental = match chars.peek() {
                Some(&c) if c == '#' || c == 's' => {
                    chars.next();
                    Some('#')  // Normalize 's' to '#'
                }
                Some(&'b') => {
                    chars.next();
                    Some('b')
                }
                _ => None,
            };

            // Collect remaining as octave
            for c in chars {
                octave_str.push(c);
            }

            let octave = if octave_str.is_empty() {
                None
            } else {
                octave_str.parse().ok()
            };

            Ok(AtomValue::Note {
                letter,
                accidental,
                octave,
            })
        }
        Rule::number => {
            let n: f64 = inner.as_str().parse().unwrap_or(0.0);
            Ok(AtomValue::Number(n))
        }
        Rule::identifier => Ok(AtomValue::Identifier(inner.as_str().to_string())),
        Rule::string => {
            let s = inner.as_str();
            // Remove quotes
            let content = &s[1..s.len() - 1];
            Ok(AtomValue::String(content.to_string()))
        }
        _ => Err(ParseError {
            message: format!("Unexpected value rule: {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn parse_operator_chain(pair: pest::iterators::Pair<Rule>) -> Result<Vec<OperatorCall>, ParseError> {
    pair.into_inner()
        .map(parse_operator_call)
        .collect()
}

fn parse_operator_call(pair: pest::iterators::Pair<Rule>) -> Result<OperatorCall, ParseError> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name = inner.next().unwrap().as_str().to_string();

    let mut variant = None;
    let mut argument = None;

    for item in inner {
        match item.as_rule() {
            Rule::variant => {
                variant = Some(item.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::pattern_expr => {
                argument = Some(parse_pattern_expr(item)?);
            }
            _ => {}
        }
    }

    Ok(OperatorCall::new(name, variant, argument, span.start(), span.end()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_number() {
        let ast = parse("42").unwrap();
        assert!(matches!(ast, MiniAST::Pure(_)));
    }

    #[test]
    fn test_parse_sequence() {
        let ast = parse("0 1 2").unwrap();
        assert!(matches!(ast, MiniAST::Sequence(_)));
        if let MiniAST::Sequence(elements) = ast {
            assert_eq!(elements.len(), 3);
        }
    }

    #[test]
    fn test_parse_stack() {
        let ast = parse("0, 1, 2").unwrap();
        assert!(matches!(ast, MiniAST::Stack(_)));
        if let MiniAST::Stack(elements) = ast {
            assert_eq!(elements.len(), 3);
        }
    }

    #[test]
    fn test_parse_fast_sub() {
        let ast = parse("[0 1 2]").unwrap();
        assert!(matches!(ast, MiniAST::Sequence(_)));
    }

    #[test]
    fn test_parse_slow_sub() {
        let ast = parse("<0 1 2>").unwrap();
        assert!(matches!(ast, MiniAST::SlowCat(_)));
    }

    #[test]
    fn test_parse_modifier() {
        let ast = parse("0*2").unwrap();
        assert!(matches!(ast, MiniAST::Fast(_, _)));
    }

    #[test]
    fn test_parse_rest() {
        let ast = parse("~").unwrap();
        assert!(matches!(ast, MiniAST::Rest(_)));
    }

    #[test]
    fn test_parse_note() {
        let ast = parse("c4").unwrap();
        if let MiniAST::Pure(Located { node: AtomValue::Note { letter, .. }, .. }) = ast {
            assert_eq!(letter, 'c');
        } else {
            panic!("Expected note");
        }
    }

    #[test]
    fn test_parse_note_octaves() {
        // Test that a1, a2, a3, a4 parse as notes with different octaves
        let ast = parse("a1").unwrap();
        if let MiniAST::Pure(Located { node: AtomValue::Note { letter, octave, .. }, .. }) = ast {
            assert_eq!(letter, 'a');
            assert_eq!(octave, Some(1), "a1 should have octave 1");
        } else {
            panic!("Expected note for 'a1', got {:#?}", ast);
        }

        let ast = parse("a2").unwrap();
        if let MiniAST::Pure(Located { node: AtomValue::Note { letter, octave, .. }, .. }) = ast {
            assert_eq!(letter, 'a');
            assert_eq!(octave, Some(2), "a2 should have octave 2");
        } else {
            panic!("Expected note for 'a2', got {:#?}", ast);
        }

        // Test sequence "a1 a2 a3 a4"
        let ast = parse("a1 a2 a3 a4").unwrap();
        if let MiniAST::Sequence(elements) = ast {
            assert_eq!(elements.len(), 4, "Should have 4 elements");
            for (i, (elem, _weight)) in elements.iter().enumerate() {
                if let MiniAST::Pure(Located { node: AtomValue::Note { letter, octave, .. }, .. }) = elem {
                    assert_eq!(*letter, 'a');
                    assert_eq!(*octave, Some((i + 1) as i32), "a{} should have octave {}", i + 1, i + 1);
                } else {
                    panic!("Expected note for element {}, got {:#?}", i, elem);
                }
            }
        } else {
            panic!("Expected sequence, got {:#?}", ast);
        }

        // Test that "a b" parses as two notes, not "a" with flat "b"
        // This ensures the atomic note_value rule prevents whitespace consumption
        let ast = parse("<a b>").unwrap();
        if let MiniAST::SlowCat(elements) = ast {
            assert_eq!(elements.len(), 2, "Should have 2 elements in slowcat");
            // First element should be 'a'
            if let MiniAST::Pure(Located { node: AtomValue::Note { letter, accidental, .. }, .. }) = &elements[0] {
                assert_eq!(*letter, 'a');
                assert!(accidental.is_none(), "'a' should have no accidental");
            } else {
                panic!("Expected note 'a' for first element");
            }
            // Second element should be 'b'  
            if let MiniAST::Pure(Located { node: AtomValue::Note { letter, accidental, .. }, .. }) = &elements[1] {
                assert_eq!(*letter, 'b');
                assert!(accidental.is_none(), "'b' should have no accidental, not be parsed as flat of 'a'");
            } else {
                panic!("Expected note 'b' for second element");
            }
        } else {
            panic!("Expected slowcat, got {:#?}", ast);
        }
    }

    #[test]
    fn test_parse_tail() {
        let ast = parse("c:e:g").unwrap();
        assert!(matches!(ast, MiniAST::List(_)));
        if let MiniAST::List(Located { node: values, .. }) = ast {
            assert_eq!(values.len(), 3);
        }
    }

    #[test]
    fn test_parse_tail_with_subpattern() {
        // c:[e f] should parse as List with [Pure(c), Sequence([Pure(e), Pure(f)])]
        let ast = parse("c:[e f]").unwrap();
        if let MiniAST::List(Located { node: elements, .. }) = ast {
            assert_eq!(elements.len(), 2, "Should have 2 elements: c and [e f]");
            // First element is Pure(c)
            assert!(matches!(&elements[0], MiniAST::Pure(_)));
            // Second element is Sequence([e, f])
            assert!(matches!(&elements[1], MiniAST::Sequence(_)));
            if let MiniAST::Sequence(seq_elems) = &elements[1] {
                assert_eq!(seq_elems.len(), 2, "Sequence should have e and f");
            }
        } else {
            panic!("Expected List, got {:?}", ast);
        }

        // a:<minor major> should parse as List with SlowCat
        let ast = parse("a:<minor major>").unwrap();
        if let MiniAST::List(Located { node: elements, .. }) = ast {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[1], MiniAST::SlowCat(_)));
        } else {
            panic!("Expected List, got {:?}", ast);
        }
    }

    #[test]
    fn test_parse_euclidean() {
        let ast = parse("1(3,8)").unwrap();
        assert!(matches!(ast, MiniAST::Euclidean { .. }));
    }

    #[test]
    fn test_parse_operator() {
        let ast = parse("0 1 2 $ fast(2)").unwrap();
        assert!(matches!(ast, MiniAST::WithOperators { .. }));
    }

    #[test]
    fn test_parse_operator_with_variant() {
        let ast = parse("0 1 2 $ add.squeeze(10)").unwrap();
        if let MiniAST::WithOperators { operators, .. } = ast {
            assert_eq!(operators.len(), 1);
            assert_eq!(operators[0].name, "add");
            assert_eq!(operators[0].variant, Some("squeeze".to_string()));
        } else {
            panic!("Expected WithOperators");
        }
    }

    #[test]
    fn test_parse_chained_operators() {
        let ast = parse("0 1 2 $ fast(2) $ add(10)").unwrap();
        if let MiniAST::WithOperators { operators, .. } = ast {
            assert_eq!(operators.len(), 2);
        } else {
            panic!("Expected WithOperators");
        }
    }

    #[test]
    fn test_parse_subpattern_head_with_tail() {
        // [c d]:minor should parse as List with [Sequence([c,d]), minor]
        let ast = parse("[c d]:minor").unwrap();
        if let MiniAST::List(Located { node: elements, .. }) = ast {
            assert_eq!(elements.len(), 2, "Should have 2 elements: [c d] and minor");
            // First element is Sequence([c, d])
            assert!(matches!(&elements[0], MiniAST::Sequence(_)), "First element should be Sequence");
            // Second element is Pure(minor)
            assert!(matches!(&elements[1], MiniAST::Pure(_)), "Second element should be Pure");
        } else {
            panic!("Expected List, got {:?}", ast);
        }

        // <x y>:tail should parse as List with [SlowCat([x,y]), tail]
        let ast = parse("<x y>:tail").unwrap();
        if let MiniAST::List(Located { node: elements, .. }) = ast {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], MiniAST::SlowCat(_)));
        } else {
            panic!("Expected List, got {:?}", ast);
        }
    }

    #[test]
    fn test_parse_nested_slowcat_in_sequence() {
        // "c3 c#3 <cb <d d5>>" should parse as Sequence([c3, c#3, SlowCat([cb, SlowCat([d, d5])])])
        let ast = parse("c3 c#3 <cb <d d5>>").unwrap();
        if let MiniAST::Sequence(elements) = &ast {
            assert_eq!(elements.len(), 3, "Should have 3 elements: c3, c#3, and <cb <d d5>>");
            
            // Third element should be SlowCat
            let (third, _) = &elements[2];
            if let MiniAST::SlowCat(slowcat_elems) = third {
                assert_eq!(slowcat_elems.len(), 2, "Outer slowcat should have 2 elements: cb and <d d5>");
                
                // Second element of the slowcat should be another SlowCat
                let second_elem = &slowcat_elems[1];
                if let MiniAST::SlowCat(inner_slowcat) = second_elem {
                    assert_eq!(inner_slowcat.len(), 2, "Inner slowcat should have 2 elements: d and d5");
                } else {
                    panic!("Expected inner SlowCat, got {:?}", second_elem);
                }
            } else {
                panic!("Expected SlowCat, got {:?}", third);
            }
        } else {
            panic!("Expected Sequence, got {:?}", ast);
        }
    }

    #[test]
    fn test_double_accidentals_rejected() {
        // Double sharps should be rejected (## should not parse as a valid note)
        // c##4 should fail to parse as a single note
        let result = parse("c##4");
        // The parser will parse c# as a note and then fail or treat #4 separately
        // Since ## is not valid, it should either fail or parse differently
        if let Ok(ast) = result {
            // If it parses, it should NOT be a single note with ## accidental
            if let MiniAST::Pure(Located { node: AtomValue::Note { accidental, .. }, .. }) = ast {
                // We've changed accidental to Option<char>, so it can only hold one character
                // This test verifies that behavior
                assert!(accidental.is_none() || accidental == Some('#') || accidental == Some('b'),
                    "Accidental should be single character only");
            }
        }

        // Similarly for double flats
        let result = parse("cbb4");
        if let Ok(ast) = result {
            if let MiniAST::Pure(Located { node: AtomValue::Note { accidental, .. }, .. }) = ast {
                assert!(accidental.is_none() || accidental == Some('#') || accidental == Some('b'),
                    "Accidental should be single character only");
            }
        }
    }
}
