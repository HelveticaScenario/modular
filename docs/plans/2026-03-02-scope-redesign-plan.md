# Scope Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor scope to capture per-channel data with collection grouping, replacing the current port-level capture model.

**Architecture:** A scope targets a `Vec<ScopeChannel>` (each being `(module_id, port_name, channel)`). Audio thread uses per-channel `ScopeBuffer`s keyed by `ScopeBufferKey` (channel identity + collection config). Frontend groups per-channel data into scope canvases.

**Tech Stack:** Rust (napi-rs, audio thread), TypeScript (Electron renderer, DSL)

---

### Task 1: Update Rust types — ScopeChannel and Scope

**Files:**

- Modify: `crates/modular_core/src/types.rs:959-1013`

**Step 1: Add Hash derive to ScopeMode**

In `crates/modular_core/src/types.rs`, find:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[napi(string_enum)]
pub enum ScopeMode {
```

Add `Hash`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[napi(string_enum)]
pub enum ScopeMode {
```

**Step 2: Replace ScopeItem with ScopeChannel**

Replace the `ScopeItem` enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[napi]
pub enum ScopeItem {
    ModuleOutput {
        module_id: String,
        port_name: String,
    },
}
```

With the new `ScopeChannel` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct ScopeChannel {
    pub module_id: String,
    pub port_name: String,
    pub channel: u32,
}
```

**Step 3: Add ScopeBufferKey**

Add a new struct after `ScopeChannel`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct ScopeBufferKey {
    pub module_id: String,
    pub port_name: String,
    pub channel: u32,
    pub ms_per_frame: u32,
    pub trigger_threshold: Option<(i32, ScopeMode)>,
}
```

**Step 4: Update Scope struct**

Replace `item: ScopeItem` with `channels: Vec<ScopeChannel>`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct Scope {
    pub channels: Vec<ScopeChannel>,
    pub ms_per_frame: u32,
    pub trigger_threshold: Option<(i32, ScopeMode)>,
    #[serde(default = "default_scope_range")]
    pub range: (f64, f64),
}
```

**Step 5: Update ScopeStats**

Change `read_offset` from `Vec<u32>` to a single `u32` since buffers are now single-channel:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[napi(object)]
pub struct ScopeStats {
    pub min: f64,
    pub max: f64,
    pub peak_to_peak: f64,
    pub read_offset: u32,
}
```

**Step 6: Verify compilation fails expectedly**

Run: `cargo check -p modular_core 2>&1 | head -30`
Expected: Compiles (types.rs is self-contained changes). Downstream crates will fail.

**Step 7: Commit**

```
git add crates/modular_core/src/types.rs
git commit -m "refactor(types): replace ScopeItem with ScopeChannel, add ScopeBufferKey"
```

---

### Task 2: Update Rust commands

**Files:**

- Modify: `crates/modular/src/commands.rs`

**Step 1: Update imports**

Change:

```rust
use modular_core::types::{Message, ModuleIdRemap, Sampleable, Scope, ScopeItem};
```

To:

```rust
use modular_core::types::{Message, ModuleIdRemap, Sampleable, ScopeBufferKey};
```

(Remove `Scope` and `ScopeItem` — no longer needed here.)

**Step 2: Update PatchUpdate fields**

Replace the three scope fields:

```rust
  pub scope_adds: Vec<(ScopeItem, ScopeBuffer)>,
  pub scope_removes: Vec<ScopeItem>,
  pub scope_updates: Vec<Scope>,
```

With just two (no more updates — key includes config, so config change = remove + add):

```rust
  pub scope_adds: Vec<(ScopeBufferKey, ScopeBuffer)>,
  pub scope_removes: Vec<ScopeBufferKey>,
```

**Step 3: Update PatchUpdate::new initializer**

Remove `scope_updates: Vec::new(),` — only `scope_adds` and `scope_removes` remain.

**Step 4: Update is_empty check**

Remove `&& self.scope_updates.is_empty()`.

**Step 5: Verify**

Run: `cargo check -p modular 2>&1 | head -50`
Expected: Fails on audio.rs (next task), but commands.rs itself should be clean.

**Step 6: Commit**

```
git add crates/modular/src/commands.rs
git commit -m "refactor(commands): update PatchUpdate for ScopeBufferKey, remove scope_updates"
```

---

### Task 3: Rewrite ScopeBuffer as single-channel

**Files:**

- Modify: `crates/modular/src/audio.rs:632-836`

**Step 1: Rewrite ScopeBuffer struct**

Replace the entire `ScopeBuffer` struct and its `impl` block with:

```rust
pub struct ScopeBuffer {
    sample_counter: u32,
    skip_rate: u32,
    trigger_threshold: Option<(f32, ScopeMode)>,
    trigger: SchmittTrigger,
    buffer: [[f32; SCOPE_CAPACITY as usize]; 2],
    buffer_select: bool,
    recording: bool,
    buffer_idx: usize,
    read_idx: usize,
}

impl ScopeBuffer {
    pub fn new(ms_per_frame: u32, trigger_threshold: Option<(i32, ScopeMode)>, sample_rate: f32) -> Self {
        let trigger_f = trigger_threshold.map(|(t, mode)| ((t as f32) / 1000.0, mode));
        let thresh_val = trigger_f.map(|(t, _)| t).unwrap_or(0.0);
        Self {
            buffer: [[0.0; SCOPE_CAPACITY as usize]; 2],
            sample_counter: 0,
            skip_rate: calculate_skip_rate(ms_to_samples(ms_per_frame, sample_rate)),
            trigger_threshold: trigger_f,
            trigger: SchmittTrigger::new(thresh_val, thresh_val + 0.001),
            buffer_select: false,
            recording: false,
            buffer_idx: 0,
            read_idx: 0,
        }
    }

    pub fn push(&mut self, value: f32) {
        if self.trigger_threshold.is_none() {
            self.trigger.reset();
            self.recording = true;
            self.read_idx = self.buffer_idx;
        } else if self.trigger.process(value) && !self.recording {
            self.trigger.reset();
            self.recording = true;
            self.buffer_idx = 0;
            self.read_idx = 0;
            self.sample_counter = 0;
        }

        self.buffer_idx %= SCOPE_CAPACITY as usize;
        self.read_idx %= SCOPE_CAPACITY as usize;

        let write_buf = if self.buffer_select { 1 } else { 0 };

        if self.recording {
            if self.sample_counter == 0 {
                self.buffer[write_buf][self.buffer_idx] = value;
                self.buffer_idx += 1;
                if self.buffer_idx >= SCOPE_CAPACITY as usize {
                    match self.trigger_threshold {
                        Some((_, ScopeMode::Wait)) => {
                            self.recording = false;
                            self.buffer_select = !self.buffer_select;
                        }
                        Some((_, ScopeMode::Roll)) => {
                            self.recording = false;
                        }
                        None => { /* keep recording continuously */ }
                    }
                }
            }
            self.sample_counter += 1;
            if self.sample_counter > self.skip_rate {
                self.sample_counter = 0;
            }
        }
    }

    fn read_buffer_idx(&self) -> usize {
        let write_buf = if self.buffer_select { 1 } else { 0 };
        let other_buf = if write_buf == 0 { 1 } else { 0 };
        match self.trigger_threshold {
            Some((_, ScopeMode::Wait)) => other_buf,
            Some((_, ScopeMode::Roll)) => write_buf,
            None => write_buf,
        }
    }

    pub fn get_buffer(&self) -> Float32Array {
        Float32Array::new(self.buffer[self.read_buffer_idx()].to_vec())
    }

    pub fn compute_stats(&self) -> ScopeStats {
        let buf = &self.buffer[self.read_buffer_idx()];
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &val in buf.iter() {
            if val < min { min = val; }
            if val > max { max = val; }
        }
        if min == f32::MAX { min = 0.0; }
        if max == f32::MIN { max = 0.0; }
        ScopeStats {
            min: min as f64,
            max: max as f64,
            peak_to_peak: (max - min) as f64,
            read_offset: self.read_idx as u32,
        }
    }
}
```

**Step 2: Verify ScopeBuffer compiles**

Run: `cargo check -p modular 2>&1 | head -50`
Expected: ScopeBuffer compiles. Other code in audio.rs still has errors (next task).

**Step 3: Commit**

```
git add crates/modular/src/audio.rs
git commit -m "refactor(audio): rewrite ScopeBuffer as single-channel buffer"
```

---

### Task 4: Update audio.rs scope management

**Files:**

- Modify: `crates/modular/src/audio.rs` (scope_collection, apply_patch, process_frame, get_audio_buffers)

**Step 1: Update imports**

Find all imports of `ScopeItem` and replace with `ScopeBufferKey`, `ScopeChannel`. Add imports as needed:

```rust
use modular_core::types::{ScopeBufferKey, ScopeChannel, ScopeStats, ...};
```

**Step 2: Update scope_collection type**

Change:

```rust
scope_collection: Arc<Mutex<HashMap<ScopeItem, ScopeBuffer>>>,
```

To:

```rust
scope_collection: Arc<Mutex<HashMap<ScopeBufferKey, ScopeBuffer>>>,
```

And the initializer:

```rust
scope_collection: Arc::new(Mutex::new(HashMap::new())),
```

(Same, but now the type parameter changes.)

**Step 3: Update apply_patch scope diff**

Replace the scope diff block (~lines 1103-1130) with:

```rust
    // Compute scopes to add/remove (no updates — key includes config)
    {
        let scope_collection = self.scope_collection.lock();
        let current_keys: HashSet<ScopeBufferKey> = scope_collection.keys().cloned().collect();

        // Expand desired scopes into per-channel buffer keys
        let desired_keys: HashSet<ScopeBufferKey> = scopes.iter()
            .flat_map(|scope| {
                scope.channels.iter().map(move |ch| ScopeBufferKey {
                    module_id: ch.module_id.clone(),
                    port_name: ch.port_name.clone(),
                    channel: ch.channel,
                    ms_per_frame: scope.ms_per_frame,
                    trigger_threshold: scope.trigger_threshold,
                })
            })
            .collect();

        // Scopes to remove
        update.scope_removes = current_keys
            .difference(&desired_keys)
            .cloned()
            .collect();

        // Scopes to add (pre-build ScopeBuffers on main thread)
        update.scope_adds = desired_keys
            .difference(&current_keys)
            .map(|key| {
                let buffer = ScopeBuffer::new(key.ms_per_frame, key.trigger_threshold, sample_rate);
                (key.clone(), buffer)
            })
            .collect();
    }
```

**Step 4: Update apply_patch_update scope section**

Replace (~lines 1457-1479) with:

```rust
    // Update scopes
    {
        let mut scope_collection = self.scope_collection.lock();

        // Remove scopes
        for key in &update.scope_removes {
            if let Some(buffer) = scope_collection.remove(key) {
                let _ = self.garbage_tx.push(GarbageItem::Scope(buffer));
            }
        }

        // Add new scopes
        for (key, buffer) in update.scope_adds {
            scope_collection.insert(key, buffer);
        }
    }
```

(No more update step.)

**Step 5: Update process_frame scope capture**

Replace (~lines 1582-1605) with:

```rust
    // Capture audio for scopes
    {
        profiling::scope!("capture_scopes");
        let mut scope_lock = self.scope_collection.lock();
        for (key, scope_buffer) in scope_lock.iter_mut() {
            if let Some(module) = self.patch.sampleables.get(&key.module_id)
                && let Ok(poly) = module.get_poly_sample(&key.port_name)
            {
                let sample = if (key.channel as usize) < poly.channels() {
                    poly.get(key.channel as usize)
                } else {
                    0.0
                };
                scope_buffer.push(sample);
            }
        }
    }
```

**Step 6: Update get_audio_buffers return type**

Change:

```rust
pub fn get_audio_buffers(&self) -> Vec<(ScopeItem, Vec<Float32Array>, ScopeStats)> {
```

To:

```rust
pub fn get_audio_buffers(&self) -> Vec<(ScopeBufferKey, Float32Array, ScopeStats)> {
```

And update the body:

```rust
pub fn get_audio_buffers(&self) -> Vec<(ScopeBufferKey, Float32Array, ScopeStats)> {
    if self.is_stopped() {
        return Vec::new();
    }
    let scope_collection = match self.scope_collection.try_lock() {
        Some(sc) => sc,
        None => return Vec::new(),
    };
    scope_collection
        .iter()
        .map(|(key, buffer)| {
            let data = buffer.get_buffer();
            let stats = buffer.compute_stats();
            (key.clone(), data, stats)
        })
        .collect()
}
```

**Step 7: Verify compilation**

Run: `cargo check -p modular 2>&1 | head -50`
Expected: audio.rs compiles. Errors remain in validation.rs and lib.rs.

**Step 8: Commit**

```
git add crates/modular/src/audio.rs
git commit -m "refactor(audio): update scope collection to use ScopeBufferKey, per-channel capture"
```

---

### Task 5: Update validation

**Files:**

- Modify: `crates/modular/src/validation.rs:533-585`

**Step 1: Update scope validation**

Replace the scope validation block with:

```rust
    // === Scope validation ===
    for scope in &patch.scopes {
        if scope.channels.is_empty() {
            errors.push(ValidationError {
                field: "scopes".to_string(),
                message: "Scope has no channels".to_string(),
                location: None,
                expected_type: None,
                actual_value: None,
            });
            continue;
        }

        for channel in &scope.channels {
            // Scope target module must exist
            let Some(module) = module_by_id.get(channel.module_id.as_str()).copied() else {
                errors.push(ValidationError {
                    field: "scopes".to_string(),
                    message: format!("Scope references missing module '{}'", channel.module_id),
                    location: None,
                    expected_type: None,
                    actual_value: None,
                });
                continue;
            };

            // Target module type must be known
            let Some(schema) = schema_map.get(module.module_type.as_str()).copied() else {
                errors.push(ValidationError {
                    field: "scopes".to_string(),
                    message: format!(
                        "Scope references module '{}' with unknown type '{}'",
                        channel.module_id, module.module_type
                    ),
                    location: None,
                    expected_type: None,
                    actual_value: None,
                });
                continue;
            };

            // Output port must exist in module schema
            if !schema.outputs.iter().any(|o| o.name == *channel.port_name) {
                errors.push(ValidationError {
                    field: "scopes".to_string(),
                    message: format!(
                        "Scope references missing output port '{}' on module '{}'",
                        channel.port_name, channel.module_id
                    ),
                    location: None,
                    expected_type: None,
                    actual_value: None,
                });
            }
        }
    }
```

**Step 2: Update imports if needed**

Remove `ScopeItem` from imports if present.

**Step 3: Verify**

Run: `cargo check -p modular 2>&1 | head -30`
Expected: validation.rs compiles. lib.rs may still have errors.

**Step 4: Commit**

```
git add crates/modular/src/validation.rs
git commit -m "refactor(validation): validate each ScopeChannel individually"
```

---

### Task 6: Update NAPI binding

**Files:**

- Modify: `crates/modular/src/lib.rs:586-589`

**Step 1: Update get_scopes return type**

Change:

```rust
#[napi]
pub fn get_scopes(&self) -> Vec<(ScopeItem, Vec<Float32Array>, ScopeStats)> {
    self.state.get_audio_buffers()
}
```

To:

```rust
#[napi]
pub fn get_scopes(&self) -> Vec<(ScopeBufferKey, Float32Array, ScopeStats)> {
    self.state.get_audio_buffers()
}
```

**Step 2: Update imports**

Replace `ScopeItem` with `ScopeBufferKey` in the imports.

**Step 3: Build to regenerate index.d.ts**

Run: `cargo build -p modular 2>&1 | tail -20`
Expected: Full build succeeds. `index.d.ts` is auto-regenerated.

**Step 4: Verify generated types**

Check that `crates/modular/index.d.ts` now has:

- `ScopeChannel` with `moduleId`, `portName`, `channel`
- `ScopeBufferKey` with `moduleId`, `portName`, `channel`, `msPerFrame`, `triggerThreshold`
- `Scope` with `channels: Array<ScopeChannel>` instead of `item: ScopeItem`
- `ScopeStats` with `readOffset: number` (not `Array<number>`)
- `getScopes()` returns `Array<[ScopeBufferKey, Float32Array, ScopeStats]>`
- `ScopeItem` type is gone

**Step 5: Commit**

```
git add crates/modular/src/lib.rs crates/modular/index.d.ts
git commit -m "refactor(napi): update getScopes to return per-channel ScopeBufferKey data"
```

---

### Task 7: Update TypeScript DSL

**Files:**

- Modify: `src/main/dsl/GraphBuilder.ts`
- Modify: `src/main/dsl/factories.ts`

**Step 1: Update ScopeWithLocation type**

In `GraphBuilder.ts`, update the `ScopeWithLocation` type. Since `Scope` now has `channels` instead of `item`, this type extends correctly:

```typescript
export type ScopeWithLocation = Scope & {
    sourceLocation?: { line: number; column: number };
};
```

No change needed — it extends `Scope` which now has `channels`.

**Step 2: Update addScope()**

Replace the `addScope` method (around lines 796-836) with:

```typescript
    addScope(
        value: ModuleOutput | ModuleOutput[],
        config: {
            msPerFrame?: number;
            triggerThreshold?: number;
            triggerWaitToRender?: boolean;
            range?: [number, number];
        } = {},
        sourceLocation?: { line: number; column: number },
    ) {
        const { msPerFrame = 500, triggerThreshold, range = [-5, 5] } = config;
        const realTriggerThreshold: number | undefined =
            triggerThreshold !== undefined
                ? triggerThreshold * 1000
                : undefined;
        const triggerWaitToRender = config.triggerWaitToRender ?? true;
        let thresh: [number, ScopeMode] | undefined = undefined;
        if (realTriggerThreshold !== undefined) {
            thresh = [
                realTriggerThreshold,
                triggerWaitToRender ? 'Wait' : 'Roll',
            ];
        }

        const outputs = Array.isArray(value) ? value : [value];
        const channels = outputs.map((o) => ({
            moduleId: o.moduleId,
            portName: o.portName,
            channel: o.channel,
        }));

        this.scopes.push({
            channels,
            msPerFrame,
            triggerThreshold: thresh,
            range,
            sourceLocation,
        });
    }
```

**Step 3: Update BaseCollection.scope()**

Change (around lines 170-184) to pass all items:

```typescript
    scope(config?: {
        msPerFrame?: number;
        triggerThreshold?: number;
        triggerWaitToRender?: boolean;
        range?: [number, number];
    }): this {
        if (this.items.length > 0) {
            const loc = captureSourceLocation();
            this.items[0].builder.addScope(this.items, config, loc);
        }
        return this;
    }
```

(Only change: pass `this.items` instead of `this.items[0]`.)

**Step 4: Update toPatch() scope serialization**

Replace the scope mapping in `toPatch()` (around lines 681-707). The deferred output resolution now needs to handle each channel:

```typescript
            scopes: this.scopes
                .map((scope) => {
                    const resolvedChannels = scope.channels.map((ch) => {
                        const deferredOutput = this.deferredOutputs.get(
                            ch.moduleId,
                        );
                        if (deferredOutput) {
                            const resolved = deferredOutput.resolve();
                            if (resolved) {
                                return {
                                    moduleId: resolved.moduleId,
                                    portName: resolved.portName,
                                    channel: ch.channel,
                                };
                            }
                            return null;
                        }
                        return ch;
                    });
                    // If any channel couldn't be resolved, skip the scope
                    if (resolvedChannels.some((ch) => ch === null)) {
                        return null;
                    }
                    return {
                        ...scope,
                        channels: resolvedChannels,
                    } as ScopeWithLocation;
                })
                .filter(
                    (s: ScopeWithLocation | null): s is ScopeWithLocation =>
                        s !== null,
                ),
```

**Step 5: Update factories.ts DSLContext.scope()**

The `scope` method in `factories.ts` (around lines 340-358) spreads collections and delegates to `addScope`. No change needed — `addScope` now handles arrays correctly.

However, check the `config` type: it has `scale` instead of `range`. Verify and align if needed. The current signature is fine as-is; addScope handles the mapping.

**Step 6: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | head -30`
Expected: DSL layer compiles. Renderer still has errors (next task).

**Step 7: Commit**

```
git add src/main/dsl/GraphBuilder.ts src/main/dsl/factories.ts
git commit -m "refactor(dsl): addScope collects all channels, Collection.scope passes all items"
```

---

### Task 8: Update renderer

**Files:**

- Modify: `src/renderer/app/oscilloscope.ts`
- Modify: `src/renderer/types/editor.ts`
- Modify: `src/renderer/App.tsx`

**Step 1: Update oscilloscope.ts key derivation**

Replace `scopeKeyFromSubscription`:

```typescript
import type {
    ScopeBufferKey,
    ScopeChannel,
    ScopeMode,
    ScopeStats,
} from '@modular/core';

export const scopeBufferKeyToString = (key: ScopeBufferKey): string => {
    const trigger = key.triggerThreshold
        ? `${key.triggerThreshold[0]}:${key.triggerThreshold[1]}`
        : 'none';
    return `:module:${key.moduleId}:${key.portName}:${key.channel}:${key.msPerFrame}:${trigger}`;
};

export const scopeBufferKeyFromChannel = (
    channel: ScopeChannel,
    msPerFrame: number,
    triggerThreshold?: [number, ScopeMode],
): string => {
    const trigger = triggerThreshold
        ? `${triggerThreshold[0]}:${triggerThreshold[1]}`
        : 'none';
    return `:module:${channel.moduleId}:${channel.portName}:${channel.channel}:${msPerFrame}:${trigger}`;
};
```

Keep `drawOscilloscope` unchanged — it already takes `channels: Float32Array[]` and draws multiple traces.

**Step 2: Update ScopeView type**

In `src/renderer/types/editor.ts`, add channel keys to `ScopeView`:

```typescript
export type ScopeView = {
    key: string;
    file: string;
    range: [number, number];
    channelKeys: string[]; // buffer keys for this scope's channels
};
```

**Step 3: Update App.tsx — ScopeView construction**

In the patch result handler (around lines 489-534), update scope view construction:

```typescript
const scopes = result.appliedPatch?.scopes || [];
// ...
const views: ScopeView[] = [];
// ...

for (let i = 0; i < scopes.length; i++) {
    const scope = scopes[i];

    // Derive buffer keys for each channel in this scope
    const channelKeys = scope.channels.map((ch: any) =>
        scopeBufferKeyFromChannel(ch, scope.msPerFrame, scope.triggerThreshold),
    );

    // Use first channel's key as the scope's identity
    const scopeKey =
        channelKeys.length > 0
            ? `scope:${i}:${channelKeys.join('+')}`
            : `scope:${i}:empty`;

    const loc = (scope as any).sourceLocation as
        | { line: number; column: number }
        | undefined;

    views.push({
        key: scopeKey,
        file: activeBufferId,
        range: scope.range ?? [-5, 5],
        channelKeys,
    });

    // ... decoration logic stays the same, but use `loc` ...
}
```

**Step 4: Update App.tsx — requestAnimationFrame loop**

In the render loop (around lines 349-410), update to group per-channel data into scope canvases:

```typescript
                    .then(([scopeData, transport]) => {
                        // Build a map of buffer key → (Float32Array, ScopeStats)
                        const bufferMap = new Map<string, { data: Float32Array; stats: ScopeStats }>();
                        for (const [bufferKey, data, stats] of scopeData) {
                            const key = scopeBufferKeyToString(bufferKey);
                            bufferMap.set(key, { data, stats });
                        }

                        // For each scope canvas, collect its channels' data and draw
                        for (const [scopeKey, canvas] of scopeCanvasMapRef.current.entries()) {
                            const rangeMin = parseFloat(canvas.dataset.scopeRangeMin || '-5');
                            const rangeMax = parseFloat(canvas.dataset.scopeRangeMax || '5');
                            const channelKeysStr = canvas.dataset.scopeChannelKeys;
                            if (!channelKeysStr) continue;

                            const channelKeys = JSON.parse(channelKeysStr) as string[];
                            const channels: Float32Array[] = [];
                            const readOffsets: number[] = [];
                            let globalMin = Infinity;
                            let globalMax = -Infinity;

                            for (const chKey of channelKeys) {
                                const entry = bufferMap.get(chKey);
                                if (entry) {
                                    channels.push(entry.data);
                                    readOffsets.push(entry.stats.readOffset);
                                    if (entry.stats.min < globalMin) globalMin = entry.stats.min;
                                    if (entry.stats.max > globalMax) globalMax = entry.stats.max;
                                }
                            }

                            if (channels.length > 0) {
                                drawOscilloscope(channels, canvas, {
                                    range: [rangeMin, rangeMax],
                                    stats: {
                                        min: globalMin,
                                        max: globalMax,
                                        peakToPeak: globalMax - globalMin,
                                        readOffset: readOffsets,
                                    },
                                });
                            }
                        }

                        setTransportState(transport);
                        // ... pending UI state check ...
```

**Step 5: Update scopeViewZones to pass channelKeys to canvas**

In `src/renderer/components/monaco/scopeViewZones.ts`, when creating the canvas element for a scope view, store the `channelKeys` as a data attribute:

```typescript
canvas.dataset.scopeChannelKeys = JSON.stringify(scopeView.channelKeys);
```

Find where `scopeRangeMin` and `scopeRangeMax` are set on the canvas and add this line nearby.

**Step 6: Update drawOscilloscope stats type**

The `ScopeStats` type changed (`readOffset` is now `number` from Rust). But we're constructing a combined stats object in the renderer with `readOffset: number[]` (one per channel in the scope). Update the `ScopeDrawOptions` interface:

```typescript
export interface ScopeDrawOptions {
    range: [number, number];
    stats: {
        min: number;
        max: number;
        peakToPeak: number;
        readOffset: number[];
    };
}
```

This decouples the draw options from the Rust `ScopeStats` type (which is now per-channel).

**Step 7: Verify**

Run: `npx tsc --noEmit 2>&1 | head -30`
Expected: Full compilation succeeds.

**Step 8: Commit**

```
git add src/renderer/
git commit -m "refactor(renderer): group per-channel scope data into scope canvases"
```

---

### Task 9: Update tests

**Files:**

- Modify: `src/main/dsl/__tests__/executor.test.ts`
- Modify: `crates/modular/__test__/napi.test.ts`

**Step 1: Update executor tests**

In `executor.test.ts` (~lines 306-320), update assertions:

```typescript
test('.scope() adds a scope entry', () => {
    const patch = execPatch('$sine("C4").scope().out()');
    expect(findModules(patch, '$sine').length).toBe(1);
    expect(patch.scopes.length).toBeGreaterThan(0);
    expect(patch.scopes[0].channels).toBeDefined();
    expect(patch.scopes[0].channels.length).toBe(1);
    expect(patch.scopes[0].channels[0].channel).toBe(0);
});

test('.scope() with config', () => {
    const patch = execPatch(
        '$sine("C4").scope({ msPerFrame: 100, range: [-10, 10] }).out()',
    );
    expect(patch.scopes.length).toBeGreaterThan(0);
    const scope = patch.scopes[0];
    expect(scope.msPerFrame).toBe(100);
    expect(scope.range).toEqual([-10, 10]);
    expect(scope.channels.length).toBe(1);
});
```

**Step 2: Add new test for collection scope**

```typescript
test('.scope() on collection captures all channels', () => {
    const patch = execPatch('$sine(["C4", "E4"]).scope().out()');
    expect(patch.scopes.length).toBe(1);
    expect(patch.scopes[0].channels.length).toBe(2);
    expect(patch.scopes[0].channels[0].channel).toBe(0);
    expect(patch.scopes[0].channels[1].channel).toBe(1);
});
```

**Step 3: Add test for single channel scope from collection index**

```typescript
test('.scope() on indexed output captures single channel', () => {
    const patch = execPatch('$sine(["C4", "E4"])[1].scope().out()');
    expect(patch.scopes.length).toBe(1);
    expect(patch.scopes[0].channels.length).toBe(1);
    expect(patch.scopes[0].channels[0].channel).toBe(1);
});
```

**Step 4: Update NAPI test**

In `crates/modular/__test__/napi.test.ts` (~lines 183-200), update the scope test to use `channels` instead of `item`:

Find references to `scope.item` or `ScopeItem` and update to `scope.channels`. The test for referencing a non-existent module should still work — the validation error message is the same.

**Step 5: Run tests**

Run: `npm test 2>&1 | tail -30`
Expected: All tests pass.

**Step 6: Commit**

```
git add src/main/dsl/__tests__/executor.test.ts crates/modular/__test__/napi.test.ts
git commit -m "test: update scope tests for per-channel model, add collection/index tests"
```

---

### Task 10: Clean up — remove ScopeItem references

**Files:**

- Check all files for remaining `ScopeItem` references

**Step 1: Search for remaining ScopeItem references**

Run: `rg "ScopeItem" --type rust --type ts`
Expected: No references remain (or only in generated code that auto-regenerates).

**Step 2: Update typescriptLibGen.ts if needed**

Check `src/main/dsl/typescriptLibGen.ts` for any references to `ScopeItem` in the generated type definitions. Update to use `ScopeChannel` if present.

**Step 3: Update IPC types if needed**

Check `src/shared/ipcTypes.ts` and `src/preload/preload.ts` for any explicit `ScopeItem` references.

**Step 4: Final build verification**

Run: `npm run build 2>&1 | tail -20`
Expected: Full build succeeds.

**Step 5: Run E2E tests**

Run: `npm run test:e2e 2>&1 | tail -30`
Expected: E2E tests pass.

**Step 6: Commit**

```
git add -A
git commit -m "chore: clean up remaining ScopeItem references"
```
