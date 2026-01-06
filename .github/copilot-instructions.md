# Copilot instructions (modular)

## Big picture
- This repo is an **Electron app** with a Rust DSP engine exposed via **NAPI bindings**.
- The frontend (React/TypeScript) runs a **JavaScript DSL** that builds a `PatchGraph` (JSON) and sends it to the Rust audio engine via IPC.
- Audio runs in-process via cpal; Rust exposes `Synthesizer` class to Node.js/Electron main process.

Key crates/apps:
- `crates/modular_core/`: DSP engine + patch graph types + module schemas (`types.rs`, `dsp/`, `patch.rs`).
- `crates/modular/`: NAPI bindings (`lib.rs`, `audio.rs`) that expose `Synthesizer` to Node.js + handle cpal audio thread.
- `crates/modular_derive/`: proc-macros for schema/message plumbing (used in `modular_core/src/dsp/*` modules).
- `crates/mi-plaits-dsp-rs/`: vendored Mutable Instruments DSP algorithms.
- `src/`: Electron app (main process: `main.ts` `preload.ts`; renderer: `renderer.ts`, `App.tsx`, `dsl/`, `components/`).

## Critical data flow
1. Frontend executes DSL via `new Function(...)` in `src/dsl/executor.ts` → produces `PatchGraph` JSON.
2. `PatchGraph` sent to main process via Electron IPC (`src/ipcTypes.ts` defines channels).
3. Main process calls `synthesizer.updatePatch(graph)` → Rust validates (`crates/modular/src/validation.rs`) → applies to audio thread.
4. Audio thread (cpal callback in `crates/modular/src/audio.rs`): uses `lock()` for real-time safety; processes graph + streams scope buffers back via `RingBuffer`.
5. Renderer polls scope data via IPC → draws oscilloscopes in Monaco decorations.

## Architecture evolution
- Originally WebSocket-based (server/client split).
- Now unified Electron app: audio runs in-process; types shared via NAPI (`@modular/core` package built from `crates/modular`).
- IPC replaces WebSocket; scope streaming uses `RingBuffer` → `getScopes()` polling instead of binary WS frames.


## Developer workflows
**Run Electron app (from repo root):**
```bash
yarn start  # Runs electron-forge; auto-rebuilds Rust on changes
```

**Rebuild Rust NAPI module only:**
```bash
cd crates/modular && yarn build  # or yarn build:debug
```

**Type generation (Rust → TypeScript via napi-rs):**
- Rust types with `#[napi]` macros automatically generate TypeScript definitions during build.
- **Manual step:** After changing Rust types, rebuild `crates/modular` (`yarn build`).
- Generated types appear in `crates/modular/index.d.ts` and are imported as `@modular/core`.

**Lint/typecheck:**
```bash
yarn lint
yarn typecheck  # tsc --noEmit for renderer
```

## Repo-specific conventions
- **Patch graphs are the contract:** prefer changing `modular_core::types` with `#[napi]` attributes over hand-editing TS types.
- **Adding/changing module params:**
  1. Update Rust param struct + DSP implementation in `modular_core/src/dsp/**/*.rs`.
  2. Wire schema/validator in category modules (e.g., `oscillators/mod.rs`) → `install_constructors` / `install_param_validators`.
  3. Rebuild NAPI (`cd crates/modular && yarn build`) to update TS types.
  4. Update DSL factories in `src/dsl/factories.ts` if needed.
- **Real-time safety (audio callback):** avoid allocations, logging. Do heavy work on main thread (e.g., patch validation).
- **Voltage convention:** signals/params clamped to `-10.0..10.0`; audio outputs attenuated by `AUDIO_OUTPUT_ATTENUATION` (0.2) in `crates/modular/src/audio.rs`.
- **Workspace structure:** Cargo workspace at root; Electron app also at root; `crates/modular` is a yarn workspace package (`@modular/core`).

## File operations & patches
- Electron main process handles file I/O (`src/main.ts`): workspace folder selection, reading/writing `.mjs` patches.
- Patches execute in `src/dsl/executor.ts` → DSL globals injected (`sine`, `saw`, `track`, `scope`, `hz`, `note`, etc.).
- Root-level `.mjs` files are example patches; users can open any folder as workspace and edit/run patches with Alt+Enter.