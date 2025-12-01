# WebSocket API

The modular synthesizer server uses a WebSocket-based API with YAML message format.

## Connecting

```bash
# Start the server
cargo run --bin modular_server -- --port 7812

# Connect via WebSocket at ws://localhost:7812/ws
```

The server supports both YAML and JSON message formats for input (backward compatibility), but responses are always in YAML format.

## Message Types

### Echo

```yaml
type: echo
message: Hello
```

### Get Schema

```yaml
type: schema
```

### Get All Modules

```yaml
type: get-modules
```

### Set Patch (Declarative API)

The recommended way to configure the synthesizer is using the declarative `set-patch` message, which specifies the complete desired state:

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
    - id: signal
      module_type: signal
      params:
        input:
          param_type: cable
          module: sine-1
          port: output
```

Note: The `freq` parameter uses voltage control (v/oct), where 4.0v ≈ 440Hz.

### Mute/Unmute Audio

```yaml
type: mute
```

```yaml
type: unmute
```

Note: Applying a patch with `set-patch` automatically unmutes audio.

## Audio Streaming

### Subscribe to Audio

Stream audio samples from any module/port to visualize or process:

```yaml
type: subscribe-audio
module_id: sine-1
port: output
buffer_size: 512
```

Response:
```yaml
type: audio-subscribed
subscription_id: uuid-here
```

Then audio buffers are sent as **binary WebSocket frames** (not YAML):
```
[subscription_id as UTF-8][null byte][f32 samples as little-endian bytes]
```

### Unsubscribe from Audio

```yaml
type: unsubscribe-audio
subscription_id: uuid-here
```

See [OSCILLOSCOPE.md](./OSCILLOSCOPE.md) for complete documentation and browser-based oscilloscope client.

## Response Format

All responses are YAML objects with a `type` field indicating the message type. Examples:

```yaml
type: echo
message: Hello!
```

```yaml
type: schema
schemas:
  - name: sine-oscillator
    description: Sine wave oscillator
    params:
      - name: freq
        description: Frequency in v/oct
    outputs:
      - name: output
        description: Audio output
```

```yaml
type: patch-state
modules:
  - id: sine-1
    module_type: sine-oscillator
    params:
      freq:
        param_type: value
        value: 4.0
```

```yaml
type: error
message: Module not found
```

```yaml
type: muted
```

```yaml
type: unmuted
```
