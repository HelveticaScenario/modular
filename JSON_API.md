# JSON API Test

Test the JSON API by connecting with a TCP client:

```bash
# Start the server
cargo run --bin modular_server -- --port 7812

# In another terminal, connect with netcat
nc localhost 7812
```

## Example JSON Messages

### Echo
```json
{"type":"echo","message":"Hello"}
```

### Get Schema
```json
{"type":"schema"}
```

### Get All Modules
```json
{"type":"get-modules"}
```

### Create a Sine Oscillator
```json
{"type":"create-module","module_type":"sine-oscillator","id":"550e8400-e29b-41d4-a716-446655440000"}
```

### Update a Parameter
```json
{"type":"update-param","id":"550e8400-e29b-41d4-a716-446655440000","param_name":"freq","param":{"param_type":"value","value":4.0}}
```

Note: The `freq` parameter uses voltage control (v/oct), where 4.0v ≈ 440Hz.

## Audio Streaming (NEW)

### Subscribe to Audio
Stream audio samples from any module/port to visualize or process:

```json
{"type":"subscribe-audio","module_id":"sine-1","port":"output","buffer_size":512}
```

Response (JSON):
```json
{"type":"audio-subscribed","subscription_id":"uuid-here"}
```

Then audio buffers are sent as **binary WebSocket frames** (not JSON):
```
[subscription_id as UTF-8][null byte][f32 samples as little-endian bytes]
```

### Unsubscribe from Audio
```json
{"type":"unsubscribe-audio","subscription_id":"uuid-here"}
```

See [OSCILLOSCOPE.md](./OSCILLOSCOPE.md) for complete documentation and browser-based oscilloscope client.

## Response Format

All responses are JSON objects with a `type` field indicating the message type. Examples:

```json
{"type":"echo","message":"Hello!"}
```

```json
{"type":"schema","schemas":[...]}
```

```json
{"type":"patch-state","modules":[...]}
```

```json
{"type":"error","message":"Module not found"}
```
