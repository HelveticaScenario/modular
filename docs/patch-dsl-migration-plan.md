# Patch DSL Migration Plan

## Objectives
- **Complete removal of YAML**: Replace the YAML patch format entirely with a fluent JavaScript DSL.
- **DSL-first workflow**: Users author patches in JavaScript. The client executes this JS to generate a `PatchGraph` JSON payload, which is sent to the server for audio processing.
- **JSON as transport only**: JSON is used strictly for client-to-server updates. It is not used for persistence.
- **DSL Persistence**: Patches are saved and loaded as `.js` files on the server's filesystem via a new file explorer API.
- Preserve validation, schema generation, and audio-thread safety guarantees.

## Current State Snapshot
- **Server protocol** (`modular_server/src/protocol.rs` & `http_server.rs`): `InputMessage::SetPatch { yaml: String }` receives YAML inside a JSON message. The YAML is converted to a `PatchGraph` with `serde_yaml`. Helper functions serialize/deserialize entire messages with YAML even though the WebSocket already transports JSON.
- **Persistence/utilities** (`persistence.rs`): currently reads/writes YAML files.
- **Frontend editor** (`modular_web/src/components/PatchEditor.tsx`): CodeMirror is configured for YAML. Alt/Ctrl+Enter triggers `onSubmit`, which passes the raw YAML string to `setPatch`.

## Workstream 1 — Protocol & Serialization (Backend)
1. **Schema & Message updates**
   - Replace `SetPatch { yaml: String }` with `SetPatch { patch: PatchGraph }` in `InputMessage`.
   - Remove `OutputMessage::Patch`.
   - Update `ts-rs` exports and regenerate TypeScript types.
2. **Remove YAML dependencies**
   - Remove `serde_yaml` from `Cargo.toml` and all Rust files.
   - Ensure `protocol.rs` uses `serde_json` exclusively.
3. **Patch ingestion**
   - Update `handle_message` for `SetPatch` to accept `PatchGraph` directly.
4. **Persistence & File API**
   - **Remove YAML persistence**: Delete existing YAML file loading/saving logic in `persistence.rs`.
   - **File Explorer API**: Implement endpoints (or WebSocket messages) to:
     - List files in the server's working directory (filter for `.js` DSL files).
     - Read a file's content (string).
     - Write content to a file (string).
     - Create/Delete files.
   - Security: Restrict file operations to the server's root directory (no path traversal).
5. **Testing**
   - Add backend tests that deserialize a JSON `SetPatch` payload and ensure the resulting `PatchGraph` matches expectations.


## Workstream 2 — JavaScript Patch DSL
1. **DSL surface design**
    - Core primitives: module factory functions (`sine(id)`, `saw(id)`, `mix(id)`, etc.), helper functions (`hz(value)`, `note("c3")`, `volts(value)`), and graph-level helpers (`out.source(node)`). Factories accept an explicit string `id` to ensure stable identities across live re-runs.
   - Fluent API requirements:
   ```ts
   const mod = sine('mod').freq(hz(1))
   const carrier = saw('carrier').freq(mod.scale(0.2).shift(note('c3')))
   out.source(carrier)
   ```
   - Each builder call should lazily register module instances with the provided `id` (or a deterministic fallback if omitted) and accumulate parameter/cable info matching the `PatchGraph` schema.
    - Module factories return an object with:
       - **Param setters**: a function per param (e.g., `.freq(x)`) that assigns the provided source (value/cable/output) to that param and returns the same module node for chaining.
       - **Outputs as properties**: each output port is an object that supports `.scale(factor)` and `.shift(offset)`. Calling either creates a single scale-and-shift module, wires the original output into its input, sets the requested param(s), and returns the scale-and-shift module (so `out.scale(0.2).shift(note('c3'))` yields one module, not two). The scale-and-shift module exposes its own output for further wiring. Single-output modules expose a default output (e.g., `.out` or direct property access) so they can be passed directly into param setters without naming the port.
    - **Factory generation**: prefer generating DSL factories from Rust module schemas to avoid manual drift. One option:
       - **Runtime generation in browser**: fetch schemas (`GetSchemas`) and build factories dynamically (methods per param, outputs with `.scale/.shift`). Pro: instant availability after adding a Rust module. Con: loses tree-shakeability and type safety unless paired with a TS codegen step.
    - **Autocomplete strategy**: Implement a runtime schema-driven completion source for CodeMirror.
       - Fetch `GetSchemas` JSON from the server at runtime.
       - Build in-memory completion tables for modules, params, outputs, helpers, and `out`.
       - Create a CodeMirror extension that detects DSL contexts (factory calls, `.param(`, `.scale/.shift`, `out.source`) and serves completions from the tables.
       - Use schema `description` fields for tooltips.
       - **No TS Language Server**: Do not rely on a TypeScript language server or `.d.ts` files.
       - Fallback: Use cached tables or a minimal keyword list if schema fetch fails.
   - Provide a predefined `out` object that maps to the root module's input; users call `out.source(node)` (or equivalent) to route the final signal to the root.
2. **Internal representation**
   - Implement a `GraphBuilder` class that tracks:
     - `modules: Record<ModuleId, ModuleState>`
     - Auto-incrementing numeric suffixes per module type (`sine-1`, `saw-2`, ...)
     - Connections between module outputs/params (mirrors `PatchGraph` structure).
   - Module factory returns an object that exposes methods for each param (derived from schemas or defined manually for MVP). Each method updates the builder state and returns the node for chaining.
3. **Signals & math helpers**
   - `hz(number)` converts to V/oct using existing `InternalParam` rules (needs helper replicating logic from Rust or pulling the same formula from generated types).
   - `note(noteName)` parses note strings (include enharmonic support, e.g., `c#4`).
   - `scale(multiplier)` and `shift(offset)` produce intermediate virtual modules (e.g., multiply/adder under the hood) or encode them as param transformations if the backend already supports such semantics. Clarify whether these map to actual modules (preferred for transparency) or to param metadata.
4. **Execution contract**
   - DSL scripts must return (or implicitly produce) the final builder so the runtime can call `.toPatch()` → `PatchGraph` JSON.
   - Provide a predefined global `out` node that wraps the root module's inputs for convenience.
   - Determine error handling (throw descriptive errors when required params missing, when a node is reused incorrectly, etc.).
5. **Packaging**
   - Place DSL runtime under `modular_web/src/dsl/` with clear entry point (e.g., `executePatchScript(source: string): PatchGraph`).
   - Consider shipping helper type definitions for editor IntelliSense (via `monaco-types` or `ts-morph`?), but keep MVP simple.
6. **Testing**
   - Add Jest/Vitest tests that run DSL snippets and snapshot resulting `PatchGraph` structures.
   - Ensure DSL rejects invalid constructs (e.g., calling `out.source()` twice without mix module).
7. **Live-coding stability**
   - Preserve module IDs across executions so the server diff only updates what changed. Proposed approach:
     - Deterministic IDs per factory call based on declaration order plus type (e.g., `sine-1`, `sine-2`) using a stable counter that resets per run but can reuse explicit names when provided.
     - Provide an explicit `.name("foo")` (or factory option) to pin IDs across edits; if unchanged, the server will reuse the module and only update params.
     - Avoid generating new IDs when only param expressions change; ensure the builder reuses existing node instances within one run instead of cloning.
     - On the server, the existing diff already keeps modules with the same ID/type and only updates params, so stable IDs are the key to avoiding unnecessary teardown.

## Workstream 3 — Editor & WebSocket Integration
1. **Editor & File Explorer**
   - **File Explorer Pane**: Add a UI component to list `.js` files from the server.
   - **DSL Editor**: Update `PatchEditor` to use JavaScript mode.
   - **Load/Save**: Clicking a file in the explorer loads its content into the editor. "Save" (Ctrl+S) writes the current editor content back to the server file.
   - **Execution**: Alt+Enter executes the current code, generates the JSON graph, and sends it to the server via `SetPatch`.
2. **Execution on Alt+Enter**
   - Replace the `onSubmit` handler so it:
     1. Invokes the DSL executor with the current script text.
     2. If execution succeeds, calls `setPatch(patchGraph)` where `setPatch` now expects structured data.
     3. Surfaces runtime or validation errors in `ErrorDisplay` with stack traces / line numbers when available.
3. **State synchronization**
   - No server-to-client patch echo; the DSL script is the client-side source of truth. Keep the last executed script in UI state/local storage. If future features need server-side inspection, add explicit debug/introspection endpoints rather than automatic patch returns.
4. **WebSocket hook updates**
   - Change `useModularWebSocket.setPatch` to accept `PatchGraph` and send `{ type: 'setPatch', patch }`.
   - Update TypeScript types after regenerating from backend.
   - Ensure binary audio handling remains unchanged.
5. **UX niceties**
   - Show execution status (Running…, Success, Failed) near the editor.
   - Disable execution while another request is in flight to avoid race conditions, or queue them.

## Workstream 4 — Validation, Tooling, and Rollout
1. **Type generation pipeline**
   - After backend changes, run `pnpm run codegen` to refresh `modular_web/src/types/generated/` and commit the new types.
2. **End-to-end manual test plan**
   - Start backend (`cargo run`), frontend (`pnpm dev`), write sample DSL (as provided), confirm:
     - Server receives JSON patch and logs modules correctly.
     - Audio plays as expected.
     - Validation errors from server surface in UI with file/line mapping where possible.
3. **Documentation updates**
   - Update `AUDIO_STREAMING_IMPLEMENTATION.md` (or create a new doc) explaining the DSL, execution shortcut, and JSON payload.
   - Remove migration notes about YAML.
4. **Feature flag / rollout strategy**
   - Consider shipping DSL editor behind a toggle to allow quick rollback.
5. **Future enhancements**
   - Schema-driven DSL (auto-generate param methods from module schema map).
   - Autocomplete + inline docs inside CodeMirror leveraging generated TypeScript types.

## Open Questions
- Security is out of scope for now; assume local-only execution without sandboxing concerns.

## Suggested Implementation Order
1. Backend protocol/type updates (JSON `SetPatch`, remove `OutputMessage::Patch`, remove `serde_yaml`).
2. Implement File Explorer API (backend).
3. Regenerate frontend types and adapt `useModularWebSocket` + supporting hooks.
4. Implement DSL runtime + evaluator.
5. Swap editor to DSL mode, add File Explorer UI, and wire Alt+Enter execution path.
6. Add tests/documentation and remove YAML-specific UI affordances.
