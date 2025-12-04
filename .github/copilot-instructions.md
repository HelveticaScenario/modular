# Modular Synth Codebase Guide

## Architecture Overview

This is a **modular audio synthesis system** with a Rust backend and React/TypeScript frontend. The architecture follows strict separation of concerns across three main crates:

### Core Architecture

```
┌─────────────────────────────────────────────────────┐
│  modular_web (React/TS)                             │
│  - WebSocket client, YAML editor, oscilloscope      │
└──────────────────┬──────────────────────────────────┘
                   │ WebSocket (JSON + Binary)
┌──────────────────▼──────────────────────────────────┐
│  modular_server (Rust)                              │
│  - Axum HTTP/WebSocket server                       │
│  - Patch validation, audio subscriptions            │
│  - Type generation (ts-rs exports)                  │
└──────────────────┬──────────────────────────────────┘
                   │ Arc<Mutex<Patch>>
┌──────────────────▼──────────────────────────────────┐
│  modular_core (Pure Rust DSP)                       │
│  - NO I/O, protocol, or serialization               │
│  - Real-time audio processing only                  │
└─────────────────────────────────────────────────────┘
```

**Critical: `modular_core` is a pure DSP library.** Never add HTTP, WebSocket, serialization, or I/O code here. Server concerns belong in `modular_server`.

## Key Concepts

### Thread Boundaries & Locking

The audio callback runs on a **real-time thread** and must NEVER block. Throughout the codebase:
- Audio thread: Uses `try_lock()` and skips frames if locks fail
- Server thread: Can use blocking `.lock().await`
- Smooth params via `SMOOTHING_COEFF` (0.99) to prevent clicks when values change

### The Sampleable Trait Pattern

Modules implement `Sampleable` via the `#[derive(Module)]` proc macro from `modular_derive`. Example structure from `modular_core/src/dsp/oscillators/sine.rs`:

```rust
#[derive(Default, Params)]
struct SineOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("phase", "the phase of the oscillator")]  
    phase: InternalParam,
}

#[derive(Default, Module)]
#[module("sine-osc", "A sine wave oscillator")]
pub struct SineOscillator {
    #[output("output", "signal output")]
    sample: f32,
    phase: f32,
    smoothed_freq: f32,
    params: SineOscillatorParams,
}
```

The `Module` derive generates:
- Constructor registration in `get_constructors()` map
- `Sampleable` trait implementation with `tick()`, `update()`, `get_sample()`
- TypeScript type exports for the web frontend

### V/Oct Signal Convention

Audio modules use **modular synth voltage conventions**:
- **Voltage range: -10.0V to 10.0V** (standard for all parameters and signals)
- 1V/octave for pitch: 4.0V = 440Hz (A4), 5.0V = 880Hz (A5)
- Base frequency: 27.5Hz at 0V (A0)
- Output signals: ±5V range (attenuated by `AUDIO_OUTPUT_ATTENUATION = 5.0` before speaker output)
- **When implementing frequency parameters, always clamp to `clamp(-10.0, 10.0, value)`**
- See `InternalParam::get_value()` for V/Oct → Hz conversion

### Patch System

A `Patch` is a graph of connected modules:
- **`PatchGraph`**: Serializable YAML representation (`modules: Vec<ModuleState>`)
- **`Patch`**: Runtime representation with `sampleables: SampleableMap` and `tracks: TrackMap`
- Params can be: `Value`, `Hz`, `Note`, `Cable` (connect to another module's output), or `Track` (automation)

Processing order in `process_frame()` (audio.rs):
1. Tick all tracks (advance automation playheads)
2. Update all modules (smooth params, prepare for next sample)
3. Tick all modules (compute output samples)
4. Capture samples for subscribed audio streams
5. Get root module output

### Hot-Reload Patch Updates

Patches are updated incrementally without stopping audio:

1. Parse YAML → `PatchGraph` struct
2. Validate against module schemas (see `validation.rs`)
3. Diff current patch to determine changes:
   - **Remove**: Modules in current but not in new patch
   - **Add**: Modules in new patch but not current
   - **Recreate**: Modules with same ID but different `module_type` (delete then recreate)
   - **Update**: Modules in both with same type - update params via `update_param()`
4. Apply changes while audio thread uses `try_lock()`

Key implementation details from `http_server.rs`:
- Compare `module_type` for modules with matching IDs to detect type changes
- Type changes require full deletion and recreation (can't just update params)
- Root module (id `"root"`) is never deleted or recreated
- Parameters are updated for all modules (new, recreated, and existing)

### Track/Automation System

Tracks enable timeline-based parameter automation. **Note: YAML serialization is planned but not yet implemented** - tracks currently exist only in the runtime `Patch` and must be created programmatically.

Structure:
- **Keyframes**: Store param values at specific times (Duration-based)
- **Playback modes**: `Once` (stop at end) or `Loop` (repeat)
- **Real-time interpolation**: Playhead advances by 1 sample per `tick()`
- **Parameter connection**: `Param::Track { track: "track_id" }` references a track by ID

When implementing YAML support, tracks will need to be added to the `PatchGraph` struct (currently only contains `modules: Vec<ModuleState>`). Keyframes are sorted by time on insertion, and `InnerTrack` maintains `playhead_idx` for efficient interpolation during playback.

### Type Generation & Validation

**TypeScript types are auto-generated** from Rust types via `ts-rs`:
```bash
cargo test export_types -- --ignored
# Or from frontend: pnpm run codegen
```

Exports go to `modular_web/src/types/generated/`. Any type annotated with `#[derive(TS)]` and `#[ts(export, export_to = "...")]` gets generated.

Patch validation happens in `modular_server/src/validation.rs` before applying changes - checks module types exist, cable connections valid, etc.

## Development Workflows

### Building & Running

```bash
# Backend server (default port 7812)
cd modular_server
cargo run

# Frontend dev server
cd modular_web
pnpm install
pnpm run dev

# Build optimized release
cargo build --release
```

### Adding a New DSP Module

1. Create file in `modular_core/src/dsp/{category}/`
2. Derive `Params` for parameter struct, `Module` for main struct
3. Implement `update()` method for DSP logic
4. Add to parent `mod.rs` module exports
5. Register in category's `install_constructors()` and `schemas()`
6. Regenerate TS types: `cargo test export_types -- --ignored`

Example modules: `modular_core/src/dsp/oscillators/sine.rs`, `filters/lowpass.rs`, `core/mix.rs`

### WebSocket Protocol

Binary messages for audio streaming:
```
[module_id UTF-8][0x00][port UTF-8][0x00][f32 samples as little-endian]
```

JSON messages for control (see `protocol.rs`):
- `SetPatch { yaml }` - Update entire patch from YAML
- `SubscribeAudio { subscription }` - Stream audio from module output
- `Mute/Unmute` - Control audio output
- `StartRecording/StopRecording` - WAV file recording

### Recording Workflow

WAV recording captures audio output to disk in real-time:

1. **Start recording**: Send `StartRecording { filename }` message
   - Creates WAV file with timestamp if no filename provided
   - Configures `WavWriter` with sample rate from audio device
   - Stores writer in `AudioState.recording_writer`

2. **Audio capture**: In audio callback (`make_stream()`):
   ```rust
   if let Ok(mut writer_guard) = audio_state.recording_writer.try_lock() {
       if let Some(ref mut writer) = *writer_guard {
           let _ = writer.write_sample(output_sample);
       }
   }
   ```
   Uses `try_lock()` to never block audio thread

3. **Stop recording**: Send `StopRecording` message
   - Finalizes WAV file (writes headers)
   - Returns recorded file path to client

Recording captures post-attenuation, post-mute audio - exactly what goes to speakers.

## Project Conventions

### Error Handling
- Use `anyhow::Result` for recoverable errors
- Audio thread uses `.unwrap_or(0.0)` or `.unwrap_or_default()` to never panic
- Validation errors are collected and returned as `Vec<ValidationError>` (don't fail fast)

### Module Organization
- `modular_core/src/dsp/`: Pure DSP modules (oscillators, filters, core utilities)
- `modular_derive/`: Proc macros for code generation
- `modular_server/src/`: HTTP/WebSocket server, protocol, validation, audio I/O
- `modular_web/src/`: React frontend with WebSocket client

### Testing
- DSP tests in `modular_core/tests/dsp_tests.rs` validate audio output
- Pattern: Create patch, call `process_frame()` multiple times, check sample values
- Run tests: `cargo test` (not in workspace root - use individual crate directories)

**DSP module testing guidelines**:
- **Frequency accuracy**: For oscillators, verify output frequency matches input V/Oct value
- **DC offset**: Oscillators should produce signals centered around 0.0 (no DC bias)
- **Range limiting**: Output values should stay within ±5V range to prevent clipping
- **Parameter smoothing**: Test rapid param changes don't produce clicks (smoothing works)
- **Edge cases**: Test with 0Hz, negative frequencies, extreme resonance values
- **Silence**: When disconnected params, output should be predictable (often 0.0)

Example test pattern from `dsp_tests.rs`:
```rust
let mut patch = create_patch_from_yaml(sample_rate, yaml)?;
for _ in 0..1000 {
    process_frame(&patch);
}
let output = patch.get_output();
assert!((output - expected).abs() < tolerance);
```

### Fish Shell Commands
This project uses fish shell. Note: fish doesn't support heredocs - use `printf` or `echo` instead for multi-line strings.

## Integration Points

- **Audio subscriptions**: Ring buffers in `AudioState.subscription_collection` capture samples per-frame, sent via tokio channels to WebSocket clients
- **YAML patches**: Serialize `PatchGraph` ↔ YAML for patch storage/loading (see `persistence.rs`)
- **Root module**: Special "root" module (id `ROOT_ID`) with "output" port - its output goes to speakers
- **Tracks**: Timeline automation system with keyframes, supports Once/Loop playback modes

## Common Pitfalls

- Don't add async or I/O to `modular_core` - it's a pure library
- Don't use blocking locks in audio callback (`process_frame()`) - always `try_lock()`
- Remember to register new modules in `get_constructors()` and export TS types
- Patch updates must be validated before applying to prevent invalid graph states
- Sample rate is passed to module constructors - store it for frequency calculations
