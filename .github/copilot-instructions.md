# Copilot instructions (modular)

NEVER STAGE OR COMMIT FILES

## Big picture
- Electron app (React/TypeScript renderer) + Rust DSP engine exposed via N-API.
- The JS DSL builds a `PatchGraph` JSON, sent over Electron IPC to the Rust `Synthesizer`.
- Audio runs in-process via cpal; scope data streams back to the renderer for Monaco oscilloscope overlays.

Key areas:
- Rust DSP/types: `crates/modular_core/` (`types.rs`, `patch.rs`, `dsp/`).
- N-API bindings + audio thread: `crates/modular/` (`lib.rs`, `audio.rs`, `validation.rs`).
- Electron app: `src/` (main: `main.ts`, `preload.ts`; renderer: `renderer.tsx`, `App.tsx`, `dsl/`, `components/`).

## Critical data flow
1. DSL executed via `new Function(...)` in `src/dsl/executor.ts` → `PatchGraph`.
2. IPC channels defined in `src/ipcTypes.ts` → main process calls `synthesizer.updatePatch(graph)`.
3. Rust validates in `crates/modular/src/validation.rs`, applies on audio thread in `crates/modular/src/audio.rs`.
4. Renderer polls scope buffers (ring buffer) and draws Monaco overlays.

## Workflows
- Run app: `yarn start` (electron-forge; rebuilds Rust on changes).
- Rebuild N-API module only: `cd crates/modular && yarn build` (or `yarn build:debug`).
- After Rust type changes (`#[napi]`), rebuild to refresh `crates/modular/index.d.ts` (`@modular/core`).
- Lint/typecheck: `yarn lint`, `yarn typecheck` (renderer only).

## Project-specific conventions
- Patch graphs are the contract: update Rust types in `modular_core::types` instead of hand-editing TS.
- Adding/changing module params:
  1) update `modular_core/src/dsp/**/*.rs` param structs + DSP
  2) wire schema/validators in category modules (e.g., `oscillators/mod.rs` via `install_constructors` / `install_param_validators`)
  3) rebuild N-API for updated TS types
  4) adjust DSL factories in `src/dsl/factories.ts` if needed
- Real-time safety in audio callback: avoid allocations/logging; validate on main thread.
- Voltage conventions:
  - **V/Oct pitch**: use 1V/oct (0V = C4 ~261.63Hz) for frequency
  - **Gates and triggers**: output `GATE_HIGH_VOLTAGE` (5V) when high, `GATE_LOW_VOLTAGE` (0V) when low. Use constants from `crates/modular_core/src/dsp/utils.rs`.
  - **Gate/trigger detection**: use Schmitt trigger with hysteresis. High threshold `GATE_DETECTION_HIGH_THRESHOLD` (1.0V), low threshold `GATE_DETECTION_LOW_THRESHOLD` (0.1V). Use `SchmittTrigger::default()` for standard behavior, or the constants for custom logic.
  - Output attenuation: `AUDIO_OUTPUT_ATTENUATION` in `crates/modular/src/audio.rs`.
- Prefer Electron APIs over web/React APIs when either could solve a task (see `src/**/*.ts`).
- **Dependency size**: This is an Electron app with locally-served bundles. NPM package size doesn't matter (no CDN/network concerns). Heavy dependencies like `ts-morph` are acceptable for developer experience.
- Reserved output names: when adding methods to `ModuleOutput`, `ModuleOutputWithRange`, `BaseCollection`, `Collection`, or `CollectionWithRange`, add the method name to `RESERVED_OUTPUT_NAMES` in all three locations:
  - `crates/modular_derive/src/lib.rs` (Rust compile-time check)
  - `src/dsl/factories.ts` (runtime sanitization)
  - `src/dsl/typescriptLibGen.ts` (type generation)

## File I/O + patches
- Main process handles workspace selection and `.mjs` patch read/write in `src/main.ts`.
- DSL globals live in `src/dsl/executor.ts` (e.g., `sine`, `saw`, `track`, `scope`, `hz`, `note`).
- Example patches: root-level `.mjs`; execute with Alt+Enter in the editor.