# Dynamic Buffer Processing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace per-sample pull (`update()` → `get_poly_sample()`) with block-buffered processing that computes up to `block_size` samples per CPAL callback in one shot, cutting CPU overhead by ~30–50% via cache-friendly block writes.

**Architecture:** Proc-macro wrapper pattern. Inner module DSP code (`update()`, field assignments like `self.outputs.bar_trigger = 0.0`) is **completely unchanged**. The generated `{Name}Sampleable` wrapper gains `block_outputs: UnsafeCell<{Name}BlockOutputs>`, `index: Cell<usize>`, `computing: Cell<bool>`, `mode: ProcessingMode`, and `block_size: usize`. Modules in feedback cycles (detected by Tarjan's SCC) run in `Sample` mode (one sample per call); all others run in `Block` mode (all `block_size` samples per call). Signal inputs gain an `index_ptr` back-pointer so upstream modules know which sample position to serve. The CPAL callback is restructured to call `tick()` → `ensure_processed()` → `get_value_at()` across the full block.

**Tech Stack:** Rust proc-macros (`syn`, `quote`), `modular_core` (DSP library), `modular` (N-API + CPAL), `serde_json` for graph analysis.

---

## File Map

### New Files
| File | Responsibility |
|------|----------------|
| `crates/modular_core/src/block_port.rs` | `BlockPort` type: heap-allocated sample-major buffer `data[sample_idx][ch]` |
| `crates/modular/src/graph_analysis.rs` | Tarjan's SCC on `PatchGraph`; returns `HashMap<String, ProcessingMode>` |

### Modified Files
| File | Changes |
|------|---------|
| `crates/modular_core/src/lib.rs` | Export `block_port`, `BlockPort`, `ProcessingMode`, `ExternalClockState`, `InjectIndexPtr` |
| `crates/modular_core/src/types.rs` | `Sampleable` trait: add `ensure_processed`, `get_value_at`, new `sync_external_clock`, `inject_audio_in_block`; `OutputStruct`: add `tick_buffers`; new types `ProcessingMode`, `ExternalClockState`; `Signal::Cable` gains `index_ptr`; `SampleableConstructor` gains `block_size`, `mode` params |
| `crates/modular_core/src/poly.rs` | `InjectIndexPtr` impls for `Signal`, `Option<Signal>`, `PolySignal`, `MonoSignal`, `Option<PolySignal>`, `Option<MonoSignal>` |
| `crates/modular_core/src/dsp/core/clock.rs` | Rename `sync_external_clock` → `sync_external_clock_impl`; accept `ExternalClockState` |
| `crates/modular_core/src/dsp/utilities/buffer.rs` | `BufferWrite::update()` uses `current_block_index()`; `BufferWriteOutputs::tick_buffers(n)` advances `write_index`; `ensure_source_updated` calls `ensure_processed` |
| `crates/modular_core/src/dsp/utilities/delay.rs` | `DelayRead::update()` uses `current_block_index()` for read offset |
| `crates/modular_core/tests/types_tests.rs` | Update `DummySampleable` for new `Sampleable` trait surface |
| `crates/modular_derive/src/outputs.rs` | Generate `{Name}BlockOutputs` struct alongside existing `OutputStruct` impl |
| `crates/modular_derive/src/connect.rs` | Generate `InjectIndexPtr` impl for each params struct |
| `crates/modular_derive/src/module_attr.rs` | Inject `_block_index: Cell<usize>` into inner struct; add wrapper fields; implement new `Sampleable` methods; clock_sync/audio_input flags updated |
| `crates/modular/src/lib.rs` | Add `graph_analysis` module |
| `crates/modular/src/audio.rs` | `SampleableConstructor` call sites updated; new CPAL callback structure; `process_frame` replaced by block dispatch |

---

## Task 1: BlockPort Type

**Files:**
- Create: `crates/modular_core/src/block_port.rs`
- Modify: `crates/modular_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

  Add to `crates/modular_core/src/block_port.rs` (create the file with just the test module first):

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::poly::PORT_MAX_CHANNELS;

      #[test]
      fn block_port_new_zeroed() {
          let bp = BlockPort::new(4);
          assert_eq!(bp.data.len(), 4);
          for slot in bp.data.iter() {
              assert_eq!(*slot, [0.0f32; PORT_MAX_CHANNELS]);
          }
      }

      #[test]
      fn block_port_get_in_range() {
          let mut bp = BlockPort::new(4);
          bp.data[2][3] = 1.5;
          assert_eq!(bp.get(2, 3), 1.5);
      }

      #[test]
      fn block_port_get_out_of_range() {
          let bp = BlockPort::new(4);
          assert_eq!(bp.get(99, 0), 0.0);
          assert_eq!(bp.get(0, 99), 0.0);
      }

      #[test]
      fn block_port_set() {
          let mut bp = BlockPort::new(4);
          bp.set(1, 2, 3.14);
          assert!((bp.get(1, 2) - 3.14).abs() < 1e-6);
      }
  }
  ```

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular_core block_port
  ```
  Expected: compile error — `BlockPort` not found.

- [ ] **Step 3: Implement `BlockPort`**

  Write the full file `crates/modular_core/src/block_port.rs`:

  ```rust
  //! Block-sized port buffer.
  //!
  //! Layout: `data[sample_index][channel_index]`
  //!
  //! All channel values at the same sample index are contiguous in memory,
  //! enabling future SIMD optimisation. Heap-allocated once at construction;
  //! never resized on the audio thread.

  use crate::poly::PORT_MAX_CHANNELS;

  /// A pre-allocated buffer holding `block_size` samples, each with `PORT_MAX_CHANNELS` channels.
  ///
  /// `data[i][ch]` is the value for sample index `i`, channel `ch`.
  pub struct BlockPort {
      /// `data.len() == block_size` (set at construction, never changed).
      pub data: Box<[[f32; PORT_MAX_CHANNELS]]>,
  }

  impl BlockPort {
      /// Allocate a new zeroed port buffer for the given block size.
      ///
      /// **Must not be called on the audio thread** (allocates heap memory).
      pub fn new(block_size: usize) -> Self {
          Self {
              data: vec![[0.0f32; PORT_MAX_CHANNELS]; block_size].into_boxed_slice(),
          }
      }

      /// Read value at `(index, ch)`, returning `0.0` for out-of-range accesses.
      #[inline]
      pub fn get(&self, index: usize, ch: usize) -> f32 {
          self.data
              .get(index)
              .and_then(|slot| slot.get(ch).copied())
              .unwrap_or(0.0)
      }

      /// Write value at `(index, ch)`. Silently ignored if out of range.
      #[inline]
      pub fn set(&mut self, index: usize, ch: usize, value: f32) {
          if let Some(slot) = self.data.get_mut(index) {
              if let Some(cell) = slot.get_mut(ch) {
                  *cell = value;
              }
          }
      }
  }

  #[cfg(test)]
  mod tests { /* ... paste test module here ... */ }
  ```

- [ ] **Step 4: Export from `lib.rs`**

  In `crates/modular_core/src/lib.rs`, add before other module declarations:

  ```rust
  pub mod block_port;
  pub use block_port::BlockPort;
  ```

- [ ] **Step 5: Run tests to verify they pass**

  ```bash
  cargo test -p modular_core block_port
  ```
  Expected: 4 tests PASS.

- [ ] **Step 6: Commit**

  ```bash
  git add crates/modular_core/src/block_port.rs crates/modular_core/src/lib.rs
  git commit -m "feat(core): add BlockPort sample-major block buffer"
  ```

---

## Task 2: Foundation Types — ProcessingMode, ExternalClockState, InjectIndexPtr

**Files:**
- Modify: `crates/modular_core/src/types.rs`
- Modify: `crates/modular_core/src/poly.rs`
- Modify: `crates/modular_core/src/lib.rs`

- [ ] **Step 1: Write failing tests**

  Add to `crates/modular_core/tests/types_tests.rs` (near the top of the test module):

  ```rust
  #[test]
  fn processing_mode_default_is_block() {
      assert_eq!(ProcessingMode::default(), ProcessingMode::Block);
  }

  #[test]
  fn external_clock_state_default() {
      let s = ExternalClockState::default();
      assert!((s.bar_phase - 0.0).abs() < f64::EPSILON);
      assert!((s.bpm - 120.0).abs() < f64::EPSILON);
      assert!(!s.playing);
  }

  #[test]
  fn inject_index_ptr_signal_fixed() {
      use std::cell::Cell;
      let idx = Cell::new(7usize);
      let mut sig = Signal::Fixed(1.0);
      sig.inject_index_ptr(&idx as *const _);
      // Fixed signals ignore inject — no panic
  }
  ```

  Also add these imports at the top of the test file if not already present:
  ```rust
  use modular_core::types::{ExternalClockState, ProcessingMode, Signal};
  use modular_core::InjectIndexPtr;
  ```

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular_core processing_mode_default
  ```
  Expected: compile error — `ProcessingMode` not found.

- [ ] **Step 3: Add `ProcessingMode` and `ExternalClockState` to `types.rs`**

  In `crates/modular_core/src/types.rs`, near the top (after the imports, before the `Sampleable` trait):

  ```rust
  /// Determines how a module wrapper processes samples.
  ///
  /// - `Block`: compute all `block_size` samples in one `ensure_processed()` call.
  /// - `Sample`: compute exactly one sample per `ensure_processed()` call (used for
  ///   modules inside feedback cycles and ROOT_CLOCK/HiddenAudioIn which have
  ///   external per-sample data injection).
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum ProcessingMode {
      #[default]
      Block,
      Sample,
  }

  /// Per-sample external clock state injected into ROOT_CLOCK by the audio callback.
  ///
  /// The callback pre-computes one entry per sample in the block by querying the
  /// Link timeline. The ROOT_CLOCK wrapper injects the appropriate entry before
  /// calling inner `Clock::update()` for each sample position.
  #[derive(Debug, Clone, Copy, Default)]
  pub struct ExternalClockState {
      /// Bar phase in [0, 1).
      pub bar_phase: f64,
      /// Tempo in BPM.
      pub bpm: f64,
      /// Whether the Link session is playing.
      pub playing: bool,
  }
  ```

- [ ] **Step 4: Add `InjectIndexPtr` trait to `types.rs`**

  Immediately after `ExternalClockState`:

  ```rust
  /// Implemented by signal-bearing types to receive the back-pointer to the
  /// consuming wrapper's `index: Cell<usize>`.
  ///
  /// The `Connect` derive macro generates an impl for every params struct,
  /// iterating over each signal field. Base impls for primitive types are
  /// no-ops so the generated code can call `inject_index_ptr` on any field.
  pub trait InjectIndexPtr {
      /// Store `ptr` (which points to the consuming wrapper's `index` cell) so
      /// that `Signal::get_value()` can pass the correct sample index upstream.
      ///
      /// # Safety
      /// `ptr` must remain valid for the lifetime of this signal connection
      /// (i.e., until `connect()` is called again with a new patch).
      fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>);
  }

  /// Blanket no-op impl for types that carry no signals.
  macro_rules! noop_inject {
      ($($t:ty),*) => {
          $(impl InjectIndexPtr for $t {
              #[inline]
              fn inject_index_ptr(&mut self, _ptr: *const std::cell::Cell<usize>) {}
          })*
      };
  }

  noop_inject!(f32, f64, i32, i64, u32, u64, usize, bool, String, serde_json::Value);

  impl<T: InjectIndexPtr> InjectIndexPtr for Option<T> {
      #[inline]
      fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>) {
          if let Some(inner) = self {
              inner.inject_index_ptr(ptr);
          }
      }
  }

  impl<T: InjectIndexPtr> InjectIndexPtr for Vec<T> {
      #[inline]
      fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>) {
          for item in self.iter_mut() {
              item.inject_index_ptr(ptr);
          }
      }
  }
  ```

- [ ] **Step 5: Add `InjectIndexPtr` impls for `Signal` in `types.rs`**

  Find the `impl Signal` block (near line 1999) and add after the existing methods:

  ```rust
  impl InjectIndexPtr for Signal {
      fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>) {
          if let Signal::Cable { ref mut index_ptr, .. } = self {
              *index_ptr = ptr;
          }
      }
  }
  ```

  > Note: `Signal::Cable` doesn't have `index_ptr` yet — that comes in Task 3. For now the compiler will error on this; comment it out with `// TODO: Task 3` and uncomment in Task 3.

- [ ] **Step 6: Add `InjectIndexPtr` impls for `PolySignal`/`MonoSignal` in `poly.rs`**

  In `crates/modular_core/src/poly.rs`, after the existing signal type impls:

  ```rust
  use crate::types::InjectIndexPtr;

  impl InjectIndexPtr for PolySignal {
      fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>) {
          for sig in self.signals_mut() {
              sig.inject_index_ptr(ptr);
          }
      }
  }

  impl InjectIndexPtr for MonoSignal {
      fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>) {
          self.inner_mut().inject_index_ptr(ptr);
      }
  }
  ```

  > `PolySignal::signals_mut()` and `MonoSignal::inner_mut()` may not exist yet. If not, add private helpers or use direct field access. Check `poly.rs` for the actual field structure and adjust accordingly. The key requirement: iterate over all `Signal` fields and call `inject_index_ptr` on each.

- [ ] **Step 7: Export from `lib.rs`**

  In `crates/modular_core/src/lib.rs`:

  ```rust
  pub use types::{ExternalClockState, InjectIndexPtr, ProcessingMode};
  ```

- [ ] **Step 8: Run tests**

  ```bash
  cargo test -p modular_core processing_mode_default_is_block external_clock_state_default
  ```
  Expected: PASS (the `inject_index_ptr_signal_fixed` test may still fail until Task 3 adds `index_ptr` to `Signal::Cable`; that's OK).

- [ ] **Step 9: Commit**

  ```bash
  git add crates/modular_core/src/types.rs crates/modular_core/src/poly.rs crates/modular_core/src/lib.rs
  git commit -m "feat(core): add ProcessingMode, ExternalClockState, InjectIndexPtr"
  ```

---

## Task 3: Signal Index Back-Pointer

Add `index_ptr: *const Cell<usize>` to `Signal::Cable` and update all construction/pattern-match sites.

**Files:**
- Modify: `crates/modular_core/src/types.rs`

- [ ] **Step 1: Write the failing test**

  In `crates/modular_core/tests/types_tests.rs`:

  ```rust
  #[test]
  fn signal_cable_index_ptr_null_by_default() {
      use std::cell::Cell;
      // WellKnownModule::to_cable creates Cable with null index_ptr
      let sig = WellKnownModule::RootClock.to_cable(0, "barTrigger");
      if let Signal::Cable { index_ptr, .. } = sig {
          assert!(index_ptr.is_null());
      } else {
          panic!("expected Cable");
      }
  }

  #[test]
  fn inject_index_ptr_wires_cable() {
      use std::cell::Cell;
      let idx = Cell::new(3usize);
      let mut sig = WellKnownModule::RootClock.to_cable(0, "barTrigger");
      sig.inject_index_ptr(&idx as *const _);
      if let Signal::Cable { index_ptr, .. } = sig {
          assert!(!index_ptr.is_null());
          assert_eq!(unsafe { (*index_ptr).get() }, 3);
      }
  }
  ```

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular_core signal_cable_index_ptr_null_by_default
  ```
  Expected: compile error — `Signal::Cable` has no `index_ptr` field.

- [ ] **Step 3: Add `index_ptr` to `Signal::Cable`**

  In `types.rs`, find the `Signal` enum definition (around line 1764). Change the `Cable` variant to add `index_ptr`:

  ```rust
  Cable {
      module_id: String,
      port: String,
      channel: usize,
      /// Back-pointer to the consuming wrapper's `index: Cell<usize>`.
      /// Null until `inject_index_ptr` is called during `connect()`.
      /// Raw pointer is safe because the wrapper owns the `Cell` and outlives
      /// all `Signal`s that reference it.
      index_ptr: *const std::cell::Cell<usize>,
      module_ref: Weak<Box<dyn Sampleable>>,
  },
  ```

- [ ] **Step 4: Add `unsafe impl Send/Sync for Signal`**

  Raw pointers are `!Send + !Sync`. `Signal` must be `Send` to pass through the command queue. Add immediately after the `Signal` enum definition:

  ```rust
  // SAFETY: `index_ptr` is null during construction and transport; it is only
  // written/read on the audio thread after `inject_index_ptr` is called during
  // `connect()`. The wrapper that owns the pointed-to `Cell` also upholds
  // `unsafe impl Send + Sync`.
  unsafe impl Send for Signal {}
  unsafe impl Sync for Signal {}
  ```

- [ ] **Step 5: Update `WellKnownModule::to_cable()` (line ~99)**

  Find the `to_cable` method and add `index_ptr: std::ptr::null()` to the `Cable { .. }` constructor:

  ```rust
  Signal::Cable {
      module_id: self.id().to_string(),
      port: port.to_string(),
      channel: ch,
      index_ptr: std::ptr::null(),
      module_ref: Weak::new(),
  }
  ```

- [ ] **Step 6: Update `Signal` deserialization**

  Find the `Deserialize` impl for `Signal` (around line 1787). In the `Cable` variant deserializer arm, add `index_ptr: std::ptr::null()` to the struct construction.

- [ ] **Step 7: Update `Signal::PartialEq` (line ~2071)**

  The existing `PartialEq` impl pattern-matches `Cable` fields by name. Add `..` to ignore `index_ptr` (it's a runtime pointer, not a semantic equality field):

  ```rust
  (Signal::Cable { module_id: a_id, port: a_port, channel: a_ch, .. },
   Signal::Cable { module_id: b_id, port: b_port, channel: b_ch, .. }) => {
      a_id == b_id && a_port == b_port && a_ch == b_ch
  }
  ```

- [ ] **Step 8: Update `Signal::get_value()` to use `index_ptr`**

  Find `Signal::get_value()` (around line 1999). Change the `Cable` arm to read the index from `index_ptr`:

  ```rust
  Signal::Cable { port, channel, module_ref, index_ptr, .. } => {
      match module_ref.upgrade() {
          Some(arc) => {
              arc.ensure_processed();
              let idx = if index_ptr.is_null() {
                  0
              } else {
                  unsafe { (*index_ptr).get() }
              };
              arc.get_value_at(port, *channel, idx)
          }
          None => 0.0,
      }
  }
  ```

  > `ensure_processed()` and `get_value_at()` are added to the `Sampleable` trait in Task 8. For now, keep the old `update()` and `get_poly_sample()` calls alongside — they'll coexist until Task 12.

- [ ] **Step 9: Uncomment the `InjectIndexPtr for Signal` impl from Task 2**

  Remove the `// TODO: Task 3` comment and ensure the impl compiles:

  ```rust
  impl InjectIndexPtr for Signal {
      fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>) {
          if let Signal::Cable { ref mut index_ptr, .. } = self {
              *index_ptr = ptr;
          }
      }
  }
  ```

- [ ] **Step 10: Fix all remaining pattern-match exhaustiveness errors**

  Run `cargo check -p modular_core` and add `index_ptr: std::ptr::null()` (or `..`) to any remaining `Cable { .. }` construction sites the compiler flags.

- [ ] **Step 11: Run tests**

  ```bash
  cargo test -p modular_core signal_cable_index_ptr
  ```
  Expected: both new tests PASS.

- [ ] **Step 12: Run full modular_core tests**

  ```bash
  cargo test -p modular_core
  ```
  Expected: all existing tests pass.

- [ ] **Step 13: Commit**

  ```bash
  git add crates/modular_core/src/types.rs crates/modular_core/src/poly.rs
  git commit -m "feat(core): add index_ptr back-pointer to Signal::Cable"
  ```

---

## Task 4: Graph Cycle Detection (Tarjan's SCC)

**Files:**
- Create: `crates/modular/src/graph_analysis.rs`
- Modify: `crates/modular/src/lib.rs`

- [ ] **Step 1: Write failing tests**

  Create `crates/modular/src/graph_analysis.rs` with just the test module:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use modular_core::types::{ModuleState, PatchGraph, ProcessingMode};
      use serde_json::json;

      fn make_graph(edges: &[(&str, &str, &str)]) -> PatchGraph {
          // edges: (consumer_id, producer_id, port)
          let mut modules = Vec::new();
          let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
          for (c, p, _) in edges {
              ids.insert(c);
              ids.insert(p);
          }
          // Add standalone modules with no params
          for id in &ids {
              let params = if edges.iter().any(|(c, _, _)| c == id) {
                  // consumer: inject a cable param
                  let producer = edges.iter().find(|(c, _, _)| c == id).unwrap();
                  json!({ "input": { "type": "cable", "module": producer.1, "port": producer.2, "channel": 0 } })
              } else {
                  json!({})
              };
              modules.push(ModuleState {
                  id: id.to_string(),
                  module_type: "test".to_string(),
                  id_is_explicit: None,
                  params,
              });
          }
          PatchGraph { modules, module_id_remaps: None, scopes: vec![] }
      }

      #[test]
      fn no_cycle_is_block_mode() {
          // A -> B -> C (A produces, C consumes)
          let graph = make_graph(&[("B", "A", "out"), ("C", "B", "out")]);
          let modes = classify_modules(&graph);
          assert_eq!(modes["A"], ProcessingMode::Block);
          assert_eq!(modes["B"], ProcessingMode::Block);
          assert_eq!(modes["C"], ProcessingMode::Block);
      }

      #[test]
      fn two_node_cycle_is_sample_mode() {
          // A <-> B (A reads B and B reads A)
          let graph = make_graph(&[("A", "B", "out"), ("B", "A", "out")]);
          let modes = classify_modules(&graph);
          assert_eq!(modes["A"], ProcessingMode::Sample);
          assert_eq!(modes["B"], ProcessingMode::Sample);
      }

      #[test]
      fn self_loop_is_sample_mode() {
          let graph = make_graph(&[("A", "A", "out")]);
          let modes = classify_modules(&graph);
          assert_eq!(modes["A"], ProcessingMode::Sample);
      }

      #[test]
      fn cycle_plus_independent_node() {
          // A <-> B, C is independent
          let mut graph = make_graph(&[("A", "B", "out"), ("B", "A", "out")]);
          graph.modules.push(ModuleState {
              id: "C".to_string(),
              module_type: "test".to_string(),
              id_is_explicit: None,
              params: serde_json::json!({}),
          });
          let modes = classify_modules(&graph);
          assert_eq!(modes["A"], ProcessingMode::Sample);
          assert_eq!(modes["B"], ProcessingMode::Sample);
          assert_eq!(modes["C"], ProcessingMode::Block);
      }
  }
  ```

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular graph_analysis
  ```
  Expected: compile error — `classify_modules` not found.

- [ ] **Step 3: Implement Tarjan's SCC**

  Write the full `crates/modular/src/graph_analysis.rs`:

  ```rust
  //! Graph cycle detection via Tarjan's SCC algorithm.
  //!
  //! Returns `ProcessingMode` per module ID. Modules in a strongly-connected
  //! component with >1 member, or a single node with a self-loop, are assigned
  //! `Sample` mode. All others get `Block` mode.

  use std::collections::HashMap;
  use modular_core::types::{PatchGraph, ProcessingMode};

  /// Analyse `graph` and return the processing mode for each module id.
  ///
  /// Modules not present in the graph are not in the returned map.
  pub fn classify_modules(graph: &PatchGraph) -> HashMap<String, ProcessingMode> {
      // Build adjacency: consumer_id → [producer_id, ...]
      let mut deps: HashMap<String, Vec<String>> = HashMap::new();

      for state in &graph.modules {
          deps.entry(state.id.clone()).or_default();
          collect_cable_deps(&state.params, &state.id, &mut deps);
      }

      let mut ctx = TarjanCtx::default();
      let nodes: Vec<String> = deps.keys().cloned().collect();
      for node in &nodes {
          if !ctx.index_map.contains_key(node.as_str()) {
              ctx.strongconnect(node, &deps);
          }
      }

      let mut result = HashMap::new();
      for scc in &ctx.sccs {
          let cyclic = scc.len() > 1
              || deps
                  .get(&scc[0])
                  .map_or(false, |d| d.iter().any(|x| x == &scc[0]));
          let mode = if cyclic {
              ProcessingMode::Sample
          } else {
              ProcessingMode::Block
          };
          for id in scc {
              result.insert(id.clone(), mode);
          }
      }
      result
  }

  /// Recursively scan a params JSON value and record any `{type:"cable"}` edges.
  fn collect_cable_deps(
      value: &serde_json::Value,
      consumer_id: &str,
      deps: &mut HashMap<String, Vec<String>>,
  ) {
      match value {
          serde_json::Value::Object(map) => {
              if map.get("type").and_then(|v| v.as_str()) == Some("cable") {
                  if let Some(producer_id) = map.get("module").and_then(|v| v.as_str()) {
                      deps.entry(consumer_id.to_string())
                          .or_default()
                          .push(producer_id.to_string());
                      deps.entry(producer_id.to_string()).or_default();
                  }
              } else {
                  for val in map.values() {
                      collect_cable_deps(val, consumer_id, deps);
                  }
              }
          }
          serde_json::Value::Array(arr) => {
              for val in arr {
                  collect_cable_deps(val, consumer_id, deps);
              }
          }
          _ => {}
      }
  }

  #[derive(Default)]
  struct TarjanCtx {
      counter: usize,
      stack: Vec<String>,
      on_stack: HashMap<String, bool>,
      index_map: HashMap<String, usize>,
      lowlink: HashMap<String, usize>,
      sccs: Vec<Vec<String>>,
  }

  impl TarjanCtx {
      fn strongconnect(&mut self, v: &str, deps: &HashMap<String, Vec<String>>) {
          self.index_map.insert(v.to_string(), self.counter);
          self.lowlink.insert(v.to_string(), self.counter);
          self.counter += 1;
          self.stack.push(v.to_string());
          self.on_stack.insert(v.to_string(), true);

          let neighbors = deps.get(v).cloned().unwrap_or_default();
          for w in neighbors {
              if !self.index_map.contains_key(w.as_str()) {
                  self.strongconnect(&w, deps);
                  let lv = self.lowlink[v];
                  let lw = self.lowlink[&w];
                  self.lowlink.insert(v.to_string(), lv.min(lw));
              } else if *self.on_stack.get(&w).unwrap_or(&false) {
                  let lv = self.lowlink[v];
                  let iw = self.index_map[&w];
                  self.lowlink.insert(v.to_string(), lv.min(iw));
              }
          }

          if self.lowlink[v] == self.index_map[v] {
              let mut scc = Vec::new();
              loop {
                  let w = self.stack.pop().unwrap();
                  self.on_stack.insert(w.clone(), false);
                  let is_v = w == v;
                  scc.push(w);
                  if is_v {
                      break;
                  }
              }
              self.sccs.push(scc);
          }
      }
  }

  #[cfg(test)]
  mod tests { /* paste tests from Step 1 here */ }
  ```

- [ ] **Step 4: Add module to `crates/modular/src/lib.rs`**

  Add near the top of `lib.rs`:

  ```rust
  mod graph_analysis;
  pub use graph_analysis::classify_modules;
  ```

- [ ] **Step 5: Run tests**

  ```bash
  cargo test -p modular graph_analysis
  ```
  Expected: all 4 tests PASS.

- [ ] **Step 6: Commit**

  ```bash
  git add crates/modular/src/graph_analysis.rs crates/modular/src/lib.rs
  git commit -m "feat(modular): Tarjan SCC cycle detection for block/sample mode classification"
  ```

---

## Task 5: Outputs Derive — Generate `{Name}BlockOutputs`

Extend `#[derive(Outputs)]` to also emit a `{Name}BlockOutputs` struct with `BlockPort` per field, plus `new(block_size)`, `get_at`, `copy_from_inner`, `tick_buffers` methods.

**Files:**
- Modify: `crates/modular_derive/src/outputs.rs`

- [ ] **Step 1: Write a compile-test for the generated struct**

  Create `crates/modular_derive/tests/block_outputs.rs` (or add to existing integration test file):

  ```rust
  // Integration test: verify BlockOutputs is generated for a simple Outputs struct.
  use modular_core::poly::PolyOutput;
  use modular_derive::Outputs;

  #[derive(Outputs)]
  struct SimpleOutputs {
      #[output("value", "A value", default)]
      value: f32,
      #[output("poly", "Poly out")]
      poly: PolyOutput,
  }

  #[test]
  fn block_outputs_struct_exists() {
      // Should compile; new() must accept block_size.
      let bo = SimpleBlockOutputs::new(4);
      // get_at returns 0.0 for fresh buffer
      assert_eq!(bo.get_at("value", 0, 0), 0.0);
      assert_eq!(bo.get_at("poly", 3, 2), 0.0);
  }

  #[test]
  fn copy_from_inner_fills_block_outputs() {
      let inner = SimpleOutputs {
          value: 2.5,
          poly: PolyOutput::mono(1.0),
      };
      let mut bo = SimpleBlockOutputs::new(4);
      bo.copy_from_inner(&inner, 2);
      assert!((bo.get_at("value", 0, 2) - 2.5).abs() < 1e-6);
      assert!((bo.get_at("poly", 0, 2) - 1.0).abs() < 1e-6);
  }
  ```

  > The convention for port name matching in `get_at` is the camelCase string from the `#[output("name", ...)]` attribute.

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular_derive block_outputs
  ```
  Expected: compile error — `SimpleBlockOutputs` not found.

- [ ] **Step 3: Extend `impl_outputs_macro` in `outputs.rs`**

  After the existing `generated` quote block (line 368–401), add generation of the `BlockOutputs` struct. Key additions to `impl_outputs_macro`:

  ```rust
  // Derive BlockOutputs struct name: strip "Outputs" suffix, append "BlockOutputs"
  let name_str = name.to_string();
  let block_outputs_name = if name_str.ends_with("Outputs") {
      format_ident!("{}BlockOutputs", &name_str[..name_str.len() - 7])
  } else {
      format_ident!("{}BlockOutputs", name_str)
  };

  // Generate BlockPort fields
  let block_fields: Vec<_> = outputs.iter().map(|o| {
      let field_name = &o.field_name;
      quote! { pub #field_name: crate::block_port::BlockPort }
  }).collect();

  // Generate new() constructor
  let block_new_inits: Vec<_> = outputs.iter().map(|o| {
      let field_name = &o.field_name;
      quote! { #field_name: crate::block_port::BlockPort::new(block_size) }
  }).collect();

  // Generate get_at() match arms: port name (camelCase) → field
  let get_at_arms: Vec<_> = outputs.iter().map(|o| {
      let output_name = &o.output_name;
      let field_name = &o.field_name;
      quote! {
          #output_name => self.#field_name.get(index, ch),
      }
  }).collect();

  // Generate copy_from_inner() statements
  let copy_inner_stmts: Vec<_> = outputs.iter().map(|o| {
      let field_name = &o.field_name;
      match o.precision {
          OutputPrecision::F32 => quote! {
              self.#field_name.data[slot][0] = inner.#field_name;
          },
          OutputPrecision::PolySignal => quote! {
              {
                  let poly = &inner.#field_name;
                  for ch in 0..crate::poly::PORT_MAX_CHANNELS {
                      self.#field_name.data[slot][ch] = poly.get(ch);
                  }
              }
          },
      }
  }).collect();

  let block_generated = quote! {
      /// Generated block-output buffer for #name.
      /// One `BlockPort` per output port; indexed `data[sample_index][channel]`.
      pub struct #block_outputs_name {
          #(#block_fields,)*
      }

      impl #block_outputs_name {
          /// Allocate all ports for the given block size. Call only on the main thread.
          pub fn new(block_size: usize) -> Self {
              Self {
                  #(#block_new_inits,)*
              }
          }

          /// Read the value at `(ch, index)` for port `port`. Returns 0.0 for unknown ports.
          pub fn get_at(&self, port: &str, ch: usize, index: usize) -> f32 {
              match port {
                  #(#get_at_arms)*
                  _ => 0.0,
              }
          }

          /// Copy inner module outputs at `slot` into this block outputs buffer.
          pub fn copy_from_inner(&mut self, inner: &#name, slot: usize) {
              #(#copy_inner_stmts)*
          }

          /// Called once per CPAL callback to advance any stateful per-block fields.
          /// Default is a no-op; `BufferWrite` overrides this to advance write_index.
          pub fn tick_buffers(&mut self, _block_size: usize) {}
      }
  };

  // Combine both generated blocks
  let mut all_generated = quote!(#generated);
  all_generated.extend(block_generated);
  all_generated.into()
  ```

  Replace the final `generated.into()` with `all_generated.into()`.

- [ ] **Step 4: Run tests**

  ```bash
  cargo test -p modular_derive block_outputs
  cargo test -p modular_core    # ensure existing tests unaffected
  ```
  Expected: compile-test PASS, all modular_core tests PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add crates/modular_derive/src/outputs.rs
  git commit -m "feat(derive): generate {Name}BlockOutputs alongside Outputs derive"
  ```

---

## Task 6: Connect Derive — Generate `InjectIndexPtr` impl

Extend `#[derive(Connect)]` to also generate `InjectIndexPtr for {Name}Params` that calls `inject_index_ptr` on each signal field.

**Files:**
- Modify: `crates/modular_derive/src/connect.rs`

- [ ] **Step 1: Write compile-test**

  Add to the derive integration tests:

  ```rust
  use modular_core::types::{InjectIndexPtr, Signal};
  use modular_derive::Connect;
  use std::cell::Cell;

  #[derive(Connect)]
  struct TestParams {
      input: Signal,
      gain: f32,
  }

  #[test]
  fn inject_index_ptr_impl_exists() {
      use modular_core::InjectIndexPtr;
      let idx = Cell::new(5usize);
      let mut params = TestParams {
          input: Signal::Fixed(1.0),
          gain: 0.5,
      };
      // Must compile and not panic:
      params.inject_index_ptr(&idx as *const _);
  }
  ```

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular_derive inject_index_ptr_impl_exists
  ```
  Expected: compile error — `InjectIndexPtr` not implemented for `TestParams`.

- [ ] **Step 3: Extend `impl_connect_macro` in `connect.rs`**

  In the function, after building `connect_body`, add an `inject_body`:

  ```rust
  // Generate inject_index_ptr calls for all fields
  let mut inject_stmts = TokenStream2::new();
  for field in fields.named.iter() {
      let Some(field_ident) = &field.ident else { continue };
      inject_stmts.extend(quote_spanned! {field.span()=>
          crate::types::InjectIndexPtr::inject_index_ptr(&mut self.#field_ident, ptr);
      });
  }
  ```

  Then in the `generated` quote block, add the `InjectIndexPtr` impl:

  ```rust
  let generated = quote! {
      impl crate::types::Connect for #name {
          fn connect(&mut self, patch: &crate::Patch) {
              #default_connection_stmts
              #connect_body
          }
      }

      impl crate::types::InjectIndexPtr for #name {
          fn inject_index_ptr(&mut self, ptr: *const std::cell::Cell<usize>) {
              #inject_stmts
          }
      }
  };
  ```

- [ ] **Step 4: Run tests**

  ```bash
  cargo test -p modular_derive inject_index_ptr_impl_exists
  cargo test -p modular     # ensure existing tests unaffected
  ```
  Expected: PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add crates/modular_derive/src/connect.rs
  git commit -m "feat(derive): generate InjectIndexPtr impl from Connect derive"
  ```

---

## Task 7: Sampleable Trait — Add New Methods (Additive)

Add `ensure_processed`, `get_value_at`, updated `tick_buffers` on `OutputStruct`, and new `sync_external_clock` signature. Keep old `update()` and `get_poly_sample()` to maintain backward compatibility during transition.

**Files:**
- Modify: `crates/modular_core/src/types.rs`

- [ ] **Step 1: Write failing test**

  In `crates/modular_core/tests/types_tests.rs`, update `DummySampleable`:

  ```rust
  struct DummySampleable { id: String, value: f32 }
  impl Sampleable for DummySampleable {
      fn get_id(&self) -> &str { &self.id }
      fn tick(&self) {}
      fn update(&self) {}
      fn get_poly_sample(&self, _port: &str) -> napi::Result<PolyOutput> {
          Ok(PolyOutput::mono(self.value))
      }
      fn ensure_processed(&self) {}
      fn get_value_at(&self, _port: &str, _ch: usize, _index: usize) -> f32 { self.value }
      fn get_module_type(&self) -> &str { "dummy" }
      fn connect(&self, _patch: &Patch) {}
      fn as_any(&self) -> &dyn std::any::Any { self }
      fn transfer_state_from(&self, _old: &dyn Sampleable) {}
  }
  ```

  Add a test that calls the new methods:

  ```rust
  #[test]
  fn sampleable_ensure_processed_and_get_value_at() {
      let s: Arc<Box<dyn Sampleable>> = Arc::new(Box::new(DummySampleable {
          id: "test".to_string(),
          value: 3.0,
      }));
      s.ensure_processed();
      assert!((s.get_value_at("value", 0, 0) - 3.0).abs() < 1e-6);
  }
  ```

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular_core sampleable_ensure_processed
  ```
  Expected: compile error — `ensure_processed` not in `Sampleable`.

- [ ] **Step 3: Add new methods to `Sampleable` trait in `types.rs`**

  In the `Sampleable` trait (around line 154), add after `update()`:

  ```rust
  /// Process outputs up to the current block boundary.
  ///
  /// - `Block` mode: computes all `block_size` samples in one call.
  /// - `Sample` mode: computes exactly the next uncomputed sample.
  ///
  /// Idempotent: safe to call multiple times per block.
  /// Reentrancy-safe: if called while already computing (feedback cycle),
  /// returns immediately — callers read the previous block's last value.
  fn ensure_processed(&self) {}

  /// Read the computed value for `(port, ch)` at sample `index` within the
  /// current block. Calls `ensure_processed()` internally.
  ///
  /// Returns 0.0 for unknown ports or out-of-range indices.
  fn get_value_at(&self, _port: &str, _ch: usize, _index: usize) -> f32 { 0.0 }
  ```

  Also add to the `Sampleable` trait (new default-no-op method for audio input injection):

  ```rust
  /// Pre-fill the audio input block for HiddenAudioIn modules.
  /// Only the HiddenAudioIn wrapper overrides this. Default: no-op.
  fn inject_audio_in_block(&self, _block: &[[f32; crate::poly::PORT_MAX_CHANNELS]]) {}
  ```

- [ ] **Step 4: Add `tick_buffers` to `OutputStruct` trait in `types.rs`**

  In the `OutputStruct` trait (around line 2187), add:

  ```rust
  /// Called once per CPAL callback (after all modules are ticked) to advance
  /// any stateful buffers. Default: no-op. `BufferWrite` overrides this to
  /// advance `BufferData::write_index` by `block_size`.
  fn tick_buffers(&mut self, _block_size: usize) {}
  ```

- [ ] **Step 5: Run tests**

  ```bash
  cargo test -p modular_core
  ```
  Expected: all tests PASS (new methods have default impls so existing code compiles).

- [ ] **Step 6: Commit**

  ```bash
  git add crates/modular_core/src/types.rs crates/modular_core/tests/types_tests.rs
  git commit -m "feat(core): add ensure_processed, get_value_at to Sampleable; tick_buffers to OutputStruct"
  ```

---

## Task 8: Module_attr Wrapper Rewrite

This is the largest task. Replace `processed: AtomicBool` + `outputs: UnsafeCell<O>` with `index: Cell<usize>` + `computing: Cell<bool>` + `block_outputs: UnsafeCell<{Name}BlockOutputs>`. Implement `ensure_processed`, `get_value_at`, updated `tick`. Inject `_block_index: Cell<usize>` into inner struct. Wire `InjectIndexPtr` in `connect()`. Handle clock_sync/audio_input injection per-sample.

**Files:**
- Modify: `crates/modular_derive/src/module_attr.rs`

- [ ] **Step 1: Write failing compile-test**

  Create a test module that uses a simple `#[module(...)]` and verifies the new wrapper fields compile:

  ```rust
  // In tests/module_attr_block.rs or as a doc-test:
  // After this task, the generated wrapper must have:
  // - ensure_processed() that computes block or sample
  // - get_value_at() that reads from block_outputs
  // - tick() that resets index to 0
  // Verified by the existing cargo test -p modular passing.
  ```

  The real "test" here is `cargo check -p modular` compiling. Run it after each sub-step.

- [ ] **Step 2: Inject `_block_index: Cell<usize>` into inner struct**

  In `module_impl` (around line 225), after injecting `_channel_count`:

  ```rust
  // Inject _block_index for current_block_index() in block-aware DSP code
  if let Data::Struct(ref mut data_struct) = ast.data {
      if let Fields::Named(ref mut fields) = data_struct.fields {
          let field: syn::Field = syn::parse_quote! {
              pub _block_index: std::cell::Cell<usize>
          };
          fields.named.push(field);
      }
  }
  ```

- [ ] **Step 3: Generate `current_block_index()` on inner struct**

  In `impl_module_macro_attr`, in the `impl #name` block generation, add:

  ```rust
  impl #impl_generics #name #ty_generics #where_clause {
      /// Returns the current sample index within the block being processed.
      /// Only valid to call from within `update()`.
      #[inline]
      pub fn current_block_index(&self) -> usize {
          self._block_index.get()
      }

      // ... existing channel_count() method ...
  }
  ```

- [ ] **Step 4: Handle `_block_index` in field_inits**

  In the `field_inits` match arm, add:

  ```rust
  "_block_index" => Ok(quote! { _block_index: Default::default() }),
  ```

- [ ] **Step 5: Derive block outputs type name in `impl_module_macro_attr`**

  After extracting `outputs_ty`, add:

  ```rust
  // Derive the BlockOutputs type name from the outputs type.
  // Convention: {Name}Outputs → {Name}BlockOutputs
  let block_outputs_ty_name = match &outputs_ty {
      syn::Type::Path(tp) => {
          let last = tp.path.segments.last().unwrap();
          let s = last.ident.to_string();
          let base = if s.ends_with("Outputs") { &s[..s.len() - 7] } else { &s[..] };
          format_ident!("{}BlockOutputs", base)
      }
      _ => return Err(syn::Error::new(proc_macro2::Span::call_site(), "outputs type must be a simple path")),
  };
  ```

- [ ] **Step 6: Replace wrapper struct definition**

  Change the struct definition in the `generated` quote block (currently line 664) from:

  ```rust
  struct #struct_name {
      id: String,
      outputs: std::cell::UnsafeCell<#outputs_ty>,
      module: std::cell::UnsafeCell<#name #static_ty_generics>,
      processed: core::sync::atomic::AtomicBool,
      sample_rate: f32,
      argument_spans: ...,
  }
  ```

  To:

  ```rust
  struct #struct_name {
      id: String,
      block_outputs: std::cell::UnsafeCell<#block_outputs_ty_name>,
      module: std::cell::UnsafeCell<#name #static_ty_generics>,
      /// Next sample slot to write into block_outputs. Resets to 0 on tick().
      index: std::cell::Cell<usize>,
      /// True while ensure_processed() is executing. Guards against reentrant calls.
      computing: std::cell::Cell<bool>,
      mode: modular_core::types::ProcessingMode,
      block_size: usize,
      sample_rate: f32,
      argument_spans: std::cell::UnsafeCell<std::collections::HashMap<String, crate::params::ArgumentSpan>>,
  }
  ```

- [ ] **Step 7: Update constructor function signature and body**

  Change `#constructor_name` signature to accept `block_size: usize, mode: modular_core::types::ProcessingMode`:

  ```rust
  fn #constructor_name(
      id: &String,
      sample_rate: f32,
      deserialized: crate::params::DeserializedParams,
      block_size: usize,
      mode: modular_core::types::ProcessingMode,
  ) -> napi::Result<std::sync::Arc<Box<dyn crate::types::Sampleable>>> {
      // ... existing param downcast ...
      let mut inner = #name #static_ty_generics { #(#module_field_inits),* };
      crate::types::OutputStruct::set_all_channels(&mut inner.outputs, deserialized.channel_count);

      let sampleable = #struct_name {
          id: id.clone(),
          sample_rate,
          block_outputs: std::cell::UnsafeCell::new(#block_outputs_ty_name::new(block_size)),
          module: std::cell::UnsafeCell::new(inner),
          index: std::cell::Cell::new(0),
          computing: std::cell::Cell::new(false),
          mode,
          block_size,
          argument_spans: std::cell::UnsafeCell::new(deserialized.argument_spans),
      };

      #has_init_call
      Ok(std::sync::Arc::new(Box::new(sampleable)))
  }
  ```

- [ ] **Step 8: Update `SampleableConstructor` registration**

  In `Module::install_constructor`, the closure now passes `block_size` and `mode`:

  ```rust
  fn install_constructor(map: &mut std::collections::HashMap<String, crate::types::SampleableConstructor>) {
      map.insert(#module_name.into(), Box::new(#constructor_name));
  }
  ```

  The `SampleableConstructor` type itself will be updated in Task 11.

- [ ] **Step 9: Implement `tick()` on the wrapper**

  Replace the existing `tick()` impl:

  ```rust
  fn tick(&self) {
      self.index.set(0);
      // Let buffer modules advance their write position once per callback.
      unsafe { (*self.module.get()).outputs.tick_buffers(self.block_size); }
  }
  ```

- [ ] **Step 10: Implement `ensure_processed()` on the wrapper**

  Replace the existing `update()` impl with a new `ensure_processed()`:

  ```rust
  fn ensure_processed(&self) {
      // Already computed full block?
      if self.index.get() >= self.block_size {
          return;
      }
      // Reentrancy guard — feedback cycle detected at runtime.
      if self.computing.get() {
          return;
      }
      self.computing.set(true);

      let target = match self.mode {
          modular_core::types::ProcessingMode::Block => self.block_size,
          modular_core::types::ProcessingMode::Sample => self.index.get() + 1,
      };

      while self.index.get() < target {
          let i = self.index.get();
          unsafe {
              let module = &mut *self.module.get();
              // Set current block index so inner DSP can call current_block_index()
              module._block_index.set(i);
              // [clock_sync injection here — see Step 11]
              // [audio_input injection here — see Step 12]
              module.update(self.sample_rate);
              // Copy inner outputs to block buffer at slot i
              (*self.block_outputs.get()).copy_from_inner(&module.outputs, i);
          }
          self.index.set(i + 1);
      }

      self.computing.set(false);
  }
  ```

- [ ] **Step 11: Implement `get_value_at()` on the wrapper**

  ```rust
  fn get_value_at(&self, port: &str, ch: usize, index: usize) -> f32 {
      // Reentrancy: return 1-sample-delayed value (feedback cycle).
      if self.computing.get() {
          let prev = if index == 0 {
              self.block_size.saturating_sub(1)
          } else {
              index - 1
          };
          return unsafe { (*self.block_outputs.get()).get_at(port, ch, prev) };
      }
      self.ensure_processed();
      unsafe { (*self.block_outputs.get()).get_at(port, ch, index) }
  }
  ```

- [ ] **Step 12: Add `InjectIndexPtr` call in `connect()`**

  ```rust
  fn connect(&self, patch: &crate::Patch) {
      let module = unsafe { &mut *self.module.get() };
      crate::types::Connect::connect(&mut module.params, patch);
      // Wire index back-pointer into all Signal fields so upstream modules
      // serve the correct sample position.
      crate::types::InjectIndexPtr::inject_index_ptr(
          &mut module.params,
          &self.index as *const std::cell::Cell<usize>,
      );
  }
  ```

- [ ] **Step 13: Update `clock_sync_impl` generation**

  Change the `clock_sync_impl` from the old `(bar_phase, bpm, playing)` signature to the new `ExternalClockState` block injection:

  ```rust
  let clock_sync_impl = if attr_args.clock_sync {
      quote! {
          // Pre-allocated block of clock states; filled by inject before ensure_processed.
          // Uses UnsafeCell because it is written from the audio callback (single-threaded)
          // and read inside ensure_processed (also single-threaded, same call stack).
          // field: clock_state_block: UnsafeCell<Box<[ExternalClockState]>>
          // method:
          fn sync_external_clock(&self, states: &[modular_core::types::ExternalClockState]) {
              let block = unsafe { &mut *self.clock_state_block.get() };
              let n = states.len().min(block.len());
              block[..n].copy_from_slice(&states[..n]);
          }
      }
  } else {
      quote! {}
  };
  ```

  And in the wrapper struct, if `clock_sync`, add field:

  ```rust
  clock_state_block: std::cell::UnsafeCell<Box<[modular_core::types::ExternalClockState]>>,
  ```

  Initialized in constructor:

  ```rust
  clock_state_block: std::cell::UnsafeCell::new(
      vec![modular_core::types::ExternalClockState::default(); block_size]
          .into_boxed_slice()
  ),
  ```

  In `ensure_processed` while loop, if `clock_sync`:

  ```rust
  // [clock_sync injection — insert before module.update()] :
  {
      let clock_block = unsafe { &*self.clock_state_block.get() };
      if let Some(state) = clock_block.get(i) {
          module.sync_external_clock_impl(*state);
      }
  }
  ```

- [ ] **Step 14: Update `transfer_state_from` to swap `block_outputs`**

  ```rust
  fn transfer_state_from(&self, old: &dyn crate::types::Sampleable) {
      if let Some(old_typed) = old.as_any().downcast_ref::<Self>() {
          if std::ptr::eq(self as *const Self, old_typed as *const Self) {
              return;
          }
          let new_inner = unsafe { &mut *self.module.get() };
          let old_inner = unsafe { &mut *old_typed.module.get() };
          #transfer_state_body
          crate::types::OutputStruct::transfer_buffers_from(
              &mut new_inner.outputs,
              &mut old_inner.outputs,
          );
          // Swap block outputs so feedback cycles read previous-frame values.
          unsafe {
              let new_bo = &mut *self.block_outputs.get();
              let old_bo = &mut *old_typed.block_outputs.get();
              std::mem::swap(new_bo, old_bo);
          }
      }
  }
  ```

- [ ] **Step 15: Keep backward-compat `update()` and `get_poly_sample()` for now**

  These will be removed in Task 12. For now, keep them in the generated code:

  ```rust
  fn update(&self) {
      // Deprecated: calls ensure_processed() for backward compatibility.
      self.ensure_processed();
  }

  fn get_poly_sample(&self, port: &str) -> napi::Result<crate::poly::PolyOutput> {
      self.ensure_processed();
      let idx = self.index.get().saturating_sub(1);
      let block_outputs = unsafe { &*self.block_outputs.get() };
      let value = block_outputs.get_at(port, 0, idx);
      Ok(crate::poly::PolyOutput::mono(value))
  }
  ```

  > Note: `get_poly_sample` here is simplified (mono only). The full polyphonic version can read all channels from the `BlockPort`.

- [ ] **Step 16: Compile check**

  ```bash
  cargo check -p modular
  ```
  Expected: compiles with no errors.

- [ ] **Step 17: Run all tests**

  ```bash
  cargo test -p modular_core && cargo test -p modular
  ```
  Expected: all tests PASS.

- [ ] **Step 18: Commit**

  ```bash
  git add crates/modular_derive/src/module_attr.rs
  git commit -m "feat(derive): rewrite wrapper with block_outputs, index, computing, ensure_processed"
  ```

---

## Task 9: Clock Module Adaptation

Rename `Clock::sync_external_clock` → `Clock::sync_external_clock_impl` and update its signature to accept `ExternalClockState`. Remove `clear_external_sync` (the wrapper handles this differently now — no external sync is just an absent state in the block array).

**Files:**
- Modify: `crates/modular_core/src/dsp/core/clock.rs`

- [ ] **Step 1: Write failing test**

  In the clock tests (find them with `cargo test -p modular_core -- clock`), add:

  ```rust
  #[test]
  fn clock_sync_external_clock_impl_exists() {
      let mut clock = Clock::default();
      let state = ExternalClockState { bar_phase: 0.5, bpm: 120.0, playing: true };
      clock.sync_external_clock_impl(state); // Must compile
  }
  ```

- [ ] **Step 2: Run test to verify it fails**

  ```bash
  cargo test -p modular_core clock_sync_external_clock_impl_exists
  ```
  Expected: compile error — `sync_external_clock_impl` not found.

- [ ] **Step 3: Rename and update the method in `clock.rs`**

  In `crates/modular_core/src/dsp/core/clock.rs`, find the `sync_external_clock` method on the `Clock` struct (the inner DSP struct, not the wrapper). Change:

  ```rust
  // Old:
  pub fn sync_external_clock(&mut self, bar_phase: f64, bpm: f64, playing: bool) { ... }
  pub fn clear_external_sync(&mut self) { ... }
  ```

  To:

  ```rust
  // New:
  pub fn sync_external_clock_impl(&mut self, state: modular_core::types::ExternalClockState) {
      // Extract fields from state and apply same logic as before:
      let bar_phase = state.bar_phase;
      let bpm = state.bpm;
      let playing = state.playing;
      // ... same body as old sync_external_clock ...
  }
  ```

  Remove `clear_external_sync` (the `clock_sync` flag wrapper no longer calls it — injecting a Default state is equivalent to "no sync" if needed).

  > If `clock_sync` wrapper previously called `clear_external_sync()` between frames, replace that with: simply not calling `sync_external_clock_impl` when there is no Link session (the default `ExternalClockState` has `playing: false`).

- [ ] **Step 4: Run tests**

  ```bash
  cargo test -p modular_core
  ```
  Expected: all clock tests PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add crates/modular_core/src/dsp/core/clock.rs
  git commit -m "feat(clock): rename sync_external_clock → sync_external_clock_impl, accept ExternalClockState"
  ```

---

## Task 10: Buffer/Delay Adaptation

Update `BufferWrite::update()` to use `current_block_index()` for write position. Implement `tick_buffers` on `BufferWriteOutputs` to advance `write_index`. Update `DelayRead` to use `current_block_index()` for read offset. Fix `Buffer::ensure_source_updated`.

**Files:**
- Modify: `crates/modular_core/src/dsp/utilities/buffer.rs`
- Modify: `crates/modular_core/src/dsp/utilities/delay.rs`

- [ ] **Step 1: Write failing test**

  In `buffer.rs` tests, add:

  ```rust
  #[test]
  fn buffer_write_index_advances_by_block_size_on_tick() {
      // Set up a BufferWrite with block_size samples.
      // After tick_buffers(4), write_index should advance by 4.
      // ... (use MockBufferOwner or construct directly)
  }
  ```

  > The exact test depends on the `BufferWriteOutputs` structure. See the existing buffer tests for setup patterns.

- [ ] **Step 2: Update `ensure_source_updated` in `buffer.rs`**

  Find `Buffer::ensure_source_updated()` (around line 1267 in `types.rs`, or in `buffer.rs`). Change:

  ```rust
  // Old:
  pub fn ensure_source_updated(&self) {
      if let Some(module) = &self.cached_source_module {
          module.update();
      }
  }

  // New:
  pub fn ensure_source_updated(&self) {
      if let Some(module) = &self.cached_source_module {
          module.ensure_processed();
      }
  }
  ```

- [ ] **Step 3: Update `BufferWrite::update()` to use `current_block_index()`**

  In `crates/modular_core/src/dsp/utilities/buffer.rs`, find `BufferWrite::update()`. Change the write index calculation from a per-frame increment to using `current_block_index()`:

  ```rust
  fn update(&mut self, _sample_rate: f32) {
      // Write position within the current block
      let offset = self.current_block_index();
      let write_pos = (self.outputs.buffer.write_index + offset) % frame_count;

      // Write input sample to buffer at current block position
      // ... rest of write logic using write_pos instead of a per-call increment ...
  }
  ```

  > The key change: instead of `write_index` advancing by 1 each `update()` call, `write_index` stays fixed during the block and `current_block_index()` provides the offset. `write_index` advances by `block_size` once per callback via `tick_buffers`.

- [ ] **Step 4: Implement `tick_buffers` on `BufferWriteOutputs`**

  In `buffer.rs`, `BufferWriteOutputs` manually implements `OutputStruct`. Add a `tick_buffers` override:

  ```rust
  impl OutputStruct for BufferWriteOutputs {
      // ... existing impls ...

      fn tick_buffers(&mut self, block_size: usize) {
          let buffer = &self.buffer;
          let frame_count = buffer.frame_count();
          // Advance write_index by block_size, wrapping around.
          // SAFETY: Only called from the audio thread's tick() path.
          let new_index = (buffer.write_index() + block_size) % frame_count.max(1);
          buffer.set_write_index(new_index);
      }
  }
  ```

  > This requires `BufferData` to expose `write_index()`, `set_write_index()`, and `frame_count()` accessors. Add them if they don't exist.

- [ ] **Step 5: Update `DelayRead::update()` to use `current_block_index()`**

  In `crates/modular_core/src/dsp/utilities/delay.rs`, find `DelayRead::update()`. The read index calculation should use `current_block_index()`:

  ```rust
  fn update(&mut self, _sample_rate: f32) {
      self.params.buffer.ensure_source_updated();

      let block_idx = self.current_block_index();
      let write_index = self.params.buffer.read_write_index();

      // Read position: write_index + block_idx - delay_samples, wrapping
      let frame_count = ...; // from buffer params
      let delay_samples = ...; // computed from params
      let read_pos = (write_index + block_idx + frame_count - delay_samples) % frame_count;

      // ... read from buffer at read_pos ...
  }
  ```

- [ ] **Step 6: Update `MockBufferOwner` in buffer.rs tests**

  `MockBufferOwner` implements `Sampleable` directly. Update it to implement the new trait methods:

  ```rust
  impl Sampleable for MockBufferOwner {
      fn get_id(&self) -> &str { "mock-buffer" }
      fn tick(&self) {}
      fn update(&self) {}
      fn ensure_processed(&self) {}
      fn get_value_at(&self, _port: &str, _ch: usize, _index: usize) -> f32 { 0.0 }
      fn get_poly_sample(&self, _port: &str) -> napi::Result<PolyOutput> {
          Ok(PolyOutput::default())
      }
      fn get_module_type(&self) -> &str { "mock-buffer" }
      fn connect(&self, _patch: &Patch) {}
      fn as_any(&self) -> &dyn std::any::Any { self }
      fn get_buffer_output(&self, _port: &str) -> Option<&Arc<BufferData>> { None }
  }
  ```

- [ ] **Step 7: Run tests**

  ```bash
  cargo test -p modular_core buffer
  cargo test -p modular_core delay
  ```
  Expected: all tests PASS.

- [ ] **Step 8: Commit**

  ```bash
  git add crates/modular_core/src/dsp/utilities/buffer.rs crates/modular_core/src/dsp/utilities/delay.rs
  git commit -m "feat(dsp): buffer/delay use current_block_index; ensure_source_updated → ensure_processed"
  ```

---

## Task 11: SampleableConstructor + Audio Callback Restructure

Update `SampleableConstructor` type to pass `block_size` and `mode`. Add graph analysis before module construction. Restructure CPAL callback to use block API.

**Files:**
- Modify: `crates/modular_core/src/types.rs`
- Modify: `crates/modular/src/audio.rs`

- [ ] **Step 1: Update `SampleableConstructor` type in `types.rs`**

  At line 2355, change:

  ```rust
  // Old:
  pub type SampleableConstructor =
      Box<dyn Fn(&String, f32, crate::params::DeserializedParams) -> Result<Arc<Box<dyn Sampleable>>>>;

  // New:
  pub type SampleableConstructor = Box<
      dyn Fn(
          &String,
          f32,
          crate::params::DeserializedParams,
          usize,                // block_size
          ProcessingMode,       // mode
      ) -> Result<Arc<Box<dyn Sampleable>>>,
  >;
  ```

- [ ] **Step 2: Run `cargo check -p modular` to see all call sites that break**

  ```bash
  cargo check -p modular 2>&1 | head -60
  ```

- [ ] **Step 3: Update all `constructor(...)` call sites in `audio.rs`**

  In `apply_patch_update` / `build_patch_update` (around line 1104), add graph analysis before the construction loop:

  ```rust
  // Run cycle detection on the desired graph
  let mode_map = crate::classify_modules(&graph);
  let block_size = num_frames; // captured from config at make_stream time

  for (id, module_state) in desired_modules {
      // ...existing deserialization...
      let mode = mode_map.get(id).copied().unwrap_or(ProcessingMode::Block);
      // Force certain modules to always Sample mode:
      let mode = match module_state.module_type.as_str() {
          "__root_clock" | "__hidden_audio_in" | "__root_input" => ProcessingMode::Sample,
          _ => mode,
      };

      if let Some(constructor) = constructors.get(&module_state.module_type) {
          match constructor(&id, sample_rate, deserialized, block_size, mode) {
              // ...
          }
      }
  }
  ```

  > The exact module type strings for ROOT_CLOCK, HiddenAudioIn, and RootInput — check the `#[module(name = "...")]` attribute on each of those structs in `dsp/core/`.

- [ ] **Step 4: Store `block_size` in `AudioProcessor`**

  In the `AudioProcessor` struct definition (around line 1223), add:

  ```rust
  block_size: usize,
  ```

  In `AudioProcessor::new(...)`, initialize from the CPAL callback buffer size (passed in from `make_stream`).

  In `make_stream` (around line 1787), after computing `num_frames`:

  ```rust
  let block_size = num_frames; // Invariant: CPAL delivers fixed buffer size
  let mut audio_processor = AudioProcessor::new(command_rx, error_tx, garbage_tx, shared, sample_rate, block_size);
  ```

- [ ] **Step 5: Restructure the CPAL callback**

  Replace the entire `for frame in output.chunks_mut(num_channels)` block (lines ~1887–1924) with the new block dispatch:

  ```rust
  let num_frames = output.len() / num_channels;

  // === 1. Tick all modules (reset block cursors) ===
  for module in audio_processor.patch.sampleables.values() {
      module.tick();
  }

  // === 2. Pre-fill audio input block from input ring buffer ===
  let mut input_block: Vec<[f32; PORT_MAX_CHANNELS]> =
      Vec::with_capacity(num_frames);
  for _ in 0..num_frames {
      let frame_samples = input_reader.read_frame();
      let mut slot = [0.0f32; PORT_MAX_CHANNELS];
      for (i, &s) in frame_samples.iter().enumerate().take(PORT_MAX_CHANNELS) {
          slot[i] = s * AUDIO_INPUT_GAIN;
      }
      input_block.push(slot);
  }
  // Inject audio_in block into HiddenAudioIn module
  if let Some(audio_in_module) = audio_processor.patch.sampleables.get(&*HIDDEN_AUDIO_IN_ID) {
      audio_in_module.inject_audio_in_block(&input_block);
  }

  // === 3. Pre-compute ExternalClockState for full block, sync ROOT_CLOCK ===
  if !audio_processor.is_stopped() {
      if let Some(ref link_state) = audio_processor.current_link_state {
          if let Some(ref ss) = audio_processor.link_session_state {
              if let Some(root_clock) = audio_processor.patch.sampleables.get(&*ROOT_CLOCK_ID) {
                  let states: Vec<ExternalClockState> = (0..num_frames)
                      .map(|i| {
                          let frame_offset_micros =
                              ((audio_processor.frame_in_buffer + i) as f64
                                  * link_state.micros_per_sample) as i64;
                          let frame_host_time =
                              link_state.host_time_micros + frame_offset_micros;
                          let phase =
                              ss.phase_at_time(frame_host_time, link_state.quantum);
                          ExternalClockState {
                              bar_phase: phase / link_state.quantum,
                              bpm: link_state.tempo,
                              playing: link_state.playing,
                          }
                      })
                      .collect();
                  root_clock.sync_external_clock(&states);
              }
          }
          audio_processor.frame_in_buffer += num_frames as u64;
      }
  }

  // === 4. Ensure ROOT_CLOCK processed (reads trigger outputs for queued update check) ===
  if let Some(root_clock) = audio_processor.patch.sampleables.get(&*ROOT_CLOCK_ID) {
      root_clock.ensure_processed();
  }

  // === 5. Check queued update trigger (use sample 0 of the block) ===
  {
      let should_apply = if let Some((_, trigger)) = audio_processor.queued_update.as_ref() {
          match trigger {
              QueuedTrigger::Immediate => true,
              QueuedTrigger::NextBar => {
                  audio_processor.patch.sampleables.get(&*ROOT_CLOCK_ID)
                      .map(|c| c.get_value_at("barTrigger", 0, 0) >= 1.0)
                      .unwrap_or(true)
              }
              QueuedTrigger::NextBeat => {
                  audio_processor.patch.sampleables.get(&*ROOT_CLOCK_ID)
                      .map(|c| c.get_value_at("beatTrigger", 0, 0) >= 1.0)
                      .unwrap_or(true)
              }
          }
      } else {
          false
      };

      if should_apply {
          let (update, _) = audio_processor.queued_update.take().unwrap();
          let applied_id = update.update_id;
          audio_processor.apply_patch_update(update);
          audio_processor.transport_meter.write_applied_update_id(applied_id);
          // Re-tick and re-sync after patch update
          for module in audio_processor.patch.sampleables.values() {
              module.tick();
          }
          if let Some(root_clock) = audio_processor.patch.sampleables.get(&*ROOT_CLOCK_ID) {
              root_clock.ensure_processed();
          }
      }
  }

  // === 6. Extract audio output: per-frame loop (lightweight — block modules cache) ===
  //    Audio_in for HiddenAudioIn was pre-injected; block-mode modules compute on
  //    first get_value_at call; subsequent calls are cache reads.
  {
      profiling::scope!("process_frames");
      for (i, frame) in output.chunks_mut(num_channels).enumerate() {
          // Pull audio output for frame i
          if let Some(root) = audio_processor.patch.sampleables.get(&*ROOT_ID) {
              for (ch, s) in frame.iter_mut().enumerate() {
                  if ch < num_channels {
                      let v = root.get_value_at(&ROOT_OUTPUT_PORT, ch, i)
                          * AUDIO_OUTPUT_ATTENUATION;
                      *s = T::from_sample(v);
                  }
              }
              // Record first channel if recording
              if let Some(mut writer_guard) = recording_writer.try_lock()
                  && let Some(ref mut writer) = *writer_guard
              {
                  let v = root.get_value_at(&ROOT_OUTPUT_PORT, 0, i) * AUDIO_OUTPUT_ATTENUATION;
                  let _ = writer.write_sample(T::from_sample(v));
              }
          }
      }
  }

  // === 7. Ensure all remaining modules processed ===
  {
      profiling::scope!("ensure_remaining");
      for module in audio_processor.patch.sampleables.values() {
          module.ensure_processed();
      }
  }

  // === 8. Transport meter (read ROOT_CLOCK at last block index) ===
  {
      let last = num_frames.saturating_sub(1);
      let has_queued = audio_processor.queued_update.is_some();
      if let Some(clock) = audio_processor.patch.sampleables.get(&*ROOT_CLOCK_ID) {
          let bar_phase = clock.get_value_at("playhead", 0, last) as f64;
          let bar_count = clock.get_value_at("playhead", 1, last) as u64;
          let beat_in_bar = clock.get_value_at("beatInBar", 0, last) as u32;
          let is_playing = !audio_processor.is_stopped()
              && audio_processor.current_link_state.as_ref().map_or(true, |s| s.playing);
          audio_processor.transport_meter.write_from_audio(
              bar_phase, bar_count, beat_in_bar, is_playing, has_queued,
          );
      }
  }

  // === 9. Scope collection (scan block_size samples for each scope channel) ===
  {
      profiling::scope!("capture_scopes");
      let mut scope_lock = audio_processor.scope_collection.lock();
      for (key, scope_buffer) in scope_lock.iter_mut() {
          if let Some(module) = audio_processor.patch.sampleables.get(&key.module_id) {
              for i in 0..num_frames {
                  let sample = module.get_value_at(&key.port_name, key.channel as usize, i);
                  scope_buffer.push(sample);
              }
          }
      }
  }
  ```

- [ ] **Step 6: Remove `process_frame` and `process_frame_with_processor`**

  These functions (around lines 1601–1750) are replaced by the block dispatch above. Delete them.

- [ ] **Step 7: Compile check**

  ```bash
  cargo check -p modular
  ```
  Expected: no errors.

- [ ] **Step 8: Run unit tests**

  ```bash
  yarn test:unit
  ```
  Expected: all PASS.

- [ ] **Step 9: Run E2E tests**

  > Requires a previous webpack build. Run `yarn start` once if no build exists, then:

  ```bash
  yarn test:e2e
  ```
  Expected: all PASS (audio correctness is regression-tested here).

- [ ] **Step 10: Commit**

  ```bash
  git add crates/modular_core/src/types.rs crates/modular/src/audio.rs
  git commit -m "feat(audio): restructure CPAL callback to block dispatch; wire SampleableConstructor mode/block_size"
  ```

---

## Task 12: Sampleable Trait Cleanup — Remove Old Methods

Remove `update()` (public trait method), `get_poly_sample()`, old `sync_external_clock(bar_phase, bpm, playing)`, and `clear_external_sync()` from the `Sampleable` trait. Update all remaining call sites.

**Files:**
- Modify: `crates/modular_core/src/types.rs`
- Modify: `crates/modular_core/tests/types_tests.rs`
- Modify: `crates/modular_derive/src/module_attr.rs`
- Modify: `crates/modular/src/audio.rs`

- [ ] **Step 1: Remove methods from `Sampleable` trait**

  In `types.rs`, delete from the `Sampleable` trait:

  - `fn update(&self) -> ();`
  - `fn get_poly_sample(&self, port: &str) -> Result<PolyOutput>;`
  - `fn sync_external_clock(&self, _bar_phase: f64, _bpm: f64, _playing: bool) {}`
  - `fn clear_external_sync(&self) {}`

  Replace `sync_external_clock` with the new signature (already default no-op):

  ```rust
  fn sync_external_clock(&self, _states: &[ExternalClockState]) {}
  ```

- [ ] **Step 2: Run `cargo check -p modular_core` to find all `update()` call sites**

  ```bash
  cargo check -p modular_core 2>&1 | grep "update"
  ```

- [ ] **Step 3: Update `DummySampleable` in `types_tests.rs`**

  Remove `update()` and `get_poly_sample()` from its impl. The trait no longer requires them.

- [ ] **Step 4: Remove deprecated backward-compat methods from generated wrapper**

  In `module_attr.rs`, remove the `update()` and `get_poly_sample()` from the generated `Sampleable` impl (added in Task 8 Step 15 as backward compat).

- [ ] **Step 5: Run `cargo check -p modular` to find remaining call sites**

  ```bash
  cargo check -p modular 2>&1 | head -40
  ```

  Fix any remaining `module.update()` or `module.get_poly_sample()` calls in `audio.rs` — replace with `module.ensure_processed()` and `module.get_value_at(...)` respectively.

- [ ] **Step 6: Final full test run**

  ```bash
  cargo test -p modular_core && cargo test -p modular && yarn test:unit
  ```
  Expected: all PASS.

- [ ] **Step 7: Run E2E tests**

  ```bash
  yarn test:e2e
  ```
  Expected: all PASS.

- [ ] **Step 8: Commit**

  ```bash
  git add crates/modular_core/src/types.rs crates/modular_core/tests/types_tests.rs \
          crates/modular_derive/src/module_attr.rs crates/modular/src/audio.rs
  git commit -m "feat(core): remove deprecated update/get_poly_sample from Sampleable trait"
  ```

---

## Post-Implementation Self-Review Checklist

- [ ] **BlockPort**: `data[i][ch]` layout correct; `new()` never called on audio thread.
- [ ] **ProcessingMode**: `Default` is `Block`; ROOT_CLOCK/HiddenAudioIn/RootInput always forced to `Sample`.
- [ ] **Signal::index_ptr**: Null during construction/transport; written only on audio thread during `connect()`; `unsafe impl Send` justified.
- [ ] **Graph analysis**: Tarjan's SCC reads cable deps from `ModuleState.params` JSON (`{type:"cable", module:"...", port:"..."}`). Self-loop and multi-node SCCs both → `Sample`.
- [ ] **BlockOutputs**: `{Name}BlockOutputs::new(block_size)` allocates; `copy_from_inner` for f32 fields writes to `data[slot][0]`; for `PolyOutput` fields writes all `PORT_MAX_CHANNELS` channels.
- [ ] **ensure_processed**: In `Block` mode, target = `block_size`; in `Sample` mode, target = `index + 1`. Loop guard checks `index < target`. `computing.set(false)` after loop.
- [ ] **get_value_at reentrancy**: Returns `data[index-1 wrapping]` when `computing == true`. No panic on `index == 0` (uses `saturating_sub`).
- [ ] **tick()**: Resets `index` to 0. Calls `tick_buffers(block_size)` on inner `outputs` (for `$buffer`'s `write_index` advance). Does NOT clear `block_outputs` data (retains previous block for feedback wraparound).
- [ ] **transfer_state_from**: Swaps both inner `module.outputs` AND `block_outputs`. Self-aliasing guard via `ptr::eq`.
- [ ] **Audio callback**: `tick()` all → pre-fill audio_in block → inject to HiddenAudioIn → pre-compute clock states → sync ROOT_CLOCK → ensure ROOT_CLOCK → trigger check → per-frame output loop → ensure remaining → transport → scope.
- [ ] **Buffer/Delay**: `BufferWrite` writes at `(write_index + current_block_index()) % frame_count`. `BufferWriteOutputs::tick_buffers` advances `write_index += block_size`. `DelayRead` reads at `(write_index + current_block_index() - delay_samples + frame_count) % frame_count`.
- [ ] **Tests**: `cargo test -p modular_core`, `cargo test -p modular`, `yarn test:unit`, `yarn test:e2e` all PASS.

---

## Known Limitations / Future Work

- **HiddenAudioIn audio_in injection**: The `inject_audio_in_block` mechanism on `Sampleable` is a new default-no-op trait method. The HiddenAudioIn wrapper's implementation must fill `patch.audio_in` from the pre-built input block before each inner `update()`. Full implementation requires the `audio_input` module_attr flag (similar to `clock_sync`) and a stored `Arc<Mutex<PolyOutput>>` from `connect()`. If deferred, HiddenAudioIn remains in `Sample` mode reading the mutex directly (set once per frame in the output loop), with at most one block of stale audio input for block-mode downstream modules.
- **No SIMD**: `BlockPort` layout (`data[i][ch]` contiguous per sample) is SIMD-ready but SIMD is not implemented.
- **No topo ordering**: Pull-driven lazy evaluation (ensure_processed cascade) handles ordering. Topo sort is deferred.
- **Buffer module complexity**: Manual `OutputStruct` impl for `BufferWriteOutputs` means manual `BlockOutputs` too. The derive macro doesn't cover this; implement manually in `buffer.rs`.
