Activate the serena project modular

# Copilot instructions (modular)

## Big picture
- This repo is a Rust workspace + a Vite/React frontend for a real-time modular synth.
- The browser runs a **JavaScript DSL** that builds a `PatchGraph` (JSON) and sends it to the server.
- The Rust server hosts `/ws` (WebSocket) and streams control as JSON and audio buffers as **binary** frames.

Key crates/apps:
- `modular_core/`: DSP engine + patch graph types + module schemas (`modular_core/src/types.rs`, `modular_core/src/dsp/`).
- `modular_server/`: Axum HTTP/WS server + cpal audio thread (`modular_server/src/http_server.rs`, `modular_server/src/audio.rs`).
- `modular_derive/`: proc-macros used for schema/message plumbing (module registration patterns live in `modular_core/src/dsp/*`).
- `mi-plaits-dsp-rs/`: vendored DSP algorithms used by `modular_core`.
- `modular_web/`: React UI + DSL runtime (`modular_web/src/dsl/*`) + generated Rust→TS types (`modular_web/src/types/generated/`).

## Critical data flow
- Frontend executes DSL via `new Function(...)` and uses `GraphBuilder` to create a `PatchGraph` (`modular_web/src/dsl/executor.ts`, `modular_web/src/dsl/GraphBuilder.ts`).
- Server validates patches against schema + typed param validators before applying (`modular_server/src/validation.rs`, `modular_core/src/dsp/mod.rs`).
- Audio thread is real-time: it uses `try_lock()` and skips work on contention; do not add blocking/alloc-heavy work to the callback (`modular_server/src/audio.rs`).

## WebSocket protocol (practical notes)
- WS endpoint: `ws://localhost:7812/ws` (server default port is **7812**, see `modular_server/src/main.rs`).
- Control messages: JSON `InputMessage`/`OutputMessage` (`modular_server/src/protocol.rs`).
- Audio buffers: binary frames with a null-terminated header:
  - `[moduleId UTF-8][0x00][portName UTF-8 ("" for track)][0x00][f32 LE samples...]`
  - Client parses this in `modular_web/src/hooks/useWebSocket.ts`.
- Scopes drive streaming: `patch.scopes` becomes server-side subscriptions (`sync_scopes_for_connection` in `modular_server/src/http_server.rs`).

## Developer workflows
- Run server (from repo root so file ops see `*.mjs` patches in CWD): `cargo run -p modular_server`
- Frontend dev:
  - `cd modular_web && pnpm install && pnpm dev`
- Regenerate Rust→TS types (ts-rs):
  - `cd modular_web && pnpm run codegen`
  - This runs ignored test `tests::export_types` in `modular_server/src/lib.rs` and writes to `modular_web/src/types/generated/`.

## Repo-specific conventions to follow
- Patch graphs are the contract: prefer changing `modular_core::types` + exporting via ts-rs rather than hand-editing TS “schema” types.
- When adding/changing module params:
  - Update the Rust param struct/module implementation in `modular_core/src/dsp/**`.
  - Ensure schema/validator registration is wired through `install_constructors` / `install_param_validators` (see `modular_core/src/dsp/mod.rs`).
  - Regenerate TS types (`pnpm run codegen`) and update DSL builders if needed.
- For real-time safety: avoid allocations, logging, blocking locks in the audio callback; do “heavy” work on the async server thread.
- Voltage convention: signals/params are generally clamped to `-10.0..10.0`; audio outputs target ~±5V (see `InternalParam` + DSP modules).