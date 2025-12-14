# Data Parameters: Implementation Examples

This document provides concrete code examples for implementing the data parameters strategy. These examples show how the system would work once implemented, but **no code changes should be made yet** - this is for planning purposes only.

## Table of Contents

1. [Simple Data Parameter Module](#simple-data-parameter-module)
2. [Complex Module with Multiple Data Types](#complex-module-with-multiple-data-types)
3. [Validation Examples](#validation-examples)
4. [DSL Usage Patterns](#dsl-usage-patterns)
5. [Migration Examples](#migration-examples)

## Simple Data Parameter Module

### Example: Sample & Hold with Mode Selection

```rust
// modular_core/src/dsp/utilities/sample_and_hold.rs

use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};
use anyhow::Result;

/// Audio parameters (real-time signals)
#[derive(Default, Params)]
struct SampleAndHoldParams {
    #[param("input", "signal to sample")]
    input: InternalParam,
    #[param("trigger", "trigger input (rising edge samples)")]
    trigger: InternalParam,
}

/// Data parameters (static configuration)
#[derive(Default, DataParams)]
struct SampleAndHoldData {
    #[data_param("mode", "sampling mode", enum("edge", "gate", "continuous"))]
    mode: String,
    
    #[data_param("smooth", "enable interpolation", bool)]
    smooth: bool,
}

#[derive(Module)]
#[module("sample-and-hold", "Sample and hold utility")]
pub struct SampleAndHold {
    #[output("output", "held output", default)]
    output: ChannelBuffer,
    
    held_value: ChannelBuffer,
    last_trigger: ChannelBuffer,
    
    params: SampleAndHoldParams,
    data: SampleAndHoldData,
}

impl Default for SampleAndHold {
    fn default() -> Self {
        Self {
            output: ChannelBuffer::default(),
            held_value: ChannelBuffer::default(),
            last_trigger: ChannelBuffer::default(),
            params: SampleAndHoldParams::default(),
            data: SampleAndHoldData {
                mode: "edge".to_string(),
                smooth: false,
            },
        }
    }
}

impl SampleAndHold {
    fn update(&mut self, sample_rate: f32) {
        let mut input = ChannelBuffer::default();
        let mut trigger = ChannelBuffer::default();
        
        self.params.input.get_value(&mut input);
        self.params.trigger.get_value(&mut trigger);
        
        for i in 0..NUM_CHANNELS {
            let should_sample = match self.data.mode.as_str() {
                "edge" => trigger[i] > 0.0 && self.last_trigger[i] <= 0.0,
                "gate" => trigger[i] > 0.0,
                "continuous" => true,
                _ => false,
            };
            
            if should_sample {
                if self.data.smooth {
                    // Smooth interpolation
                    self.held_value[i] = crate::types::smooth_value(self.held_value[i], input[i]);
                } else {
                    // Immediate update
                    self.held_value[i] = input[i];
                }
            }
            
            self.output[i] = self.held_value[i];
            self.last_trigger[i] = trigger[i];
        }
    }
}
```

**DSL Usage**:
```javascript
const input = sine("sig").freq(hz(440));
const clock = pulse("clk").freq(hz(2));

// Create with data params in constructor
const sh = sampleAndHold("sh1", { 
    mode: "edge", 
    smooth: true 
});

sh.input(input);
sh.trigger(clock);

out.source(sh);
```

## Complex Module with Multiple Data Types

### Example: Sequencer with Step Data

```rust
// modular_core/src/dsp/utilities/sequencer.rs

use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};
use anyhow::Result;

#[derive(Default, Params)]
struct SequencerParams {
    #[param("clock", "clock input (rising edge advances step)")]
    clock: InternalParam,
    
    #[param("reset", "reset input (rising edge goes to step 0)")]
    reset: InternalParam,
}

#[derive(Default, DataParams)]
struct SequencerData {
    #[data_param("steps", "sequence step values", array(float(-10.0, 10.0), 1, 32))]
    steps: Vec<f64>,
    
    #[data_param("length", "number of active steps", int(1, 32))]
    length: i64,
    
    #[data_param("direction", "playback direction", enum("forward", "reverse", "pingpong", "random"))]
    direction: String,
    
    #[data_param("quantize", "quantize to semitones", bool)]
    quantize: bool,
}

#[derive(Module)]
#[module("sequencer", "CV sequencer")]
pub struct Sequencer {
    #[output("cv", "control voltage output", default)]
    cv_out: ChannelBuffer,
    
    #[output("gate", "gate output (5V on active steps)")]
    gate_out: ChannelBuffer,
    
    current_step: usize,
    last_clock: f32,
    last_reset: f32,
    ping_pong_direction: i32,
    
    params: SequencerParams,
    data: SequencerData,
}

impl Default for Sequencer {
    fn default() -> Self {
        Self {
            cv_out: ChannelBuffer::default(),
            gate_out: ChannelBuffer::default(),
            current_step: 0,
            last_clock: 0.0,
            last_reset: 0.0,
            ping_pong_direction: 1,
            params: SequencerParams::default(),
            data: SequencerData {
                steps: vec![0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0, 12.0],
                length: 8,
                direction: "forward".to_string(),
                quantize: false,
            },
        }
    }
}

impl Sequencer {
    fn update(&mut self, sample_rate: f32) {
        let mut clock = ChannelBuffer::default();
        let mut reset = ChannelBuffer::default();
        
        self.params.clock.get_value(&mut clock);
        self.params.reset.get_value(&mut reset);
        
        // Only process on channel 0 for step sequencing
        // Reset detection
        if reset[0] > 0.0 && self.last_reset <= 0.0 {
            self.current_step = 0;
            self.ping_pong_direction = 1;
        }
        self.last_reset = reset[0];
        
        // Clock detection
        if clock[0] > 0.0 && self.last_clock <= 0.0 {
            self.advance_step();
        }
        self.last_clock = clock[0];
        
        // Output current step value
        let step_value = if self.current_step < self.data.steps.len() {
            let mut value = self.data.steps[self.current_step];
            
            if self.data.quantize {
                // Quantize to nearest semitone (1/12 of a volt in v/oct)
                value = (value * 12.0).round() / 12.0;
            }
            
            value as f32
        } else {
            0.0
        };
        
        // Broadcast to all channels
        for i in 0..NUM_CHANNELS {
            self.cv_out[i] = step_value;
            self.gate_out[i] = 5.0; // Always high when playing
        }
    }
    
    fn advance_step(&mut self) {
        let length = self.data.length.min(self.data.steps.len() as i64) as usize;
        
        match self.data.direction.as_str() {
            "forward" => {
                self.current_step = (self.current_step + 1) % length;
            }
            "reverse" => {
                self.current_step = if self.current_step == 0 {
                    length - 1
                } else {
                    self.current_step - 1
                };
            }
            "pingpong" => {
                self.current_step = (self.current_step as i32 + self.ping_pong_direction) as usize;
                if self.current_step >= length - 1 {
                    self.ping_pong_direction = -1;
                    self.current_step = length - 1;
                } else if self.current_step == 0 {
                    self.ping_pong_direction = 1;
                }
            }
            "random" => {
                // Would need RNG - simplified for example
                self.current_step = (self.current_step + 3) % length;
            }
            _ => {}
        }
    }
}
```

**DSL Usage**:
```javascript
const clock = pulse("clk").freq(hz(4));

// Create sequencer with step data
const seq = sequencer("seq1", {
    steps: [
        note("c4"),
        note("e4"),
        note("g4"),
        note("c5"),
        note("g4"),
        note("e4"),
        note("c4"),
        note("c3"),
    ],
    length: 8,
    direction: "pingpong",
    quantize: true,
});

seq.clock(clock);

// Use sequence to control oscillator
const osc = sine("osc1").freq(seq.cv);
out.source(osc);
```

## Validation Examples

### Type Validation

```rust
// In validation.rs

fn validate_data_param(
    param_name: &str,
    param_value: &DataParam,
    param_schema: &ParamSchema,
    location: &str,
) -> Result<(), ValidationError> {
    match (&param_schema.param_type, param_value) {
        // String validation
        (ParamType::String, DataParam::String { value }) => {
            Ok(())
        }
        
        // Int validation with bounds
        (ParamType::Int { min, max }, DataParam::Int { value }) => {
            if let Some(min_val) = min {
                if *value < *min_val {
                    return Err(ValidationError::with_location(
                        param_name,
                        format!("Value {} is below minimum {}", value, min_val),
                        location,
                    ));
                }
            }
            if let Some(max_val) = max {
                if *value > *max_val {
                    return Err(ValidationError::with_location(
                        param_name,
                        format!("Value {} is above maximum {}", value, max_val),
                        location,
                    ));
                }
            }
            Ok(())
        }
        
        // Enum validation
        (ParamType::Enum { variants }, DataParam::Enum { variant }) => {
            if !variants.contains(variant) {
                return Err(ValidationError::with_location(
                    param_name,
                    format!(
                        "Invalid variant '{}'. Must be one of: {}",
                        variant,
                        variants.join(", ")
                    ),
                    location,
                ));
            }
            Ok(())
        }
        
        // Array validation
        (
            ParamType::Array { element_type, min_length, max_length },
            DataParam::Array { values }
        ) => {
            // Check length constraints
            if let Some(min_len) = min_length {
                if values.len() < *min_len {
                    return Err(ValidationError::with_location(
                        param_name,
                        format!("Array length {} is below minimum {}", values.len(), min_len),
                        location,
                    ));
                }
            }
            if let Some(max_len) = max_length {
                if values.len() > *max_len {
                    return Err(ValidationError::with_location(
                        param_name,
                        format!("Array length {} exceeds maximum {}", values.len(), max_len),
                        location,
                    ));
                }
            }
            
            // Validate each element
            for (i, elem) in values.iter().enumerate() {
                let elem_location = format!("{}[{}]", location, i);
                validate_data_param_type(element_type, elem, &elem_location)?;
            }
            Ok(())
        }
        
        // Type mismatch
        _ => {
            Err(ValidationError::with_location(
                param_name,
                format!(
                    "Type mismatch: expected {:?}, got {:?}",
                    param_schema.param_type, param_value
                ),
                location,
            ))
        }
    }
}
```

**Test Cases**:

```rust
#[test]
fn test_validate_enum_valid() {
    let schema = ParamSchema {
        name: "mode".to_string(),
        description: "Filter mode".to_string(),
        param_type: ParamType::Enum {
            variants: vec!["lowpass".to_string(), "highpass".to_string()],
        },
        optional: false,
        default: None,
    };
    
    let param = DataParam::Enum {
        variant: "lowpass".to_string(),
    };
    
    assert!(validate_data_param("mode", &param, &schema, "test").is_ok());
}

#[test]
fn test_validate_enum_invalid_variant() {
    let schema = ParamSchema {
        name: "mode".to_string(),
        description: "Filter mode".to_string(),
        param_type: ParamType::Enum {
            variants: vec!["lowpass".to_string(), "highpass".to_string()],
        },
        optional: false,
        default: None,
    };
    
    let param = DataParam::Enum {
        variant: "bandpass".to_string(),
    };
    
    let result = validate_data_param("mode", &param, &schema, "test");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Invalid variant"));
}

#[test]
fn test_validate_array_bounds() {
    let schema = ParamSchema {
        name: "steps".to_string(),
        description: "Sequence steps".to_string(),
        param_type: ParamType::Array {
            element_type: Box::new(ParamType::Float { min: Some(-10.0), max: Some(10.0) }),
            min_length: Some(1),
            max_length: Some(16),
        },
        optional: false,
        default: None,
    };
    
    // Too many elements
    let param = DataParam::Array {
        values: vec![DataParam::Float { value: 0.0 }; 20],
    };
    
    let result = validate_data_param("steps", &param, &schema, "test");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("exceeds maximum"));
}
```

## DSL Usage Patterns

### Pattern 1: Configuration in Constructor

```javascript
// Preferred for data that rarely changes
const filter = lowpass("lpf", {
    filterType: "butterworth",
    order: 4,
    resonance: 0.7
});

filter.cutoff(hz(1000));
filter.input(osc);
```

### Pattern 2: Fluent API Methods

```javascript
// Alternative API for common data params
const osc = wavetable("osc")
    .waveform("saw")
    .octave(1)
    .freq(note("a4"));
```

### Pattern 3: Array/Object Data

```javascript
// Complex configuration data
const reverb = reverb("verb", {
    algorithm: "plate",
    earlyReflections: [
        { time: 0.01, gain: 0.8 },
        { time: 0.02, gain: 0.6 },
        { time: 0.03, gain: 0.4 },
    ],
    preDelay: 0.05,
    roomSize: 0.8,
});

reverb.input(signal);
```

### Pattern 4: Preset Systems

```javascript
// Load predefined configurations
const delay = delay("d1", presets.delay.tape);
// Where presets.delay.tape = { mode: "tape", saturation: true, wow: 0.1, flutter: 0.05 }

// Or factory functions
const ks = karplusStrong("ks", stringPreset("guitar", "e2"));
```

### Pattern 5: Computed Data

```javascript
// Helper functions to generate data
function majorScale() {
    return [0, 2, 4, 5, 7, 9, 11].map(s => s / 12);
}

const quantizer = quantize("q1", {
    scale: majorScale(),
    rootNote: note("c4")
});
```

## Migration Examples

### Before: Encoding as Voltages

```rust
// Old approach: Use voltage to represent mode
#[derive(Default, Params)]
struct FilterParams {
    #[param("input", "audio input")]
    input: InternalParam,
    
    #[param("cutoff", "cutoff frequency")]
    cutoff: InternalParam,
    
    // Mode encoded as voltage: 0V = lowpass, 1V = highpass, 2V = bandpass
    #[param("mode", "filter mode (0=LP, 1=HP, 2=BP)")]
    mode: InternalParam,
}

impl Filter {
    fn update(&mut self, sample_rate: f32) {
        let mut mode_voltage = [0.0; NUM_CHANNELS];
        self.params.mode.get_value(&mut mode_voltage);
        
        let filter_mode = if mode_voltage[0] < 0.5 {
            FilterMode::LowPass
        } else if mode_voltage[0] < 1.5 {
            FilterMode::HighPass
        } else {
            FilterMode::BandPass
        };
        
        // ... rest of implementation
    }
}
```

**DSL (Old)**:
```javascript
// Unergonomic: user must know voltage encoding
const filt = filter("f1").mode(0.0);  // What does 0.0 mean?
```

### After: Using Data Parameters

```rust
// New approach: Use data parameter
#[derive(Default, Params)]
struct FilterParams {
    #[param("input", "audio input")]
    input: InternalParam,
    
    #[param("cutoff", "cutoff frequency")]
    cutoff: InternalParam,
}

#[derive(Default, DataParams)]
struct FilterData {
    #[data_param("mode", "filter mode", enum("lowpass", "highpass", "bandpass"))]
    mode: String,
}

impl Filter {
    fn update(&mut self, sample_rate: f32) {
        let filter_mode = match self.data.mode.as_str() {
            "lowpass" => FilterMode::LowPass,
            "highpass" => FilterMode::HighPass,
            "bandpass" => FilterMode::BandPass,
            _ => FilterMode::LowPass,  // Default
        };
        
        // ... rest of implementation
    }
}
```

**DSL (New)**:
```javascript
// Clear and type-safe
const filt = filter("f1", { mode: "lowpass" });
// TypeScript autocomplete suggests: "lowpass" | "highpass" | "bandpass"
```

### Migration Checklist for Existing Modules

When migrating a module:

1. ✅ Identify parameters that are configuration vs. signals
   - Configuration: discrete choices, settings, initialization data
   - Signals: continuous values, modulation sources, audio

2. ✅ Create `DataParams` struct for configuration parameters
   - Add appropriate type constraints
   - Set sensible defaults

3. ✅ Remove configuration `InternalParam` fields from `Params` struct
   - Keep audio signal parameters

4. ✅ Update `update()` method to read from `self.data` instead of params
   - No `get_value()` calls needed - direct field access

5. ✅ Regenerate TypeScript types
   - Run codegen to update DSL factories

6. ✅ Update documentation and examples
   - Show new DSL usage patterns

7. ✅ Test backward compatibility if needed
   - Provide migration path for existing patches

## Advanced Patterns

### Pattern: Data Parameter Affecting Buffer Sizes

```rust
#[derive(DataParams)]
struct DelayData {
    #[data_param("max_delay", "maximum delay time in seconds", float(0.001, 10.0))]
    max_delay: f64,
}

impl Delay {
    fn new(sample_rate: f32, data: DelayData) -> Self {
        let buffer_size = (data.max_delay * sample_rate as f64) as usize;
        Self {
            buffer: vec![0.0; buffer_size],
            // ...
        }
    }
}
```

This requires **module recreation** when `max_delay` changes, which is why data param changes trigger full module recreation rather than hot-reload.

### Pattern: Lookup Table Generation

```rust
#[derive(DataParams)]
struct WavetableData {
    #[data_param("waveform", "waveform type", enum("sine", "saw", "square", "triangle"))]
    waveform: String,
    
    #[data_param("harmonics", "number of harmonics", int(1, 32))]
    harmonics: i64,
}

impl Wavetable {
    fn new(sample_rate: f32, data: WavetableData) -> Self {
        let table = Self::generate_table(&data.waveform, data.harmonics as usize);
        Self {
            table,
            // ...
        }
    }
    
    fn generate_table(waveform: &str, harmonics: usize) -> Vec<f32> {
        // Generate lookup table based on data params
        // ...
    }
}
```

### Pattern: Conditional Behavior

```rust
#[derive(DataParams)]
struct EnvelopeData {
    #[data_param("loop", "loop envelope", bool)]
    loop_enabled: bool,
    
    #[data_param("legato", "legato mode (no retrigger)", bool)]
    legato: bool,
}

impl Envelope {
    fn update(&mut self, sample_rate: f32) {
        // Behavior changes based on data params
        if self.data.legato && self.gate_is_high() {
            // Don't retrigger on overlapping notes
            return;
        }
        
        if self.stage == Stage::Release && self.data.loop_enabled {
            // Loop back to attack
            self.stage = Stage::Attack;
        }
        
        // ...
    }
}
```

## Testing Strategy

### Unit Tests for Data Parameters

```rust
#[test]
fn test_sequencer_forward_direction() {
    let data = SequencerData {
        steps: vec![0.0, 1.0, 2.0, 3.0],
        length: 4,
        direction: "forward".to_string(),
        quantize: false,
    };
    
    let mut seq = Sequencer::with_data(data);
    
    // Test step advancement
    assert_eq!(seq.current_step, 0);
    seq.advance_step();
    assert_eq!(seq.current_step, 1);
    seq.advance_step();
    assert_eq!(seq.current_step, 2);
    seq.advance_step();
    assert_eq!(seq.current_step, 3);
    seq.advance_step();
    assert_eq!(seq.current_step, 0);  // Wraps around
}

#[test]
fn test_sequencer_invalid_direction_defaults() {
    let data = SequencerData {
        steps: vec![0.0, 1.0],
        length: 2,
        direction: "invalid".to_string(),  // Invalid value
        quantize: false,
    };
    
    let mut seq = Sequencer::with_data(data);
    let start_step = seq.current_step;
    seq.advance_step();
    
    // Should not advance with invalid direction
    assert_eq!(seq.current_step, start_step);
}
```

### Integration Tests

```rust
#[test]
fn test_patch_with_data_params() {
    let patch_json = r#"
    {
        "modules": [
            {
                "id": "seq-1",
                "moduleType": "sequencer",
                "params": {
                    "clock": { "type": "value", "value": [0.0, ...] }
                },
                "data": {
                    "steps": { 
                        "type": "array", 
                        "values": [
                            { "type": "float", "value": 0.0 },
                            { "type": "float", "value": 2.0 },
                            { "type": "float", "value": 4.0 }
                        ]
                    },
                    "direction": { "type": "enum", "variant": "forward" }
                }
            }
        ],
        "tracks": [],
        "scopes": []
    }
    "#;
    
    let patch_graph: PatchGraph = serde_json::from_str(patch_json).unwrap();
    let schemas = get_schemas();
    
    // Should validate successfully
    assert!(validate_patch(&patch_graph, &schemas).is_ok());
    
    // Should build successfully
    let patch = build_patch(&patch_graph).unwrap();
    assert!(patch.sampleables.contains_key("seq-1"));
}
```

## Conclusion

These examples demonstrate:

- ✅ **Clean separation** between audio params and data params
- ✅ **Type safety** at every level (Rust, validation, TypeScript)
- ✅ **Ergonomic DSL** usage with strong typing
- ✅ **Clear migration path** from encoded voltages
- ✅ **Flexible patterns** for different use cases
- ✅ **Testable** components at all levels

The data parameter system enables modules to express their configuration requirements naturally while maintaining the real-time audio performance of the existing system.
