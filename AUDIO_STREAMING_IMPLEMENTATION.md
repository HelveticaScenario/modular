# Audio Streaming Implementation Summary

## Overview

Implemented WebSocket-based binary audio streaming from arbitrary modules in the synthesis graph to a browser-based oscilloscope client.

## What Was Implemented

### 1. Message Protocol Extensions (`modular_core/src/message.rs`)

Added new message types for audio subscription:

**Input Messages:**
- `SubscribeAudio { module_id, port, buffer_size }` - Subscribe to audio from a specific module/port
- `UnsubscribeAudio { subscription_id }` - Unsubscribe from audio stream

**Output Messages:**
- `AudioSubscribed { subscription_id }` - Confirmation of subscription with unique ID
- `AudioBuffer { subscription_id, samples: Vec<f32> }` - Audio sample buffer

### 2. Audio Capture System (`modular_core/src/patch.rs`)

**New structures:**
- `ScopeItem` - Identifies a module output or track scope (module_id + port_name, or track_id)
- Added `audio_subscriptions` and `audio_buffers` fields to `Patch`

**Capture mechanism:**
- In `process_frame()`: Captures samples from subscribed modules during audio callback
- Accumulates samples in per-subscription buffers
- When buffer reaches target size, sends via crossbeam channel
- Includes buffer size limiting to prevent unbounded growth

### 3. WebSocket Binary Streaming (`modular_server/src/http_server.rs`)

**Binary frame format:**
```
[scope id UTF-8][0x00][port name UTF-8 or empty for tracks][0x00][f32 samples as little-endian bytes]
```

**Handling:**
- Modified WebSocket handler to detect `AudioBuffer` messages
- Converts to binary format automatically
- Sends binary frames for audio, JSON for control messages
- Efficient: no base64 encoding needed

### 4. Browser Oscilloscope Client (`oscilloscope.html`)

**Features:**
- Real-time waveform visualization
- WebSocket connection management
- Audio subscription controls (module ID, port, buffer size)
- Canvas rendering with grid and waveform
- FPS counter and signal statistics (min/max)
- Responsive design with dark theme

**Architecture:**
- Uses native WebSocket API with `arraybuffer` binary type
- Parses binary frames to extract Float32Array samples
- Renders at up to ~94 FPS (with 512-sample buffers @ 48kHz)

### 5. Test Example (`modular_client/examples/oscilloscope_test.rs`)

- Creates a 440Hz sine wave oscillator
- Connects to root output for audio playback
- Demonstrates declarative patch API
- Ready-to-run test case for oscilloscope

## File Changes

### Modified Files:
1. `modular_core/src/message.rs` - Added audio subscription messages and handlers
2. `modular_core/src/patch.rs` - Added audio capture and subscription management
3. `modular_core/src/lib.rs` - Export ScopeItem for subscriptions
4. `modular_server/src/http_server.rs` - Binary audio frame handling in WebSocket

### New Files:
1. `oscilloscope.html` - Browser-based oscilloscope client
2. `OSCILLOSCOPE.md` - Complete documentation and usage guide
3. `modular_client/examples/oscilloscope_test.rs` - Example setup for testing

## Architecture Flow

```
┌─────────────────┐
│  Audio Thread   │ (Real-time, modular_core)
│  - Processes    │
│    audio graph  │
│  - Captures     │
│    samples from │
│    subscribed   │
│    modules      │
└────────┬────────┘
         │ crossbeam::channel
         │ (AudioBuffer messages)
         ▼
┌─────────────────┐
│  HTTP Server    │ (Async, modular_server)
│  - Receives     │
│    from channel │
│  - Converts to  │
│    binary       │
└────────┬────────┘
         │ WebSocket Binary
         │ (subscription_id + f32[])
         ▼
┌─────────────────┐
│  Browser Client │ (JavaScript)
│  - Parses binary│
│  - Renders      │
│    waveform     │
└─────────────────┘
```

## Performance Characteristics

- **Latency**: ~10-20ms at typical buffer sizes (512-1024 samples @ 48kHz)
- **Throughput**: Easily handles multiple simultaneous streams
- **CPU Impact**: Minimal - samples copied once, no encoding overhead
- **Network**: Binary frames are ~2KB for 512 samples (vs ~4KB+ for JSON)

## Usage Example

```bash
# Terminal 1: Start server
cargo run --bin modular_server -- --port 7812

# Terminal 2: Create audio modules
cargo run --example oscilloscope_test

# Browser: Open oscilloscope.html
# 1. Click "Connect"
# 2. Enter module: "sine-1", port: "output"
# 3. Click "Subscribe to Audio"
```

## Future Enhancements

Possible extensions:
- **Multi-channel**: Display multiple waveforms simultaneously
- **XY mode**: Plot two signals against each other (Lissajous)
- **Triggering**: Stable waveform display with level/edge triggers
- **Zoom/Pan**: Navigate time axis for detailed inspection
- **FFT display**: Frequency spectrum analysis
- **Recording**: Capture audio to WAV file in browser
- **Stereo scope**: Left/right channel visualization

## Technical Notes

### Why Binary WebSocket?

1. **Efficiency**: No JSON encoding/parsing for audio data
2. **Native support**: Float32Array maps directly to audio samples
3. **Low latency**: Minimal processing overhead
4. **Standard**: Works in all modern browsers

### Buffer Size Selection

- **Small (64-256)**: Lower latency, higher frame rate, more CPU/network overhead
- **Medium (512-1024)**: Good balance for visualization (~10-20ms, 50-90 FPS)
- **Large (2048+)**: Lower overhead, but higher latency and choppier display

### Thread Safety

- Audio capture happens in real-time audio thread
- Uses lock-free crossbeam channel to send to async runtime
- No blocking operations in audio thread
- WebSocket operates in async Tokio runtime
