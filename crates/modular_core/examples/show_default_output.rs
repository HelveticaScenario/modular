// Example demonstrating the default output feature
//
// Run with: cargo run --example show_default_output

use modular_core::dsp;

fn main() {
    println!("Module Schemas with Default Output Information\n");
    println!("==============================================\n");

    let schemas = dsp::schema();

    // Find modules with multiple outputs
    for schema in schemas.iter() {
        if schema.outputs.len() > 1 {
            println!("Module: {} ({})", schema.name, schema.description);
            println!("  Outputs:");
            for output in &schema.outputs {
                let default_marker = if output.default { " [DEFAULT]" } else { "" };
                println!(
                    "    - {}: {}{}",
                    output.name, output.description, default_marker
                );
            }
            println!();
        }
    }

    // Show a specific example: state-variable-filter
    println!("\nDetailed Example: State Variable Filter");
    println!("========================================");

    if let Some(svf) = schemas.iter().find(|s| s.name == "state-variable-filter") {
        println!("Module: {}", svf.name);
        println!("Description: {}", svf.description);
        println!("\nOutputs:");
        for output in &svf.outputs {
            println!("  - Name: {}", output.name);
            println!("    Description: {}", output.description);
            println!("    Default: {}", output.default);
            println!();
        }

        // Serialize to JSON to show how it looks
        println!("JSON representation:");
        let json = serde_json::to_string_pretty(&svf.outputs).unwrap();
        println!("{}", json);
    }
}
