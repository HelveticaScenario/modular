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
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::polymeter => parse_polymeter(inner),
        Rule::fast_sub => parse_fast_sub(inner),
        Rule::slow_sub => parse_slow_sub(inner),
        Rule::group => {
            let inner_expr = inner.into_inner().next().unwrap();
            parse_pattern_expr(inner_expr)
        }
        Rule::modified_atom => parse_modified_atom(inner),
        _ => Err(ParseError {
            message: format!("Unexpected element rule: {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
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

        Rule::value_with_tail => {
            let mut values: Vec<AtomValue> = Vec::new();
            let mut first_span = None;
            let mut last_span = None;

            for value_pair in inner.into_inner() {
                let value_span = value_pair.as_span();
                if first_span.is_none() {
                    first_span = Some(value_span.start());
                }
                last_span = Some(value_span.end());
                values.push(parse_value(value_pair)?);
            }

            let start = first_span.unwrap_or(span.start());
            let end = last_span.unwrap_or(span.end());

            if values.len() == 1 {
                Ok(MiniAST::Pure(Located::new(values.remove(0), start, end)))
            } else {
                Ok(MiniAST::List(Located::new(values, start, end)))
            }
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
            let s = inner.as_str();
            let mut chars = s.chars();
            let letter = chars.next().unwrap_or('c');
            let mut accidental = None;
            let mut octave_str = String::new();

            for c in chars {
                if c == '#' || c == 'b' || c == 's' || c == 'f' {
                    accidental = Some(if c == 's' { '#' } else if c == 'f' { 'b' } else { c });
                } else {
                    octave_str.push(c);
                }
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
    fn test_parse_tail() {
        let ast = parse("c:e:g").unwrap();
        assert!(matches!(ast, MiniAST::List(_)));
        if let MiniAST::List(Located { node: values, .. }) = ast {
            assert_eq!(values.len(), 3);
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
}
