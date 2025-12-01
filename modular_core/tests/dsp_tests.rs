//! Integration tests for modular_core DSP functionality
//! 
//! These tests verify that the DSP modules produce correct audio output
//! by checking sample values rather than listening to audio.

use modular_core::dsp::get_constructors;
use modular_core::patch::Patch;
use modular_core::types::{InternalParam, Param, ROOT_ID};
use std::collections::HashMap;
use std::sync::Arc;

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

/// Process one frame of audio (tick + update all modules)
fn process_frame(patch: &Patch) {
    // Tick all modules (reset processed flag)
    for (_, module) in patch.sampleables.iter() {
        module.tick();
    }
    // Update all modules (compute new samples)
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
fn test_sine_oscillator_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "sine-1", "sine-oscillator");
    
    // Set frequency to 4.0 v/oct (440Hz)
    if let Some(module) = patch.sampleables.get("sine-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 4.0 },
        );
    }
    
    // Process several frames and collect samples
    let mut samples = Vec::new();
    for _ in 0..100 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "sine-1", "output"));
    }
    
    // Verify we get non-zero output (sine wave should oscillate)
    let max_sample = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_sample = samples.iter().cloned().fold(f32::INFINITY, f32::min);
    
    // Sine wave should have amplitude ~5.0 (from the oscillator implementation)
    assert!(max_sample > 0.0, "Sine oscillator should produce positive samples");
    assert!(min_sample < max_sample, "Sine oscillator should produce varying samples");
}

#[test]
fn test_sine_oscillator_amplitude() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "sine-1", "sine-oscillator");
    
    // Set frequency to produce several cycles
    if let Some(module) = patch.sampleables.get("sine-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 4.0 },
        );
    }
    
    // Process enough frames to see full amplitude range
    let mut max_sample = f32::NEG_INFINITY;
    let mut min_sample = f32::INFINITY;
    
    for _ in 0..1000 {
        process_frame(&patch);
        let sample = get_sample(&patch, "sine-1", "output");
        max_sample = max_sample.max(sample);
        min_sample = min_sample.min(sample);
    }
    
    // The oscillator outputs in range [-5, 5] based on the implementation
    assert!(max_sample >= 4.0, "Max amplitude should be near 5.0, got {}", max_sample);
    assert!(min_sample <= -4.0, "Min amplitude should be near -5.0, got {}", min_sample);
}

#[test]
fn test_cable_connection() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "sine-1", "sine-oscillator");
    
    // Set sine frequency
    if let Some(module) = patch.sampleables.get("sine-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 4.0 },
        );
    }
    
    // Connect sine to root
    if let Some(root) = patch.sampleables.get(&*ROOT_ID) {
        let sine_module = patch.sampleables.get("sine-1").unwrap();
        let _ = root.update_param(
            &"source".to_string(),
            &InternalParam::Cable {
                module: Arc::downgrade(sine_module),
                port: "output".to_string(),
            },
        );
    }
    
    // Process and verify root outputs sine values
    let mut samples = Vec::new();
    for _ in 0..100 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "root", "output"));
    }
    
    let max_sample = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(max_sample > 0.0, "Root should receive signal from sine oscillator");
}

#[test]
fn test_scale_and_shift_module() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "sine-1", "sine-oscillator");
    add_module(&mut patch, "scaler", "scale-and-shift");
    
    // Set sine frequency
    if let Some(module) = patch.sampleables.get("sine-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 4.0 },
        );
    }
    
    // Connect sine to scaler input
    if let Some(scaler) = patch.sampleables.get("scaler") {
        let sine_module = patch.sampleables.get("sine-1").unwrap();
        let _ = scaler.update_param(
            &"input".to_string(),
            &InternalParam::Cable {
                module: Arc::downgrade(sine_module),
                port: "output".to_string(),
            },
        );
        // Set scale to 0.5 (halve amplitude)
        let _ = scaler.update_param(
            &"scale".to_string(),
            &InternalParam::Value { value: 0.5 },
        );
    }
    
    // Process and collect samples
    let mut sine_samples = Vec::new();
    let mut scaled_samples = Vec::new();
    
    for _ in 0..100 {
        process_frame(&patch);
        sine_samples.push(get_sample(&patch, "sine-1", "output"));
        scaled_samples.push(get_sample(&patch, "scaler", "output"));
    }
    
    // Verify scaling works
    let sine_max = sine_samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let scaled_max = scaled_samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    
    // Scaled should be approximately half of original
    assert!(scaled_max < sine_max, "Scaled output should be smaller than original");
    assert!(scaled_max > 0.0, "Scaled output should still be positive");
}

#[test]
fn test_patch_get_state() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "sine-1", "sine-oscillator");
    
    // Set a parameter
    if let Some(module) = patch.sampleables.get("sine-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 4.0 },
        );
    }
    
    let state = patch.get_state();
    
    // Should have root and sine-1
    assert_eq!(state.len(), 2);
    
    // Find sine-1 in state
    let sine_state = state.iter().find(|m| m.id == "sine-1");
    assert!(sine_state.is_some());
    
    let sine_state = sine_state.unwrap();
    assert_eq!(sine_state.module_type, "sine-oscillator");
    
    // Check freq param
    if let Some(Param::Value { value }) = sine_state.params.get("freq") {
        assert!((value - 4.0).abs() < 0.01, "Freq param should be 4.0");
    } else {
        panic!("Freq param should be a Value");
    }
}

#[test]
fn test_disconnected_input_produces_zero() {
    let patch = create_test_patch();
    
    // Root with no input connected should produce 0
    process_frame(&patch);
    let sample = get_sample(&patch, "root", "output");
    
    assert!((sample - 0.0).abs() < 0.001, "Disconnected input should produce 0");
}

#[test]
fn test_saw_oscillator_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "saw-1", "saw-oscillator");
    
    // Set frequency
    if let Some(module) = patch.sampleables.get("saw-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 4.0 },
        );
    }
    
    // Process several frames - need more iterations for saw to ramp up
    let mut samples = Vec::new();
    for _ in 0..500 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "saw-1", "output"));
    }
    
    let max_sample = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_sample = samples.iter().cloned().fold(f32::INFINITY, f32::min);
    
    // Saw oscillator should oscillate between -5 and 5
    assert!(max_sample > 1.0 || min_sample < -1.0, 
        "Saw oscillator should produce varying samples, got max={}, min={}", max_sample, min_sample);
}

#[test]
fn test_pulse_oscillator_produces_output() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "pulse-1", "pulse-oscillator");
    
    // Set frequency
    if let Some(module) = patch.sampleables.get("pulse-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 4.0 },
        );
        // Set pulse width to square wave (2.5)
        let _ = module.update_param(
            &"width".to_string(),
            &InternalParam::Value { value: 2.5 },
        );
    }
    
    // Process several frames - need more for pulse to complete cycles
    let mut samples = Vec::new();
    for _ in 0..500 {
        process_frame(&patch);
        samples.push(get_sample(&patch, "pulse-1", "output"));
    }
    
    let max_sample = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_sample = samples.iter().cloned().fold(f32::INFINITY, f32::min);
    
    // Pulse oscillator alternates between +5 and -5
    assert!(max_sample > 1.0 || min_sample < -1.0, 
        "Pulse oscillator should produce varying samples, got max={}, min={}", max_sample, min_sample);
}

#[test]
fn test_lowpass_filter() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "sine-1", "sine-oscillator");
    add_module(&mut patch, "lp", "lowpass");
    
    // High frequency sine
    if let Some(module) = patch.sampleables.get("sine-1") {
        let _ = module.update_param(
            &"freq".to_string(),
            &InternalParam::Value { value: 8.0 }, // High frequency
        );
    }
    
    // Connect sine to filter input and set low cutoff
    if let Some(lp) = patch.sampleables.get("lp") {
        let sine_module = patch.sampleables.get("sine-1").unwrap();
        let _ = lp.update_param(
            &"input".to_string(),
            &InternalParam::Cable {
                module: Arc::downgrade(sine_module),
                port: "output".to_string(),
            },
        );
        // Very low cutoff to attenuate high frequencies
        let _ = lp.update_param(
            &"cutoff".to_string(),
            &InternalParam::Value { value: 1.0 },
        );
    }
    
    // Process and compare
    let mut sine_samples = Vec::new();
    let mut filtered_samples = Vec::new();
    
    for _ in 0..500 {
        process_frame(&patch);
        sine_samples.push(get_sample(&patch, "sine-1", "output"));
        filtered_samples.push(get_sample(&patch, "lp", "output"));
    }
    
    // Filter should attenuate the signal
    let sine_rms: f32 = (sine_samples.iter().map(|s| s * s).sum::<f32>() / sine_samples.len() as f32).sqrt();
    let filtered_rms: f32 = (filtered_samples.iter().map(|s| s * s).sum::<f32>() / filtered_samples.len() as f32).sqrt();
    
    assert!(filtered_rms < sine_rms, "Lowpass filter should attenuate high frequency signal");
}

#[test]
fn test_sum_module() {
    let mut patch = create_test_patch();
    add_module(&mut patch, "sine-1", "sine-oscillator");
    add_module(&mut patch, "sine-2", "sine-oscillator");
    add_module(&mut patch, "summer", "sum");
    
    // Set different frequencies
    if let Some(module) = patch.sampleables.get("sine-1") {
        let _ = module.update_param(&"freq".to_string(), &InternalParam::Value { value: 4.0 });
    }
    if let Some(module) = patch.sampleables.get("sine-2") {
        let _ = module.update_param(&"freq".to_string(), &InternalParam::Value { value: 5.0 });
    }
    
    // Connect both sines to sum (using correct param names: input-1, input-2)
    if let Some(summer) = patch.sampleables.get("summer") {
        let sine1 = patch.sampleables.get("sine-1").unwrap();
        let sine2 = patch.sampleables.get("sine-2").unwrap();
        let _ = summer.update_param(
            &"input-1".to_string(),
            &InternalParam::Cable { module: Arc::downgrade(sine1), port: "output".to_string() },
        );
        let _ = summer.update_param(
            &"input-2".to_string(),
            &InternalParam::Cable { module: Arc::downgrade(sine2), port: "output".to_string() },
        );
    }
    
    // Process and verify sum has larger amplitude than individual
    let mut sum_samples = Vec::new();
    for _ in 0..500 {
        process_frame(&patch);
        sum_samples.push(get_sample(&patch, "summer", "output"));
    }
    
    let sum_max = sum_samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    
    // Sum of two ~5V signals can reach ~10V at constructive interference
    assert!(sum_max > 5.0, "Sum module should combine signals, got max {}", sum_max);
}
