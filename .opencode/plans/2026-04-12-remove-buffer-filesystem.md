# Remove Buffer Filesystem Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove all filesystem I/O (WAV loading/flushing) from the buffer system, replacing path-based identity with name-based identity while preserving in-memory persistence across patch re-evaluations.

**Architecture:** Buffers currently use filesystem paths as their identity key and persist data as WAV files on disk. This change replaces the `path` field with a `name` field throughout the stack (TypeScript DSL → JSON PatchGraph → Rust core types → Rust N-API runtime). The `RuntimeBuffer` struct is simplified to remove all disk I/O, the `hound` WAV crate dependency is removed, and the DSL `$buffer()` factory drops workspace/path resolution logic. In-memory persistence is retained: buffers with the same name and shape survive re-evaluation.

**Tech Stack:** Rust (modular_core, modular crates), TypeScript (DSL executor, GraphBuilder, typescriptLibGen), Vitest, cargo test

---

### Task 1: Rename `path` → `name` in `BufferSpec` (Rust core)

**Files:**

- Modify: `crates/modular_core/src/types.rs:635-678` (BufferSpec struct + impl)

**Step 1: Rename the field**

Change the `BufferSpec` struct field from `path` to `name`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Deserr, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
pub struct BufferSpec {
    pub name: String,
    pub channels: usize,
    pub frame_count: usize,
}
```

**Step 2: Update `BufferSpec::new` and `validate`**

```rust
impl BufferSpec {
    pub fn new(name: String, channels: usize, frame_count: usize) -> StdResult<Self, String> {
        let spec = Self {
            name,
            channels,
            frame_count,
        };
        spec.validate()?;
        Ok(spec)
    }

    pub fn same_shape(&self, other: &Self) -> bool {
        self.channels == other.channels && self.frame_count == other.frame_count
    }

    pub fn validate(&self) -> StdResult<(), String> {
        if self.name.trim().is_empty() {
            return Err("Buffer name must not be empty".to_string());
        }

        if !(1..=crate::poly::PORT_MAX_CHANNELS).contains(&self.channels) {
            return Err(format!(
                "Buffer channels must be between 1 and {}, got {}",
                crate::poly::PORT_MAX_CHANNELS,
                self.channels
            ));
        }

        if self.frame_count == 0 {
            return Err("Buffer frameCount must be greater than 0".to_string());
        }

        Ok(())
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check -p modular_core 2>&1 | head -50`
Expected: Compilation errors in downstream code referencing `.path` — these will be fixed in subsequent tasks.

---

### Task 2: Rename `path` → `name` in `Buffer` and serialization types (Rust core)

**Files:**

- Modify: `crates/modular_core/src/types.rs:680-940` (BufferSerde, BufferSchema, Buffer impls)

**Step 1: Update `BufferSerde` and `BufferSchema` enums**

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum BufferSerde {
    Buffer {
        name: String,
        channels: usize,
        frame_count: usize,
    },
}

#[derive(JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[allow(dead_code)]
enum BufferSchema {
    Buffer {
        name: String,
        channels: usize,
        frame_count: usize,
    },
}
```

**Step 2: Update `Buffer` accessor**

Rename the `path()` method to `name()`:

```rust
pub fn name(&self) -> &str {
    &self.spec.name
}
```

**Step 3: Update `Deserialize` impl for `Buffer`**

Change destructuring from `path` to `name`:

```rust
impl<'de> Deserialize<'de> for Buffer {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let tagged = BufferSerde::deserialize(deserializer)?;
        let spec = match tagged {
            BufferSerde::Buffer {
                name,
                channels,
                frame_count,
            } => BufferSpec::new(name, channels, frame_count).map_err(serde::de::Error::custom)?,
        };
        Ok(Buffer::new(spec))
    }
}
```

**Step 4: Update `Serialize` impl for `Buffer`**

```rust
impl Serialize for Buffer {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        BufferSerde::Buffer {
            name: self.spec.name.clone(),
            channels: self.spec.channels,
            frame_count: self.spec.frame_count,
        }
        .serialize(serializer)
    }
}
```

**Step 5: Update `Deserr` impl for `Buffer`**

Change all references from `"path"` to `"name"`:

```rust
let name = map.remove("name").and_then(|v| match v.into_value() {
    deserr::Value::String(s) => Some(s),
    _ => None,
});
```

And the missing-field error:

```rust
let name = name.ok_or_else(|| {
    deserr::take_cf_content(E::error::<V>(
        None,
        ErrorKind::MissingField { field: "name" },
        location,
    ))
})?;
let spec = BufferSpec::new(name, channels, frame_count).map_err(|msg| { ... })?;
```

**Step 6: Update `Connect` impl for `Buffer`**

```rust
impl Connect for Buffer {
    fn connect(&mut self, patch: &Patch) {
        if let Some(buffer) = patch.buffers.get(&self.spec.name) {
            self.buffer_ptr = Arc::downgrade(buffer);
        } else {
            self.buffer_ptr = sync::Weak::new();
        }
    }
}
```

**Step 7: Verify compilation**

Run: `cargo check -p modular_core 2>&1 | head -50`
Expected: modular_core compiles cleanly. Downstream `modular` crate will still have errors.

---

### Task 3: Simplify `RuntimeBuffer` — remove all filesystem I/O (Rust N-API)

**Files:**

- Modify: `crates/modular/src/buffer.rs` (entire file rewrite)
- Modify: `crates/modular/Cargo.toml` (remove `hound` dependency)

**Step 1: Rewrite `buffer.rs`**

Replace the entire file with a simplified version that has no filesystem operations:

```rust
use modular_core::types::{BufferData, BufferSpec};
use std::sync::Arc;

#[derive(Debug)]
pub struct RuntimeBuffer {
  spec: BufferSpec,
  shared: Arc<BufferData>,
}

impl RuntimeBuffer {
  pub fn zeroed(spec: BufferSpec) -> Self {
    Self {
      shared: Arc::new(BufferData::new_zeroed(spec.channels, spec.frame_count)),
      spec,
    }
  }

  pub fn spec(&self) -> &BufferSpec {
    &self.spec
  }

  pub fn name(&self) -> &str {
    &self.spec.name
  }

  pub fn shared(&self) -> Arc<BufferData> {
    self.shared.clone()
  }

  pub fn same_shape(&self, other: &Self) -> bool {
    self.spec.same_shape(&other.spec)
  }

  pub fn copy_overlap_from(&self, other: &Self) {
    self.shared.copy_overlap_from(&other.shared);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn zeroed_buffer_has_correct_shape() {
    let spec = BufferSpec::new("test".to_string(), 2, 100).unwrap();
    let buffer = RuntimeBuffer::zeroed(spec);
    assert_eq!(buffer.shared().channel_count(), 2);
    assert_eq!(buffer.shared().frame_count(), 100);
    assert_eq!(buffer.shared().read(0, 0), 0.0);
  }

  #[test]
  fn copy_overlap_from_preserves_old_data_and_leaves_rest_zeroed() {
    let old_spec = BufferSpec::new("old".to_string(), 2, 4).unwrap();
    let new_spec = BufferSpec::new("new".to_string(), 3, 6).unwrap();

    let old = RuntimeBuffer::zeroed(old_spec);
    let new = RuntimeBuffer::zeroed(new_spec);

    old.shared().write(0, 0, 1.25);
    old.shared().write(1, 3, 2.5);

    new.copy_overlap_from(&old);
    let snapshot = new.shared().snapshot();
    assert_eq!(snapshot[0][0], 1.25);
    assert_eq!(snapshot[1][3], 2.5);
    assert_eq!(snapshot[2][0], 0.0);
    assert_eq!(snapshot[0][5], 0.0);
  }

  #[test]
  fn same_shape_compares_channels_and_frames() {
    let a = RuntimeBuffer::zeroed(BufferSpec::new("a".to_string(), 2, 100).unwrap());
    let b = RuntimeBuffer::zeroed(BufferSpec::new("b".to_string(), 2, 100).unwrap());
    let c = RuntimeBuffer::zeroed(BufferSpec::new("c".to_string(), 1, 100).unwrap());
    assert!(a.same_shape(&b));
    assert!(!a.same_shape(&c));
  }
}
```

**Step 2: Remove `hound` from Cargo.toml**

In `crates/modular/Cargo.toml`, remove the line:

```toml
hound = "3.5"
```

**Step 3: Verify Rust compilation (expect errors in audio.rs)**

Run: `cargo check -p modular 2>&1 | head -80`
Expected: Errors in `audio.rs` referencing removed methods (`load_or_zero`, `enable_flush_on_drop`, `suppress_flush_on_drop`, `path()`). These are fixed in Task 4.

---

### Task 4: Update `audio.rs` buffer lifecycle (Rust N-API)

**Files:**

- Modify: `crates/modular/src/audio.rs` (lines ~1032-1071, ~1220-1245, ~1353-1360, ~1439-1478)

**Step 1: Update `apply_patch()` — buffer preparation (main thread)**

Replace `load_or_zero` with `zeroed` and change `path` references to `name`. Around line 1032:

```rust
let mut desired_buffer_specs: HashMap<String, BufferSpec> = HashMap::new();
for module_state in desired_modules.values() {
  let mut specs = Vec::new();
  modular_core::types::collect_buffer_specs_in_json_value(&module_state.params, &mut specs);

  for spec in specs {
    match desired_buffer_specs.get(&spec.name) {
      Some(existing) if existing.same_shape(&spec) => {}
      Some(_) => {
        return Err(napi::Error::from_reason(format!(
          "Conflicting Buffer specs found for name '{}'",
          spec.name
        )));
      }
      None => {
        desired_buffer_specs.insert(spec.name.clone(), spec);
      }
    }
  }
}

let current_buffer_specs = self.buffer_specs.lock().clone();
update.desired_buffer_names = desired_buffer_specs.keys().cloned().collect();
for spec in desired_buffer_specs.values() {
  let Some(runtime_buffer) = (match current_buffer_specs.get(&spec.name) {
    Some(existing) if existing.same_shape(spec) => None,
    Some(_) | None => Some(RuntimeBuffer::zeroed(spec.clone())),
  }) else {
    continue;
  };

  update.buffer_adds.push(runtime_buffer);
}
```

Note: both `Some(_)` (shape changed) and `None` (new buffer) now use `RuntimeBuffer::zeroed()`. The in-memory persistence for same-name-same-shape is preserved because we skip those entirely (`None` branch).

**Step 2: Remove the `map_err` block for buffer creation**

`RuntimeBuffer::zeroed` no longer returns `Result`, so remove the `.map_err(...)` chain that follows it. If you prefer to keep it as `Result` for future-proofing, that's fine too — just use `spec.name` instead of `spec.path`.

**Step 3: Update `AudioProcessor` comments**

Around line 1223, update the comment:

```rust
/// Runtime-owned buffer resources keyed by name
active_buffers: HashMap<String, RuntimeBuffer>,
```

**Step 4: Update `apply_patch_update()` — audio thread buffer handling**

Around line 1439, replace `path()` with `name()` and remove flush calls:

```rust
for incoming in buffer_adds {
  let name = incoming.name().to_string();
  if let Some(existing) = self.active_buffers.get(&name) {
    if existing.same_shape(&incoming) {
      let _ = self.garbage_tx.push(GarbageItem::Buffer(incoming));
      continue;
    }
  }

  if let Some(replaced) = self.active_buffers.remove(&name) {
    incoming.copy_overlap_from(&replaced);
    let _ = self.garbage_tx.push(GarbageItem::Buffer(replaced));
  }

  self.patch.buffers.insert(name.clone(), incoming.shared());
  self.active_buffers.insert(name, incoming);
}

let stale_buffer_names: Vec<String> = self
  .active_buffers
  .keys()
  .filter(|name| !desired_buffer_names.contains(*name))
  .cloned()
  .collect();
for name in stale_buffer_names {
  self.patch.buffers.remove(&name);
  if let Some(buffer) = self.active_buffers.remove(&name) {
    let _ = self.garbage_tx.push(GarbageItem::Buffer(buffer));
  }
}
```

**Step 5: Update `ClearPatch` handler**

Around line 1353, rename variables from `path` to `name`:

```rust
let buffer_names: Vec<String> = self.active_buffers.keys().cloned().collect();
for name in buffer_names {
  self.patch.buffers.remove(&name);
  if let Some(buffer) = self.active_buffers.remove(&name) {
    let _ = self.garbage_tx.push(GarbageItem::Buffer(buffer));
  }
}
```

**Step 6: Update buffer spec snapshot**

Around line 1475:

```rust
for (name, buffer) in &self.active_buffers {
  shared_specs.insert(name.clone(), buffer.spec().clone());
}
```

**Step 7: Verify compilation**

Run: `cargo check -p modular 2>&1 | head -50`
Expected: Errors in `commands.rs` referencing `desired_buffer_paths`. Fixed in Task 5.

---

### Task 5: Update `commands.rs` — rename `desired_buffer_paths` (Rust N-API)

**Files:**

- Modify: `crates/modular/src/commands.rs` (lines 48-51, 73, 87)

**Step 1: Rename the field and update references**

In `PatchUpdate`:

```rust
/// Set of desired buffer names after this update is applied.
pub desired_buffer_names: std::collections::HashSet<String>,
```

In `PatchUpdate::new`:

```rust
desired_buffer_names: std::collections::HashSet::new(),
```

In `PatchUpdate::is_empty`:

```rust
&& self.desired_buffer_names.is_empty()
```

**Step 2: Verify compilation**

Run: `cargo check -p modular 2>&1 | head -50`
Expected: Errors in `validation.rs`. Fixed in Task 6.

---

### Task 6: Update `validation.rs` — rename path references to name (Rust N-API)

**Files:**

- Modify: `crates/modular/src/validation.rs` (lines ~301, ~353-378)

**Step 1: Rename variables and error messages**

Around line 301:

```rust
let mut buffer_specs_by_name: HashMap<String, BufferSpec> = HashMap::new();
```

Around lines 353-378:

```rust
let mut buffer_specs = Vec::new();
modular_core::types::collect_buffer_specs_in_json_value(param_value, &mut buffer_specs);
for spec in buffer_specs {
  match buffer_specs_by_name.get(&spec.name) {
    Some(existing) if existing.same_shape(&spec) => {}
    Some(existing) => {
      errors.push(ValidationError {
        field: field.clone(),
        message: format!(
          "Buffer '{}' is used with conflicting shapes (existing: {} channels × {} frames, got: {} channels × {} frames)",
          spec.name,
          existing.channels,
          existing.frame_count,
          spec.channels,
          spec.frame_count
        ),
        location: Some(location_str.clone()),
        expected_type: None,
        actual_value: None,
      });
    }
    None => {
      buffer_specs_by_name.insert(spec.name.clone(), spec);
    }
  }
}
```

**Step 2: Verify full Rust compilation**

Run: `cargo check -p modular 2>&1 | head -50`
Expected: Clean compilation.

---

### Task 7: Run and fix Rust tests

**Files:**

- Modify: `crates/modular/src/validation.rs` (test section around lines 738-817)
- Verify: `crates/modular_core/` tests and `crates/modular/src/buffer.rs` tests

**Step 1: Run Rust tests**

Run: `cargo test -p modular_core 2>&1 | tail -30`
Run: `cargo test -p modular 2>&1 | tail -30`

**Step 2: Fix validation tests**

The validation tests at lines ~738-817 create `BufferSpec` with path strings. Update them to use `name` semantics:

In `test_buffer_path_conflict_is_rejected` (rename to `test_buffer_name_conflict_is_rejected`): Change the test to use buffer name strings instead of paths. The buffer specs in the test JSON should use `"name"` instead of `"path"`.

In `test_buffer_path_shared_shape_is_allowed` (rename to `test_buffer_name_shared_shape_is_allowed`): Same treatment.

**Step 3: Verify all Rust tests pass**

Run: `cargo test -p modular_core && cargo test -p modular`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: rename buffer path to name and remove filesystem I/O in Rust"
```

---

### Task 8: Update TypeScript `Buffer` type and `$buffer()` factory

**Files:**

- Modify: `src/main/dsl/GraphBuilder.ts:50-55`
- Modify: `src/main/dsl/executor.ts:173-226`

**Step 1: Update `Buffer` type in `GraphBuilder.ts`**

```typescript
export type Buffer = {
    type: 'buffer';
    name: string;
    channels: number;
    frameCount: number;
};
```

**Step 2: Simplify `$buffer()` in `executor.ts`**

Replace the entire `$buffer` function with a simplified version that takes a name instead of a path:

```typescript
const $buffer = (
    name: string,
    lengthSeconds: number,
    channels: number = 1,
): Buffer => {
    if (typeof name !== 'string' || name.trim().length === 0) {
        throw new Error('$buffer() name must be a non-empty string');
    }
    if (typeof lengthSeconds !== 'number' || !Number.isFinite(lengthSeconds)) {
        throw new Error('$buffer() lengthSeconds must be a finite number');
    }
    if (lengthSeconds <= 0) {
        throw new Error(
            `$buffer() lengthSeconds must be greater than 0, got ${lengthSeconds}`,
        );
    }
    if (!Number.isInteger(channels) || channels < 1 || channels > 16) {
        throw new Error(
            `$buffer() channels must be an integer between 1 and 16, got ${channels}`,
        );
    }

    const frameCount = Math.max(1, Math.ceil(lengthSeconds * sampleRate));
    return {
        channels,
        frameCount,
        name: name.trim(),
        type: 'buffer',
    };
};
```

This removes:

- `path.isAbsolute()` check
- `workspaceRoot` requirement
- `.wav` extension appending
- `path.resolve()` to `tmp/` directory
- Path sandboxing check
- The `import path from 'node:path'` may become unused — check and remove if so.

**Step 3: Check for unused `path` import**

If the `path` module import in `executor.ts` is only used by `$buffer()`, remove it. If other code uses it, leave it.

**Step 4: Check for unused `workspaceRoot` references**

The `workspaceRoot` parameter may still be used elsewhere in `executor.ts`. If `$buffer()` was the only consumer, it can potentially be removed from the function signature — but check first. If other code uses it, leave it.

**Step 5: Verify TypeScript compilation**

Run: `yarn typecheck 2>&1 | head -50`
Expected: Errors in test files and other TS files referencing `buffer.path`. Fixed in subsequent tasks.

---

### Task 9: Update TypeScript DSL lib generation

**Files:**

- Modify: `src/main/dsl/typescriptLibGen.ts:301-312, 1335-1339`

**Step 1: Update the `Buffer` type documentation and definition**

Around line 301:

```typescript
/**
 * A named audio buffer resource shared across modules.
 *
 * Buffers are mutable channel × frame arrays stored in memory.
 * Buffers with the same name share data across modules.
 */
type Buffer = {
    readonly type: 'buffer';
    readonly name: string;
    readonly channels: number;
    readonly frameCount: number;
};
```

**Step 2: Update the `$buffer()` function signature**

Around line 1335:

```typescript
lines.push('/** Create or reuse a named audio buffer. */');
lines.push(
    'export function $buffer(name: string, lengthSeconds: number, channels?: number): Buffer;',
);
```

**Step 3: Verify TypeScript compilation**

Run: `yarn typecheck 2>&1 | head -50`

---

### Task 10: Update `paramsSchema.ts` buffer detection

**Files:**

- Modify: `src/main/dsl/paramsSchema.ts:319-358`

**Step 1: Replace `path` property check with `name`**

In the `isBufferParamSchema` function, change the property name check from `path` to `name`:

```typescript
const nameSchema = resolved.properties?.name
    ? resolveAndMerge(root, resolved.properties.name)
    : null;
```

And in the return statement:

```typescript
return (
    extractTypeTag(typeSchema ?? {}) === 'buffer' &&
    nameSchema?.type === 'string' &&
    (channelsSchema?.type === 'integer' || channelsSchema?.type === 'number') &&
    (frameCountSchema?.type === 'integer' ||
        frameCountSchema?.type === 'number')
);
```

Remove the `pathSchema` variable entirely.

**Step 2: Verify TypeScript compilation**

Run: `yarn typecheck 2>&1 | head -50`

---

### Task 11: Update TypeScript tests

**Files:**

- Modify: `src/main/dsl/__tests__/executor.test.ts` (lines ~450-462, ~679-707)

**Step 1: Update `$bufRead` and `$bufWrite` tests**

Around line 450, the test strings should work as-is since they just check that the module is created. But verify the `$buffer()` call still works with name-based API (the first arg `"loops/read-test"` is now treated as a name, not a path).

**Step 2: Rewrite the `$buffer()` test block**

Replace the test at lines 679-707:

```typescript
describe('$buffer()', () => {
    test('creates a buffer with name and computed frame count', () => {
        expect(() =>
            exec(`
                const buffer = $buffer('kick', 0.5, 2);
                if (buffer.name !== 'kick') {
                    throw new Error('expected name "kick", got ' + buffer.name);
                }
                if (buffer.frameCount !== 24000) {
                    throw new Error(String(buffer.frameCount));
                }
                if (buffer.channels !== 2) {
                    throw new Error(String(buffer.channels));
                }
            `),
        ).not.toThrow();
    });

    test('trims whitespace from name', () => {
        expect(() =>
            exec(`
                const buffer = $buffer('  padded  ', 1);
                if (buffer.name !== 'padded') {
                    throw new Error('expected trimmed name, got "' + buffer.name + '"');
                }
            `),
        ).not.toThrow();
    });

    test('rejects empty name', () => {
        expect(() =>
            executePatchScript(
                '$buffer("", 1)',
                schemas,
                DEFAULT_EXECUTION_OPTIONS,
            ),
        ).toThrow(/name must be a non-empty string/);
    });
});
```

The old test that checked path escaping (`../escape`) is no longer relevant and should be removed.

**Step 3: Run TypeScript unit tests**

Run: `yarn test:unit 2>&1 | tail -30`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: rename buffer path to name and simplify $buffer() factory in TypeScript"
```

---

### Task 12: Run full test suite and verify

**Step 1: Run all Rust tests**

Run: `cargo test -p modular_core && cargo test -p modular`
Expected: All pass.

**Step 2: Run TypeScript type check**

Run: `yarn typecheck`
Expected: No errors.

**Step 3: Run TypeScript unit tests**

Run: `yarn test:unit`
Expected: All pass.

**Step 4: Build native module**

Run: `yarn build-native`
Expected: Clean build.

**Step 5: Regenerate DSL types**

Run: `yarn generate-lib`
Expected: Types regenerated (the JSON schema from Rust now uses `name` instead of `path`).

**Step 6: Final commit if any generated files changed**

```bash
git add -A && git commit -m "chore: regenerate DSL types after buffer name rename"
```
