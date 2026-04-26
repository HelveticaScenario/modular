# Dynamic Buffer Processing Design

## Overview

Replace the current per-sample pull model with a block-based processing model. Modules process audio in blocks rather than one sample at a time, reducing call overhead and enabling future SIMD optimisation. Modules in feedback loops fall back to sample-accurate per-sample processing to preserve correct delay semantics. All block infrastructure lives in the proc-macro generated wrapper — inner module DSP code requires no changes.

## Goals

- Process audio in blocks for reduced per-sample call overhead
- Preserve exactly 1-sample feedback delay regardless of block size
- Keep inner module `update()` code unchanged — zero DSP module changes
- Maintain sample-accurate clock sync (Ableton Link, ROOT_CLOCK)
- Keep module state fresh on reconnect by always processing all modules

## Non-Goals

- Per-module SIMD vectorisation (enabled by layout, not implemented here)
- Changing DSP semantics visible to patch authors
- Changing the send-only ownership model (builds on top of `feature-remove-sync-from-sampleable`)

## Background

This design builds on `feature-remove-sync-from-sampleable`. Module ownership is `Box<dyn Sampleable>`, intra-graph connections use `Option<NonNull<dyn Sampleable>>` rather than `Arc`/`Weak`. That ownership model is a prerequisite.

Currently `process_frame()` is called once per audio sample inside the CPAL callback. Each call triggers a pull through the graph via the `processed: AtomicBool` CAS guard. Feedback cycles are handled implicitly — a cyclic read sees `processed = true` and reuses the previous sample's cached output.

## Architecture

### Block Size

Block size equals the CPAL output callback buffer size, determined when the audio device is opened. It is invariant for the lifetime of the runtime. If the device is re-opened (sample rate change, device switch), the runtime is reconstructed with the new block size. All module output buffers are allocated to this size at construction and never resized.

### BlockPort — Output Storage

Each output port's `PolyOutput` (`[f32; 16]`, one sample all channels) is replaced by `BlockPort`:

```rust
pub struct BlockPort {
    data: Box<[[f32; MAX_CHANNELS]]>,  // length = block_size; sample-major interleaved
    write_index: usize,                // current write cursor, managed by wrapper
}
```

`data` is a single contiguous heap allocation of `block_size × MAX_CHANNELS × 4` bytes. Layout is sample-major: `data[i]` is `[f32; MAX_CHANNELS]` — all channels at sample index `i` are contiguous in memory, enabling future per-channel SIMD loads across voices.

`BlockPort` exposes the same `set(ch, value)` API as the current `PolyOutput`. The write cursor is advanced by the wrapper, not the module:

```rust
impl BlockPort {
    // Same signature as PolyOutput::set — inner module code unchanged
    pub fn set(&mut self, ch: usize, value: f32) {
        self.data[self.write_index][ch] = value;
    }

    // Called by wrapper after each update() — module never sees this
    pub fn advance(&mut self) {
        self.write_index += 1;
    }

    // Called by wrapper at frame start — data retained for feedback reads
    pub fn reset_write_cursor(&mut self) {
        self.write_index = 0;
    }

    pub fn get(&self, ch: usize, index: usize) -> f32 {
        self.data[index][ch]
    }
}
```

The proc-macro generates a `BlockOutputs` struct replacing the current `Outputs` struct, with one `BlockPort` per output port. All `#[output(...)]` annotations are unchanged.

### ProcessingMode

```rust
pub enum ProcessingMode {
    Block,   // default — computes full block on first request
    Sample,  // feedback loops — computes one sample per request
}
```

Assigned per module before construction via cycle detection. Stored as an immutable field on the wrapper.

### Wrapper Fields

The proc-macro generated wrapper gains:

```rust
struct ModuleWrapper<M, O> {
    // existing
    module:      UnsafeCell<M>,
    outputs:     UnsafeCell<O>,   // O is now BlockOutputs
    sample_rate: f32,
    // new
    index:       Cell<usize>,     // current block position (0..=block_size)
    computing:   Cell<bool>,      // reentrancy guard
    mode:        ProcessingMode,  // set at construction, never changes
    block_size:  usize,           // invariant once created
}
```

`new()` gains `mode: ProcessingMode` and `block_size: usize` parameters.

### Block Index on Inner Module

The proc-macro injects a `block_index: Cell<usize>` field into the inner module struct (parallel to the existing `_channel_count: usize`), and generates a `current_block_index(&self) -> usize` method. The wrapper sets this before each `update()` call, giving modules like `ROOT_CLOCK` access to their current position in the block without a separate counter.

### Internal `update()` Step

`update()` is removed from the public `Sampleable` trait — it becomes an internal wrapper method only:

```rust
fn update(&self) {
    let i = self.index.get();
    let module  = unsafe { &mut *self.module.get() };
    let outputs = unsafe { &mut *self.outputs.get() };
    module.block_index.set(i);
    module.update(self.sample_rate);   // inner module unchanged
    outputs.advance_all();             // all BlockPorts increment write_index
    self.index.set(i + 1);
}
```

### `Sampleable` Trait Changes

```rust
pub trait Sampleable: MessageHandler + Send {
    // tick() resets index to 0. Does NOT clear block output buffers — previous values
    // retained for feedback reads. $buffer overrides tick() to also advance its
    // BufferData write_index by block_size.
    fn tick(&self);

    // Replaces get_poly_sample(). Signals call this; block/sample dispatch is internal.
    fn get_value_at(&self, port: &str, ch: usize, index: usize) -> f32;

    // Completes any remaining block computation. Called on all modules after sink pull.
    fn ensure_processed(&self);

    // Unchanged:
    fn get_id(&self) -> &str;
    fn get_module_type(&self) -> &str;
    fn connect(&self, patch: &Patch);
    fn on_patch_update(&self) {}
    fn get_state(&self) -> Option<serde_json::Value> { None }
    fn get_buffer_output(&self, _port: &str) -> Option<*const BufferData> { None }
    fn prepare_resources(&self, _wav_data: &HashMap<String, Arc<WavData>>) {}
    fn as_any(&self) -> &dyn std::any::Any;
    fn transfer_state_from(&self, _old: &dyn Sampleable) {}

    // Signature changes: accepts full block of clock states rather than per-sample values.
    // ClockState carries { bar_phase: f64, bpm: f64, playing: bool } for one sample position.
    fn sync_external_clock(&self, _states: &[ClockState]) {}
    fn clear_external_sync(&self) {}

    // Removed:
    // fn update(&self)  — now internal to wrapper
    // fn get_poly_sample(&self, port: &str) -> Result<PolyOutput>  — replaced by get_value_at
}
```

### `ensure_processed` and `get_value_at`

`ensure_processed` contains the block completion logic. The block mode branch of `get_value_at` calls it:

```rust
fn ensure_processed(&self) {
    if self.index.get() < self.block_size {
        self.computing.set(true);
        for _ in self.index.get()..self.block_size { self.update(); }
        self.computing.set(false);
    }
}

fn get_value_at(&self, port: &str, ch: usize, index: usize) -> f32 {
    // Reentrancy: module is mid-computation and a cycle has read back to it.
    // Return the previous sample — exactly 1-sample delay regardless of block size.
    if self.computing.get() && index >= self.index.get() {
        let i = self.index.get();
        let prev = if i == 0 { self.block_size - 1 } else { i - 1 };
        return unsafe { &*self.outputs.get() }.get(port, ch, prev);
    }

    match self.mode {
        ProcessingMode::Block => {
            self.ensure_processed();
        }
        ProcessingMode::Sample => {
            self.computing.set(true);
            while self.index.get() <= index { self.update(); }
            self.computing.set(false);
        }
    }

    unsafe { &*self.outputs.get() }.get(port, ch, index)
}
```

### Signal Index Back-Pointer

Signals resolve the current block index implicitly so inner module code (`self.params.freq.get_value(ch)`) requires no changes.

`Signal::Cable` gains one field:

```rust
Cable {
    module:    String,                          // identity / serialisation
    resolved:  Option<NonNull<dyn Sampleable>>, // from send-only spec
    port:      String,
    channel:   usize,
    index_ptr: *const Cell<usize>,              // injected during connect()
}
```

The wrapper's `connect()` injects a pointer to its own `index` into all signals via a new `inject_index_ptr` pass on the `Connect` derive macro:

```rust
fn connect(&self, patch: &Patch) {
    let module = unsafe { &mut *self.module.get() };
    module.params.connect(patch);
    module.params.inject_index_ptr(&self.index as *const _);
}
```

`Signal::get_value(ch)` reads the index implicitly:

```rust
Signal::Cable { resolved, port, channel, index_ptr, .. } => {
    let index = unsafe { (*index_ptr).get() };
    let upstream = unsafe { resolved.unwrap().as_ref() };
    upstream.get_value_at(port, *channel, index)
}
```

### Cycle Detection

Runs on the main thread after validation, before module construction, as a pure function over the `PatchGraph` JSON structure.

**Graph model:** edges point downstream → upstream (B has a cable to A means B depends on A, edge B→A). A feedback loop is a cycle in this graph.

**Algorithm:** Tarjan's SCC, O(V+E). Modules in SCCs with more than one node are in a feedback loop. Single-node SCCs with a self-edge (module cabled to itself) are also feedback loops. All such modules get `ProcessingMode::Sample`; all others get `ProcessingMode::Block`.

```rust
// crates/modular/src/graph_analysis.rs
pub fn classify_processing_modes(graph: &PatchGraph) -> HashMap<String, ProcessingMode> {
    let adjacency = build_dependency_edges(graph);
    let sccs = tarjans_scc(&adjacency);

    let mut modes = HashMap::new();
    for scc in &sccs {
        let mode = if scc.len() > 1 || has_self_loop(&adjacency, &scc[0]) {
            ProcessingMode::Sample
        } else {
            ProcessingMode::Block
        };
        for id in scc {
            modes.insert(id.clone(), mode);
        }
    }
    modes
}
```

Result flows into module construction as `HashMap<String, ProcessingMode>`, then into each wrapper's `mode` field.

**Pipeline:**

```
PatchGraph JSON
  → validation.rs          (existing)
  → graph_analysis.rs      (new) → HashMap<id, ProcessingMode>
  → module construction    (mode + block_size passed to wrapper new())
  → command queue → audio thread
```

### Audio Callback

Block size is determined at audio device open time from the CPAL stream config and held on `AudioProcessor`. The callback shape becomes:

```rust
// 1. Reset all modules for the new block
for module in patch.sampleables.values() {
    module.tick();
}

// 2. Pre-compute clock values for all block_size sample positions
let clock_states = compute_clock_states(block_size, sample_rate, &link_timeline);
root_clock.sync_external_clock(&clock_states);

// 3. Pull from sink modules — computes the connected graph
for ch in 0..num_output_channels {
    for i in 0..block_size {
        output_buffer[i * num_output_channels + ch] =
            audio_out.get_value_at("out", ch, i);
    }
}

// 4. Ensure all modules processed regardless of connectivity — keeps state fresh
for module in patch.sampleables.values() {
    module.ensure_processed();
}
```

`ROOT_CLOCK` reads the pre-computed clock states via `current_block_index()` inside its `update()` — no sample mode required. Link accuracy is fully preserved: the Link timeline is queried once per sample position per block, giving sample-accurate beat phase throughout.

### `$buffer` and Delay Modules

`BufferData`'s `write_index` represents the write position at the **start of the current frame**. Within a block:

- `$buffer` writes to `write_index + current_block_index()`
- `$delayRead` reads from `write_index + current_block_index() - delay_samples`

After the block completes, `write_index += block_size` (in `tick()` at the next frame start).

Both modules access `current_block_index()` directly. No extra coordination or snapshot logic required. For feedback delay loops, cycle detection puts both modules in sample mode, which preserves the existing per-sample write/read interleaving exactly as today.

### `transfer_state_from`

The existing proc-macro generated output struct swap is sufficient. Swapping the `BlockOutputs` struct between old and new module hands the full block buffer (including `data[block_size - 1]`) to the new module. Feedback reads at `index == 0` of the next frame correctly see the last computed sample of the previous frame.

### Scope

Scope collection advances its accumulator by `block_size` samples per callback instead of 1. Trigger detection (threshold crossing for display stabilisation) scans `data[0..block_size]` for the trigger condition after the block is filled, finding the exact sample index — still sample-accurate. The ms-per-frame rate limiter on scope data transfer to the renderer is unchanged.

## Raw Pointer Safety Contract

All invariants from `feature-remove-sync-from-sampleable` apply. Additional invariants for block processing:

1. `index_ptr` in `Signal::Cable` is valid for the lifetime of the owning wrapper. It is set during `connect()` and cleared or refreshed on every reconnect.
2. `BlockPort::data` pointers are stable for the lifetime of the module — the box is never reallocated.
3. `write_index` in `BufferData` is accessed only on the audio thread during processing phase.

## Testing Strategy

- Block mode module produces identical output to per-sample processing for non-feedback patches
- Sample mode module (in detected cycle) preserves exactly 1-sample feedback delay
- Self-loop module detected as sample mode
- Reentrancy guard returns `data[block_size - 1]` when `index == 0`
- `ensure_processed()` completes a disconnected module's block
- Signal `get_value(ch)` reads correct index via back-pointer for both block and sample mode upstreams
- `transfer_state_from` preserves `data[block_size - 1]` across patch updates for feedback paths
- `$buffer` write position and `$delayRead` read position are correct across block boundaries
- ROOT_CLOCK receives sample-accurate clock states across the full block
- Scope trigger detection finds correct sample index within block
