use modular_core::dsp::schema;

fn main() {
    let schemas = schema();
    let json = serde_json::to_string_pretty(&schemas).expect("failed to serialize schemas");
    print!("{json}");
}
