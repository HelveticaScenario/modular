# Scope Redesign: Per-Channel Capture with Collection Grouping

## Problem

Currently a scope targets a module port (`ScopeItem::ModuleOutput { module_id, port_name }`) and captures all polyphonic channels of that port. This means:

- `$sine(['a','b'])[0].scope()` captures both channels (channel index is ignored)
- `Collection.scope()` only scopes the first item, discarding the rest
- No way to scope a collection of outputs from different modules as a grouped visualization

## Goal

A scope works on individual `ModuleOutput`s and `Collection`s. Each channel is recorded individually. Collections render as a group (multiple traces on one canvas).

### User-facing examples

```js
// Single channel: 1 trace
$sine(['a', 'b'])[0].scope();

// Full polyphonic output: 2 traces
$sine(['a', 'b']).scope();

// Collection of outputs: 4 traces
$c($sine(['a', 'b']), $sine(['d', 'e'])).scope();

// Mixed collection: 3 traces
$c($sine(['a', 'b']), $saw('c')[0]).scope();
```

## Design

### Unified model

A scope always targets a list of `(moduleId, portName, channel)` tuples. A single-channel scope is a list of one. A polyphonic output scope expands to one tuple per channel. A collection scope flattens all channels from all outputs.

### Data model (Rust)

Replace `ScopeItem` enum with `ScopeChannel` struct. `Scope` holds a `Vec<ScopeChannel>`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ScopeChannel {
    pub module_id: String,
    pub port_name: String,
    pub channel: u32,
}

pub struct Scope {
    pub channels: Vec<ScopeChannel>,
    pub ms_per_frame: u32,
    pub trigger_threshold: Option<(i32, ScopeMode)>,
    pub range: (f64, f64),
}
```

### ScopeBuffer keying and deduplication

The same channel can appear in multiple scopes with different configs. Config params that affect data collection (ms_per_frame, trigger_threshold, trigger_mode) are part of the buffer key. Display-only params (range) are not.

**ScopeBufferKey** = `(module_id, port_name, channel, ms_per_frame, trigger_threshold, trigger_mode)`

- Same channel + same collection config = one recording, shared across scopes (deduplicated)
- Same channel + different collection config = separate recordings

Audio thread scope collection: `HashMap<ScopeBufferKey, ScopeBuffer>` where `ScopeBuffer` is now a single-channel buffer (one ping-pong pair of 1024 samples).

### DSL layer (TypeScript)

**`GraphBuilder.addScope()`**: Collects all `ModuleOutput`s (not just the first), extracts `moduleId`, `portName`, and `channel` from each:

```typescript
addScope(value: ModuleOutput | ModuleOutput[], config, sourceLocation) {
    const outputs = Array.isArray(value) ? value : [value];
    const channels = outputs.map(o => ({
        moduleId: o.moduleId,
        portName: o.portName,
        channel: o.channel,
    }));
    this.scopes.push({ channels, msPerFrame, triggerThreshold, range, sourceLocation });
}
```

**`BaseCollection.scope()`**: Passes all items instead of just `items[0]`:

```typescript
scope(config): this {
    if (this.items.length > 0) {
        const loc = captureSourceLocation();
        this.items[0].builder.addScope(this.items, config, loc);
    }
    return this;
}
```

**`ModuleOutput.scope()`**: Unchanged externally. Channel index is now respected downstream.

### Audio thread

`ScopeBuffer` simplifies to a single-channel buffer. On each audio frame:

```rust
for (key, buffer) in scope_collection.iter_mut() {
    let module = &modules[key.module_id];
    let sample = module.get_sample(&key.port_name, key.channel);
    buffer.push(sample);
}
```

### NAPI return

`get_scopes()` returns `Vec<(ScopeChannel, Float32Array, ScopeStats)>` — flat per-channel data. The ScopeChannel includes the config params that form the buffer key so the frontend can match data to scopes.

### Renderer

**Per-channel keys**: Each channel's data is keyed as `:module:${moduleId}:${portName}:${channel}:${msPerFrame}:${trigger}`.

**Frontend grouping**: Each `ScopeView` knows its scope definition (channels + config). It derives buffer keys for its channels, collects matching data from `getScopes()`, and draws all traces on one canvas.

**`drawOscilloscope()`**: No change needed — already handles multiple `Float32Array` traces.

**View zones**: No structural changes. One canvas per scope, positioned at the `.scope()` call site.

### Validation

- Each `ScopeChannel` validated individually: module exists, port is valid output, channel is within bounds
- Empty `channels` vec is a validation error

### User-facing API

No changes. `ModuleOutput.scope()` and `Collection.scope()` signatures remain the same.

## Files affected

| File                               | Change                                                                                                          |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `crates/modular_core/src/types.rs` | Replace `ScopeItem` with `ScopeChannel`, update `Scope` struct                                                  |
| `crates/modular/src/audio.rs`      | `ScopeBuffer` becomes single-channel, keying changes to `ScopeBufferKey`, capture logic reads specific channels |
| `crates/modular/src/commands.rs`   | Update `PatchUpdate` scope diff types                                                                           |
| `crates/modular/src/validation.rs` | Validate each `ScopeChannel` individually                                                                       |
| `crates/modular/src/lib.rs`        | Update `get_scopes()` return type                                                                               |
| `crates/modular/index.d.ts`        | Update NAPI type declarations                                                                                   |
| `src/main/dsl/GraphBuilder.ts`     | `addScope()` collects all channels, `BaseCollection.scope()` passes all items                                   |
| `src/main/dsl/factories.ts`        | `DSLContext.scope()` already spreads collections (minor update)                                                 |
| `src/main/dsl/typescriptLibGen.ts` | Update generated scope types if needed                                                                          |
| `src/renderer/App.tsx`             | Group per-channel data by scope for rendering                                                                   |
| `src/renderer/app/oscilloscope.ts` | Update `scopeKeyFromSubscription()`                                                                             |
| `src/renderer/types/editor.ts`     | Update `ScopeView` if needed                                                                                    |
