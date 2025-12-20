// Krill mini-notation parser for Rust
// Based on krill.pegjs from Strudel project

use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[grammar = "pattern.pest"]
pub struct KrillParser;

/// Location information for parsed elements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

/// Atom represents a basic step (e.g., "bd", "sd")
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomStub {
    pub type_: String,
    pub source_: String,
    pub location_: Location,
}

impl AtomStub {
    pub fn new(source: String, location: Location) -> Self {
        Self {
            type_: "atom".to_string(),
            source_: source,
            location_: location,
        }
    }
}

/// Pattern represents a collection of elements with alignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternStub {
    pub type_: String,
    pub source_: Vec<Box<ParsedElement>>,
    pub arguments_: PatternArguments,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternArguments {
    pub alignment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _steps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stepsPerCycle: Option<Box<ParsedElement>>,
}

impl PatternStub {
    pub fn new(
        source: Vec<Box<ParsedElement>>,
        alignment: String,
        seed: Option<u32>,
        _steps: bool,
    ) -> Self {
        Self {
            type_: "pattern".to_string(),
            source_: source,
            arguments_: PatternArguments {
                alignment,
                seed,
                _steps: Some(_steps),
                stepsPerCycle: None,
            },
        }
    }

    pub fn with_steps_per_cycle(mut self, steps_per_cycle: ParsedElement) -> Self {
        self.arguments_.stepsPerCycle = Some(Box::new(steps_per_cycle));
        self
    }
}

/// Operator represents transformations applied to patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorStub {
    pub type_: String,
    pub arguments_: OperatorArguments,
    pub source_: Box<ParsedElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OperatorArguments {
    Scale { scale: String },
    Stretch { amount: f64 },
    Shift { amount: String },
    Bjorklund { pulse: i32, step: i32, rotation: Option<i32> },
    Target { name: String },
    Struct { mini: Box<ParsedElement> },
}

impl OperatorStub {
    pub fn new(name: String, args: OperatorArguments, source: ParsedElement) -> Self {
        Self {
            type_: name,
            arguments_: args,
            source_: Box::new(source),
        }
    }
}

/// Element represents a slice with optional operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementStub {
    pub type_: String,
    pub source_: Box<ParsedElement>,
    pub options_: ElementOptions,
    pub location_: Location,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementOptions {
    pub ops: Vec<SliceOperation>,
    pub weight: f64,
    pub reps: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type_")]
pub enum SliceOperation {
    #[serde(rename = "replicate")]
    Replicate { arguments_: ReplicateArgs },
    #[serde(rename = "bjorklund")]
    Bjorklund { arguments_: BjorklundArgs },
    #[serde(rename = "stretch")]
    Stretch { arguments_: StretchArgs },
    #[serde(rename = "degradeBy")]
    DegradeBy { arguments_: DegradeArgs },
    #[serde(rename = "tail")]
    Tail { arguments_: TailArgs },
    #[serde(rename = "range")]
    Range { arguments_: RangeArgs },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplicateArgs {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BjorklundArgs {
    pub pulse: Box<ParsedElement>,
    pub step: Box<ParsedElement>,
    pub rotation: Option<Box<ParsedElement>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StretchArgs {
    pub amount: Box<ParsedElement>,
    #[serde(rename = "type")]
    pub stretch_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DegradeArgs {
    pub amount: Option<f64>,
    pub seed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TailArgs {
    pub element: Box<ParsedElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeArgs {
    pub element: Box<ParsedElement>,
}

impl ElementStub {
    pub fn new(source: ParsedElement, location: Location) -> Self {
        Self {
            type_: "element".to_string(),
            source_: Box::new(source),
            options_: ElementOptions {
                ops: Vec::new(),
                weight: 1.0,
                reps: 1,
            },
            location_: location,
        }
    }
}

/// Command represents control commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandStub {
    pub type_: String,
    pub name_: String,
    pub options_: CommandOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

impl CommandStub {
    pub fn new(name: String, value: Option<f64>) -> Self {
        Self {
            type_: "command".to_string(),
            name_: name,
            options_: CommandOptions { value },
        }
    }
}

/// Union type for all parsed elements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParsedElement {
    Atom(AtomStub),
    Pattern(PatternStub),
    Operator(OperatorStub),
    Element(ElementStub),
    Command(CommandStub),
}

/// Parse a mini-notation string into an AST
pub fn parse_pattern(input: &str) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let mut pairs = KrillParser::parse(Rule::start, input)?;
    let statement = pairs.next().unwrap().into_inner().next().unwrap();

    parse_statement(statement, &mut 0)
}

fn get_location(pair: &pest::iterators::Pair<Rule>) -> Location {
    let (line, column) = pair.as_span().start_pos().line_col();
    Location { line, column }
}

fn parse_statement(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    match pair.as_rule() {
        Rule::statement => {
            let inner = pair.into_inner().next().unwrap();
            parse_statement(inner, seed)
        }
        Rule::mini_definition => {
            let inner = pair.into_inner().next().unwrap();
            parse_sequ_or_operator_or_comment(inner, seed)
        }
        Rule::command => {
            let inner = pair.into_inner().next().unwrap();
            parse_command(inner)
        }
        _ => unreachable!("Unexpected rule in statement: {:?}", pair.as_rule()),
    }
}

fn parse_command(pair: pest::iterators::Pair<Rule>) -> Result<ParsedElement, pest::error::Error<Rule>> {
    match pair.as_rule() {
        Rule::setcps => {
            let value = pair.into_inner().next().unwrap().as_str().parse::<f64>().unwrap();
            Ok(ParsedElement::Command(CommandStub::new(
                "setcps".to_string(),
                Some(value),
            )))
        }
        Rule::setbpm => {
            let value = pair.into_inner().next().unwrap().as_str().parse::<f64>().unwrap();
            let cps_value = value / 120.0 / 2.0;
            Ok(ParsedElement::Command(CommandStub::new(
                "setcps".to_string(),
                Some(cps_value),
            )))
        }
        Rule::hush => Ok(ParsedElement::Command(CommandStub::new(
            "hush".to_string(),
            None,
        ))),
        _ => unreachable!("Unexpected command rule: {:?}", pair.as_rule()),
    }
}

fn parse_sequ_or_operator_or_comment(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    match pair.as_rule() {
        Rule::sequ_or_operator_or_comment => {
            let inner = pair.into_inner().next().unwrap();
            parse_sequ_or_operator_or_comment(inner, seed)
        }
        Rule::mini_or_operator => parse_mini_or_operator(pair, seed),
        Rule::comment => {
            // Comments are ignored in the AST
            Ok(ParsedElement::Command(CommandStub::new(
                "comment".to_string(),
                None,
            )))
        }
        _ => unreachable!("Unexpected rule: {:?}", pair.as_rule()),
    }
}

fn parse_mini_or_operator(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::mini_or_group => {
            // Skip comments
            Ok(parse_mini_or_group(first, seed)?)
        }
        Rule::operator => {
            let op = parse_operator_def(first)?;
            // The grammar is: operator ~ "$" ~ mini_or_operator
            // So we should have: operator, "$", mini_or_operator
            // But pest might inline the "$" since it's whitespace-separated
            // Let's check what we actually have
            let dollar = inner.next();
            let source = if dollar.as_ref().map(|p| p.as_str()) == Some("$") {
                // We got the $ explicitly
                inner.next()
            } else {
                // The $ might have been implicit or dollar is actually the source
                dollar
            };
            let source_element = parse_mini_or_operator(source.unwrap(), seed)?;

            Ok(ParsedElement::Operator(OperatorStub::new(
                op.0,
                op.1,
                source_element,
            )))
        }
        _ => unreachable!("Unexpected mini_or_operator rule: {:?}", first.as_rule()),
    }
}

fn parse_operator_def(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, OperatorArguments), pest::error::Error<Rule>> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::scale => {
            let mut parts = inner.into_inner();
            parts.next(); // Skip quote
            let scale_str = parts.next().unwrap().as_str();
            Ok((
                "scale".to_string(),
                OperatorArguments::Scale {
                    scale: scale_str.to_string(),
                },
            ))
        }
        Rule::slow => {
            let amount = inner.into_inner().next().unwrap().as_str().parse::<f64>().unwrap();
            Ok((
                "stretch".to_string(),
                OperatorArguments::Stretch { amount },
            ))
        }
        Rule::fast => {
            let amount = inner.into_inner().next().unwrap().as_str().parse::<f64>().unwrap();
            Ok((
                "stretch".to_string(),
                OperatorArguments::Stretch { amount: 1.0 / amount },
            ))
        }
        Rule::rotL => {
            let amount = inner.into_inner().next().unwrap().as_str().parse::<f64>().unwrap();
            Ok((
                "shift".to_string(),
                OperatorArguments::Shift {
                    amount: format!("-{}", amount),
                },
            ))
        }
        Rule::rotR => {
            let amount = inner.into_inner().next().unwrap().as_str().parse::<f64>().unwrap();
            Ok((
                "shift".to_string(),
                OperatorArguments::Shift {
                    amount: amount.to_string(),
                },
            ))
        }
        Rule::target => {
            let mut parts = inner.into_inner();
            parts.next(); // Skip quote
            let name = parts.next().unwrap().as_str().to_string();
            Ok((
                "target".to_string(),
                OperatorArguments::Target { name },
            ))
        }
        Rule::bjorklund => {
            let mut parts = inner.into_inner();
            let pulse = parts.next().unwrap().as_str().parse::<i32>().unwrap();
            let step = parts.next().unwrap().as_str().parse::<i32>().unwrap();
            let rotation = parts.next().map(|p| p.as_str().parse::<i32>().unwrap());
            Ok((
                "bjorklund".to_string(),
                OperatorArguments::Bjorklund {
                    pulse,
                    step,
                    rotation,
                },
            ))
        }
        Rule::struct_op => {
            let mini = inner.into_inner().next().unwrap();
            let mini_element = parse_mini_or_operator(mini, &mut 0)?;
            Ok((
                "struct".to_string(),
                OperatorArguments::Struct {
                    mini: Box::new(mini_element),
                },
            ))
        }
        _ => unreachable!("Unexpected operator rule: {:?}", inner.as_rule()),
    }
}

fn parse_mini_or_group(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::group_operator => parse_group_operator(inner, seed),
        Rule::mini => parse_mini(inner, seed),
        _ => unreachable!("Unexpected mini_or_group rule: {:?}", inner.as_rule()),
    }
}

fn parse_group_operator(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::cat => {
            let mut elements = Vec::new();
            for part in inner.into_inner() {
                if part.as_rule() == Rule::mini_or_operator {
                    elements.push(Box::new(parse_mini_or_operator(part, seed)?));
                }
            }
            Ok(ParsedElement::Pattern(PatternStub::new(
                elements,
                "slowcat".to_string(),
                None,
                false,
            )))
        }
        _ => unreachable!("Unexpected group_operator rule: {:?}", inner.as_rule()),
    }
}

fn parse_mini(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let mut inner = pair.into_inner();
    inner.next(); // Skip opening quote
    let stack = inner.next().unwrap();
    parse_stack_or_choose(stack, seed)
}

fn parse_stack_or_choose(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let mut inner = pair.into_inner();
    let head = inner.next().unwrap();
    let head_element = parse_sequence(head, seed)?;

    if let Some(tail) = inner.next() {
        let (alignment, tail_seed) = match tail.as_rule() {
            Rule::stack_tail => ("stack", None),
            Rule::choose_tail => {
                let current_seed = *seed;
                *seed += 1;
                ("rand", Some(current_seed))
            }
            Rule::dot_tail => {
                let current_seed = *seed;
                *seed += 1;
                ("feet", Some(current_seed))
            }
            _ => unreachable!("Unexpected tail rule: {:?}", tail.as_rule()),
        };

        let mut elements = vec![Box::new(head_element)];
        for seq in tail.into_inner() {
            if seq.as_rule() == Rule::sequence {
                elements.push(Box::new(parse_sequence(seq, seed)?));
            }
        }

        Ok(ParsedElement::Pattern(PatternStub::new(
            elements,
            alignment.to_string(),
            tail_seed,
            false,
        )))
    } else {
        Ok(head_element)
    }
}

fn parse_polymeter_stack(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<PatternStub, pest::error::Error<Rule>> {
    let mut inner = pair.into_inner();
    let head = inner.next().unwrap();
    let head_element = parse_sequence(head, seed)?;

    let mut elements = vec![Box::new(head_element)];

    if let Some(tail) = inner.next() {
        for seq in tail.into_inner() {
            if seq.as_rule() == Rule::sequence {
                elements.push(Box::new(parse_sequence(seq, seed)?));
            }
        }
    }

    Ok(PatternStub::new(
        elements,
        "polymeter".to_string(),
        None,
        false,
    ))
}

fn parse_sequence(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let inner = pair.into_inner();
    let mut has_steps = false;
    let mut elements = Vec::new();

    for part in inner {
        match part.as_rule() {
            Rule::slice_with_ops => {
                elements.push(Box::new(parse_slice_with_ops(part, seed)?));
            }
            _ if part.as_str() == "^" => {
                has_steps = true;
            }
            _ => {}
        }
    }

    if elements.len() == 1 {
        Ok(*elements.into_iter().next().unwrap())
    } else {
        Ok(ParsedElement::Pattern(PatternStub::new(
            elements,
            "fastcat".to_string(),
            None,
            has_steps,
        )))
    }
}

fn parse_slice_with_ops(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let location = get_location(&pair);
    let mut inner = pair.into_inner();
    let slice = inner.next().unwrap();
    let slice_element = parse_slice(slice, seed)?;

    let mut element = ElementStub::new(slice_element, location);

    for op in inner {
        apply_slice_op(&mut element, op, seed)?;
    }

    Ok(ParsedElement::Element(element))
}

fn apply_slice_op(
    element: &mut ElementStub,
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<(), pest::error::Error<Rule>> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::op_weight => {
            let amount = inner
                .into_inner()
                .next()
                .map(|p| p.as_str().parse::<f64>().unwrap())
                .unwrap_or(2.0);
            element.options_.weight += amount - 1.0;
        }
        Rule::op_replicate => {
            let amount = inner
                .into_inner()
                .next()
                .map(|p| p.as_str().parse::<u32>().unwrap())
                .unwrap_or(2);
            let reps = element.options_.reps + amount - 1;
            element.options_.reps = reps;
            element.options_.ops.retain(|op| {
                !matches!(op, SliceOperation::Replicate { .. })
            });
            element.options_.ops.push(SliceOperation::Replicate {
                arguments_: ReplicateArgs { amount: reps },
            });
            element.options_.weight = reps as f64;
        }
        Rule::op_bjorklund => {
            let mut parts = inner.into_inner();
            let pulse = parse_slice_with_ops(parts.next().unwrap(), seed)?;
            let step = parse_slice_with_ops(parts.next().unwrap(), seed)?;
            let rotation = parts.next().map(|p| parse_slice_with_ops(p, seed)).transpose()?;

            element.options_.ops.push(SliceOperation::Bjorklund {
                arguments_: BjorklundArgs {
                    pulse: Box::new(pulse),
                    step: Box::new(step),
                    rotation: rotation.map(Box::new),
                },
            });
        }
        Rule::op_slow => {
            let slice = inner.into_inner().next().unwrap();
            let amount = parse_slice(slice, seed)?;
            element.options_.ops.push(SliceOperation::Stretch {
                arguments_: StretchArgs {
                    amount: Box::new(amount),
                    stretch_type: "slow".to_string(),
                },
            });
        }
        Rule::op_fast => {
            let slice = inner.into_inner().next().unwrap();
            let amount = parse_slice(slice, seed)?;
            element.options_.ops.push(SliceOperation::Stretch {
                arguments_: StretchArgs {
                    amount: Box::new(amount),
                    stretch_type: "fast".to_string(),
                },
            });
        }
        Rule::op_degrade => {
            let amount = inner
                .into_inner()
                .next()
                .map(|p| p.as_str().parse::<f64>().unwrap());
            let current_seed = *seed;
            *seed += 1;
            element.options_.ops.push(SliceOperation::DegradeBy {
                arguments_: DegradeArgs {
                    amount,
                    seed: current_seed,
                },
            });
        }
        Rule::op_tail => {
            let slice = inner.into_inner().next().unwrap();
            let tail_element = parse_slice(slice, seed)?;
            element.options_.ops.push(SliceOperation::Tail {
                arguments_: TailArgs {
                    element: Box::new(tail_element),
                },
            });
        }
        Rule::op_range => {
            let slice = inner.into_inner().next().unwrap();
            let range_element = parse_slice(slice, seed)?;
            element.options_.ops.push(SliceOperation::Range {
                arguments_: RangeArgs {
                    element: Box::new(range_element),
                },
            });
        }
        _ => unreachable!("Unexpected slice_op rule: {:?}", inner.as_rule()),
    }
    Ok(())
}

fn parse_slice(
    pair: pest::iterators::Pair<Rule>,
    seed: &mut u32,
) -> Result<ParsedElement, pest::error::Error<Rule>> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::step => {
            let location = get_location(&inner);
            Ok(ParsedElement::Atom(AtomStub::new(
                inner.as_str().to_string(),
                location,
            )))
        }
        Rule::sub_cycle => {
            let stack = inner.into_inner().next().unwrap();
            parse_stack_or_choose(stack, seed)
        }
        Rule::polymeter => {
            let mut parts = inner.into_inner();
            let stack = parts.next().unwrap();
            let mut pattern = parse_polymeter_stack(stack, seed)?;

            if let Some(steps_part) = parts.next() {
                let steps_slice = steps_part.into_inner().next().unwrap();
                let steps_element = parse_slice(steps_slice, seed)?;
                pattern = pattern.with_steps_per_cycle(steps_element);
            }

            Ok(ParsedElement::Pattern(pattern))
        }
        Rule::slow_sequence => {
            let stack = inner.into_inner().next().unwrap();
            let mut pattern = parse_polymeter_stack(stack, seed)?;
            pattern.arguments_.alignment = "polymeter_slowcat".to_string();
            Ok(ParsedElement::Pattern(pattern))
        }
        _ => unreachable!("Unexpected slice rule: {:?}", inner.as_rule()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let pairs = KrillParser::parse(Rule::number, "42").unwrap();
        assert_eq!(pairs.as_str(), "42");

        let pairs = KrillParser::parse(Rule::number, "-3.14").unwrap();
        assert_eq!(pairs.as_str(), "-3.14");

        let pairs = KrillParser::parse(Rule::number, "1.5e-10").unwrap();
        assert_eq!(pairs.as_str(), "1.5e-10");
    }

    #[test]
    fn test_parse_step() {
        let pairs = KrillParser::parse(Rule::step, "bd").unwrap();
        assert_eq!(pairs.as_str(), "bd");

        let pairs = KrillParser::parse(Rule::step, "kick1").unwrap();
        assert_eq!(pairs.as_str(), "kick1");
    }

    #[test]
    fn test_parse_simple_sequence() {
        let result = parse_pattern("\"bd sd hh\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "fastcat");
                assert_eq!(p.source_.len(), 3);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_parse_stack() {
        let result = parse_pattern("\"bd, sd\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "stack");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_parse_choose() {
        let result = parse_pattern("\"bd | sd\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "rand");
                assert_eq!(p.source_.len(), 2);
                assert!(p.arguments_.seed.is_some());
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_parse_sub_cycle() {
        let result = parse_pattern("\"bd [sd hh]\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_parse_weight() {
        let result = parse_pattern("\"bd@3\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.weight, 3.0);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_replicate() {
        let result = parse_pattern("\"bd!4\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.reps, 4);
                assert!(e.options_.ops.iter().any(|op| matches!(
                    op,
                    SliceOperation::Replicate { .. }
                )));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_slow() {
        let result = parse_pattern("\"bd/2\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(
                        op,
                        SliceOperation::Stretch {
                            arguments_: StretchArgs { stretch_type, .. }
                        } if stretch_type == "slow"
                    )
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_fast() {
        let result = parse_pattern("\"bd*2\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(
                        op,
                        SliceOperation::Stretch {
                            arguments_: StretchArgs { stretch_type, .. }
                        } if stretch_type == "fast"
                    )
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_degrade() {
        let result = parse_pattern("\"bd?\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| matches!(
                    op,
                    SliceOperation::DegradeBy { .. }
                )));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_polymeter() {
        let result = parse_pattern("\"{bd sd, hh}\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                match *e.source_ {
                    ParsedElement::Pattern(p) => {
                        assert_eq!(p.arguments_.alignment, "polymeter");
                    }
                    _ => panic!("Expected pattern source"),
                }
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_slow_sequence() {
        let result = parse_pattern("\"<bd sd hh>\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                match *e.source_ {
                    ParsedElement::Pattern(p) => {
                        assert_eq!(p.arguments_.alignment, "polymeter_slowcat");
                    }
                    _ => panic!("Expected pattern source"),
                }
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_parse_operator_slow() {
        let result = parse_pattern("slow 2 $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "stretch");
                match op.arguments_ {
                    OperatorArguments::Stretch { amount } => {
                        assert_eq!(amount, 2.0);
                    }
                    _ => panic!("Expected stretch arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_parse_operator_fast() {
        let result = parse_pattern("fast 2 $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "stretch");
                match op.arguments_ {
                    OperatorArguments::Stretch { amount } => {
                        assert_eq!(amount, 0.5);
                    }
                    _ => panic!("Expected stretch arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_parse_operator_scale() {
        let result = parse_pattern("scale 'major' $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "scale");
                match &op.arguments_ {
                    OperatorArguments::Scale { scale } => {
                        assert_eq!(scale, "major");
                    }
                    _ => panic!("Expected scale arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_parse_command_setcps() {
        let result = parse_pattern("setcps 0.5").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                assert_eq!(cmd.name_, "setcps");
                assert_eq!(cmd.options_.value, Some(0.5));
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_parse_command_setbpm() {
        let result = parse_pattern("setbpm 120").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                assert_eq!(cmd.name_, "setcps");
                // 120 bpm = 120/120/2 = 0.5 cps
                assert_eq!(cmd.options_.value, Some(0.5));
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_parse_command_hush() {
        let result = parse_pattern("hush").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                assert_eq!(cmd.name_, "hush");
                assert_eq!(cmd.options_.value, None);
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_parse_bjorklund_operator() {
        let result = parse_pattern("euclid 3 8 $ \"bd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "bjorklund");
                match op.arguments_ {
                    OperatorArguments::Bjorklund { pulse, step, rotation } => {
                        assert_eq!(pulse, 3);
                        assert_eq!(step, 8);
                        assert_eq!(rotation, None);
                    }
                    _ => panic!("Expected bjorklund arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_parse_cat() {
        let result = parse_pattern("cat [\"bd\", \"sd\"]").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "slowcat");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_parse_complex_pattern() {
        let result = parse_pattern("\"bd sd*2 [hh hh] cp@3\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "fastcat");
                assert_eq!(p.source_.len(), 4);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_parse_nested_operators() {
        let result = parse_pattern("slow 2 $ fast 3 $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op1) => {
                assert_eq!(op1.type_, "stretch");
                match *op1.source_ {
                    ParsedElement::Operator(op2) => {
                        assert_eq!(op2.type_, "stretch");
                    }
                    _ => panic!("Expected nested operator"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_parse_dots() {
        let result = parse_pattern("\"bd . sd . hh\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "feet");
                assert_eq!(p.source_.len(), 3);
            }
            _ => panic!("Expected pattern"),
        }
    }
}
