//! Tests for Plaits-inspired synthesis modules

use modular_core::dsp::get_constructors;
use modular_core::patch::Patch;
use modular_core::types::InternalParam;
use std::collections::HashMap;

const SAMPLE_RATE: f32 = 48000.0;

/// Create a patch with a root module
fn create_test_patch() -> Patch {
    let mut sampleables = HashMap::new();
    let constructors = get_constructors();
    
    if let Some(constructor) = constructors.get("signal") {
        if let Ok(module) = constructor(&"root".to_string(), SAMPLE_RATE) {
            sampleables.insert("root".to_string(), module);
        }
    }
    
    Patch::new(sampleables, HashMap::new())
}

/// Add a module to the patch
fn add_module(patch: &mut Patch, id: &str, module_type: &str) {
    let constructors = get_constructors();
    if let Some(constructor) = constructors.get(module_type) {
        if let Ok(module) = constructor(&id.to_string(), SAMPLE_RATE) {
            patch.sampleables.insert(id.to_string(), module);
        }
    }
}

/// Process one frame of audio
fn process_frame(patch: &Patch) {
    for (_, module) in patch.sampleables.iter() {
        module.tick();
    }
    for (_, module) in patch.sampleables.iter() {
        module.update();
    }
}

/// Get a sample from a module
fn get_sample(patch: &Patch, module_id: &str, port: &str) -> f32 {
    patch.sampleables
        .get(module_id)
        .and_then(|m| m.get_sample(&port.to_string()).ok())
        .unwrap_or(0.0)
}

#[test]
fn test_plaits_fm_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "fm-1", "plaits-fm");
    
    // Set frequency to 4.0 v/oct (A4 = 440Hz at standard tuning)
    if let Some(module) = patch.sampleables.get("fm-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Volts { value: 4.0 },
        );
    }
    
    // Process several frames
    let mut samples = Vec::new();
    for _ in 0..100 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "fm-1", "output"));
    }
    
    // Verify we get non-zero output
    let max_sample = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_sample = samples.iter().cloned().fold(f32::INFINITY, f32::min);
    
    assert!(max_sample > 0.1, "FM oscillator should produce positive samples");
    assert!(min_sample < -0.1, "FM oscillator should produce negative samples");
    assert!(max_sample < 6.0, "FM output should be within ±5V range");
    assert!(min_sample > -6.0, "FM output should be within ±5V range");
}

#[test]
fn test_plaits_va_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "va-1", "plaits-va");
    
    if let Some(module) = patch.sampleables.get("va-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Volts { value: 4.0 },
        );
        // Also set timbre to ensure we're not in a zero-output state
        let _ = module.update_param(
            &"timbre".to_string(),
            &InternalParam::Volts { value: 0.5 },
        );
    }
    
    // Process some frames to let smoothing settle
    for _ in 0..50 {
        process_frame(&patch);
    }
    
    let mut samples = Vec::new();
    for _ in 0..100 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "va-1", "output"));
    }
    
    let max_sample = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_sample = samples.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(max_sample > 0.1, "VA oscillator should produce positive output");
    assert!(min_sample < -0.1, "VA oscillator should produce negative output");
}

#[test]
fn test_plaits_grain_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "grain-1", "plaits-grain");
    
    if let Some(module) = patch.sampleables.get("grain-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Volts { value: 4.0 },
        );
        // Set harmonics to increase grain density
        let _ = module.update_param(
            &"harmonics".to_string(),
            &InternalParam::Volts { value: 0.5 },
        );
    }
    
    // Process some frames to let smoothing settle first
    for _ in 0..100 {
        process_frame(&patch);
    }
    
    // Process enough frames for grains to trigger
    let mut samples = Vec::new();
    for _ in 0..2000 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "grain-1", "output"));
    }
    
    // Should have some non-zero samples from grains
    let has_output = samples.iter().any(|&s| s.abs() > 0.01);
    assert!(has_output, "Grain engine should produce output");
}

#[test]
fn test_plaits_wavetable_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "wt-1", "plaits-wavetable");
    
    if let Some(module) = patch.sampleables.get("wt-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Volts { value: 4.0 },
        );
    }
    
    let mut samples = Vec::new();
    for _ in 0..100 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "wt-1", "output"));
    }
    
    let max_sample = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(max_sample > 0.1, "Wavetable should produce output");
}

#[test]
fn test_plaits_noise_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "noise-1", "plaits-noise");
    
    let mut samples = Vec::new();
    for _ in 0..100 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "noise-1", "output"));
    }
    
    // Noise should produce varying output
    let has_variation = samples.windows(2).any(|w| (w[0] - w[1]).abs() > 0.01);
    assert!(has_variation, "Noise engine should produce varying output");
}

#[test]
fn test_plaits_modal_with_trigger() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "modal-1", "plaits-modal");
    
    if let Some(module) = patch.sampleables.get("modal-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Volts { value: 4.0 },
        );
        // Trigger the modal resonators
        let _ = module.update_param(
            &"trigger".to_string(),
            &InternalParam::Volts { value: 1.0 },
        );
    }
    
    // Process frames and collect samples
    let mut samples = Vec::new();
    for _ in 0..500 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "modal-1", "output"));
    }
    
    // Should have output after trigger
    let has_output = samples.iter().any(|&s| s.abs() > 0.1);
    assert!(has_output, "Modal engine should produce output after trigger");
}

#[test]
fn test_plaits_string_with_trigger() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "string-1", "plaits-string");
    
    if let Some(module) = patch.sampleables.get("string-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Volts { value: 4.0 },
        );
        // Pluck the string
        let _ = module.update_param(
            &"trigger".to_string(),
            &InternalParam::Volts { value: 1.0 },
        );
    }
    
    // Process frames
    let mut samples = Vec::new();
    for _ in 0..500 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "string-1", "output"));
    }
    
    // String should produce output after pluck
    let has_output = samples.iter().any(|&s| s.abs() > 0.1);
    assert!(has_output, "String engine should produce output after pluck");
}

#[test]
fn test_plaits_modules_respect_voltage_range() {
    let module_types = vec![
        "plaits-fm",
        "plaits-va",
        "plaits-wavetable",
        "plaits-noise",
    ];
    
    for module_type in module_types {
        let mut patch = create_test_patch();
        add_module(&mut patch, "test-1", module_type);
        
        if let Some(module) = patch.sampleables.get("test-1") {
            let _ = module.update_param(
                &"freq".to_string(),
                &InternalParam::Volts { value: 4.0 },
            );
        }
        
        // Process many frames
        for _ in 0..1000 {
            process_frame(&patch);
            let sample = get_sample(&patch, "test-1", "output");
            
            // Verify output is within expected range (±5V with small margin)
            assert!(
                sample.abs() < 6.0,
                "{} output {} exceeds ±5V range",
                module_type,
                sample
            );
        }
    }
}
