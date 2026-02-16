# Agent Testing Research: Improving Test Coverage for Modular Synthesizer

## Executive Summary

This document outlines strategies to enable AI agents to effectively test the Modular Synthesizer Electron application, which currently relies heavily on manual user testing for UI/UX and audio verification. The goal is to provide automated, programmatic testing that aligns with how users actually interact with the application.

## Current State Analysis

### Existing Test Infrastructure

- **Unit tests**: Ava test runner with 2 TypeScript test files in `src/__tests__/`
    - `interpolationMapping.spec.ts` - Tests DSL template interpolation
    - `patchSimilarityRemap.spec.ts` - Tests patch remapping logic
- **Rust tests**: Some DSP tests exist in `crates/modular_core/tests/` (currently disabled - requires legacy feature flag)
- **Manual testing**: All UI/UX and audio verification requires human observation

### Application Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Electron Renderer                     │
│  ┌──────────────┐    ┌─────────────┐   ┌────────────┐  │
│  │ Monaco Editor│───▶│ DSL Executor│──▶│ IPC Handler│  │
│  │   (.mjs)     │    │  (JS->JSON) │   └─────┬──────┘  │
│  └──────────────┘    └─────────────┘         │          │
└──────────────────────────────────────────────┼──────────┘
                                                │
                        ┌───────────────────────▼──────────┐
                        │       Electron Main Process      │
                        │  ┌────────────────────────────┐  │
                        │  │  executePatchScript()      │  │
                        │  │  - Builds PatchGraph JSON  │  │
                        │  └────────────┬───────────────┘  │
                        └───────────────┼──────────────────┘
                                        │
                        ┌───────────────▼──────────────────┐
                        │      Rust N-API Module           │
                        │  ┌──────────────────────────┐    │
                        │  │ Synthesizer::update_patch│    │
                        │  │ - Validates graph        │    │
                        │  │ - Applies to audio thread│    │
                        │  └────────┬─────────────────┘    │
                        │           │                       │
                        │  ┌────────▼─────────────────┐    │
                        │  │  Audio Thread (cpal)     │    │
                        │  │  - DSP processing        │    │
                        │  │  - Scope data collection │    │
                        │  └────────┬─────────────────┘    │
                        └───────────┼──────────────────────┘
                                    │
                        ┌───────────▼──────────────────┐
                        │  Scope Buffers (Ring Buffer) │
                        │  - Read via get_scopes()     │
                        └──────────────────────────────┘
```

### Key Entry Points for Testing

1. **DSL Execution** (`src/dsl/executor.ts`)
    - `executePatchScript(source: string, schemas: ModuleSchema[]): DSLExecutionResult`
    - Returns: `{ patch: PatchGraph, sourceLocationMap }`

2. **N-API Synthesizer** (`crates/modular/src/lib.rs`)
    - `new Synthesizer(config?: AudioConfigOptions)` - Creates audio engine
    - `updatePatch(patch: PatchGraph): ApplyPatchError[]` - Validates and applies patch
    - `getScopes(): Array<[ScopeItem, Float32Array[], ScopeStats]>` - Gets audio data
    - `startRecording(path?: string): string` - Records to WAV file
    - `stopRecording(): string` - Stops recording, returns path

3. **IPC Channels** (`src/ipcTypes.ts`)
    - `EXECUTE_DSL` - Executes DSL script and applies patch
    - `UPDATE_PATCH` - Updates patch graph
    - `GET_SCOPES` - Polls scope data

## Proposed Testing Strategies

### Strategy 1: Headless Electron Testing with Spectron/Playwright

**Overview**: Run the full Electron app in headless mode, interact with it programmatically.

**Pros**:

- Tests the complete user workflow (UI → DSL → Audio)
- Can verify Monaco editor interactions
- Tests IPC communication
- Tests actual audio device initialization

**Cons**:

- Slow (full app startup per test)
- Requires display server even if headless
- Audio device dependencies (may need virtual audio devices)
- Complex setup and flaky tests
- High maintenance burden

**Implementation Approach**:

```javascript
// Example with @playwright/test (Electron support)
import { _electron as electron } from '@playwright/test';

test('DSL execution produces correct patch', async () => {
    const app = await electron.launch({ args: ['.'] });
    const window = await app.firstWindow();

    // Type DSL code
    await window
        .locator('.monaco-editor')
        .fill('const osc = sine(440); out(osc);');

    // Execute with Alt+Enter
    await window.keyboard.press('Alt+Enter');

    // Check for errors
    const errors = await window.locator('.error-display').count();
    expect(errors).toBe(0);

    await app.close();
});
```

**Recommendation**: ❌ **Not recommended** - Too heavy for agent workflows, better suited for E2E smoke tests.

---

### Strategy 2: Direct N-API Testing (Node.js)

**Overview**: Import the Rust N-API module directly in Node.js and test synthesizer behavior.

**Pros**:

- Fast execution (no Electron overhead)
- Direct access to synthesizer methods
- Can test audio processing with scope buffers
- Can record WAV files for verification
- Agents can easily run with `node -e` or test files

**Cons**:

- Requires audio device (can be solved with dummy device or config)
- Doesn't test UI/UX or DSL execution
- Doesn't test IPC layer

**Implementation Approach**:

```javascript
// test-synthesizer.mjs
import { Synthesizer, getSchemas } from '@modular/core';

// Test basic patch application
const synth = new Synthesizer({
    hostId: 'Dummy', // Use dummy audio host
    sampleRate: 48000,
    bufferSize: 512,
});

const patch = {
    modules: [
        {
            id: 'sine-1',
            moduleType: 'sine',
            params: { freq: { type: 'volts', value: 4.0 } },
        },
        {
            id: 'root',
            moduleType: 'signal',
            params: {
                /* ... */
            },
        },
    ],
    connections: [
        /* ... */
    ],
};

const errors = synth.updatePatch(patch);
console.log('Validation errors:', errors);

// Wait for audio processing
await new Promise((resolve) => setTimeout(resolve, 100));

// Get scope data
const scopes = synth.getScopes();
console.log('Scope data:', scopes);
```

**Recommendation**: ✅ **Highly recommended** - Great for testing Rust audio engine and validation logic.

---

### Strategy 3: DSL Integration Testing (Node.js)

**Overview**: Test DSL execution in Node.js environment without full Electron.

**Pros**:

- Tests the critical DSL → PatchGraph transformation
- Fast execution
- Can test validation errors
- Tests how users actually write patches
- Easy for agents to run

**Cons**:

- Requires mocking IPC layer
- Doesn't test audio output
- May need to stub Electron APIs

**Implementation Approach**:

```javascript
// test-dsl-execution.mjs
import { executePatchScript } from './src/dsl/executor.ts';
import { getSchemas } from '@modular/core';

const schemas = getSchemas();

const dslCode = `
const freq = signal(440);
const osc = sine(freq);
out(osc);
`;

const result = executePatchScript(dslCode, schemas);

console.log('Generated patch:', JSON.stringify(result.patch, null, 2));
console.log('Module count:', result.patch.modules.length);
console.log('Connection count:', result.patch.connections.length);

// Assert expectations
if (result.patch.modules.length !== 3) {
    // freq, osc, root
    throw new Error(`Expected 3 modules, got ${result.patch.modules.length}`);
}
```

**Recommendation**: ✅ **Highly recommended** - Essential for testing the user-facing DSL.

---

### Strategy 4: Audio Verification Testing

**Overview**: Use the synthesizer's recording capability to generate WAV files, then analyze them programmatically.

**Pros**:

- Tests actual audio output
- Can verify waveforms, frequencies, amplitudes
- WAV files can be analyzed with npm packages
- Can detect audio glitches, DC offset, clipping

**Cons**:

- Takes time to render audio
- Requires audio analysis libraries
- Float comparison with tolerance needed

**Implementation Approach**:

```javascript
// test-audio-output.mjs
import { Synthesizer } from '@modular/core';
import WavDecoder from 'wav-decoder';
import fs from 'fs/promises';

const synth = new Synthesizer();

// Apply patch for 440Hz sine wave
synth.updatePatch({
    modules: [
        {
            id: 'sine-1',
            moduleType: 'sine',
            params: { freq: { type: 'volts', value: 4.0 } },
        },
        {
            id: 'root',
            moduleType: 'signal',
            params: {
                signals: [{ type: 'cable', module: 'sine-1', port: 'output' }],
            },
        },
    ],
    connections: [],
});

// Record 1 second of audio
const path = synth.startRecording();
await new Promise((resolve) => setTimeout(resolve, 1000));
const wavPath = synth.stopRecording();

// Analyze WAV file
const buffer = await fs.readFile(wavPath);
const audioData = await WavDecoder.decode(buffer);

// Verify frequency using FFT or zero-crossing analysis
const samples = audioData.channelData[0];
const freq = estimateFrequency(samples, audioData.sampleRate);

console.log(`Detected frequency: ${freq}Hz (expected ~440Hz)`);
if (Math.abs(freq - 440) > 5) {
    throw new Error(`Frequency mismatch: ${freq}Hz vs 440Hz`);
}
```

**Recommendation**: ✅ **Recommended** for audio correctness tests, but may be overkill for most agent workflows.

---

### Strategy 5: Scope Buffer Testing (Real-time)

**Overview**: Use `getScopes()` to read real-time audio data from scope buffers without recording.

**Pros**:

- Fast (no file I/O)
- Real-time feedback
- Can test signal flow
- Can verify module connections
- Lightweight for agents

**Cons**:

- Limited buffer size (ring buffer)
- Timing-dependent
- May need multiple polls to get data

**Implementation Approach**:

```javascript
// test-scope-buffers.mjs
import { Synthesizer } from '@modular/core';

const synth = new Synthesizer();

// Apply patch with scope on sine output
synth.updatePatch({
    modules: [
        {
            id: 'sine-1',
            moduleType: 'sine',
            params: { freq: { type: 'volts', value: 4.0 } },
        },
        {
            id: 'scope-1',
            moduleType: 'scope',
            params: {
                signal: { type: 'cable', module: 'sine-1', port: 'output' },
            },
        },
    ],
    connections: [],
});

// Wait for audio thread to process
await new Promise((resolve) => setTimeout(resolve, 100));

// Read scope data
const scopes = synth.getScopes();
console.log(`Found ${scopes.length} scopes`);

for (const [scopeItem, buffers, stats] of scopes) {
    console.log('Scope:', scopeItem);
    console.log('Channels:', buffers.length);
    console.log('Buffer size:', buffers[0]?.length || 0);
    console.log('Stats:', stats);

    // Verify non-zero signal
    const samples = Array.from(buffers[0] || []);
    const max = Math.max(...samples.map(Math.abs));
    console.log('Max amplitude:', max);

    if (max < 0.01) {
        throw new Error('No signal detected in scope buffer');
    }
}
```

**Recommendation**: ✅ **Highly recommended** - Best balance of speed and verification depth for agents.

---

### Strategy 6: Validation Testing (Unit)

**Overview**: Test the Rust validation layer directly through the N-API.

**Pros**:

- Fast
- Tests error messages
- Critical for user experience (error feedback)
- Easy for agents to verify

**Cons**:

- Doesn't test audio
- Limited to validation logic

**Implementation Approach**:

```javascript
// test-validation.mjs
import { Synthesizer } from '@modular/core';

const synth = new Synthesizer();

// Test invalid patch (missing module)
const invalidPatch = {
    modules: [
        {
            id: 'root',
            moduleType: 'signal',
            params: {
                signals: [
                    { type: 'cable', module: 'nonexistent', port: 'output' },
                ],
            },
        },
    ],
    connections: [],
};

const errors = synth.updatePatch(invalidPatch);
console.log('Validation errors:', errors);

if (errors.length === 0) {
    throw new Error('Expected validation error for missing module');
}

if (!errors[0].message.includes('nonexistent')) {
    throw new Error('Error message should mention missing module ID');
}
```

**Recommendation**: ✅ **Highly recommended** - Critical for ensuring good error messages.

---

### Strategy 7: Rust Unit/Integration Tests

**Overview**: Expand Rust test coverage in `modular_core` and `modular`.

**Pros**:

- Tests DSP correctness at the source
- Fast (compiled tests)
- Catches bugs before they reach JS
- Standard Rust workflow (`cargo test`)

**Cons**:

- Agents may struggle with Rust testing patterns
- Doesn't test JS/TS layer
- Requires Rust toolchain

**Implementation Approach**:

```rust
// crates/modular_core/tests/dsp_integration_tests.rs
use modular_core::dsp::get_constructors;
use modular_core::patch::Patch;

#[test]
fn test_sine_oscillator_output() {
    let constructors = get_constructors();
    let sine_constructor = constructors.get("sine").unwrap();

    let sine_module = sine_constructor(&"sine-1".to_string(), 48000.0).unwrap();

    // Set frequency to A4 (440Hz)
    sine_module.try_update_params(
        json!({ "freq": { "type": "volts", "value": 4.0 } }),
        1
    ).unwrap();

    // Process 100 samples
    let mut samples = vec![];
    for _ in 0..100 {
        sine_module.tick();
        sine_module.update();
        let output = sine_module.get_poly_sample("output").unwrap();
        samples.push(output.get(0));
    }

    // Verify oscillation
    let max = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min = samples.iter().cloned().fold(f32::INFINITY, f32::min);

    assert!(max > 0.5, "Sine should produce positive peaks");
    assert!(min < -0.5, "Sine should produce negative peaks");
}
```

**Recommendation**: ✅ **Recommended** - Core DSP tests should be in Rust, but may not be primary focus for agent workflows.

---

## Recommended Implementation Plan

### Phase 1: Core Testing Infrastructure (Week 1)

**Goal**: Enable basic programmatic testing for agents

1. **Setup N-API testing harness**
    - Create `test-harness/` directory
    - Add helper scripts for common test patterns
    - Document how to run tests with `node -e`

2. **Implement Scope Buffer Testing**
    - Create `test-harness/scope-test-helper.mjs`
    - Add utilities for reading and analyzing scope data
    - Add example test cases

3. **Implement Validation Testing**
    - Create `test-harness/validation-test-helper.mjs`
    - Add utilities for testing error messages
    - Add example test cases

**Deliverables**:

```
test-harness/
├── README.md                      # How agents can use the test harness
├── scope-test-helper.mjs          # Scope buffer utilities
├── validation-test-helper.mjs     # Validation utilities
├── examples/
│   ├── test-sine-oscillator.mjs
│   ├── test-validation-errors.mjs
│   └── test-module-connections.mjs
└── utils/
    ├── audio-analysis.mjs         # FFT, frequency detection, etc.
    └── patch-builder.mjs          # Helper for building patches
```

### Phase 2: DSL Testing (Week 2)

**Goal**: Test user-facing DSL execution

1. **Add DSL test runner**
    - Create `test-harness/dsl-test-helper.mjs`
    - Support inline DSL code execution
    - Capture patch graph output

2. **Add DSL assertion utilities**
    - Verify module counts
    - Verify connection counts
    - Verify parameter values

**Deliverables**:

```
test-harness/
├── dsl-test-helper.mjs
└── examples/
    ├── test-dsl-basic.mjs
    ├── test-dsl-connections.mjs
    └── test-dsl-errors.mjs
```

### Phase 3: Audio Verification (Week 3)

**Goal**: Verify audio output correctness

1. **Add recording utilities**
    - Create `test-harness/audio-recording-helper.mjs`
    - Add WAV analysis utilities

2. **Add audio analysis**
    - Frequency detection
    - Amplitude measurement
    - DC offset detection
    - Clipping detection

**Deliverables**:

```
test-harness/
├── audio-recording-helper.mjs
└── examples/
    ├── test-audio-frequency.mjs
    └── test-audio-amplitude.mjs
```

### Phase 4: Integration with Existing Test Suite (Week 4)

**Goal**: Integrate with Ava test runner

1. **Convert test harness to Ava tests**
    - Add test-harness tests to `src/__tests__/`
    - Update `package.json` scripts

2. **Add CI integration**
    - Run tests in GitHub Actions
    - Add audio device mocking for CI

---

## Proof of Concept: Quick Win Tests

Here are 3 tests that can be implemented immediately with minimal setup:

### Test 1: Basic Synthesizer Instantiation

```javascript
// test-synth-init.mjs
import { Synthesizer } from '@modular/core';

const synth = new Synthesizer();
console.log('Sample rate:', synth.sampleRate());
console.log('Channels:', synth.channels());
console.log('✓ Synthesizer initialized successfully');
```

**Run**: `node test-synth-init.mjs`

### Test 2: Patch Validation

```javascript
// test-patch-validation.mjs
import { Synthesizer } from '@modular/core';

const synth = new Synthesizer();

// Valid patch
const validPatch = {
    modules: [
        { id: 'sine-1', moduleType: 'sine', params: {} },
        { id: 'root', moduleType: 'signal', params: {} },
    ],
    connections: [],
};

let errors = synth.updatePatch(validPatch);
if (errors.length > 0) {
    throw new Error('Valid patch should not produce errors');
}
console.log('✓ Valid patch accepted');

// Invalid patch
const invalidPatch = {
    modules: [
        {
            id: 'root',
            moduleType: 'signal',
            params: {
                signals: [{ type: 'cable', module: 'missing', port: 'out' }],
            },
        },
    ],
    connections: [],
};

errors = synth.updatePatch(invalidPatch);
if (errors.length === 0) {
    throw new Error('Invalid patch should produce errors');
}
console.log('✓ Invalid patch rejected with error:', errors[0].message);
```

**Run**: `node test-patch-validation.mjs`

### Test 3: Scope Data Collection

```javascript
// test-scope-data.mjs
import { Synthesizer } from '@modular/core';

const synth = new Synthesizer();

const patch = {
    modules: [
        {
            id: 'sine-1',
            moduleType: 'sine',
            params: {
                freq: { type: 'volts', value: 4.0 },
            },
        },
        {
            id: 'scope-1',
            moduleType: 'scope',
            params: {
                signal: {
                    type: 'cable',
                    module: 'sine-1',
                    port: 'output',
                    channel: 0,
                },
            },
        },
        { id: 'root', moduleType: 'signal', params: {} },
    ],
    connections: [],
};

synth.updatePatch(patch);

// Wait for audio processing
await new Promise((resolve) => setTimeout(resolve, 200));

const scopes = synth.getScopes();
console.log(`Found ${scopes.length} scope(s)`);

if (scopes.length === 0) {
    throw new Error('No scope data collected');
}

const [scopeItem, buffers, stats] = scopes[0];
const samples = Array.from(buffers[0]);
console.log(`Collected ${samples.length} samples`);
console.log('First 10 samples:', samples.slice(0, 10));

const maxAmplitude = Math.max(...samples.map(Math.abs));
console.log('Max amplitude:', maxAmplitude);

if (maxAmplitude < 0.01) {
    throw new Error('Expected non-zero signal amplitude');
}

console.log('✓ Scope data collected successfully');
```

**Run**: `node test-scope-data.mjs`

---

## Follow-up Questions

Before finalizing the implementation plan, please clarify:

### 1. Audio Device Handling

**Question**: How should tests handle audio devices?

- **Option A**: Require real audio device (current behavior)
- **Option B**: Add "null" audio device support in Rust (no actual audio output)
- **Option C**: Mock audio device in CI, real device locally
- **Your preference**: ?

### 2. Test Scope

**Question**: What level of audio verification is needed?

- **Option A**: Just validate patch graphs (no audio analysis)
- **Option B**: Basic signal detection (scope buffers show non-zero values)
- **Option C**: Full audio analysis (FFT, frequency detection, waveform matching)
- **Your preference**: ?

### 3. DSL Testing Depth

**Question**: How deeply should DSL tests cover edge cases?

- **Option A**: Basic syntax and module creation
- **Option B**: Complex patches (FM synthesis, sequences, collections)
- **Option C**: Error handling and edge cases (missing modules, circular connections)
- **Your preference**: ?

### 4. UI/UX Testing

**Question**: Should we implement any UI testing?

- **Option A**: No UI testing (focus on audio/DSL)
- **Option B**: Basic Playwright tests for smoke testing
- **Option C**: Full E2E test suite
- **Your preference**: ?

### 5. Integration with Agent Workflow

**Question**: How should agents discover and run tests?

- **Option A**: Document in README, agents run `node test-harness/examples/*.mjs`
- **Option B**: Add yarn scripts like `yarn test:audio`, `yarn test:dsl`
- **Option C**: Integrate into existing `yarn test` (Ava)
- **Your preference**: ?

### 6. Audio Analysis Libraries

**Question**: Should we add audio analysis dependencies?

- **Libraries to consider**:
    - `wav-decoder` - WAV file parsing
    - `dsp.js` or `fft.js` - FFT for frequency analysis
    - `audio-buffer-utils` - Audio buffer manipulation
- **Trade-off**: More dependencies vs. better audio verification
- **Your preference**: ?

### 7. Rust Test Coverage

**Question**: Should we prioritize expanding Rust tests?

- **Option A**: Focus on JS/TS tests (easier for agents)
- **Option B**: Expand Rust tests in parallel
- **Option C**: Rust tests only for DSP-critical code
- **Your preference**: ?

---

## Pros and Cons Summary

### Overall Approach

#### Pros ✅

1. **Programmatic testing** - Agents can run tests without GUI
2. **Fast feedback** - Most tests run in < 1 second
3. **Real audio engine** - Tests actual Rust implementation, not mocks
4. **Gradual adoption** - Can implement incrementally
5. **Low maintenance** - No brittle UI selectors or timing issues
6. **Reusable utilities** - Test helpers benefit both agents and humans

#### Cons ❌

1. **Audio device dependency** - May need dummy device or CI mocking
2. **Limited UI coverage** - Won't catch Monaco editor bugs
3. **Timing sensitivity** - Scope buffer tests may be flaky
4. **Setup complexity** - Requires building Rust N-API module
5. **No visual verification** - Can't verify oscilloscope rendering
6. **Partial coverage** - Doesn't test full user workflow

### Recommended Priority

**High Priority** (implement first):

1. ✅ Scope Buffer Testing (Strategy 5)
2. ✅ Validation Testing (Strategy 6)
3. ✅ DSL Integration Testing (Strategy 3)

**Medium Priority** (implement if time allows): 4. ✅ Direct N-API Testing (Strategy 2) 5. ✅ Audio Verification Testing (Strategy 4)

**Low Priority** (optional): 6. ⚠️ Rust Unit Tests (Strategy 7) - good practice but not critical for agents 7. ❌ Headless Electron (Strategy 1) - too heavy, save for E2E smoke tests

---

## Next Steps

1. **Clarify follow-up questions** (see above)
2. **Approve implementation plan** or suggest modifications
3. **Create proof-of-concept** with 3 quick-win tests
4. **Iterate based on agent feedback** - see what works in practice
5. **Document test patterns** for agent consumption
6. **Integrate with CI/CD** for continuous verification

---

## Additional Considerations

### Performance

- Scope buffer tests: ~100-200ms per test
- Validation tests: <10ms per test
- DSL tests: ~50ms per test
- Audio recording tests: 1-10 seconds per test

### Maintenance

- Test helpers should be well-documented
- Examples should cover common patterns
- Tests should fail clearly with actionable messages

### Agent Experience

- Tests should be runnable with simple commands
- Output should be easy to parse (JSON or structured text)
- Errors should include context (patch graph, module IDs, etc.)

### Future Enhancements

- Visual regression testing (screenshot comparison)
- Performance benchmarking (audio processing latency)
- Fuzzing (random patch generation + validation)
- MIDI testing (virtual MIDI devices)
