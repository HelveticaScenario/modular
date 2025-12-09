# Modular Project Overview
- Purpose: real-time modular synthesizer with JavaScript DSL for live-coding audio patches; Rust DSP/backend with React/TypeScript frontend.
- Architecture: `modular_core` (pure DSP, no I/O), `modular_server` (Axum HTTP/WebSocket server, patch validation/streaming), `modular_web` (React/TS frontend with DSL runtime/editor/oscilloscope). Frontend talks to server via WebSocket using JSON control + binary audio.
- Key docs: root `README.md` (quick start, examples), `docs/DSL_GUIDE.md`, `docs/patch-dsl-migration-plan.md`, attached copilot instructions (architecture + conventions) critical.
- Data flow: frontend DSL -> PatchGraph JSON over WS -> server validates/diffs -> core processes audio; audio thread uses try_lock and skips on contention.
- Modules: derive `Params`/`Module` macros; constructors registered via `get_constructors()` per category; TS types exported via ts-rs to `modular_web/src/types/generated/`.
