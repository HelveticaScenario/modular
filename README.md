# Modular Synthesizer

A real-time modular synthesizer with a JavaScript DSL for live-coding audio patches.

## Features

- **JavaScript DSL** - Expressive, fluent API for building synthesis patches
- **Real-time Audio** - Low-latency audio processing in Rust
- **Live Coding** - Update patches on-the-fly with Alt+Enter
- **WebSocket Streaming** - Real-time audio streaming to browser
- **Visual Feedback** - Built-in oscilloscope for waveform visualization
- **Module Library** - Oscillators, filters, utilities, and more

## Quick Start

### Prerequisites

- Rust (latest stable)
- Node.js 24.12+
- Yarn (latest)

### Running the Server

```bash
cd modular_server
cargo run
```

The server will start on `http://localhost:3000`

### Running the Frontend

```bash
cd modular_web
yarn install
yarn dev
```

The frontend will be available at `http://localhost:5173`

## Usage

1. Open the web interface
2. Write a patch using the JavaScript DSL (see examples below)
3. Press **Alt+Enter** to execute and hear the result
4. Press **Alt+.** to stop audio

### Example Patches

**Simple Sine Wave:**
```javascript
const osc = sine('osc1').freq(hz(440));
out.source(osc);
```

**Musical Note:**
```javascript
const osc = sine('osc1').freq(note('a4'));
out.source(osc);
```

**FM Synthesis:**
```javascript
const modulator = sine('mod').freq(note('a4'));
const carrier = sine('carrier')
  .freq(note('a4'))
  .phase(modulator.scale(0.5));
out.source(carrier);
```

## Documentation

- [DSL Guide](docs/DSL_GUIDE.md) - Complete DSL reference and examples
- [Migration Plan](docs/patch-dsl-migration-plan.md) - Technical migration details

## Project Structure

```
modular/
├── modular_core/       # Core audio engine (Rust)
├── modular_server/     # WebSocket server (Rust)
├── modular_web/        # Web frontend (React + TypeScript)
│   └── src/dsl/        # DSL runtime
└── docs/               # Documentation
```

## Architecture

### Backend (Rust)

- **modular_core** - Audio DSP engine with module system
- **modular_server** - WebSocket server, patch validation, audio streaming

### Frontend (TypeScript/React)

- **DSL Runtime** - Executes JavaScript patches, generates PatchGraph JSON
- **Editor** - Monaco-based editor with autocomplete and oscilloscopes
- **WebSocket Client** - Communicates with server

## Development

### Type Generation

After modifying Rust types:

```bash
cd modular_web
yarn run codegen
```

### Building

```bash
# Backend
cargo build

# Frontend
cd modular_web
yarn run build
```

### Testing

```bash
# Backend tests
cargo test

# Frontend tests
cd modular_web
yarn test
```

## License

MIT

