# Multichannel Cable Abstraction Proposal

## Executive Summary

This document proposes adding VCV Rack-style multichannel (polyphonic) cables to our modular synthesizer. The design adapts Rack's proven patterns while leveraging Rust's type system and our existing architecture.

**Key Goals:**
1. Allow a single cable to carry up to 16 independent audio channels
2. Enable polyphonic voices from a single MIDI input or pattern sequencer
3. Maintain backward compatibility with existing monophonic modules
4. Preserve real-time safety guarantees

---

## Part 1: VCV Rack's Architecture (Reference)

### 1.1 Core Data Structures

Rack uses a simple, elegant design:

```cpp
// Maximum channels per cable (inspired by MIDI's 16 channels)
static const int PORT_MAX_CHANNELS = 16;

struct Port {
    float voltages[PORT_MAX_CHANNELS] = {};  // Fixed array, always allocated
    uint8_t channels = 0;                     // 0 = disconnected, 1-16 = connected
};
```

**Key insight**: Channel count is metadata, not allocation. Every port can hold 16 channels, but `channels` indicates how many are semantically valid.

### 1.2 Channel Count Meanings

| `channels` | Meaning |
|------------|---------|
| 0 | Disconnected (no cable) |
| 1 | Monophonic (single voice) |
| 2-16 | Polyphonic (N voices) |

### 1.3 Propagation Rules

1. **Outputs determine polyphony** - Source modules set their output channel count
2. **Cables transfer verbatim** - The engine copies all voltages + channel count from output to input
3. **Inputs adapt** - Receiving modules read channel count and process accordingly

### 1.4 Stacked Cable Behavior

When multiple cables connect to the same input, Rack **sums** them with special rules:
- The input's channel count = `max(all connected output channel counts)`
- Polyphonic outputs sum channel-to-channel
- **Monophonic outputs broadcast to ALL channels** (crucial for shared modulation)

Example: A monophonic LFO modulating a polyphonic oscillator applies to all voices equally.

### 1.5 The `getPolyVoltage()` Pattern

Rack provides a helper for inputs that should support either mono or poly sources:

```cpp
float getPolyVoltage(uint8_t channel) {
    return isMonophonic() ? getVoltage(0) : getVoltage(channel);
}
```

This enables "mono-to-poly broadcasting" - a monophonic input automatically fans out to all requested channels.

**Note**: We diverge from Rack here. Instead of special-casing mono, we use **modulo cycling** - the same approach as our non-signal params. This is simpler, more consistent, and enables interesting creative uses (e.g., a 2-channel signal cycling across 4 voices).

---

## Part 2: Proposed Architecture for Modular

### 2.1 Core Type: `PolySignal`

Replace the scalar `f32` signal with a fixed-capacity polyphonic buffer:

```rust
/// Maximum channels per cable (matches VCV Rack / MIDI convention)
pub const PORT_MAX_CHANNELS: usize = 16;

/// A polyphonic signal buffer with channel count metadata
#[derive(Clone, Copy)]
pub struct PolySignal {
    /// Voltage values for each channel (always allocated, not all may be active)
    voltages: [f32; PORT_MAX_CHANNELS],
    /// Number of active channels: 0 = disconnected, 1 = mono, 2-16 = poly
    channels: u8,
}

impl Default for PolySignal {
    fn default() -> Self {
        Self {
            voltages: [0.0; PORT_MAX_CHANNELS],
            channels: 0, // Disconnected
        }
    }
}

impl PolySignal {
    /// Create a monophonic signal with a single value
    pub fn mono(value: f32) -> Self {
        let mut sig = Self::default();
        sig.voltages[0] = value;
        sig.channels = 1;
        sig
    }

    /// Create a polyphonic signal from a slice (channels = slice length)
    pub fn poly(values: &[f32]) -> Self {
        let channels = values.len().min(PORT_MAX_CHANNELS);
        let mut sig = Self::default();
        sig.voltages[..channels].copy_from_slice(&values[..channels]);
        sig.channels = channels as u8;
        sig
    }

    // === Accessors ===
    
    pub fn channels(&self) -> u8 { self.channels }
    pub fn is_disconnected(&self) -> bool { self.channels == 0 }
    pub fn is_monophonic(&self) -> bool { self.channels == 1 }
    pub fn is_polyphonic(&self) -> bool { self.channels > 1 }

    /// Get voltage for a specific channel (returns 0.0 if out of range)
    pub fn get(&self, channel: usize) -> f32 {
        if channel < self.channels as usize {
            self.voltages[channel]
        } else {
            0.0
        }
    }

    /// Set voltage for a specific channel
    pub fn set(&mut self, channel: usize, value: f32) {
        if channel < PORT_MAX_CHANNELS {
            self.voltages[channel] = value;
        }
    }

    /// Get voltage with modulo cycling: channel wraps around available channels.
    /// This is consistent with Vec::cycle_get for non-signal params.
    /// A mono signal cycles to all channels, a 2-ch signal alternates, etc.
    pub fn get_cycling(&self, channel: usize) -> f32 {
        if self.channels == 0 {
            0.0 // Disconnected
        } else {
            self.voltages[channel % self.channels as usize]
        }
    }

    /// Get value with fallback for disconnected inputs (normalled input)
    pub fn get_or(&self, channel: usize, default: f32) -> f32 {
        if self.is_disconnected() {
            default
        } else {
            self.get_cycling(channel)
        }
    }

    /// Set the number of active channels (clears higher channels to 0)
    pub fn set_channels(&mut self, channels: u8) {
        let channels = channels.clamp(1, PORT_MAX_CHANNELS as u8);
        // Clear channels beyond the new count
        for c in channels as usize..self.channels as usize {
            self.voltages[c] = 0.0;
        }
        self.channels = channels;
    }
}
```

### 2.2 Updated `Signal` Enum

Extend the existing `Signal` type to carry polyphonic data:

```rust
#[derive(Clone, Debug, Default)]
pub enum Signal {
    /// Static polyphonic voltage value(s)
    Volts(PolySignal),
    
    /// Cable connection to another module's output
    Cable {
        module: String,
        module_ptr: std::sync::Weak<Box<dyn Sampleable>>,
        port: String,
    },
    
    #[default]
    Disconnected,
}

impl Signal {
    /// Get the full polyphonic signal
    pub fn get_poly_signal(&self) -> PolySignal {
        match self {
            Signal::Volts(poly) => *poly,
            Signal::Cable { module_ptr, port, .. } => {
                match module_ptr.upgrade() {
                    Some(module) => module.get_sample(port).unwrap_or_default(),
                    None => PolySignal::default(),
                }
            }
            Signal::Disconnected => PolySignal::default(),
        }
    }

    /// Get value for a specific channel with modulo cycling
    pub fn get_value(&self, channel: usize) -> f32 {
        self.get_poly_signal().get_cycling(channel)
    }
}
```

### 2.3 Updated `Sampleable` Trait

All outputs are polyphonic - monophonic is just the special case of `channels = 1`:

```rust
pub trait Sampleable: MessageHandler + Send + Sync {
    fn get_id(&self) -> &String;
    fn tick(&self);
    fn update(&self);
    
    // All outputs are PolySignal - mono is just channels=1
    fn get_sample(&self, port: &String) -> Result<PolySignal>;
    
    fn get_module_type(&self) -> String;
    fn try_update_params(&self, params: serde_json::Value) -> Result<()>;
    fn connect(&self, patch: &Patch);
}
```

No separate mono/poly methods - `PolySignal::mono(value)` is how you return a single-channel output.

### 2.4 Polyphonic Output Struct

Update the derive macro to support polyphonic outputs:

```rust
#[derive(Outputs, JsonSchema)]
struct PolyOscillatorOutputs {
    /// Polyphonic audio output
    #[output("output", "polyphonic signal output", default, poly)]
    sample: PolySignal,
    
    /// Per-voice phase (also polyphonic)
    #[output("phase", "current phase output", poly)]
    phase: PolySignal,
}
```

---

## Part 3: Module Patterns

### 3.1 Polyphonic Source (e.g., MIDI-to-CV)

A polyphonic source sets the channel count based on active voices:

```rust
#[derive(Module)]
#[module("midi_cv", "MIDI to CV converter with polyphony")]
pub struct MidiToCv {
    outputs: MidiToCvOutputs,
    voices: [VoiceState; PORT_MAX_CHANNELS],
    active_voices: u8,
    params: MidiToCvParams,
}

impl MidiToCv {
    fn update(&mut self, _sample_rate: f32) {
        // active_voices is set by MIDI note allocation logic
        let channels = self.active_voices.max(1);
        
        let mut pitch_out = PolySignal::default();
        let mut gate_out = PolySignal::default();
        
        for c in 0..channels as usize {
            pitch_out.set(c, self.voices[c].pitch);
            gate_out.set(c, if self.voices[c].gate { 10.0 } else { 0.0 });
        }
        
        pitch_out.set_channels(channels);
        gate_out.set_channels(channels);
        
        self.outputs.pitch = pitch_out;
        self.outputs.gate = gate_out;
    }
}
```

### 3.2 Polyphonic Processor (e.g., Oscillator)

A polyphonic processor matches its output channel count to a primary input:

```rust
#[derive(Module)]
#[module("poly_sine", "Polyphonic sine oscillator")]
pub struct PolySine {
    outputs: PolySineOutputs,
    phases: [f32; PORT_MAX_CHANNELS],
    params: PolySineParams,
}

impl PolySine {
    fn update(&mut self, sample_rate: f32) {
        // Get channel count from primary input (freq/pitch)
        let freq_signal = self.params.freq.get_poly_signal();
        let channels = freq_signal.channels().max(1);
        
        let mut output = PolySignal::default();
        
        for c in 0..channels as usize {
            // get_value() uses modulo cycling, so mono inputs cycle to all channels
            let freq_voct = self.params.freq.get_value(c);
            let frequency = 55.0 * 2.0f32.powf(freq_voct);
            
            self.phases[c] += frequency / sample_rate;
            while self.phases[c] >= 1.0 {
                self.phases[c] -= 1.0;
            }
            
            output.set(c, (self.phases[c] * TAU).sin() * 5.0);
        }
        
        output.set_channels(channels);
        self.outputs.sample = output;
    }
}
```

### 3.3 Channel Combiner (`combine`) - Mono Signals → Poly

Takes an array of mono signals and packs them into channels of a poly output:

```rust
#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct CombineParams {
    /// Array of signals to combine into poly channels
    /// Each signal becomes one channel of the output
    signals: Vec<Signal>,
}

#[derive(Outputs, JsonSchema)]
struct CombineOutputs {
    #[output("output", "polyphonic combined output", default, poly)]
    sample: PolySignal,
}

#[derive(Default, Module)]
#[module("combine", "Combine mono signals into a polyphonic signal")]
#[args(signals)]
pub struct Combine {
    outputs: CombineOutputs,
    params: CombineParams,
}

impl Combine {
    fn update(&mut self, _sample_rate: f32) {
        let signals = &self.params.signals;
        
        // Filter to only connected signals and take up to PORT_MAX_CHANNELS
        let connected: Vec<_> = signals.iter()
            .filter(|s| **s != Signal::Disconnected)
            .take(PORT_MAX_CHANNELS)
            .collect();
        
        let channels = connected.len();
        
        if channels == 0 {
            self.outputs.sample = PolySignal::default(); // Disconnected
            return;
        }
        
        let mut output = PolySignal::default();
        
        for (c, signal) in connected.iter().enumerate() {
            // Each input signal's channel 0 becomes one channel of output
            // If an input is itself poly, we only take its first channel
            output.set(c, signal.get_value());
        }
        
        output.set_channels(channels as u8);
        self.outputs.sample = output;
    }
}
```

**DSL Usage:**

```javascript
// Combine 3 oscillators into a 3-voice poly signal
const osc1 = sine({ freq: "C4" });
const osc2 = sine({ freq: "E4" });
const osc3 = sine({ freq: "G4" });

const chord = combine([osc1, osc2, osc3]);
// chord.output is now a 3-channel poly signal

// Can also mix direct values with module refs
const mixed = combine([
  osc1,           // Module reference
  "A4",           // Note (converted to v/oct)
  0.5,            // Direct voltage
  lfo({ rate: 2 }) // Another module
]);
```

**Edge Cases:**

| Input | Output Channels | Behavior |
|-------|-----------------|----------|
| `[]` (empty) | 0 | Disconnected |
| `[mono]` | 1 | Monophonic |
| `[mono, mono, mono]` | 3 | 3-channel poly |
| `[poly(4ch), mono]` | 2 | Takes ch0 of poly, then mono |
| `[...17 signals]` | 16 | Clamped to PORT_MAX_CHANNELS |

### 3.4 Channel Splitter (`split`) - Poly → Mono Signals

The inverse - extracts individual channels from a poly signal:

```rust
#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SplitParams {
    /// Polyphonic input to split
    input: Signal,
}

#[derive(Outputs, JsonSchema)]
struct SplitOutputs {
    /// Individual channel outputs (0-15)
    #[output("ch0", "channel 0")]
    ch0: PolySignal,
    #[output("ch1", "channel 1")]
    ch1: PolySignal,
    // ... ch2 through ch15
}

impl Split {
    fn update(&mut self, _sample_rate: f32) {
        let input = self.params.input.get_poly_signal();
        
        // Each output is mono, containing that channel's value
        self.outputs.ch0 = PolySignal::mono(input.get(0));
        self.outputs.ch1 = PolySignal::mono(input.get(1));
        // ... etc
    }
}
```

### 3.5 Channel Mixer (`poly_mix`) - Poly → Mono (Sum/Average)

Convert polyphonic to monophonic by summing/averaging:

```rust
#[derive(Module)]
#[module("poly_mix", "Mix polyphonic signal to mono")]
pub struct PolyMix {
    outputs: PolyMixOutputs,
    params: PolyMixParams,
}

impl PolyMix {
    fn update(&mut self, _sample_rate: f32) {
        let input = self.params.input.get_poly_signal();
        let channels = input.channels() as usize;
        
        if channels == 0 {
            self.outputs.sample = PolySignal::mono(0.0);
            return;
        }
        
        let sum: f32 = (0..channels).map(|c| input.get(c)).sum();
        let avg = sum / channels as f32;
        
        self.outputs.sample = PolySignal::mono(avg);
    }
}
```

### 3.4 Monophonic Module (Backward Compatible)

Existing modules continue to work - they just see channel 0:

```rust
// This module doesn't need any changes - it's inherently monophonic
#[derive(Module)]
#[module("delay", "Mono delay line")]
pub struct Delay {
    outputs: DelayOutputs,
    buffer: Vec<f32>,
    params: DelayParams,
}

impl Delay {
    fn update(&mut self, sample_rate: f32) {
        // get_value() returns channel 0, or broadcasts mono input
        let input = self.params.input.get_value();
        // ... process as before
        self.outputs.sample = PolySignal::mono(processed);
    }
}
```

---

## Part 4: Polyphonic Non-Signal Parameters

### 4.1 The Problem

Signals naturally support polyphony via `PolySignal`, but what about non-signal parameters?

```rust
struct NoiseParams {
    color: NoiseColor,  // enum: White, Pink, Brown, etc.
    // What if we want different colors for each voice?
}
```

In a 4-voice poly patch, you might want:
- Voice 0: White noise
- Voice 1: Pink noise  
- Voice 2: Brown noise
- Voice 3: White noise

### 4.2 Solution: All Non-Signal Params Are Vectors

**Simple rule: every non-signal parameter is a `Vec<T>`.**

- Single value → `vec![value]` (length 1, broadcasts to all channels)
- Multiple values → `vec![a, b, c]` (cycles through values)

No special wrapper type needed - just vectors with cycling access.

```rust
/// Extension trait for cycling access to vectors
pub trait CycleGet<T> {
    /// Get value at index with cycling (wraps around).
    /// Returns T::default() if the vec is empty.
    /// Since all param fields must implement Default (required by #[serde(default)]),
    /// this is always safe and never returns Option.
    fn cycle_get(&self, index: usize) -> T where T: Default + Clone;
}

impl<T: Default + Clone> CycleGet<T> for Vec<T> {
    fn cycle_get(&self, index: usize) -> T {
        if self.is_empty() {
            T::default()
        } else {
            self[index % self.len()].clone()
        }
    }
}

/// Returns the number of explicitly poly channels (None if length <= 1)
pub trait PolyChannels {
    fn poly_channels(&self) -> Option<usize>;
}

impl<T> PolyChannels for Vec<T> {
    fn poly_channels(&self) -> Option<usize> {
        if self.len() > 1 {
            Some(self.len())
        } else {
            None  // Length 0 or 1 doesn't constrain channel count
        }
    }
}
```

> **Note:** Params structs are always `#[derive(Default)]` with `#[serde(default)]`, meaning every field must have a `Default` impl. This guarantees `cycle_get` can always return a value - no `Option` needed.

### 4.3 Custom Deserializer: Single Value or Array

Accept either a single value or an array in JSON, always deserialize to `Vec<T>`:

```rust
use serde::{Deserialize, Deserializer};

/// Deserialize either a single value or array into Vec<T>
pub fn deserialize_poly<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SingleOrVec<T> {
        Single(T),
        Vec(Vec<T>),
    }

    match SingleOrVec::deserialize(deserializer)? {
        SingleOrVec::Single(v) => Ok(vec![v]),
        SingleOrVec::Vec(v) => Ok(v),
    }
}

// Use with serde attribute:
#[derive(Deserialize)]
struct NoiseParams {
    #[serde(deserialize_with = "deserialize_poly")]
    color: Vec<NoiseColor>,
    
    #[serde(deserialize_with = "deserialize_poly")]
    seed: Vec<u32>,
    
    // Signals stay as-is (they have their own poly handling)
    level: Signal,
}
```

### 4.4 Derive Macro for Automatic Poly Deserialization

To avoid boilerplate, a derive macro can auto-apply the deserializer:

```rust
// This derive macro wraps all non-Signal fields with deserialize_poly
#[derive(Deserialize, PolyParams)]
struct NoiseParams {
    color: Vec<NoiseColor>,  // Auto-applies deserialize_poly
    seed: Vec<u32>,          // Auto-applies deserialize_poly
    level: Signal,           // Left alone (Signal has its own handling)
}

// Expands to:
#[derive(Deserialize)]
struct NoiseParams {
    #[serde(deserialize_with = "deserialize_poly")]
    color: Vec<NoiseColor>,
    #[serde(deserialize_with = "deserialize_poly")]
    seed: Vec<u32>,
    level: Signal,
}
```

### 4.5 Usage in Module Params

```rust
#[derive(Deserialize, Default, JsonSchema, Connect, PolyParams)]
#[serde(default)]
struct NoiseParams {
    /// Noise color - single value or per-voice array
    color: Vec<NoiseColor>,
    
    /// Output level (signal, supports CV modulation)
    level: Signal,
}

#[derive(Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum NoiseColor {
    #[default]
    White,
    Pink,
    Brown,
    Blue,
    Violet,
}

impl Noise {
    fn update(&mut self, sample_rate: f32) {
        let channels = self.determine_channels();
        
        let mut output = PolySignal::default();
        
        for c in 0..channels {
            // cycle_get returns T directly (default if empty)
            let color = self.params.color.cycle_get(c);
            let sample = match color {
                NoiseColor::White => self.white_gen[c].next(),
                NoiseColor::Pink => self.pink_gen[c].next(),
                NoiseColor::Brown => self.brown_gen[c].next(),
                // ...
            };
            
            let level = self.params.level.get_value(c);
            output.set(c, sample * level);
        }
        
        output.set_channels(channels as u8);
        self.outputs.sample = output;
    }
    
    fn determine_channels(&self) -> usize {
        // Max of: vec param lengths > 1, or signal input channels
        let color_channels = self.params.color.poly_channels();
        let level_channels = self.params.level.get_poly_signal().channels() as usize;
        
        [color_channels, Some(level_channels).filter(|&c| c > 1)]
            .into_iter()
            .flatten()
            .max()
            .unwrap_or(1)
    }
}
```

### 4.6 DSL Examples

```javascript
// Mono - single value (becomes vec!["pink"])
const mono_noise = noise({ color: "pink" });

// Poly - array of values
const poly_noise = noise({ 
    color: ["white", "pink", "brown", "white"]  // 4 voices
});

// Mixed lengths - values CYCLE to align
const mixed = noise({
    color: ["white", "pink", "brown"],  // 3 values
    level: [0.8, 0.3]                   // 2 values (signal, but same idea)
});
// Output channels = max(3, 2) = 3
// Effective values:
//   ch0: color=white, level=0.8
//   ch1: color=pink,  level=0.3
//   ch2: color=brown, level=0.8  ← level cycles back to index 0

// Cycling enables musical patterns
const stereo_pan = noise({
    color: "pink",
    pan: [-1, 1]  // Alternates left/right for any channel count
});

// With a 4-voice input, pan becomes: [-1, 1, -1, 1]

// Useful for polyrhythmic parameter patterns
const varied = osc({
    freq: midi_cv.pitch,           // 8 voices from MIDI
    detune: [0, 0.1, -0.1],        // Cycles: 0, 0.1, -0.1, 0, 0.1, -0.1, 0, 0.1
    waveform: ["saw", "square"]    // Alternates: saw, square, saw, square...
});
```

### 4.7 Cycling Behavior Details

When multiple `Vec` params have different lengths, values **cycle** (wrap around):

```
color:  ["white", "pink", "brown"]  → length 3
level:  [0.8, 0.3]                  → length 2

Output channels = max(3, 2) = 3

Channel 0: color[0 % 3] = "white",  level[0 % 2] = 0.8
Channel 1: color[1 % 3] = "pink",   level[1 % 2] = 0.3
Channel 2: color[2 % 3] = "brown",  level[2 % 2] = 0.8  ← wraps!
```

This is similar to:
- Haskell's `cycle` function
- APL/J's scalar extension
- Tidal/Strudel's list cycling in mini-notation

**Why cycling over zero-fill?**
- More musical: `[left, right]` naturally alternates for any voice count
- Predictable: the pattern repeats rather than trailing off to defaults
- Composable: short patterns can define rhythmic/spatial relationships

### 4.8 Channel Count Resolution

When a module has multiple poly params/signals, output channels = max of all explicit poly sources:

```rust
fn determine_output_channels(&self) -> usize {
    // Collect all vec lengths > 1 (length 1 = mono, doesn't constrain)
    let param_channels: Vec<usize> = [
        self.params.color.poly_channels(),
        self.params.mode.poly_channels(),
        // ... other vec params
    ]
    .into_iter()
    .flatten()
    .collect();
    
    // Also consider poly signal inputs
    let signal_channels = [
        self.params.freq.get_poly_signal().channels() as usize,
        self.params.level.get_poly_signal().channels() as usize,
    ];
    
    // Output channels = max of all explicit poly sources
    // Length-1 vecs and mono signals don't constrain - they cycle/broadcast
    param_channels.into_iter()
        .chain(signal_channels.into_iter().filter(|&c| c > 1))
        .max()
        .unwrap_or(1)
}
```

### 4.9 Type Requirements

Vec params work for any type that is:
- `Clone` (to cycle values)
- `Default` (for empty vec edge case)
- `Deserialize` (for JSON parsing)

Common use cases:

| Type | Example |
|------|---------|
| `Vec<f32>` | Detune amount per voice |
| `Vec<NoiseColor>` | Different noise color per voice |
| `Vec<FilterMode>` | Different filter type per voice |
| `Vec<i32>` | Octave offset per voice |
| `Vec<bool>` | Enable/disable per voice |
| `Vec<String>` | Sample name per voice |

### 4.10 Summary: Param Type Rules

| Param Type | Usage | Poly Behavior |
|------------|-------|---------------|
| `Signal` | CV-modulatable values | Has its own `PolySignal` with `get_cycling()` modulo wrap |
| `Vec<T>` | Static configuration values | Cycles via `cycle_get()`, length > 1 sets channel count |

### 4.11 DSL TypeScript Compatibility

The auto-generated TypeScript types need to accept both `T` and `T[]` for all `Vec<T>` params. This requires updating [typescriptLibGen.ts](../src/dsl/typescriptLibGen.ts):

**Changes to `schemaToTypeExpr()`:**

When the Rust type is `Vec<T>` (JSON schema `{ type: "array", items: { ... } }`), generate a union type `T | T[]` instead of just `T[]`:

```typescript
if (type === 'array') {
    if (Array.isArray(schema.prefixItems)) {
        // Tuple - keep as-is
        const items = schema.prefixItems as JSONSchema[];
        const tuple = items.map((s) => schemaToTypeExpr(s, rootSchema)).join(', ');
        return `[${tuple}]`;
    }
    if (schema.items) {
        const itemType = schemaToTypeExpr(schema.items, rootSchema);
        // NEW: Generate T | T[] for poly params
        return `${itemType} | ${itemType}[]`;
    }
    throw new Error('Unsupported array schema: missing items/prefixItems');
}
```

**Result:**

```typescript
// Before (current behavior)
interface NoiseParams {
    color?: ("white" | "pink" | "brown")[];
    seed?: number[];
}

// After (poly-compatible)
interface NoiseParams {
    color?: ("white" | "pink" | "brown") | ("white" | "pink" | "brown")[];
    seed?: number | number[];
}
```

**DSL usage stays natural:**

```javascript
// Single value (wrapped into vec on Rust side)
noise({ color: "pink" })

// Array (passed through as-is)
noise({ color: ["white", "pink", "brown"] })
```

The Rust `deserialize_poly` function handles the conversion, and the TypeScript types accurately reflect what's accepted.

---

## Part 5: Stacked Cable (Multi-Input Summing)

When multiple cables connect to the same input, we need summing logic:

```rust
impl Signal {
    /// Sum multiple signals with VCV Rack-style mono broadcasting
    pub fn sum_signals(signals: &[Signal]) -> PolySignal {
        if signals.is_empty() {
            return PolySignal::default();
        }
        
        // Find max channel count across all inputs
        let max_channels = signals.iter()
            .map(|s| s.get_poly_signal().channels())
            .max()
            .unwrap_or(1);
        
        let mut result = PolySignal::default();
        result.channels = max_channels;
        
        for signal in signals {
            let poly = signal.get_poly_signal();
            
            if poly.is_monophonic() {
                // Mono broadcasts to ALL channels (VCV Rack behavior)
                let mono_value = poly.get(0);
                for c in 0..max_channels as usize {
                    result.voltages[c] += mono_value;
                }
            } else {
                // Poly sums channel-to-channel
                for c in 0..poly.channels() as usize {
                    result.voltages[c] += poly.get(c);
                }
            }
        }
        
        result
    }
}
```

---

## Part 5: DSL Integration

### 5.1 JavaScript/TypeScript Types

Update the DSL to support polyphonic signals:

```typescript
// New type for explicit poly values
interface PolySignal {
  channels: number;
  voltages: number[];
}

// Signal union type (backward compatible)
type Signal = 
  | number                                    // Mono shorthand: 0.5
  | string                                    // Note/Hz shorthand: "A4", "440hz"
  | { type: 'volts'; value: number }         // Explicit mono
  | { type: 'poly'; voltages: number[] }     // Explicit poly
  | { type: 'cable'; module: string; port: string }
  | { type: 'disconnected' };
```

### 5.2 DSL Factories

```typescript
// Create a polyphonic oscillator bank
function polyOsc(freqs: Signal[]): ModuleRef {
  return createModule('poly_sine', {
    freq: { type: 'poly', voltages: freqs.map(resolveSignal) }
  });
}

// Or use a voice allocator
function voices(count: number): ModuleRef {
  return createModule('midi_cv', { voices: count });
}
```

---

## Part 6: Implementation Roadmap

### Phase 1: Core Types (Non-Breaking)
1. Add `PolySignal` type
2. Update `Sampleable::get_sample()` to return `PolySignal`
3. Update `Signal` enum to support poly values
4. Migrate existing modules to return `PolySignal::mono(value)`

### Phase 2: Infrastructure
1. Update derive macros for poly outputs
2. Add SIMD helpers for processing 4 channels at a time
3. Update scope/visualization to show per-channel data

### Phase 3: Polyphonic Modules
1. Create `poly_sine`, `poly_saw`, etc. oscillators
2. Create `midi_cv` polyphonic voice allocator
3. Create `poly_mix` channel combiner
4. Create `poly_split` channel splitter

### Phase 4: Pattern System Integration
1. Update pattern sequencer to output polyphonic CV
2. Support per-voice pattern addressing (e.g., `chord([0, 4, 7])`)
3. Voice cycling / round-robin allocation

---

## Part 7: Performance Considerations

### 7.1 Memory Layout

The fixed-size `[f32; 16]` array ensures:
- No heap allocation per sample
- Cache-friendly memory access
- SIMD-aligned (with `#[repr(align(64))]` if needed)

### 7.2 SIMD Processing

For modules that process many channels:

```rust
use std::simd::{f32x4, SimdFloat};

impl PolySine {
    fn update_simd(&mut self, sample_rate: f32) {
        let channels = self.params.freq.get_poly_signal().channels() as usize;
        
        // Process 4 channels at a time
        for c in (0..channels).step_by(4) {
            let freq = f32x4::from_array([
                self.params.freq.get_value(c),
                self.params.freq.get_value(c + 1),
                self.params.freq.get_value(c + 2),
                self.params.freq.get_value(c + 3),
            ]);
            
            // SIMD math...
        }
    }
}
```

### 7.3 Lazy Channel Count

Modules should cache their output channel count and only recompute when inputs change significantly. This avoids redundant work on every sample.

---

## Part 8: Stretch Goal - Block Processing

### 8.1 Motivation

Currently, the patch processes one sample at a time:
```rust
for _ in 0..buffer_size {
    patch.tick();  // Process 1 sample
    output = patch.get_output();
}
```

This has overhead:
- Function call per sample per module
- Cache misses from jumping between modules
- No SIMD across time (only across channels)

**Block processing** processes N samples at once, enabling:
- Amortized function call overhead
- Better cache locality (process all samples in one module before moving to next)
- SIMD across both time AND channels
- More efficient for effects with internal buffers (delays, reverbs)

### 8.2 Block Signal Type

Extend `PolySignal` to hold blocks of samples:

```rust
/// Block size for audio processing (power of 2 for SIMD alignment)
pub const BLOCK_SIZE: usize = 64;

/// A block of polyphonic samples
#[derive(Clone)]
pub struct PolyBlock {
    /// [channel][sample] - channels are contiguous for SIMD across time
    data: [[f32; BLOCK_SIZE]; PORT_MAX_CHANNELS],
    /// Number of active channels
    channels: u8,
}

impl PolyBlock {
    pub fn new() -> Self {
        Self {
            data: [[0.0; BLOCK_SIZE]; PORT_MAX_CHANNELS],
            channels: 0,
        }
    }

    /// Get a single sample (for per-sample access when needed)
    pub fn get(&self, channel: usize, sample: usize) -> f32 {
        self.data[channel][sample]
    }

    /// Get entire channel as slice (for SIMD processing)
    pub fn channel(&self, channel: usize) -> &[f32; BLOCK_SIZE] {
        &self.data[channel]
    }

    /// Get mutable channel slice
    pub fn channel_mut(&mut self, channel: usize) -> &mut [f32; BLOCK_SIZE] {
        &mut self.data[channel]
    }

    /// Get value with modulo cycling (consistent with PolySignal::get_cycling)
    pub fn get_cycling(&self, channel: usize, sample: usize) -> f32 {
        if self.channels == 0 {
            0.0
        } else {
            self.data[channel % self.channels as usize][sample]
        }
    }
}
```

### 8.3 Updated Sampleable Trait for Block Processing

```rust
pub trait Sampleable: MessageHandler + Send + Sync {
    fn get_id(&self) -> &String;
    
    /// Process a block of samples
    fn process_block(&self, block_size: usize);
    
    /// Get output block for a port
    fn get_block(&self, port: &String) -> Result<&PolyBlock>;
    
    fn get_module_type(&self) -> String;
    fn try_update_params(&self, params: serde_json::Value) -> Result<()>;
    fn connect(&self, patch: &Patch);
}
```

### 8.4 Block-Based Module Example

```rust
#[derive(Module)]
#[module("poly_sine", "Polyphonic sine oscillator")]
pub struct PolySine {
    output_block: PolyBlock,
    phases: [f32; PORT_MAX_CHANNELS],
    params: PolySineParams,
    // Cache input blocks to avoid repeated lookups
    freq_block: PolyBlock,
}

impl PolySine {
    fn process_block(&mut self, block_size: usize, sample_rate: f32) {
        // 1. Fetch input blocks once
        self.freq_block = self.params.freq.get_block();
        let channels = self.freq_block.channels.max(1) as usize;
        
        // 2. Process each channel
        for c in 0..channels {
            let freq_ch = self.freq_block.channel(c);
            let out_ch = self.output_block.channel_mut(c);
            
            let mut phase = self.phases[c];
            
            // 3. Process block of samples (SIMD-friendly loop)
            for s in 0..block_size {
                let freq_voct = freq_ch[s];
                let frequency = 55.0 * 2.0f32.powf(freq_voct);
                
                phase += frequency / sample_rate;
                while phase >= 1.0 { phase -= 1.0; }
                
                out_ch[s] = (phase * TAU).sin() * 5.0;
            }
            
            self.phases[c] = phase;
        }
        
        self.output_block.channels = channels as u8;
    }
}
```

### 8.5 SIMD Across Time

With block processing, we can SIMD across the time dimension:

```rust
use std::simd::{f32x8, SimdFloat};

fn process_block_simd(&mut self, block_size: usize, sample_rate: f32) {
    let channels = self.freq_block.channels.max(1) as usize;
    
    for c in 0..channels {
        let freq_ch = self.freq_block.channel(c);
        let out_ch = self.output_block.channel_mut(c);
        
        // Process 8 samples at a time
        for s in (0..block_size).step_by(8) {
            let freq = f32x8::from_slice(&freq_ch[s..]);
            
            // SIMD operations on 8 samples simultaneously
            let frequency = f32x8::splat(55.0) * freq.exp2();
            // ... phase accumulation is trickier, needs scalar or prefix sum
            
            let output = /* ... */;
            output.copy_to_slice(&mut out_ch[s..]);
        }
    }
}
```

### 8.6 Hybrid: SIMD Across Channels AND Time

For maximum throughput, process 4 channels × 8 samples = 32 values at once:

```rust
// Process 4 channels in parallel, 8 samples at a time
for c in (0..channels).step_by(4) {
    for s in (0..block_size).step_by(8) {
        // Load 4×8 = 32 floats
        let v0 = f32x8::from_slice(&self.freq_block.data[c+0][s..]);
        let v1 = f32x8::from_slice(&self.freq_block.data[c+1][s..]);
        let v2 = f32x8::from_slice(&self.freq_block.data[c+2][s..]);
        let v3 = f32x8::from_slice(&self.freq_block.data[c+3][s..]);
        
        // Process all 32 values with SIMD
        // ...
    }
}
```

### 8.7 Block Processing Trade-offs

| Aspect | Sample-by-Sample | Block Processing |
|--------|------------------|------------------|
| Latency | Minimal | +BLOCK_SIZE samples |
| Function call overhead | High | Amortized |
| Cache efficiency | Poor (jumps between modules) | Good (process full block) |
| SIMD potential | Channels only (16 max) | Channels × Time (16 × 64) |
| Code complexity | Simple | More complex |
| Modulation resolution | Per-sample | Per-sample (within block) |

### 8.8 Implementation Strategy

When we move to block processing, we'll migrate the entire codebase at once rather than maintaining dual-mode compatibility. The codebase is small enough that this is practical and avoids the complexity of supporting both modes.

**Migration steps:**
1. Update `Sampleable` trait to block-based API
2. Update all modules in one pass
3. Update audio callback to request blocks instead of single samples

---

## Part 9: Open Questions

1. **Stacked cables in DSL**: How should the DSL represent multiple cables to one input? Array of signals?

2. **Channel count visualization**: Should the UI show cable "thickness" based on channel count (like VCV Rack)?

3. **Voice stealing**: For limited polyphony, what algorithm? (oldest, quietest, round-robin)

4. **Per-voice scope**: Should scopes be able to show individual voices?

5. **Pattern polyphony**: Should `seq()` automatically become polyphonic for chords, or require explicit `polySeq()`?

6. **Block size**: What's the optimal block size? 32? 64? 128? Trade-off between latency and efficiency.

---

## Summary

This proposal adapts VCV Rack's proven multichannel architecture:

| Aspect | VCV Rack | Our Implementation |
|--------|----------|-------------------|
| Max channels | 16 | 16 |
| Storage | Fixed `float[16]` | Fixed `[f32; 16]` |
| Disconnected | `channels = 0` | `channels = 0` |
| Channel access | `getPolyVoltage()` (mono broadcast) | `get_cycling()` (modulo wrap) |
| Stacked cables | Sum with broadcast | Same |
| Backward compat | Full | Full |

The key principles are:
1. **Output determines polyphony** - sources set channel count
2. **Mono broadcasts to all** - enables shared modulation
3. **Fixed allocation** - real-time safe, no allocations
4. **Graceful degradation** - mono modules just use channel 0
