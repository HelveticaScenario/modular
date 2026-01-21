use modular_core::pattern_system::mini::parse_ast;
use modular_core::pattern_system::mini::parse;
use modular_core::pattern_system::Fraction;

fn main() {
    // Test slowcat - should alternate
    println!("=== <a b> (slowcat) over 4 cycles ===");
    let pat: modular_core::pattern_system::Pattern<f64> = parse("<a b>").unwrap();
    for cycle in 0..4 {
        let haps = pat.query_arc(Fraction::from_integer(cycle), Fraction::from_integer(cycle + 1));
        println!("Cycle {}: {:?}", cycle, haps.iter().map(|h| h.value).collect::<Vec<_>>());
    }
    // a = 69 (A4), b = 71 (B4)
    
    // Test sequence with slowcat
    println!("\n=== c <d e> (sequence with slowcat) over 4 cycles ===");
    let pat: modular_core::pattern_system::Pattern<f64> = parse("c <d e>").unwrap();
    for cycle in 0..4 {
        let haps = pat.query_arc(Fraction::from_integer(cycle), Fraction::from_integer(cycle + 1));
        println!("Cycle {}: {:?}", cycle, haps.iter().map(|h| h.value).collect::<Vec<_>>());
    }
    // c = 60, d = 62, e = 64
    
    // Original problem
    println!("\n=== c <d [e f]> over 4 cycles ===");
    let pat: modular_core::pattern_system::Pattern<f64> = parse("c <d [e f]>").unwrap();
    for cycle in 0..4 {
        let haps = pat.query_arc(Fraction::from_integer(cycle), Fraction::from_integer(cycle + 1));
        println!("Cycle {}: {:?}", cycle, haps.iter().map(|h| h.value).collect::<Vec<_>>());
    }
    // d = 62, e = 64, f = 65
    
    // Double accidentals
    println!("\n=== c3 c#3 cb3 cbb3 (sequence) ===");
    match parse_ast("c3 c#3 cb3 cbb3") {
        Ok(ast) => {
            if let modular_core::pattern_system::mini::MiniAST::Sequence(elems) = ast {
                println!("Parsed {} elements", elems.len());
            }
        }
        Err(e) => println!("ERROR: {:?}", e),
    }
}
