# Modular Synthesizer DSL Guide

## Overview

The Modular Synthesizer uses a JavaScript-based Domain-Specific Language (DSL) for creating audio patches. This DSL provides a fluent, expressive API for building modular synthesis patches that are executed client-side and sent to the server as structured JSON.

## Quick Start

### Basic Sine Wave

```javascript
// Create a 440 Hz sine oscillator
const osc = sine('osc1').freq(hz(440));
out.source(osc);
```

### Using Musical Notes

```javascript
// Play an A4 note
const osc = sine('osc1').freq(note('a4'));
out.source(osc);
```

## Core Concepts

### Module Factories

Module factories create instances of synthesis modules. Each factory accepts an optional ID parameter for stable module identification:

```javascript
const osc1 = sine('my-osc');  // Explicit ID
const osc2 = sine();          // Auto-generated ID (sine-1, sine-2, etc.)
```

### Fluent Parameter Setting

Parameters can be set using method chaining:

```javascript
const osc = sine('osc')
  .freq(note('c4'))
  .phase(0.5);
```

### Signal Connections

Connect module outputs to parameters:

```javascript
const lfo = sine('lfo').freq(hz(5));
const osc = sine('osc')
  .freq(note('a4'))
  .phase(lfo.output);  // Connect LFO output to phase
```

### Output Routing

Every patch must specify an output source:

```javascript
out.source(myModule);
```

## Available Modules

### Oscillators

- `sine(id?)` - Sine wave oscillator
  - Parameters: `freq` (V/oct), `phase` (0-1)
  - Outputs: `output`

- `saw(id?)` - Sawtooth oscillator
  - Parameters: `freq` (V/oct), `phase` (0-1)
  - Outputs: `output`

- `pulse(id?)` - Pulse/square wave oscillator
  - Parameters: `freq` (V/oct), `phase` (0-1), `width` (0-1)
  - Outputs: `output`

### Utilities

- `signal(id?)` - Signal passthrough
  - Parameters: `source`
  - Outputs: `output`

- `scaleAndShift(id?)` - Scale and offset a signal
  - Parameters: `input`, `scale`, `shift`
  - Outputs: `output`

- `sum(id?)` - Sum multiple signals
  - Parameters: `a`, `b`
  - Outputs: `output`

- `mix(id?)` - Mix signals with crossfade
  - Parameters: `a`, `b`, `mix` (0-1)
  - Outputs: `output`

### Filters

- `lowpass(id?)` - Low-pass filter
- `highpass(id?)` - High-pass filter
- `bandpass(id?)` - Band-pass filter
- `notch(id?)` - Notch filter
- `allpass(id?)` - All-pass filter
- `stateVariable(id?)` - State variable filter
- `moogLadder(id?)` - Moog ladder filter
- `tb303(id?)` - TB-303 style filter
- `sem(id?)` - SEM filter
- `ms20(id?)` - MS-20 filter
- `formant(id?)` - Formant filter
- `sallenKey(id?)` - Sallen-Key filter

## Helper Functions

### Frequency Conversion

```javascript
hz(frequency)  // Convert Hz to V/oct
// Example: hz(440) → ~4.0 V/oct
```

### Note Names

```javascript
note(noteName)  // Convert note name to V/oct
// Examples:
note('c4')   // Middle C
note('a4')   // A440
note('c#4')  // C sharp
note('db4')  // D flat (enharmonic equivalent)
```

Supported note range: Any octave, notes a-g, with # or b modifiers.

### Voltage

```javascript
volts(value)  // Pass-through for clarity
// Example: volts(2.5) → 2.5
```

## Signal Transformations

Module outputs support chaining transformations:

```javascript
const lfo = sine('lfo').freq(hz(2));

// Scale the LFO output
const scaled = lfo.output.scale(0.5);

// Shift the LFO output
const shifted = lfo.output.shift(1.0);

// Chain transformations
const transformed = lfo.output.scale(0.5).shift(1.0);
```

## Examples

### FM Synthesis

```javascript
// Modulator at 2x carrier frequency
const modulator = sine('mod').freq(note('a4').scale(2));

// Carrier with frequency modulation
const carrier = sine('carrier')
  .freq(note('a4'))
  .phase(modulator.output.scale(0.5));

out.source(carrier);
```

### Low-Frequency Oscillator (LFO)

```javascript
// LFO modulating oscillator frequency
const lfo = sine('lfo').freq(hz(5));
const osc = sine('osc')
  .freq(note('c4'))
  .phase(lfo.output.scale(0.3));

out.source(osc);
```

## Keyboard Shortcuts

- **Alt+Enter** - Execute DSL and update patch
- **Alt+.** - Stop audio
- **Ctrl+R** - Toggle recording

## Technical Details

### Execution Model

1. DSL code is written in the browser editor
2. On Alt+Enter, the code is executed in a sandboxed JavaScript environment
3. The DSL runtime builds a PatchGraph JSON structure
4. The PatchGraph is sent to the server via WebSocket
5. The server validates and applies the patch to the audio engine

### Module IDs

Module IDs are deterministic based on:
- Explicit IDs provided to factory functions
- Auto-generated IDs using type + counter (e.g., `sine-1`, `sine-2`)

This ensures stable module identities across live-coding sessions.

