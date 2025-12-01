# Audio Oscilloscope

Browser-based oscilloscope for visualizing audio from any module in the modular synthesis graph.

## Quick Start

### 1. Start the server
```bash
cargo run --bin modular_server -- --port 7812
```

### 2. Create some audio modules

Using the WebSocket API with YAML messages, create a patch that generates audio:

```bash
# Connect with a WebSocket client (e.g., wscat)
npm install -g wscat
wscat -c ws://localhost:7812/ws
```

Send a patch configuration (declarative API):

```yaml
type: set-patch
graph:
  modules:
    - id: sine-1
      module_type: sine-oscillator
      params:
        freq:
          param_type: value
          value: 4.0
    - id: root
      module_type: signal
      params:
        source:
          param_type: cable
          module: sine-1
          port: output
```

### 3. Open the oscilloscope

Open `oscilloscope.html` in your browser (or serve it with a local server):

```bash
# Option 1: Just open the file
open oscilloscope.html

# Option 2: Serve with Python
python3 -m http.server 8000
# Then visit http://localhost:8000/oscilloscope.html
```

### 4. Connect and subscribe

1. Click "Connect" (default WebSocket URL is `ws://localhost:7812/ws`)
2. Enter the module ID you want to visualize (e.g., `sine-1` or `root`)
3. Enter the port name (usually `output`)
4. Set buffer size (512 is good for ~60 FPS at 48kHz sample rate)
5. Click "Subscribe to Audio"

You should now see the waveform visualized in real-time!

## Features

- **Real-time visualization**: Audio is streamed via WebSocket binary frames
- **Multiple module support**: Can visualize audio from any module/port in the graph
- **Adjustable buffer size**: Balance between latency and visual smoothness
- **FPS counter**: Monitor streaming performance
- **Min/Max display**: See signal amplitude range

## Architecture

### Audio Capture Flow

1. **Audio thread** (`modular_core/src/patch.rs`):
   - Captures samples from specified modules during audio callback
   - Accumulates samples in buffers per subscription
   - When buffer reaches target size, sends via crossbeam channel

2. **WebSocket handler** (`modular_server/src/http_server.rs`):
   - Receives audio buffers from channel
   - Converts to binary format (subscription_id + f32 samples)
   - Sends via WebSocket binary frames

3. **Browser client** (`oscilloscope.html`):
   - Receives binary WebSocket messages
   - Extracts samples as Float32Array
   - Renders on canvas using 2D context

### Message Protocol

#### Subscribe to audio (Client → Server)
```yaml
type: subscribe-audio
module_id: sine-1
port: output
buffer_size: 512
```

#### Audio subscribed confirmation (Server → Client)
```yaml
type: audio-subscribed
subscription_id: uuid-here
```

#### Audio buffer (Server → Client, binary)
```
[subscription_id as UTF-8][null byte][f32 samples as little-endian bytes]
```

#### Unsubscribe (Client → Server)
```yaml
type: unsubscribe-audio
subscription_id: uuid-here
```

## Tips

- **Buffer size**: Smaller buffers = lower latency but more network overhead. 512-1024 works well.
- **Sample rate**: At 48kHz, a 512-sample buffer = ~10.6ms per frame ≈ 94 FPS max
- **Performance**: Binary WebSocket transport is efficient. Can easily handle multiple simultaneous streams.
- **Debugging**: Open browser DevTools console to see connection status and messages

## Extending

To add multi-channel visualization or XY mode:
1. Subscribe to multiple audio streams with different IDs
2. Track subscription IDs in the client
3. Render each stream to a separate canvas or combine them

To add triggering or zoom:
1. Store recent audio history in a circular buffer
2. Implement trigger detection (zero-crossing, level, etc.)
3. Render from triggered position with zoom level
