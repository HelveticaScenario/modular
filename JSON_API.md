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
{"type":"create-module","module_type":"sine","id":"550e8400-e29b-41d4-a716-446655440000"}
```

### Update a Parameter
```json
{"type":"update-param","id":"550e8400-e29b-41d4-a716-446655440000","param_name":"frequency","param":{"param_type":"value","value":440.0}}
```

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
