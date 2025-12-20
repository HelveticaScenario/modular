// Comprehensive tests for Krill mini-notation parser
// Tests each aspect of the grammar

use modular_core::pattern_parser::*;
use pest::Parser;

#[cfg(test)]
mod number_tests {
    use super::*;

    #[test]
    fn test_integer() {
        let result = parse_pattern("setcps 5").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                assert_eq!(cmd.options_.value, Some(5.0));
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_negative_integer() {
        let pairs = KrillParser::parse(Rule::intneg, "-42").unwrap();
        assert_eq!(pairs.as_str(), "-42");
    }

    #[test]
    fn test_decimal() {
        let result = parse_pattern("setcps 3.14").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                assert_eq!(cmd.options_.value, Some(3.14));
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_scientific_notation() {
        let pairs = KrillParser::parse(Rule::number, "1.5e-10").unwrap();
        assert_eq!(pairs.as_str(), "1.5e-10");
    }

    #[test]
    fn test_scientific_notation_positive_exp() {
        let pairs = KrillParser::parse(Rule::number, "2.5E+3").unwrap();
        assert_eq!(pairs.as_str(), "2.5E+3");
    }

    #[test]
    fn test_zero() {
        let pairs = KrillParser::parse(Rule::number, "0").unwrap();
        assert_eq!(pairs.as_str(), "0");
    }

    #[test]
    fn test_negative_decimal() {
        let pairs = KrillParser::parse(Rule::number, "-0.5").unwrap();
        assert_eq!(pairs.as_str(), "-0.5");
    }
}

#[cfg(test)]
mod step_tests {
    use super::*;

    #[test]
    fn test_simple_step() {
        let result = parse_pattern("\"bd\"").unwrap();
        match result {
            ParsedElement::Atom(atom) => {
                assert_eq!(atom.source_, "bd");
            }
            _ => panic!("Expected atom"),
        }
    }

    #[test]
    fn test_step_with_number() {
        let result = parse_pattern("\"kick1\"").unwrap();
        match result {
            ParsedElement::Atom(atom) => {
                assert_eq!(atom.source_, "kick1");
            }
            _ => panic!("Expected atom"),
        }
    }

    #[test]
    fn test_step_with_hash() {
        let result = parse_pattern("\"c#\"").unwrap();
        match result {
            ParsedElement::Atom(atom) => {
                assert_eq!(atom.source_, "c#");
            }
            _ => panic!("Expected atom"),
        }
    }

    #[test]
    fn test_step_with_tilde() {
        let result = parse_pattern("\"bd~\"").unwrap();
        match result {
            ParsedElement::Atom(atom) => {
                assert_eq!(atom.source_, "bd~");
            }
            _ => panic!("Expected atom"),
        }
    }

    #[test]
    fn test_step_with_caret() {
        let result = parse_pattern("\"bd^\"").unwrap();
        match result {
            ParsedElement::Atom(atom) => {
                assert_eq!(atom.source_, "bd^");
            }
            _ => panic!("Expected atom"),
        }
    }
}

#[cfg(test)]
mod sequence_tests {
    use super::*;

    #[test]
    fn test_two_element_sequence() {
        let result = parse_pattern("\"bd sd\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "fastcat");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_three_element_sequence() {
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
    fn test_sequence_with_steps_marker() {
        let result = parse_pattern("\"^bd sd hh\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "fastcat");
                assert_eq!(p.arguments_._steps, Some(true));
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_single_element_not_wrapped() {
        let result = parse_pattern("\"bd\"").unwrap();
        match result {
            ParsedElement::Atom(_) => {
                // Single elements should not be wrapped in patterns
            }
            _ => panic!("Expected atom, not pattern"),
        }
    }
}

#[cfg(test)]
mod stack_tests {
    use super::*;

    #[test]
    fn test_two_layer_stack() {
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
    fn test_three_layer_stack() {
        let result = parse_pattern("\"bd, sd, hh\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "stack");
                assert_eq!(p.source_.len(), 3);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_stack_with_sequences() {
        let result = parse_pattern("\"bd sd, hh cp\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "stack");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }
}

#[cfg(test)]
mod choose_tests {
    use super::*;

    #[test]
    fn test_choose_two_options() {
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
    fn test_choose_three_options() {
        let result = parse_pattern("\"bd | sd | hh\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "rand");
                assert_eq!(p.source_.len(), 3);
            }
            _ => panic!("Expected pattern"),
        }
    }
}

#[cfg(test)]
mod subcycle_tests {
    use super::*;

    #[test]
    fn test_subcycle_in_sequence() {
        let result = parse_pattern("\"bd [sd hh]\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "fastcat");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_nested_subcycle() {
        let result = parse_pattern("\"bd [sd [hh cp]]\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_subcycle_with_stack() {
        let result = parse_pattern("\"bd [sd, hh]\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }
}

#[cfg(test)]
mod slice_operator_tests {
    use super::*;

    #[test]
    fn test_weight_with_at_default() {
        let result = parse_pattern("\"bd@\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.weight, 2.0);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_weight_with_at_number() {
        let result = parse_pattern("\"bd@3\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.weight, 3.0);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_weight_with_underscore() {
        let result = parse_pattern("\"bd_4\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.weight, 4.0);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_replicate_default() {
        let result = parse_pattern("\"bd!\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.reps, 2);
                assert_eq!(e.options_.weight, 2.0);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_replicate_with_number() {
        let result = parse_pattern("\"bd!5\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.reps, 5);
                assert_eq!(e.options_.weight, 5.0);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_multiple_replicates() {
        let result = parse_pattern("\"bd!!!\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                // Each ! adds 1, so !!! = 1 + 2 + 2 + 2 = 4 (base 1, then +1 three times starting at 2)
                assert_eq!(e.options_.reps, 4);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_slow_operator() {
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
    fn test_fast_operator() {
        let result = parse_pattern("\"bd*3\"").unwrap();
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
    fn test_degrade_no_amount() {
        let result = parse_pattern("\"bd?\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(
                        op,
                        SliceOperation::DegradeBy {
                            arguments_: DegradeArgs { amount: None, .. }
                        }
                    )
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_degrade_with_amount() {
        let result = parse_pattern("\"bd?0.5\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(
                        op,
                        SliceOperation::DegradeBy {
                            arguments_: DegradeArgs { amount: Some(amt), .. }
                        } if *amt == 0.5
                    )
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_tail_operator() {
        let result = parse_pattern("\"bd:sd\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(op, SliceOperation::Tail { .. })
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_range_operator() {
        let result = parse_pattern("\"bd..sd\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(op, SliceOperation::Range { .. })
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_bjorklund_operator() {
        let result = parse_pattern("\"bd(3,8)\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(op, SliceOperation::Bjorklund { .. })
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_bjorklund_with_rotation() {
        let result = parse_pattern("\"bd(3,8,2)\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(
                        op,
                        SliceOperation::Bjorklund {
                            arguments_: BjorklundArgs {
                                rotation: Some(_),
                                ..
                            }
                        }
                    )
                }));
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_combined_operators() {
        let result = parse_pattern("\"bd@2*3\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.weight, 2.0);
                assert!(e.options_.ops.iter().any(|op| {
                    matches!(op, SliceOperation::Stretch { .. })
                }));
            }
            _ => panic!("Expected element"),
        }
    }
}

#[cfg(test)]
mod polymeter_tests {
    use super::*;

    #[test]
    fn test_simple_polymeter() {
        let result = parse_pattern("\"{bd sd, hh}\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "polymeter");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_polymeter_with_steps_per_cycle() {
        let result = parse_pattern("\"{bd sd, hh}%4\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "polymeter");
                assert!(p.arguments_.stepsPerCycle.is_some());
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_slow_sequence() {
        let result = parse_pattern("\"<bd sd hh>\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "polymeter_slowcat");
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_slow_sequence_with_stack() {
        let result = parse_pattern("\"<bd, sd, hh>\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "polymeter_slowcat");
                assert_eq!(p.source_.len(), 3);
            }
            _ => panic!("Expected pattern"),
        }
    }
}

#[cfg(test)]
mod operator_tests {
    use super::*;

    #[test]
    fn test_slow_operator() {
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
    fn test_fast_operator() {
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
    fn test_rotL_operator() {
        let result = parse_pattern("rotL 1 $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "shift");
                match &op.arguments_ {
                    OperatorArguments::Shift { amount } => {
                        assert_eq!(amount, "-1");
                    }
                    _ => panic!("Expected shift arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_rotR_operator() {
        let result = parse_pattern("rotR 2 $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "shift");
                match &op.arguments_ {
                    OperatorArguments::Shift { amount } => {
                        assert_eq!(amount, "2");
                    }
                    _ => panic!("Expected shift arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_scale_operator_single_quote() {
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
    fn test_scale_operator_double_quote() {
        let result = parse_pattern("scale \"minor\" $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "scale");
                match &op.arguments_ {
                    OperatorArguments::Scale { scale } => {
                        assert_eq!(scale, "minor");
                    }
                    _ => panic!("Expected scale arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_target_operator() {
        let result = parse_pattern("target \"kick\" $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "target");
                match &op.arguments_ {
                    OperatorArguments::Target { name } => {
                        assert_eq!(name, "kick");
                    }
                    _ => panic!("Expected target arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_bjorklund_operator_basic() {
        let result = parse_pattern("euclid 3 8 $ \"bd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "bjorklund");
                match op.arguments_ {
                    OperatorArguments::Bjorklund {
                        pulse,
                        step,
                        rotation,
                    } => {
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
    fn test_bjorklund_operator_with_rotation() {
        let result = parse_pattern("euclid 5 16 2 $ \"bd\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "bjorklund");
                match op.arguments_ {
                    OperatorArguments::Bjorklund {
                        pulse,
                        step,
                        rotation,
                    } => {
                        assert_eq!(pulse, 5);
                        assert_eq!(step, 16);
                        assert_eq!(rotation, Some(2));
                    }
                    _ => panic!("Expected bjorklund arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_struct_operator() {
        let result = parse_pattern("struct \"bd sd\" $ \"1 0 1\"").unwrap();
        match result {
            ParsedElement::Operator(op) => {
                assert_eq!(op.type_, "struct");
                match &op.arguments_ {
                    OperatorArguments::Struct { mini } => {
                        assert!(matches!(**mini, ParsedElement::Pattern(_)));
                    }
                    _ => panic!("Expected struct arguments"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_nested_operators() {
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
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn test_setcps_integer() {
        let result = parse_pattern("setcps 1").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                assert_eq!(cmd.name_, "setcps");
                assert_eq!(cmd.options_.value, Some(1.0));
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_setcps_decimal() {
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
    fn test_setbpm() {
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
    fn test_setbpm_60() {
        let result = parse_pattern("setbpm 60").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                // 60 bpm = 60/120/2 = 0.25 cps
                assert_eq!(cmd.options_.value, Some(0.25));
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_hush() {
        let result = parse_pattern("hush").unwrap();
        match result {
            ParsedElement::Command(cmd) => {
                assert_eq!(cmd.name_, "hush");
                assert_eq!(cmd.options_.value, None);
            }
            _ => panic!("Expected command"),
        }
    }
}

#[cfg(test)]
mod cat_tests {
    use super::*;

    #[test]
    fn test_cat_two_elements() {
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
    fn test_cat_three_elements() {
        let result = parse_pattern("cat [\"bd\", \"sd\", \"hh\"]").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "slowcat");
                assert_eq!(p.source_.len(), 3);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_cat_with_sequences() {
        let result = parse_pattern("cat [\"bd sd\", \"hh cp\"]").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "slowcat");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }
}

#[cfg(test)]
mod dot_notation_tests {
    use super::*;

    #[test]
    fn test_dot_two_elements() {
        let result = parse_pattern("\"bd . sd\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "feet");
                assert_eq!(p.source_.len(), 2);
                assert!(p.arguments_.seed.is_some());
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_dot_three_elements() {
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

#[cfg(test)]
mod complex_pattern_tests {
    use super::*;

    #[test]
    fn test_complex_pattern_1() {
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
    fn test_complex_pattern_with_stack_and_subcycle() {
        let result = parse_pattern("\"bd [sd hh], cp\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "stack");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_complex_pattern_with_multiple_modifiers() {
        let result = parse_pattern("\"bd@2*3/2!4\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                // Should have weight, fast, slow, and replicate
                assert!(e.options_.ops.len() >= 3);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_pattern_with_choose_and_stack() {
        let result = parse_pattern("\"bd | sd, hh\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                // Choose has precedence over stack in the grammar
                assert!(
                    p.arguments_.alignment == "rand" || p.arguments_.alignment == "stack"
                );
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_deep_nesting() {
        let result = parse_pattern("\"bd [sd [hh [cp]]]\"").unwrap();
        match result {
            ParsedElement::Pattern(_) => {
                // Should successfully parse deeply nested structure
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_polymeter_in_sequence() {
        let result = parse_pattern("\"bd {sd hh, cp}\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "fastcat");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }

    #[test]
    fn test_slow_sequence_in_pattern() {
        let result = parse_pattern("\"bd <sd hh cp>\"").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "fastcat");
                assert_eq!(p.source_.len(), 2);
            }
            _ => panic!("Expected pattern"),
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_whitespace_handling() {
        let result1 = parse_pattern("\"bd sd\"").unwrap();
        let result2 = parse_pattern("\"bd   sd\"").unwrap();
        let result3 = parse_pattern("\"  bd  sd  \"").unwrap();

        // All should parse to same structure
        match (result1, result2, result3) {
            (
                ParsedElement::Pattern(p1),
                ParsedElement::Pattern(p2),
                ParsedElement::Pattern(p3),
            ) => {
                assert_eq!(p1.source_.len(), 2);
                assert_eq!(p2.source_.len(), 2);
                assert_eq!(p3.source_.len(), 2);
            }
            _ => panic!("Expected patterns"),
        }
    }

    #[test]
    fn test_empty_operators() {
        // Default values should be used
        let result = parse_pattern("\"bd@\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.weight, 2.0);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_decimal_in_operators() {
        let result = parse_pattern("\"bd@2.5\"").unwrap();
        match result {
            ParsedElement::Element(e) => {
                assert_eq!(e.options_.weight, 2.5);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_quote_types() {
        let result1 = parse_pattern("\"bd\"").unwrap();
        let result2 = parse_pattern("'bd'").unwrap();

        // Both should parse successfully
        match (result1, result2) {
            (ParsedElement::Atom(a1), ParsedElement::Atom(a2)) => {
                assert_eq!(a1.source_, "bd");
                assert_eq!(a2.source_, "bd");
            }
            _ => panic!("Expected atoms"),
        }
    }

    #[test]
    fn test_unicode_letters() {
        // Test that unicode letters are accepted in steps
        let pairs = KrillParser::parse(Rule::step, "αβγ");
        assert!(pairs.is_ok());
    }

    #[test]
    fn test_all_step_special_chars() {
        // Test all allowed special characters in steps
        let chars = vec!["bd-1", "c#", "bd.", "bd^", "bd~"];
        for c in chars {
            let result = parse_pattern(&format!("\"{}\"", c));
            assert!(result.is_ok(), "Failed to parse: {}", c);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_tidal_pattern() {
        let result =
            parse_pattern("slow 2 $ \"bd*4 [sd cp]*2, hh(3,8)\" # scale 'major'").unwrap();
        // Should parse complex nested structure without panicking
        match result {
            ParsedElement::Operator(_) => {}
            _ => {}
        }
    }

    #[test]
    fn test_operator_chain() {
        let result = parse_pattern("slow 2 $ fast 3 $ rotR 1 $ \"bd sd\"").unwrap();
        match result {
            ParsedElement::Operator(op1) => {
                assert_eq!(op1.type_, "stretch");
                match *op1.source_ {
                    ParsedElement::Operator(op2) => {
                        assert_eq!(op2.type_, "stretch");
                        match *op2.source_ {
                            ParsedElement::Operator(op3) => {
                                assert_eq!(op3.type_, "shift");
                            }
                            _ => panic!("Expected third operator"),
                        }
                    }
                    _ => panic!("Expected second operator"),
                }
            }
            _ => panic!("Expected operator"),
        }
    }

    #[test]
    fn test_mixed_notation() {
        let result = parse_pattern("cat [\"bd*2\", \"<sd hh>\", \"{cp, mt}\"]").unwrap();
        match result {
            ParsedElement::Pattern(p) => {
                assert_eq!(p.arguments_.alignment, "slowcat");
                assert_eq!(p.source_.len(), 3);
            }
            _ => panic!("Expected pattern"),
        }
    }
}
