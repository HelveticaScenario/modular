# Plaits Module Implementation

This document describes the implementation of Mutable Instruments Plaits-inspired synthesis modules in the modular synthesizer.

## Overview

The original Plaits module by Mutable Instruments contains 24 different synthesis engines. This implementation provides simplified versions of these engines adapted to the modular_core architecture.

## Implemented Modules (7/24)

### 1. plaits-fm (FM Synthesis)
**Module Type:** `plaits-fm`

Two-operator FM synthesis with feedback control.

**Parameters:**
- `freq`: Carrier frequency in v/oct
- `harmonics`: Modulator/carrier ratio (0.0-1.0) - maps to musically useful ratios
- `timbre`: Modulation amount/index (0.0-1.0)
- `morph`: Feedback amount (-1.0 to 1.0, negative values add phase offset)

**Outputs:**
- `output`: Main FM output
- `aux`: Sub-octave output

**Features:**
- Quantized frequency ratios for harmonic/inharmonic sounds
- Amplitude feedback for positive morph values
- Phase feedback for negative morph values
- Fast polynomial sine approximation

### 2. plaits-va (Virtual Analog)
**Module Type:** `plaits-va`

Dual variable-shape oscillators with hard sync.

**Parameters:**
- `freq`: Base frequency in v/oct
- `harmonics`: Detuning amount for second oscillator (quantized to musical intervals)
- `timbre`: Waveshape morphing (saw → triangle → square) and pulse width
- `morph`: Crossfade and hard sync amount

**Outputs:**
- `output`: Mixed output of both oscillators
- `aux`: Second oscillator only

**Features:**
- Polyblep antialiasing for saw waves
- Variable waveshapes (saw, triangle, pulse)
- Hard sync between oscillators
- Musical interval-based detuning

### 3. plaits-grain (Granular Synthesis)
**Module Type:** `plaits-grain`

Granular synthesis with 8 concurrent grains.

**Parameters:**
- `freq`: Base frequency in v/oct
- `harmonics`: Grain density (10-200 grains/second)
- `timbre`: Grain size/duration (10ms-500ms)
- `morph`: Randomization amount for frequency and size

**Outputs:**
- `output`: Main granular output
- `aux`: Alternative grain texture (odd-numbered grains)

**Features:**
- Hann window envelope for each grain
- Random frequency and size variation
- Smooth grain density control

### 4. plaits-wavetable (Wavetable Synthesis)
**Module Type:** `plaits-wavetable`

Wavetable synthesis with morphing capabilities.

**Parameters:**
- `freq`: Frequency in v/oct
- `harmonics`: Number of harmonics in wavetable (1-16)
- `timbre`: Wave selection (saw-like, square-like, triangle-like, sine)
- `morph`: Phase distortion amount

**Outputs:**
- `output`: Main wavetable output

**Features:**
- Four basic waveform types with variable harmonic content
- Smooth interpolation between waveforms
- Phase distortion morphing

### 5. plaits-noise (Noise/Particle Engine)
**Module Type:** `plaits-noise`

Filtered noise with metallic/clocked modes.

**Parameters:**
- `freq`: Filter frequency in v/oct
- `harmonics`: Metallic character/frequency spread (clocked noise)
- `timbre`: Noise color (low to high frequency - LP/BP/HP)
- `morph`: Resonance/feedback amount

**Outputs:**
- `output`: Filtered noise (color controlled by timbre)
- `aux`: Bandpass-filtered metallic texture

**Features:**
- State variable filter with resonance control
- Sample-and-hold for metallic/clocked noise
- Smooth to metallic morphing

### 6. plaits-modal (Modal Synthesis)
**Module Type:** `plaits-modal`

Physical modeling of resonant objects like bells and bars.

**Parameters:**
- `freq`: Base frequency in v/oct
- `harmonics`: Inharmonicity (0.0 = harmonic/bar, 1.0 = bell-like)
- `timbre`: Damping/decay time (0.0 = fast, 1.0 = long)
- `morph`: Exciter brightness/noise amount
- `trigger`: Strike/excitation trigger (rising edge)

**Outputs:**
- `output`: Modal resonator output

**Features:**
- 6 modal resonators with frequency ratios
- Harmonic to inharmonic morphing
- Noise excitation for realistic attacks
- Natural decay envelopes

### 7. plaits-string (Karplus-Strong String)
**Module Type:** `plaits-string`

Physical modeling of plucked strings.

**Parameters:**
- `freq`: String frequency in v/oct
- `harmonics`: Brightness/harmonic content of excitation
- `timbre`: Damping/decay time
- `morph`: Exciter position (0.0 = bridge, 1.0 = center)
- `trigger`: Pluck trigger (rising edge)

**Outputs:**
- `output`: String output

**Features:**
- Karplus-Strong algorithm with delay line
- Variable exciter position for harmonic control
- Adjustable damping and brightness
- Dynamic delay line resizing for frequency changes

## Remaining Modules (17/24)

### Original Plaits Engines (engine/)
- Additive synthesis
- Chord generation
- Speech/formant synthesis
- Swarm/particle synthesis
- Waveshaping
- 808-style bass drum
- 808-style snare drum
- 808-style hi-hat

### Extended Engines (engine2/)
- Phase distortion
- 6-operator FM
- Wave terrain synthesis
- String machine
- Chiptune/NES-style
- Virtual analog VCF

## Common Architecture

All Plaits modules follow these conventions:

### Parameter Structure
```rust
#[derive(Default, Params)]
struct ModuleParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "description")]
    harmonics: InternalParam,
    #[param("timbre", "description")]
    timbre: InternalParam,
    #[param("morph", "description")]
    morph: InternalParam,
}
```

### Module Structure
```rust
#[derive(Default, Module)]
#[module("module-name", "Description")]
pub struct ModuleName {
    #[output("output", "main output", default)]
    sample: f32,
    
    // Internal state...
    params: ModuleParams,
}
```

### Key Patterns
1. **V/Oct Frequency Control**: All modules use -10V to 10V range mapping to frequency
2. **Smooth Parameter Changes**: Use `types::smooth_value()` to prevent clicks
3. **Output Scaling**: All outputs scaled to ±5V range with clamping: `5.0 * clamp(-1.0, 1.0, value)`
4. **Four Standard Parameters**: freq, harmonics, timbre, morph provide consistent control
5. **Auxiliary Outputs**: Where applicable, modules provide alternative textures on aux output

## Testing

All modules have comprehensive test coverage in `tests/plaits_tests.rs`:
- Output generation verification
- Voltage range compliance (±5V)
- Trigger behavior (for triggered modules)
- Parameter response

## Integration

Modules are registered in `modular_core/src/dsp/oscillators/mod.rs`:
```rust
pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    plaits_fm::PlaitsFM::install_constructor(map);
    // ... other modules
}
```

## Technical Notes

### Imports Required
All Plaits modules must import:
```rust
use anyhow::{Result, anyhow};
use crate::{
    dsp::utils::clamp,
    types::InternalParam,
};
```

### Smoothing Coefficient
The global `SMOOTHING_COEFF = 0.99` is used for parameter smoothing to avoid clicks.

### Sample Rate
Sample rate is passed during module construction and used for frequency calculations:
```rust
let hz = 27.5 * 2.0f32.powf(voltage);
let phase_inc = hz / sample_rate;
```

## Future Work

1. **Complete Remaining Engines**: Implement the 17 remaining Plaits models
2. **TypeScript Type Export**: Generate frontend types for the web interface
3. **Documentation**: Add detailed parameter descriptions and usage examples
4. **Audio Quality**: Validate output against original Plaits module
5. **Optimization**: Profile and optimize DSP algorithms where needed
6. **Extended Features**: Add additional modulation inputs where appropriate

## References

- Original Plaits source: https://github.com/pichenettes/eurorack/tree/master/plaits
- Mutable Instruments: https://mutable-instruments.net/
- Plaits Manual: https://mutable-instruments.net/modules/plaits/manual/
