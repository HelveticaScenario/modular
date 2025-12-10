use std::collections::HashMap;

use crate::types::{Module, ModuleSchema, SampleableConstructor};

pub mod sine;
pub mod saw;
pub mod pulse;

// Plaits-inspired modules
pub mod plaits_fm;
pub mod plaits_va;
pub mod plaits_grain;
pub mod plaits_wavetable;
pub mod plaits_noise;
pub mod plaits_modal;
pub mod plaits_string;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    sine::SineOscillator::install_constructor(map);
    saw::SawOscillator::install_constructor(map);
    pulse::PulseOscillator::install_constructor(map);
    
    // Plaits modules
    plaits_fm::PlaitsFM::install_constructor(map);
    plaits_va::PlaitsVA::install_constructor(map);
    plaits_grain::PlaitsGrain::install_constructor(map);
    plaits_wavetable::PlaitsWavetable::install_constructor(map);
    plaits_noise::PlaitsNoise::install_constructor(map);
    plaits_modal::PlaitsModal::install_constructor(map);
    plaits_string::PlaitsString::install_constructor(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        sine::SineOscillator::get_schema(),
        saw::SawOscillator::get_schema(),
        pulse::PulseOscillator::get_schema(),
        
        // Plaits modules
        plaits_fm::PlaitsFM::get_schema(),
        plaits_va::PlaitsVA::get_schema(),
        plaits_grain::PlaitsGrain::get_schema(),
        plaits_wavetable::PlaitsWavetable::get_schema(),
        plaits_noise::PlaitsNoise::get_schema(),
        plaits_modal::PlaitsModal::get_schema(),
        plaits_string::PlaitsString::get_schema(),
    ]
}
