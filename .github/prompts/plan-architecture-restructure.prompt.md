## Plan: Modular Synthesizer Architecture Refactor

**Goal**: Restructure the modular synthesizer system into a clean, layered architecture with a declarative YAML-based API, real-time audio streaming, and an integrated web-based editor with live waveform visualization.

**Key Changes**:
- Separate `modular_core` into a pure DSP library (no I/O, no protocol, no serialization)
- Move all protocol, persistence, audio playback, and WebSocket logic to `modular_server`
- Replace REST endpoints with WebSocket-only API using YAML messages
- Add comprehensive validation with detailed error reporting
- Build React 19 + TypeScript web UI with CodeMirror editor, autocomplete, and inline oscilloscopes
- Support audio streaming, recording to WAV files, and live waveform visualization
- Generate TypeScript types from Rust for type-safe client/server communication
- Design architecture to support future migration from YAML to custom DSL

### Steps

1. **Simplify `modular_core` to be a pure library**
   - Remove all message types from `modular_core/src/message.rs` (delete entire file or module)
   - Remove `handle_message` function - this belongs in server layer
   - Keep only core types in `modular_core/src/types.rs`:
     - `PatchGraph`, `ModuleState`, `Param`, `SampleableMap`
     - `Sampleable`, `Module` traits
     - Track types if needed for automation
     - Add unit tests for type conversions and parameter handling
   - Simplify `modular_core/src/patch.rs`:
     - `Patch` struct with only DSP graph processing (sample generation)
     - Remove audio playback/cpal integration - this moves to server
     - Remove audio subscription management - this moves to server
     - Keep only: `process_frame()`, `apply_patch()`, `get_state()`, `get_sample()`
     - Add unit tests for:
       - Patch creation and module instantiation
       - Graph connectivity (cable connections)
       - Audio processing (sine wave generation, filter response, etc.)
       - State queries and module lookups
   - Remove all serde dependencies from modular_core (no serialization concerns)
   - Export clean Rust API: `Patch::new()`, `Patch::apply()`, `Patch::get_state()`, `Patch::process_frame()`, etc.
   - Add integration tests in `modular_core/tests/` for common patch scenarios

2. **Move all messaging and protocol logic to `modular_server`**
   - Create `modular_server/src/protocol.rs`:
     - Define `InputMessage` and `OutputMessage` enums here with YAML serialization
     - Add `serde_yaml` dependency to `modular_server/Cargo.toml`
     - Add `typescript-type-def` or `ts-rs` crate dependency for TypeScript generation
     - Annotate message types with `#[derive(Serialize, Deserialize, TS)]` (if using ts-rs)
     - Message types: `GetPatch`, `SetPatch`, `SubscribeAudio`, `UnsubscribeAudio`, `Mute`, `Unmute`, etc.
     - Response types: `PatchState`, `AudioSubscribed`, `AudioBuffer`, `Error`, `Muted`, `Unmuted`, etc.
     - `Error` response should include detailed validation errors:
       - `ValidationError` variant with `field: String`, `message: String`, `location: Option<String>`
       - Support for multiple errors in a single response
       - Examples: "Unknown module type 'foo'", "Module 'sine-1' not found for cable source", "Parameter 'cutoff' not found on module type 'sine-oscillator'"
     - Implement message handling logic that calls into `modular_core` Rust API
     - Add unit tests for message serialization/deserialization
     - Configure TypeScript export path to `../modular_web/src/types/messages.ts`
   - Create `modular_server/src/persistence.rs`:
     - Implement `save_patch(path: &Path, patch: &PatchGraph) -> Result<()>`
     - Implement `load_patch(path: &Path) -> Result<PatchGraph>`
     - Use YAML format for file storage
     - Add unit tests for save/load roundtrip, error handling, file corruption
   - Simplify `modular_server/src/http_server.rs`:
     - Remove REST endpoint handlers: `set_patch`, `create_module`, `delete_module`, `update_param`, `get_schemas`
     - Remove route definitions for `/patch`, `/modules`, `/modules/:id`, `/params/:id/:param_name`, `/schemas`
     - Keep only: `/ws` (WebSocket), `/health` (GET)
     - Remove unused request types: `SetPatchRequest`, `CreateModuleRequest`, `UpdateParamRequest`
     - WebSocket handler uses protocol types and deserializes/serializes YAML
   - Message handler calls modular_core methods and persists to file:
     - `SetPatch` → validate patch + `patch.apply(patch_graph)` + `save_patch(path, patch_graph)` + set mute flag to false (auto-unmute)
       - Validation checks:
         - All module types exist in schema
         - All cable source/target modules exist in patch
         - All cable source ports (outputs) exist on their respective modules
         - All cable target ports (params) exist on their respective modules
         - All parameter values are valid for their types (ranges, enums, etc.)
       - Return detailed `ValidationError` response if invalid, listing all errors found
       - Note: Cycles in the graph are allowed (not an error condition)
       - Note: Any output can be routed to any param (no type compatibility checking needed)
     - `GetPatch` → `patch.get_state()`
     - `SubscribeAudio` → register subscription in server-side tracking
     - `Mute` → set mute flag to true
     - `Unmute` → set mute flag to false
     - `StartRecording` → begin recording audio output to WAV file
     - `StopRecording` → stop recording and finalize WAV file

3. **Move audio playback to `modular_server` and update server initialization**
   - Create `modular_server/src/audio.rs`:
     - Move cpal audio stream setup from modular_core
     - Create audio callback that calls `patch.process_frame()`
     - Manage audio subscription tracking (map of subscription_id → module_id/port)
     - Capture samples from subscribed modules and accumulate into buffers
     - Send `AudioBuffer` messages via broadcast channel when buffers are full
     - Add mute flag that silences audio output when enabled
     - Add WAV recording functionality:
       - Use `hound` crate for WAV file writing
       - Maintain recording state (Arc<Mutex<Option<WavWriter>>>)
       - In audio callback, write samples to WAV file when recording is active
       - Support configurable sample rate and bit depth (default 48kHz, 32-bit float)
       - Handle file creation with timestamp-based naming or user-provided path
     - Add unit tests for audio buffer accumulation, subscription management, and recording state
   - Update `modular_server/src/lib.rs`:
     - Create server state that holds:
       - `Arc<Mutex<Patch>>` - the DSP graph instance from modular_core
       - `broadcast::Sender<OutputMessage>` - for pushing updates to all clients
       - Audio subscription map shared with audio thread
       - Mute state (Arc<AtomicBool>) shared with audio thread
       - Patch file path for persistence
       - Schema data for validation
     - Initialize modular_core `Patch` instance
     - Load patch from file on startup (create default if missing)
     - Start audio playback thread with cpal
     - Ensure `create_router` only registers WebSocket + health routes
     - Verify broadcast channel flows work for simplified message set
     - Add unit tests for server state initialization and patch loading
   - Create `modular_server/src/validation.rs`:
     - Implement `validate_patch(patch: &PatchGraph, schema: &Schema) -> Result<(), Vec<ValidationError>>`
     - Return all validation errors found (not just first error)
     - Add unit tests for each validation case

4. **Update documentation**
   - Remove examples of CRUD operations from docs

5. **Add static file serving to `modular_server`**
   - Add `tower-http` static file serving middleware to serve from `modular_server/static/` directory
   - Add route `/*file` to serve static assets
   - Ensure static files are served with appropriate MIME types

6. **Create integrated web UI with Vite + React 19 + TypeScript**
   - Initialize new Vite project at `modular_web/`:
     - `npm create vite@latest modular_web -- --template react-ts`
     - Add React 19 and React Compiler to dependencies
     - Configure `vite.config.ts` to build to `modular_server/static/`
     - Enable React Compiler in Vite config
   - Create components:
     - `App.tsx`: Main layout with split-pane (editor left, oscilloscope right)
     - `PatchEditor.tsx`: CodeMirror 6 editor component with YAML syntax highlighting and autocomplete
     - `Oscilloscope.tsx`: Canvas-based waveform visualization with useRef + useEffect
     - `WebSocketProvider.tsx`: Context provider for WebSocket connection
     - `AudioControls.tsx`: Subscription controls (module ID, port, buffer size)
     - `RecordingControls.tsx`: Record button, recording indicator, duration counter, download link when complete
   - Implement WebSocket client:
     - Connect to `/ws` on mount
     - Send `GetPatch` message (as YAML) on connection to populate editor
     - Send `GetSchemas` message to fetch module schemas for autocomplete
     - Listen for `PatchState` responses (YAML text frames) and update editor state directly
     - Listen for `Schemas` responses and build autocomplete data structures
     - Send editor YAML as `SetPatch` only when Ctrl+Enter is pressed (not on every keystroke)
     - Send `Mute` message when Ctrl+. is pressed to silence audio
     - Handle binary audio frames and render to oscilloscope canvas
     - Send `StartRecording` message with optional filename parameter
     - Send `StopRecording` message to finalize recording
     - Listen for `RecordingStarted` and `RecordingStopped` responses with file info
     - Add `js-yaml` dependency only for pretty-printing/validation if needed
   - Add keyboard shortcuts:
     - Bind Ctrl+Enter (Cmd+Enter on Mac) to apply patch (automatically unmutes audio)
     - Bind Ctrl+. (Cmd+. on Mac) to mute audio
     - Bind Ctrl+R (Cmd+R on Mac) to toggle recording
     - Show visual feedback when patch is applied, audio is muted, or recording starts/stops
   - TypeScript types for messages:
     - Auto-generated `types/messages.ts` from Rust types (via ts-rs or similar)
     - Add build step to regenerate types: `npm run codegen` calls `cargo test --package modular_server export_types` or similar
     - Ensure types are generated before `npm run dev` or `npm run build`
     - Type-safe message handling and serialization
   - Add error display component:
     - `ErrorDisplay.tsx`: Shows validation errors inline in editor
     - Parse `ValidationError` responses and highlight problematic lines
     - Show error messages as CodeMirror diagnostics/decorations
   - Configure build:
     - Output to `../modular_server/static/dist/`
     - Add build script to `package.json`
     - Ensure proper asset paths for production

7. **Add `GetPatch` and `GetSchemas` message types**
   - Add `GetPatch` to `InputMessage` enum (returns current patch state)
   - Add `GetSchemas` to `InputMessage` enum (returns module type schemas)
   - This allows the web UI to fetch current patch on load
   - Handler should return `PatchState` with all current modules
   - Handler should return `Schemas` with module type definitions for autocomplete

8. **Add waveform visualization metadata to patch format**
   - Update `PatchGraph` structure in `modular_core/src/types.rs`:
     - Change `modules: Vec<ModuleState>` to `modules: HashMap<String, ModuleState>`
     - Remove `id` field from `ModuleState` (now the map key)
     - Rename `module_type` to `type` in `ModuleState`
     - Add optional `visualize: Option<Vec<String>>` field to `ModuleState` (list of port names)
   - YAML format example:
     ```yaml
     modules:
       sine-1:
         type: sine-oscillator
         params:
           freq: 4.0
         visualize:
           - output
       filter-1:
         type: lowpass
         params:
           cutoff: 2.0
           resonance: 0.7
     ```
   - Update `PatchEditor.tsx`:
     - Parse `visualize` list from each module definition
     - When patch is applied, automatically subscribe to audio for ports listed in `visualize`
     - Create inline waveform widgets using CodeMirror decorations at module definition lines
     - Store subscription IDs mapped to module IDs for cleanup
   - Update `Oscilloscope.tsx` to support mini-mode:
     - Add `inline: boolean` prop for compact inline rendering
     - Reduce height and remove controls when `inline === true`
     - Reuse same rendering logic, just different layout

### Architecture Considerations for Future DSL

To prepare for eventual migration from YAML to a custom DSL with inline visualizations:

1. **Decouple parsing from protocol**
   - Create `patch::parse()` function that takes a string and returns `PatchGraph`
   - Currently implemented with `serde_yaml::from_str()` but can be swapped later
   - Keep serialization format (YAML/DSL) as an implementation detail
   - WebSocket layer should be agnostic to the text format

2. **Structured patch representation**
   - Keep `PatchGraph` and `ModuleState` as the canonical internal format
   - Custom DSL parser will target these same types
   - Ensures smooth migration path: DSL → PatchGraph (same as YAML → PatchGraph)

3. **Editor extensibility**
   - Use CodeMirror 6's extension system for future language support
   - Current YAML mode can be replaced with custom DSL mode
   - Design editor component to accept language/parser as prop
   - Keep editor state as plain text (string), not pre-parsed AST
   - Implement context-aware autocomplete extension:
     - Parse current YAML context to determine completion type
     - Module type completion: After `type:` field, suggest from schema module types
     - Module ID completion: In `cables` source/target, suggest from current patch module IDs
     - Port completion: After module ID in cables (e.g., `sine-1:`), suggest input/output ports from schema
     - Param name completion: Under `params:` for a module, suggest param names from schema for that module type
     - Visualization completion: In `visualize:` list, suggest output port names from schema for that module type
     - Use CodeMirror's `autocompletion()` extension with custom completion source
     - Build completion index from schema data structure (map of module_type → {params, inputs, outputs})
     - Parse editor text to extract module IDs for cable/visualization autocomplete

4. **Inline visualization architecture**
   - Design editor with "gutter" system for inline content
   - Use CodeMirror decorations/widgets for inline waveforms
   - Create `InlineWaveform` component that can be inserted at line positions
   - Subscribe to audio from modules mentioned at specific editor lines
   - Use miniature canvas elements for inline visualization

5. **Track state visualization**
   - Add track position/playhead to `PatchState` responses
   - Stream track updates via WebSocket (new message type or enhanced `PatchState`)
   - Render timeline/keyframe visualization inline or in dedicated panel
   - Consider separate visualization pane that can show expanded track views

6. **DSL design principles to consider**
   - Human-readable syntax (like YAML but more concise)
   - Support for comments and annotations (for docs/visualizations)
   - Syntax for inline visualization hints (e.g., `@visualize`, `@track`)
   - Preserve line/column information for error reporting and decorations

### Further Considerations

1. **TypeScript codegen workflow?** Consider adding a watch mode for development that auto-regenerates types when Rust code changes, or integrate into cargo watch
2. **Autocomplete performance?** For large schemas, consider lazy loading or caching completion data to avoid recomputing on every keystroke
3. **Validation granularity?** Consider client-side validation using the schema before sending SetPatch to provide immediate feedback (server validation is still required for security)
