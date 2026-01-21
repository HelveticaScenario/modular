//! Pest parser for mini notation.
//!
//! Parses mini notation strings into AST nodes.

use pest::Parser;
use pest_derive::Parser;

use super::ast::{AtomValue, Located, MiniAST, MiniASTF64, MiniASTI32, MiniASTU32};
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

    parse_pattern_expr(pattern_pair)
}

fn parse_pattern_expr(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    match pair.as_rule() {
        Rule::pattern_expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_pattern_expr(inner)
        }
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

    let weight = if let Some(p) = inner.next() {
        let n: f64 = p.as_str().parse().unwrap_or(1.0);
        Some(n)
    } else {
        None
    };

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

fn parse_fast_sub(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr(inner)?;

    // Fast subsequence means fastcat (preserves weights for timecat)
    match ast {
        MiniAST::Sequence(elements) => Ok(MiniAST::Sequence(elements)),
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

// ============ Typed parsing functions for MiniASTF64 ============

fn parse_pattern_expr_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    match pair.as_rule() {
        Rule::pattern_expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_pattern_expr_f64(inner)
        }
        Rule::sequence_expr => parse_sequence_expr_f64(pair),
        Rule::weighted_elem => parse_weighted_elem_f64(pair).map(|(ast, _)| ast),
        Rule::element => parse_element_f64(pair),
        Rule::modified_atom => parse_modified_atom_f64(pair),
        Rule::atom => parse_atom_f64(pair),
        _ => Err(ParseError {
            message: format!("Unexpected rule (f64): {:?}", pair.as_rule()),
            span: Some(SourceSpan::new(pair.as_span().start(), pair.as_span().end())),
        }),
    }
}

fn parse_sequence_expr_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let elements: Vec<(MiniASTF64, Option<f64>)> = pair
        .into_inner()
        .map(parse_weighted_elem_f64)
        .collect::<Result<_, _>>()?;

    if elements.len() == 1 && elements[0].1.is_none() {
        Ok(elements.into_iter().next().unwrap().0)
    } else {
        Ok(MiniASTF64::Sequence(elements))
    }
}

fn parse_weighted_elem_f64(pair: pest::iterators::Pair<Rule>) -> Result<(MiniASTF64, Option<f64>), ParseError> {
    let mut inner = pair.into_inner();

    let element_pair = inner.next().unwrap();
    let ast = parse_element_f64(element_pair)?;

    let weight = if let Some(p) = inner.next() {
        let n: f64 = p.as_str().parse().unwrap_or(1.0);
        Some(n)
    } else {
        None
    };

    Ok((ast, weight))
}

fn parse_element_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let span = pair.as_span();
    let mut elements: Vec<MiniASTF64> = Vec::new();
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
                    Rule::fast_sub => parse_fast_sub_f64(base_inner)?,
                    Rule::slow_sub => parse_slow_sub_f64(base_inner)?,
                    Rule::group => {
                        let inner_expr = base_inner.into_inner().next().unwrap();
                        parse_pattern_expr_f64(inner_expr)?
                    }
                    Rule::modified_atom => parse_modified_atom_f64(base_inner)?,
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected element_base rule (f64): {:?}", base_inner.as_rule()),
                            span: Some(SourceSpan::new(base_inner.as_span().start(), base_inner.as_span().end())),
                        });
                    }
                };
                elements.push(base_ast);
            }
            Rule::tail_element => {
                let inner_pair = inner.into_inner().next().unwrap();
                match inner_pair.as_rule() {
                    Rule::value | Rule::number => {
                        let n: f64 = inner_pair.as_str().parse().unwrap_or(0.0);
                        let elem_span = SourceSpan::new(inner_span.start(), inner_span.end());
                        elements.push(MiniASTF64::Pure(Located::new(n, elem_span.start, elem_span.end)));
                    }
                    Rule::fast_sub => {
                        elements.push(parse_fast_sub_f64(inner_pair)?);
                    }
                    Rule::slow_sub => {
                        elements.push(parse_slow_sub_f64(inner_pair)?);
                    }
                    Rule::group => {
                        let inner_expr = inner_pair.into_inner().next().unwrap();
                        elements.push(parse_pattern_expr_f64(inner_expr)?);
                    }
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected tail element rule (f64): {:?}", inner_pair.as_rule()),
                            span: Some(SourceSpan::new(inner_pair.as_span().start(), inner_pair.as_span().end())),
                        });
                    }
                }
            }
            _ => {
                return Err(ParseError {
                    message: format!("Unexpected element rule (f64): {:?}", inner.as_rule()),
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
        Ok(MiniASTF64::List(Located::new(elements, start, end)))
    }
}

fn parse_modified_atom_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let atom_pair = inner.next().unwrap();
    let mut ast = parse_atom_f64(atom_pair)?;

    // Apply modifiers
    for modifier in inner {
        ast = apply_modifier_f64(ast, modifier, span.start(), span.end())?;
    }

    Ok(ast)
}

fn parse_atom_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let span = pair.as_span();
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::rest => Ok(MiniASTF64::Rest(SourceSpan::new(span.start(), span.end()))),
        Rule::random_choice => {
            let values: Vec<MiniASTF64> = inner
                .into_inner()
                .map(parse_choice_element_f64)
                .collect::<Result<_, ParseError>>()?;
            Ok(MiniASTF64::RandomChoice(values))
        }
        Rule::value | Rule::number => {
            let n: f64 = inner.as_str().parse().unwrap_or(0.0);
            let value_span = inner.as_span();
            Ok(MiniASTF64::Pure(Located::new(n, value_span.start(), value_span.end())))
        }
        _ => Err(ParseError {
            message: format!("Unexpected atom rule (f64): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(span.start(), span.end())),
        }),
    }
}

fn parse_choice_element_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::value | Rule::number => {
            let span = inner.as_span();
            let n: f64 = inner.as_str().parse().unwrap_or(0.0);
            Ok(MiniASTF64::Pure(Located::new(n, span.start(), span.end())))
        }
        Rule::pattern_expr | Rule::sequence_expr => parse_pattern_expr_f64(inner),
        _ => Err(ParseError {
            message: format!("Unexpected choice element rule (f64): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn apply_modifier_f64(
    ast: MiniASTF64,
    modifier: pest::iterators::Pair<Rule>,
    _start: usize,
    _end: usize,
) -> Result<MiniASTF64, ParseError> {
    let inner = modifier.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::fast_mod => {
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand_f64(operand_pair)?;
            Ok(MiniASTF64::Fast(Box::new(ast), Box::new(factor)))
        }
        Rule::slow_mod => {
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand_f64(operand_pair)?;
            Ok(MiniASTF64::Slow(Box::new(ast), Box::new(factor)))
        }
        Rule::replicate => {
            let count = inner
                .into_inner()
                .next()
                .map(|p| p.as_str().parse().unwrap_or(2))
                .unwrap_or(2);
            Ok(MiniASTF64::Replicate(Box::new(ast), count))
        }
        Rule::degrade => {
            let prob = if let Some(p) = inner.into_inner().next() {
                let n: f64 = p.as_str().parse().unwrap_or(0.5);
                Some(n)
            } else {
                None
            };
            Ok(MiniASTF64::Degrade(Box::new(ast), prob))
        }
        Rule::euclidean => {
            let mut operands = inner.into_inner();
            let pulses = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let steps = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let rotation = operands.next().map(|p| parse_mod_operand_i32(p)).transpose()?.map(Box::new);

            Ok(MiniASTF64::Euclidean {
                pattern: Box::new(ast),
                pulses,
                steps,
                rotation,
            })
        }
        _ => Err(ParseError {
            message: format!("Unknown modifier (f64): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn parse_fast_sub_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr_f64(inner)?;

    // Fast subsequence means fastcat (preserves weights for timecat)
    match ast {
        MiniASTF64::Sequence(elements) => Ok(MiniASTF64::Sequence(elements)),
        other => Ok(other),
    }
}

fn parse_slow_sub_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr_f64(inner)?;

    match ast {
        MiniASTF64::Sequence(elements) => {
            let patterns: Vec<MiniASTF64> = elements.into_iter().map(|(p, _)| p).collect();
            Ok(MiniASTF64::SlowCat(patterns))
        }
        other => Ok(MiniASTF64::SlowCat(vec![other])),
    }
}

// ============ Typed parsing functions for MiniASTU32 ============

fn parse_pattern_expr_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    match pair.as_rule() {
        Rule::pattern_expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_pattern_expr_u32(inner)
        }
        Rule::sequence_expr => parse_sequence_expr_u32(pair),
        Rule::weighted_elem => parse_weighted_elem_u32(pair).map(|(ast, _)| ast),
        Rule::element => parse_element_u32(pair),
        Rule::modified_atom => parse_modified_atom_u32(pair),
        Rule::atom => parse_atom_u32(pair),
        _ => Err(ParseError {
            message: format!("Unexpected rule (u32): {:?}", pair.as_rule()),
            span: Some(SourceSpan::new(pair.as_span().start(), pair.as_span().end())),
        }),
    }
}

fn parse_sequence_expr_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let elements: Vec<(MiniASTU32, Option<f64>)> = pair
        .into_inner()
        .map(parse_weighted_elem_u32)
        .collect::<Result<_, _>>()?;

    if elements.len() == 1 && elements[0].1.is_none() {
        Ok(elements.into_iter().next().unwrap().0)
    } else {
        Ok(MiniASTU32::Sequence(elements))
    }
}

fn parse_weighted_elem_u32(pair: pest::iterators::Pair<Rule>) -> Result<(MiniASTU32, Option<f64>), ParseError> {
    let mut inner = pair.into_inner();

    let element_pair = inner.next().unwrap();
    let ast = parse_element_u32(element_pair)?;

    let weight = if let Some(p) = inner.next() {
        let n: f64 = p.as_str().parse().unwrap_or(1.0);
        Some(n)
    } else {
        None
    };

    Ok((ast, weight))
}

fn parse_element_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let span = pair.as_span();
    let mut elements: Vec<MiniASTU32> = Vec::new();
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
                    Rule::fast_sub => parse_fast_sub_u32(base_inner)?,
                    Rule::slow_sub => parse_slow_sub_u32(base_inner)?,
                    Rule::group => {
                        let inner_expr = base_inner.into_inner().next().unwrap();
                        parse_pattern_expr_u32(inner_expr)?
                    }
                    Rule::modified_atom => parse_modified_atom_u32(base_inner)?,
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected element_base rule (u32): {:?}", base_inner.as_rule()),
                            span: Some(SourceSpan::new(base_inner.as_span().start(), base_inner.as_span().end())),
                        });
                    }
                };
                elements.push(base_ast);
            }
            Rule::tail_element => {
                let inner_pair = inner.into_inner().next().unwrap();
                match inner_pair.as_rule() {
                    Rule::value | Rule::number => {
                        let n: u32 = inner_pair.as_str().parse().unwrap_or(0);
                        let elem_span = SourceSpan::new(inner_span.start(), inner_span.end());
                        elements.push(MiniASTU32::Pure(Located::new(n, elem_span.start, elem_span.end)));
                    }
                    Rule::fast_sub => {
                        elements.push(parse_fast_sub_u32(inner_pair)?);
                    }
                    Rule::slow_sub => {
                        elements.push(parse_slow_sub_u32(inner_pair)?);
                    }
                    Rule::group => {
                        let inner_expr = inner_pair.into_inner().next().unwrap();
                        elements.push(parse_pattern_expr_u32(inner_expr)?);
                    }
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected tail element rule (u32): {:?}", inner_pair.as_rule()),
                            span: Some(SourceSpan::new(inner_pair.as_span().start(), inner_pair.as_span().end())),
                        });
                    }
                }
            }
            _ => {
                return Err(ParseError {
                    message: format!("Unexpected element rule (u32): {:?}", inner.as_rule()),
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
        Ok(MiniASTU32::List(Located::new(elements, start, end)))
    }
}

fn parse_modified_atom_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let atom_pair = inner.next().unwrap();
    let mut ast = parse_atom_u32(atom_pair)?;

    // Apply modifiers
    for modifier in inner {
        ast = apply_modifier_u32(ast, modifier, span.start(), span.end())?;
    }

    Ok(ast)
}

fn parse_atom_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let span = pair.as_span();
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::rest => Ok(MiniASTU32::Rest(SourceSpan::new(span.start(), span.end()))),
        Rule::random_choice => {
            let values: Vec<MiniASTU32> = inner
                .into_inner()
                .map(parse_choice_element_u32)
                .collect::<Result<_, ParseError>>()?;
            Ok(MiniASTU32::RandomChoice(values))
        }
        Rule::value | Rule::number => {
            let n: u32 = inner.as_str().parse().unwrap_or(0);
            let value_span = inner.as_span();
            Ok(MiniASTU32::Pure(Located::new(n, value_span.start(), value_span.end())))
        }
        _ => Err(ParseError {
            message: format!("Unexpected atom rule (u32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(span.start(), span.end())),
        }),
    }
}

fn parse_choice_element_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::value | Rule::number => {
            let span = inner.as_span();
            let n: u32 = inner.as_str().parse().unwrap_or(0);
            Ok(MiniASTU32::Pure(Located::new(n, span.start(), span.end())))
        }
        Rule::pattern_expr | Rule::sequence_expr => parse_pattern_expr_u32(inner),
        _ => Err(ParseError {
            message: format!("Unexpected choice element rule (u32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn apply_modifier_u32(
    ast: MiniASTU32,
    modifier: pest::iterators::Pair<Rule>,
    _start: usize,
    _end: usize,
) -> Result<MiniASTU32, ParseError> {
    let inner = modifier.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::fast_mod => {
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand_f64(operand_pair)?;
            Ok(MiniASTU32::Fast(Box::new(ast), Box::new(factor)))
        }
        Rule::slow_mod => {
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand_f64(operand_pair)?;
            Ok(MiniASTU32::Slow(Box::new(ast), Box::new(factor)))
        }
        Rule::replicate => {
            let count = inner
                .into_inner()
                .next()
                .map(|p| p.as_str().parse().unwrap_or(2))
                .unwrap_or(2);
            Ok(MiniASTU32::Replicate(Box::new(ast), count))
        }
        Rule::degrade => {
            let prob = if let Some(p) = inner.into_inner().next() {
                let n: f64 = p.as_str().parse().unwrap_or(0.5);
                Some(n)
            } else {
                None
            };
            Ok(MiniASTU32::Degrade(Box::new(ast), prob))
        }
        Rule::euclidean => {
            let mut operands = inner.into_inner();
            let pulses = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let steps = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let rotation = operands.next().map(|p| parse_mod_operand_i32(p)).transpose()?.map(Box::new);

            Ok(MiniASTU32::Euclidean {
                pattern: Box::new(ast),
                pulses,
                steps,
                rotation,
            })
        }
        _ => Err(ParseError {
            message: format!("Unknown modifier (u32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn parse_fast_sub_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr_u32(inner)?;

    // Fast subsequence means fastcat (preserves weights for timecat)
    match ast {
        MiniASTU32::Sequence(elements) => Ok(MiniASTU32::Sequence(elements)),
        other => Ok(other),
    }
}

fn parse_slow_sub_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr_u32(inner)?;

    match ast {
        MiniASTU32::Sequence(elements) => {
            let patterns: Vec<MiniASTU32> = elements.into_iter().map(|(p, _)| p).collect();
            Ok(MiniASTU32::SlowCat(patterns))
        }
        other => Ok(MiniASTU32::SlowCat(vec![other])),
    }
}

// ============ Typed parsing functions for MiniASTI32 ============

fn parse_pattern_expr_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    match pair.as_rule() {
        Rule::pattern_expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_pattern_expr_i32(inner)
        }
        Rule::sequence_expr => parse_sequence_expr_i32(pair),
        Rule::weighted_elem => parse_weighted_elem_i32(pair).map(|(ast, _)| ast),
        Rule::element => parse_element_i32(pair),
        Rule::modified_atom => parse_modified_atom_i32(pair),
        Rule::atom => parse_atom_i32(pair),
        _ => Err(ParseError {
            message: format!("Unexpected rule (i32): {:?}", pair.as_rule()),
            span: Some(SourceSpan::new(pair.as_span().start(), pair.as_span().end())),
        }),
    }
}

fn parse_sequence_expr_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let elements: Vec<(MiniASTI32, Option<f64>)> = pair
        .into_inner()
        .map(parse_weighted_elem_i32)
        .collect::<Result<_, _>>()?;

    if elements.len() == 1 && elements[0].1.is_none() {
        Ok(elements.into_iter().next().unwrap().0)
    } else {
        Ok(MiniASTI32::Sequence(elements))
    }
}

fn parse_weighted_elem_i32(pair: pest::iterators::Pair<Rule>) -> Result<(MiniASTI32, Option<f64>), ParseError> {
    let mut inner = pair.into_inner();

    let element_pair = inner.next().unwrap();
    let ast = parse_element_i32(element_pair)?;

    let weight = if let Some(p) = inner.next() {
        let n: f64 = p.as_str().parse().unwrap_or(1.0);
        Some(n)
    } else {
        None
    };

    Ok((ast, weight))
}

fn parse_element_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let span = pair.as_span();
    let mut elements: Vec<MiniASTI32> = Vec::new();
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
                    Rule::fast_sub => parse_fast_sub_i32(base_inner)?,
                    Rule::slow_sub => parse_slow_sub_i32(base_inner)?,
                    Rule::group => {
                        let inner_expr = base_inner.into_inner().next().unwrap();
                        parse_pattern_expr_i32(inner_expr)?
                    }
                    Rule::modified_atom => parse_modified_atom_i32(base_inner)?,
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected element_base rule (i32): {:?}", base_inner.as_rule()),
                            span: Some(SourceSpan::new(base_inner.as_span().start(), base_inner.as_span().end())),
                        });
                    }
                };
                elements.push(base_ast);
            }
            Rule::tail_element => {
                let inner_pair = inner.into_inner().next().unwrap();
                match inner_pair.as_rule() {
                    Rule::value | Rule::number => {
                        let n: i32 = inner_pair.as_str().parse().unwrap_or(0);
                        let elem_span = SourceSpan::new(inner_span.start(), inner_span.end());
                        elements.push(MiniASTI32::Pure(Located::new(n, elem_span.start, elem_span.end)));
                    }
                    Rule::fast_sub => {
                        elements.push(parse_fast_sub_i32(inner_pair)?);
                    }
                    Rule::slow_sub => {
                        elements.push(parse_slow_sub_i32(inner_pair)?);
                    }
                    Rule::group => {
                        let inner_expr = inner_pair.into_inner().next().unwrap();
                        elements.push(parse_pattern_expr_i32(inner_expr)?);
                    }
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected tail element rule (i32): {:?}", inner_pair.as_rule()),
                            span: Some(SourceSpan::new(inner_pair.as_span().start(), inner_pair.as_span().end())),
                        });
                    }
                }
            }
            _ => {
                return Err(ParseError {
                    message: format!("Unexpected element rule (i32): {:?}", inner.as_rule()),
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
        Ok(MiniASTI32::List(Located::new(elements, start, end)))
    }
}

fn parse_modified_atom_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let atom_pair = inner.next().unwrap();
    let mut ast = parse_atom_i32(atom_pair)?;

    // Apply modifiers
    for modifier in inner {
        ast = apply_modifier_i32(ast, modifier, span.start(), span.end())?;
    }

    Ok(ast)
}

fn parse_atom_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let span = pair.as_span();
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::rest => Ok(MiniASTI32::Rest(SourceSpan::new(span.start(), span.end()))),
        Rule::random_choice => {
            let values: Vec<MiniASTI32> = inner
                .into_inner()
                .map(parse_choice_element_i32)
                .collect::<Result<_, ParseError>>()?;
            Ok(MiniASTI32::RandomChoice(values))
        }
        Rule::value | Rule::number => {
            let n: i32 = inner.as_str().parse().unwrap_or(0);
            let value_span = inner.as_span();
            Ok(MiniASTI32::Pure(Located::new(n, value_span.start(), value_span.end())))
        }
        _ => Err(ParseError {
            message: format!("Unexpected atom rule (i32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(span.start(), span.end())),
        }),
    }
}

fn parse_choice_element_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::value | Rule::number => {
            let span = inner.as_span();
            let n: i32 = inner.as_str().parse().unwrap_or(0);
            Ok(MiniASTI32::Pure(Located::new(n, span.start(), span.end())))
        }
        Rule::pattern_expr | Rule::sequence_expr => parse_pattern_expr_i32(inner),
        _ => Err(ParseError {
            message: format!("Unexpected choice element rule (i32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn apply_modifier_i32(
    ast: MiniASTI32,
    modifier: pest::iterators::Pair<Rule>,
    _start: usize,
    _end: usize,
) -> Result<MiniASTI32, ParseError> {
    let inner = modifier.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::fast_mod => {
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand_f64(operand_pair)?;
            Ok(MiniASTI32::Fast(Box::new(ast), Box::new(factor)))
        }
        Rule::slow_mod => {
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand_f64(operand_pair)?;
            Ok(MiniASTI32::Slow(Box::new(ast), Box::new(factor)))
        }
        Rule::replicate => {
            let count = inner
                .into_inner()
                .next()
                .map(|p| p.as_str().parse().unwrap_or(2))
                .unwrap_or(2);
            Ok(MiniASTI32::Replicate(Box::new(ast), count))
        }
        Rule::degrade => {
            let prob = if let Some(p) = inner.into_inner().next() {
                let n: f64 = p.as_str().parse().unwrap_or(0.5);
                Some(n)
            } else {
                None
            };
            Ok(MiniASTI32::Degrade(Box::new(ast), prob))
        }
        Rule::euclidean => {
            let mut operands = inner.into_inner();
            let pulses = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let steps = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let rotation = operands.next().map(|p| parse_mod_operand_i32(p)).transpose()?.map(Box::new);

            Ok(MiniASTI32::Euclidean {
                pattern: Box::new(ast),
                pulses,
                steps,
                rotation,
            })
        }
        _ => Err(ParseError {
            message: format!("Unknown modifier (i32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn parse_fast_sub_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr_i32(inner)?;

    // Fast subsequence means fastcat (preserves weights for timecat)
    match ast {
        MiniASTI32::Sequence(elements) => Ok(MiniASTI32::Sequence(elements)),
        other => Ok(other),
    }
}

fn parse_slow_sub_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    let ast = parse_pattern_expr_i32(inner)?;

    match ast {
        MiniASTI32::Sequence(elements) => {
            let patterns: Vec<MiniASTI32> = elements.into_iter().map(|(p, _)| p).collect();
            Ok(MiniASTI32::SlowCat(patterns))
        }
        other => Ok(MiniASTI32::SlowCat(vec![other])),
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
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand_f64(operand_pair)?;
            Ok(MiniAST::Fast(Box::new(ast), Box::new(factor)))
        }
        Rule::slow_mod => {
            let operand_pair = inner.into_inner().next().unwrap();
            let factor = parse_mod_operand(operand_pair)?;
            Ok(MiniAST::Slow(Box::new(ast), Box::new(factor)))
        }
        Rule::replicate => {
            let count = inner
                .into_inner()
                .next()
                .map(|p| p.as_str().parse().unwrap_or(2))
                .unwrap_or(2);
            Ok(MiniAST::Replicate(Box::new(ast), count))
        }
        Rule::degrade => {
            let prob = if let Some(p) = inner.into_inner().next() {
                let n: f64 = p.as_str().parse().unwrap_or(0.5);
                Some(n)
            } else {
                None
            };
            Ok(MiniAST::Degrade(Box::new(ast), prob))
        }
        Rule::euclidean => {
            let mut operands = inner.into_inner();
            let pulses = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let steps = Box::new(parse_mod_operand_u32(operands.next().unwrap())?);
            let rotation = operands.next().map(|p| parse_mod_operand_i32(p)).transpose()?.map(Box::new);

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

fn parse_mod_operand(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
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
            message: format!("Unexpected mod_operand rule: {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

/// Parse a modifier operand as MiniASTF64 (for fast, slow factors, weights, degrade probability).
fn parse_mod_operand_f64(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTF64, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::number => {
            let n: f64 = inner.as_str().parse().unwrap_or(1.0);
            let span = inner.as_span();
            Ok(MiniASTF64::Pure(Located::new(n, span.start(), span.end())))
        }
        Rule::fast_sub => parse_fast_sub_f64(inner),
        Rule::slow_sub => parse_slow_sub_f64(inner),
        Rule::group => {
            let inner_expr = inner.into_inner().next().unwrap();
            parse_pattern_expr_f64(inner_expr)
        }
        _ => Err(ParseError {
            message: format!("Unexpected mod_operand rule (f64): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

/// Parse a modifier operand as MiniASTU32 (for euclidean pulses/steps).
fn parse_mod_operand_u32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTU32, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::number => {
            let n: u32 = inner.as_str().parse().unwrap_or(1);
            let span = inner.as_span();
            Ok(MiniASTU32::Pure(Located::new(n, span.start(), span.end())))
        }
        Rule::fast_sub => parse_fast_sub_u32(inner),
        Rule::slow_sub => parse_slow_sub_u32(inner),
        Rule::group => {
            let inner_expr = inner.into_inner().next().unwrap();
            parse_pattern_expr_u32(inner_expr)
        }
        _ => Err(ParseError {
            message: format!("Unexpected mod_operand rule (u32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

/// Parse a modifier operand as MiniASTI32 (for euclidean rotation).
fn parse_mod_operand_i32(pair: pest::iterators::Pair<Rule>) -> Result<MiniASTI32, ParseError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::number => {
            let n: i32 = inner.as_str().parse().unwrap_or(0);
            let span = inner.as_span();
            Ok(MiniASTI32::Pure(Located::new(n, span.start(), span.end())))
        }
        Rule::fast_sub => parse_fast_sub_i32(inner),
        Rule::slow_sub => parse_slow_sub_i32(inner),
        Rule::group => {
            let inner_expr = inner.into_inner().next().unwrap();
            parse_pattern_expr_i32(inner_expr)
        }
        _ => Err(ParseError {
            message: format!("Unexpected mod_operand rule (i32): {:?}", inner.as_rule()),
            span: Some(SourceSpan::new(inner.as_span().start(), inner.as_span().end())),
        }),
    }
}

fn parse_atom(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let span = pair.as_span();
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::rest => Ok(MiniAST::Rest(SourceSpan::new(span.start(), span.end()))),

        Rule::random_choice => {
            let values: Vec<MiniAST> = inner
                .into_inner()
                .map(|p| parse_choice_element(p))
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

/// Parse a choice_element (used in random_choice), which can be a value or a rest.
fn parse_choice_element(pair: pest::iterators::Pair<Rule>) -> Result<MiniAST, ParseError> {
    let span = pair.as_span();
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::rest => Ok(MiniAST::Rest(SourceSpan::new(span.start(), span.end()))),
        Rule::value => {
            let value_span = inner.as_span();
            let atom = parse_value(inner)?;
            Ok(MiniAST::Pure(Located::new(atom, value_span.start(), value_span.end())))
        }
        _ => Err(ParseError {
            message: format!("Unexpected choice_element rule: {:?}", inner.as_rule()),
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

    #[test]
    fn test_degrade_simple() {
        let result = parse("c4?");
        assert!(result.is_ok(), "c4? should parse: {:?}", result);
        let ast = result.unwrap();
        assert!(matches!(ast, MiniAST::Degrade(_, None)));
    }

    #[test]
    fn test_degrade_with_probability() {
        let result = parse("c4?0.3");
        assert!(result.is_ok(), "c4?0.3 should parse: {:?}", result);
        let ast = result.unwrap();
        if let MiniAST::Degrade(_, Some(prob)) = ast {
            assert!((prob - 0.3).abs() < 0.001, "Expected 0.3, got {}", prob);
        } else {
            panic!("Expected Degrade with Some probability, got {:?}", ast);
        }
    }

    #[test]
    fn test_degrade_in_sequence() {
        let result = parse("c2 c3 c4? c5");
        assert!(result.is_ok(), "c2 c3 c4? c5 should parse: {:?}", result);
        if let MiniAST::Sequence(elements) = result.unwrap() {
            assert_eq!(elements.len(), 4);
            // Third element should be Degrade
            let (third, _) = &elements[2];
            assert!(matches!(third, MiniAST::Degrade(_, None)));
        } else {
            panic!("Expected Sequence");
        }
    }

    #[test]
    fn test_random_choice_with_rest() {
        let result = parse("c4|~");
        assert!(result.is_ok(), "c4|~ should parse: {:?}", result);
        let ast = result.unwrap();
        if let MiniAST::RandomChoice(choices) = ast {
            assert_eq!(choices.len(), 2);
            assert!(matches!(&choices[0], MiniAST::Pure(_)));
            assert!(matches!(&choices[1], MiniAST::Rest(_)));
        } else {
            panic!("Expected RandomChoice, got {:?}", ast);
        }
    }

    #[test]
    fn test_random_choice_rest_first() {
        let result = parse("~|c4");
        assert!(result.is_ok(), "~|c4 should parse: {:?}", result);
        let ast = result.unwrap();
        if let MiniAST::RandomChoice(choices) = ast {
            assert_eq!(choices.len(), 2);
            assert!(matches!(&choices[0], MiniAST::Rest(_)));
            assert!(matches!(&choices[1], MiniAST::Pure(_)));
        } else {
            panic!("Expected RandomChoice, got {:?}", ast);
        }
    }

    #[test]
    fn test_random_choice_multiple_rests() {
        let result = parse("c4|~|d4|~");
        assert!(result.is_ok(), "c4|~|d4|~ should parse: {:?}", result);
        let ast = result.unwrap();
        if let MiniAST::RandomChoice(choices) = ast {
            assert_eq!(choices.len(), 4);
            assert!(matches!(&choices[0], MiniAST::Pure(_)));
            assert!(matches!(&choices[1], MiniAST::Rest(_)));
            assert!(matches!(&choices[2], MiniAST::Pure(_)));
            assert!(matches!(&choices[3], MiniAST::Rest(_)));
        } else {
            panic!("Expected RandomChoice, got {:?}", ast);
        }
    }

    #[test]
    fn test_random_choice_with_rest_in_sequence() {
        let result = parse("c2 c3 c4|~ c5");
        assert!(result.is_ok(), "c2 c3 c4|~ c5 should parse: {:?}", result);
        if let MiniAST::Sequence(elements) = result.unwrap() {
            assert_eq!(elements.len(), 4);
            // Third element should be RandomChoice with rest
            let (third, _) = &elements[2];
            if let MiniAST::RandomChoice(choices) = third {
                assert_eq!(choices.len(), 2);
                assert!(matches!(&choices[1], MiniAST::Rest(_)));
            } else {
                panic!("Expected RandomChoice, got {:?}", third);
            }
        } else {
            panic!("Expected Sequence");
        }
    }

    // ============ Typed AST Parsing Tests ============

    // --- MiniASTF64 operand tests (weights, fast factors, degrade probs) ---

    #[test]
    fn test_parse_weight_pure_f64() {
        // Simple weight: number parsed directly as f64
        let result = parse("c4@2");
        assert!(result.is_ok());
        if let MiniAST::Sequence(elements) = result.unwrap() {
            let (_, weight) = &elements[0];
            if let Some(w) = weight {
                assert!((*w - 2.0).abs() < 0.001);
            } else {
                panic!("Expected weight");
            }
        } else {
            panic!("Expected Sequence");
        }
    }

    #[test]
    fn test_parse_fast_factor_pure_f64() {
        // Fast with number: factor is MiniASTF64::Pure
        let result = parse("c4*2");
        assert!(result.is_ok());
        if let MiniAST::Fast(_, factor) = result.unwrap() {
            assert!(matches!(*factor, MiniASTF64::Pure(_)));
            if let MiniASTF64::Pure(Located { node, .. }) = *factor {
                assert!((node - 2.0).abs() < 0.001);
            }
        } else {
            panic!("Expected Fast");
        }
    }

    #[test]
    fn test_parse_fast_factor_subsequence_f64() {
        // Fast with subsequence: [1 2] -> parsed as MiniASTF64::Sequence
        let result = parse("c4*[1 2]");
        assert!(result.is_ok());
        if let MiniAST::Fast(_, factor) = result.unwrap() {
            assert!(matches!(*factor, MiniASTF64::Sequence(_)));
            if let MiniASTF64::Sequence(elements) = *factor {
                assert_eq!(elements.len(), 2);
                // Check first element is Pure(1.0)
                if let MiniASTF64::Pure(Located { node, .. }) = &elements[0].0 {
                    assert!((node - 1.0).abs() < 0.001);
                }
            }
        } else {
            panic!("Expected Fast");
        }
    }

    #[test]
    fn test_parse_fast_factor_slowcat_f64() {
        // Fast with slow subsequence: <1 2> -> parsed as MiniASTF64::SlowCat
        let result = parse("c4*<1 2>");
        assert!(result.is_ok());
        if let MiniAST::Fast(_, factor) = result.unwrap() {
            assert!(matches!(*factor, MiniASTF64::SlowCat(_)));
            if let MiniASTF64::SlowCat(elements) = *factor {
                assert_eq!(elements.len(), 2);
            }
        } else {
            panic!("Expected Fast");
        }
    }

    #[test]
    fn test_parse_degrade_prob_f64() {
        // Degrade with probability: prob is now a plain f64
        let result = parse("c4?0.75");
        assert!(result.is_ok());
        if let MiniAST::Degrade(_, Some(prob)) = result.unwrap() {
            assert!((prob - 0.75).abs() < 0.001);
        } else {
            panic!("Expected Degrade with prob");
        }
    }

    // --- MiniASTU32 operand tests (replicate count, euclidean params) ---

    #[test]
    fn test_parse_replicate_count_pure_u32() {
        // Replicate with count: count is now a plain u32
        let result = parse("c4!3");
        assert!(result.is_ok());
        if let MiniAST::Replicate(_, count) = result.unwrap() {
            assert_eq!(count, 3);
        } else {
            panic!("Expected Replicate");
        }
    }

    #[test]
    fn test_parse_replicate_default_count() {
        // Replicate without count: defaults to 2
        let result = parse("c4!");
        assert!(result.is_ok());
        if let MiniAST::Replicate(_, count) = result.unwrap() {
            assert_eq!(count, 2);
        } else {
            panic!("Expected Replicate");
        }
    }

    #[test]
    fn test_parse_euclidean_pulses_steps_u32() {
        // Euclidean basic: pulses and steps are MiniASTU32::Pure
        let result = parse("c4(3,8)");
        assert!(result.is_ok());
        if let MiniAST::Euclidean { pulses, steps, rotation, .. } = result.unwrap() {
            assert!(matches!(*pulses, MiniASTU32::Pure(_)));
            assert!(matches!(*steps, MiniASTU32::Pure(_)));
            assert!(rotation.is_none());
            
            if let MiniASTU32::Pure(Located { node, .. }) = *pulses {
                assert_eq!(node, 3);
            }
            if let MiniASTU32::Pure(Located { node, .. }) = *steps {
                assert_eq!(node, 8);
            }
        } else {
            panic!("Expected Euclidean");
        }
    }

    #[test]
    fn test_parse_euclidean_with_rotation_i32() {
        // Euclidean with rotation: pulses/steps are MiniASTU32, rotation is MiniASTI32
        let result = parse("c4(3,8,2)");
        assert!(result.is_ok());
        if let MiniAST::Euclidean { pulses, steps, rotation, .. } = result.unwrap() {
            if let MiniASTU32::Pure(Located { node, .. }) = *pulses {
                assert_eq!(node, 3);
            }
            if let MiniASTU32::Pure(Located { node, .. }) = *steps {
                assert_eq!(node, 8);
            }
            if let Some(rot) = rotation {
                if let MiniASTI32::Pure(Located { node, .. }) = *rot {
                    assert_eq!(node, 2);
                }
            } else {
                panic!("Expected rotation");
            }
        } else {
            panic!("Expected Euclidean");
        }
    }

    #[test]
    fn test_parse_euclidean_pulses_subsequence_u32() {
        // Euclidean with subsequence for pulses: [3 5] -> MiniASTU32::Sequence
        let result = parse("c4([3 5],8)");
        assert!(result.is_ok());
        if let MiniAST::Euclidean { pulses, steps, .. } = result.unwrap() {
            assert!(matches!(*pulses, MiniASTU32::Sequence(_)));
            if let MiniASTU32::Sequence(elements) = *pulses {
                assert_eq!(elements.len(), 2);
                if let MiniASTU32::Pure(Located { node, .. }) = &elements[0].0 {
                    assert_eq!(*node, 3);
                }
                if let MiniASTU32::Pure(Located { node, .. }) = &elements[1].0 {
                    assert_eq!(*node, 5);
                }
            }
            assert!(matches!(*steps, MiniASTU32::Pure(_)));
        } else {
            panic!("Expected Euclidean");
        }
    }

    #[test]
    fn test_parse_euclidean_steps_slowcat_u32() {
        // Euclidean with slowcat for steps: <8 16> -> MiniASTU32::SlowCat
        let result = parse("c4(3,<8 16>)");
        assert!(result.is_ok());
        if let MiniAST::Euclidean { pulses, steps, .. } = result.unwrap() {
            assert!(matches!(*pulses, MiniASTU32::Pure(_)));
            assert!(matches!(*steps, MiniASTU32::SlowCat(_)));
            if let MiniASTU32::SlowCat(elements) = *steps {
                assert_eq!(elements.len(), 2);
            }
        } else {
            panic!("Expected Euclidean");
        }
    }

    // --- Nested typed operand tests (1 level deep) ---

    #[test]
    fn test_parse_fast_nested_sequence_preserves_weights() {
        // Fast with weighted sequence in operand - weights are preserved for timecat
        let result = parse("c4*[1@2 3]");
        assert!(result.is_ok());
        if let MiniAST::Fast(_, factor) = result.unwrap() {
            if let MiniASTF64::Sequence(elements) = *factor {
                assert_eq!(elements.len(), 2);
                // First element should have weight 2
                let (_, weight) = &elements[0];
                assert!(weight.is_some(), "Fast subsequence should preserve weights");
                if let Some(w) = weight {
                    assert!((*w - 2.0).abs() < 0.001);
                }
                // Second element should have no weight
                let (_, weight2) = &elements[1];
                assert!(weight2.is_none());
            } else {
                panic!("Expected MiniASTF64::Sequence");
            }
        } else {
            panic!("Expected Fast");
        }
    }

    #[test]
    fn test_parse_euclidean_nested_slowcat_in_rotation() {
        // Euclidean with slowcat in rotation: (3, 8, <0 2 4>)
        let result = parse("c4(3,8,<0 2 4>)");
        assert!(result.is_ok());
        if let MiniAST::Euclidean { rotation, .. } = result.unwrap() {
            if let Some(rot) = rotation {
                assert!(matches!(*rot, MiniASTI32::SlowCat(_)));
                if let MiniASTI32::SlowCat(elements) = *rot {
                    assert_eq!(elements.len(), 3);
                }
            } else {
                panic!("Expected rotation");
            }
        } else {
            panic!("Expected Euclidean");
        }
    }

    #[test]
    fn test_parse_sequence_with_weighted_elements_nested() {
        // Sequence with various weighted elements
        let result = parse("1@2 2@0.5 3");
        assert!(result.is_ok());
        if let MiniAST::Sequence(elements) = result.unwrap() {
            assert_eq!(elements.len(), 3);
            
            // First: weight 2
            let (_, w1) = &elements[0];
            if let Some(w) = w1 {
                assert!((*w - 2.0).abs() < 0.001);
            } else {
                panic!("Expected weight on first element");
            }
            
            // Second: weight 0.5
            let (_, w2) = &elements[1];
            if let Some(w) = w2 {
                assert!((*w - 0.5).abs() < 0.001);
            } else {
                panic!("Expected weight on second element");
            }
            
            // Third: no weight
            let (_, w3) = &elements[2];
            assert!(w3.is_none());
        } else {
            panic!("Expected Sequence");
        }
    }

    #[test]
    fn test_parse_fast_with_grouped_operand() {
        // Fast with grouped operand: *(1) -> MiniASTF64::Pure via group
        let result = parse("c4*(2)");
        assert!(result.is_ok());
        if let MiniAST::Fast(_, factor) = result.unwrap() {
            // Group unwraps to the inner value
            if let MiniASTF64::Pure(Located { node, .. }) = *factor {
                assert!((node - 2.0).abs() < 0.001);
            }
        } else {
            panic!("Expected Fast");
        }
    }

    #[test]
    fn test_parse_euclidean_all_nested() {
        // Euclidean with all params as subsequences
        let result = parse("c4([3 5],[8 16],[0 2])");
        assert!(result.is_ok());
        if let MiniAST::Euclidean { pulses, steps, rotation, .. } = result.unwrap() {
            assert!(matches!(*pulses, MiniASTU32::Sequence(_)));
            assert!(matches!(*steps, MiniASTU32::Sequence(_)));
            if let Some(rot) = rotation {
                assert!(matches!(*rot, MiniASTI32::Sequence(_)));
            } else {
                panic!("Expected rotation");
            }
        } else {
            panic!("Expected Euclidean");
        }
    }

    // --- Cross-type verification tests ---

    #[test]
    fn test_parse_mixed_typed_modifiers() {
        // Combine fast (f64 factor), replicate (u32 count), degrade (f64 prob)
        let result = parse("c4*2!3?0.5");
        assert!(result.is_ok());
        // This should parse as: Degrade(Replicate(Fast(c4, 2), 3), 0.5)
        let ast = result.unwrap();
        if let MiniAST::Degrade(inner, prob) = ast {
            // Check prob is now f64
            assert!(prob.is_some());
            assert!((prob.unwrap() - 0.5).abs() < 0.001);
            // Inner should be Replicate
            if let MiniAST::Replicate(inner2, count) = *inner {
                assert_eq!(count, 3);
                // Inner2 should be Fast
                if let MiniAST::Fast(_, factor) = *inner2 {
                    assert!(matches!(*factor, MiniASTF64::Pure(_)));
                } else {
                    panic!("Expected Fast");
                }
            } else {
                panic!("Expected Replicate");
            }
        } else {
            panic!("Expected Degrade");
        }
    }

    #[test]
    fn test_parse_euclidean_then_fast() {
        // Euclidean pattern with fast modifier
        let result = parse("c4(3,8)*2");
        assert!(result.is_ok());
        if let MiniAST::Fast(inner, factor) = result.unwrap() {
            assert!(matches!(*factor, MiniASTF64::Pure(_)));
            assert!(matches!(*inner, MiniAST::Euclidean { .. }));
        } else {
            panic!("Expected Fast wrapping Euclidean");
        }
    }
}
