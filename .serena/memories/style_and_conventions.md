# Style and Conventions
- Separation: `modular_core` is pure DSP (no HTTP/WebSocket/serialization/I/O). Server concerns stay in `modular_server`.
- Real-time safety: audio callback must never block; use `try_lock()` and skip work on contention; avoid allocations/panics on audio thread.
- Param handling: clamp V/Oct params to [-10V,10V]; base freq 27.5Hz at 0V; smoothing via `SMOOTHING_COEFF` (~0.99) to avoid clicks.
- Module pattern: derive `Params` and `Module`; implement `update()` DSP prep; register constructors/schemas in category modules; export TS types via ts-rs.
- Patch updates: validate graphs before applying; module type changes require delete+recreate; root module never deleted; apply param updates across add/recreate/existing.
- Recording/audio: audio thread writes to subscription/recording buffers with `try_lock()`; outputs attenuated to ±5V.
- Coding style: use `anyhow::Result` for recoverable errors; prefer collected validation errors; keep comments minimal/targeted; ASCII default.
