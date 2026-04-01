# Deserr Migration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace `serde::Deserialize` with `deserr::Deserr` for module param deserialization, enabling all validation errors reported at once, DSL-aware error context, and validation integrated into deserialization.

**Architecture:** Custom `ModuleParamErrors` error type accumulating errors via `ControlFlow::Continue`. Phase 1 adds deserr alongside serde (everything still compiles via old path). Phase 2 switches the deserialization path. Phase 3 removes serde derive and post-hoc error parsing.

**Tech Stack:** Rust (deserr 0.6, serde, schemars), TypeScript, Electron IPC

**Baseline:** 18 pre-existing test compilation errors (Clock::default, Supersaw state, IntervalSeq::default) — fix separately, NOT in this plan.

**Branch:** `feature/required-params` (32 commits on top of master)

---

## Key Decisions Record

| Decision             | Choice                                                                | Rationale                                                    |
| -------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------ |
| Error accumulation   | `ControlFlow::Continue`                                               | Collect all errors, not fail-fast                            |
| Error format         | DSL-context aware                                                     | Include source line from `__argument_spans`                  |
| Unknown fields       | `#[deserr(deny_unknown_fields)]`                                      | Deserr provides "did you mean?" for typos                    |
| serde derive removal | Remove `Deserialize` entirely                                         | Keep `Serialize` + `schemars::JsonSchema`                    |
| Attribute mapping    | Replace `#[serde(...)]` with `#[deserr(...)]` on structs losing serde | Critical: `#[serde(...)]` invalid without serde derive       |
| Graph validation     | Keep in `validation.rs`                                               | Cable refs, scope validation stay; remove param-level checks |

---

## Phase 1: Add Deserr alongside Deserialize

### Task 1.1: Add deserr dependency

**Files:**

- Modify: `crates/modular_core/Cargo.toml`

**Step 1: Add deserr dependency**

```toml
# In [dependencies] section
deserr = { version = "0.6", features = ["serde-json"] }
```

**Step 2: Commit**

```bash
git add crates/modular_core/Cargo.toml
git commit -m "feat: add deserr dependency"
```

---

### Task 1.2: Create ModuleParamErrors type

**Files:**

- Create: `crates/modular_core/src/param_errors.rs`
- Modify: `crates/modular_core/src/lib.rs`

**Step 1: Create param_errors.rs**

```rust
use deserr::DeserializeError;
use std::collections::HashMap;
use std::ops::ControlFlow;

/// Accumulates all deserialization errors with source location context.
///
/// Uses `ControlFlow::Continue` to collect all errors rather than failing on first.
#[derive(Debug, Clone, Default)]
pub struct ModuleParamErrors {
    /// All accumulated errors, each with field path and message.
    errors: Vec<ParamError>,
    /// Source spans extracted from `__argument_spans` field.
    spans: HashMap<String, (u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct ParamError {
    pub field: String,
    pub message: String,
}

impl ModuleParamErrors {
    pub fn new(spans: HashMap<String, (u32, u32)>) -> Self {
        Self { errors: Vec::new(), spans }
    }

    pub fn add(&mut self, field: String, message: String) {
        self.errors.push(ParamError { field, message });
    }

    pub fn into_errors(self) -> Vec<ParamError> {
        self.errors
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get source line number for a field (1-indexed, for error messages).
    pub fn get_line_for_field(&self, field: &str) -> Option<u32> {
        self.spans.get(field).map(|(start, _)| {
            // Approximate: assume ~50 chars per line, divide start by 50
            // More precise implementation would track newlines in source
            1 + (start / 50)
        })
    }
}

impl<E: DeserializeError> MergeWithError<ModuleParamErrors> for E {
    fn merge(
        self_: Option<Self>,
        other: ModuleParamErrors,
        _merge_location: deserr::ValuePointerRef<'_>,
    ) -> ControlFlow<Self, ModuleParamErrors> {
        // Extract the error from ControlFlow and add to our accumulator
        let mut errors = other;
        if let Some(cf) = self_ {
            let e = match cf {
                ControlFlow::Break(e) | ControlFlow::Continue(e) => e,
            };
            // Extract field/message from the error (deserr errors contain the path)
            // For simplicity, add a generic message — enhanced later
            errors.add("unknown".to_string(), format!("{:?}", e));
        }
        ControlFlow::Continue(errors)
    }
}

/// Helper to extract error from ControlFlow in manual Deserr impls.
pub fn take_cf<E>(cf: ControlFlow<E, E>) -> E {
    match cf {
        ControlFlow::Break(e) | ControlFlow::Continue(e) => e,
    }
}
```

**Step 2: Export in lib.rs**

```rust
// In crates/modular_core/src/lib.rs, add:
pub mod param_errors;
```

**Step 3: Commit**

```bash
git add crates/modular_core/src/param_errors.rs crates/modular_core/src/lib.rs
git commit -m "feat: add ModuleParamErrors type for accumulating deserr errors"
```

---

### Task 1.3: Add Deserr to Signal type

**Files:**

- Modify: `crates/modular_core/src/types.rs:559-606`

**Step 1: Add deserr import and derive**

```rust
use deserr::Deserr;

// Replace the existing Deserialize impl with Deserr derive + manual impl
// Keep the existing serde Serialize impl

#[derive(Clone, Debug, Deserr)]
#[deserr(untagged)]
pub enum Signal {
    Volts(f32),
    Cable {
        module: String,
        #[deserr(skip)] // Will be populated during connect()
        module_ptr: std::sync::Weak<Box<dyn Sampleable>>,
        port: String,
        channel: usize,
    },
}
```

**Step 2: Write manual Deserr impl (replaces the serde Deserialize impl)**

```rust
impl Deserr for Signal {
    fn deserialize_from_value<V: deserr::IntoValue>(
        value: deserr::Value<V>,
        location: deserr::ValuePointerRef<'_>,
    ) -> Result<Self, deserr::JsonError> {
        use deserr::{DeserializeError, ValueKind};

        match value.kind() {
            ValueKind::Integer(n) => Ok(Signal::Volts(n as f32)),
            ValueKind::Float(n) => Ok(Signal::Volts(n as f32)),
            ValueKind::String(s) => parse_signal_string(&s)
                .map(Signal::Volts)
                .map_err(|e| deserr::JsonError::custom(e, location)),
            ValueKind::Map(_) => {
                // Parse as Cable - need to extract fields manually
                // For now, delegate to serde (we'll complete this later)
                Err(deserr::JsonError::custom(
                    "Cable variant not yet implemented with deserr",
                    location,
                ))
            }
            _ => Err(deserr::JsonError::incorrect_value(
                "Signal",
                &["number", "string", "object"],
                value,
                location,
            )),
        }
    }
}
```

**Step 3: Commit**

```bash
git add crates/modular_core/src/types.rs
git commit -m "feat: add Deserr derive to Signal type"
```

---

### Task 1.4: Add Deserr to PolySignal and MonoSignal

**Files:**

- Modify: `crates/modular_core/src/poly.rs:282-313`, `399-408`

**Step 1: Add Deserr derive and impl for PolySignal**

Add `#[derive(Deserr)]` and write manual impl similar to Signal (accepts single or array).

**Step 2: Add Deserr derive and impl for MonoSignal**

MonoSignal delegates to PolySignal's deserialize.

**Step 3: Commit**

```bash
git add crates/modular_core/src/poly.rs
git commit -m "feat: add Deserr derive to PolySignal and MonoSignal"
```

---

### Task 1.5: Add Deserr to SeqPatternParam

**Files:**

- Modify: `crates/modular_core/src/dsp/seq/seq_value.rs:276-381`

**Step 1: Add Deserr derive and impl**

Replace `#[serde(transparent)]` with `#[deserr(transparent)]`. Write manual `Deserr` impl that parses the string and populates the skipped fields.

**Step 2: Commit**

```bash
git add crates/modular_core/src/dsp/seq/seq_value.rs
git commit -m "feat: add Deserr derive to SeqPatternParam"
```

---

### Task 1.6: Add Deserr to MathExpressionParam

**Files:**

- Modify: `crates/modular_core/src/dsp/utilities/math.rs:29-92`

**Step 1: Add Deserr derive and impl**

Replace `#[serde(transparent)]` with `#[deserr(transparent)]`. Write manual `Deserr` impl that parses the expression and populates skipped fields.

**Step 2: Commit**

```bash
git add crates/modular_core/src/dsp/utilities/math.rs
git commit -m "feat: add Deserr derive to MathExpressionParam"
```

---

### Task 1.7: Add Deserr to helper enums (no Serialize)

**Files:**

- Modify: `crates/modular_core/src/dsp/oscillators/noise.rs:12-23`
- Modify: `crates/modular_core/src/dsp/core/mix.rs:10-22`
- Modify: `crates/modular_core/src/dsp/oscillators/plaits.rs:30-82`

**Step 1: NoiseKind — replace serde with deserr attributes**

```rust
#[derive(Clone, Copy, Deserr, JsonSchema, Debug, PartialEq, Eq, Default)]
#[deserr(rename_all = camelCase)]
enum NoiseKind { ... }
```

**Step 2: MixMode — replace serde with deserr attributes**

```rust
#[derive(Clone, Copy, Debug, Default, Deserr, JsonSchema, PartialEq, Eq)]
#[deserr(rename_all = snake_case)]
pub enum MixMode { ... }
```

**Step 3: PlaitsEngine — replace serde with deserr attributes**

```rust
#[derive(Clone, Copy, Deserialize, Deserr, JsonSchema, Debug, PartialEq, Eq)]
#[deserr(rename_all = camelCase)]
pub enum PlaitsEngine { ... }
```

**Step 4: Commit**

```bash
git add crates/modular_core/src/dsp/oscillators/noise.rs crates/modular_core/src/dsp/core/mix.rs crates/modular_core/src/dsp/oscillators/plaits.rs
git commit -m "feat: add Deserr derive to helper enums (NoiseKind, MixMode, PlaitsEngine)"
```

---

### Task 1.8: Add Deserr to all params structs (~49 files)

**Files:**

- Modify: All params structs in `crates/modular_core/src/dsp/*/`

**Step 1: For each params struct, add:**

- `#[derive(Deserr)]`
- `#[deserr(deny_unknown_fields)]`
- Replace `#[serde(rename_all = "camelCase")]` with `#[deserr(rename_all = camelCase)]`
- Replace `#[serde(default)]` with `#[deserr(default)]`

**Step 2: Sample struct transformation (SineOscillatorParams):**

```rust
// Before:
#[derive(Clone, Deserialize, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
struct SineOscillatorParams {
    freq: Option<PolySignal>,
}

// After:
#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[deserr(rename_all = camelCase)]
#[deserr(deny_unknown_fields)]
struct SineOscillatorParams {
    #[deserr(default)]
    freq: Option<PolySignal>,
}
```

**Step 3: Commit after adding deserr to all params structs**

```bash
git add crates/modular_core/src/dsp/
git commit -m "feat: add Deserr derive to all params structs"
```

---

## Phase 2: Switch deserialization path

### Task 2.1: Update ParamsDeserializer type alias

**Files:**

- Modify: `crates/modular_core/src/params.rs:106`

**Step 1: Change the type alias**

```rust
// Before:
pub type ParamsDeserializer = fn(serde_json::Value) -> napi::Result<CachedParams>;

// After:
pub type ParamsDeserializer = fn(serde_json::Value) -> Result<CachedParams, ModuleParamErrors>;
```

**Step 2: Commit**

```bash
git add crates/modular_core/src/params.rs
git commit -m "refactor: update ParamsDeserializer to return ModuleParamErrors"
```

---

### Task 2.2: Update proc macro to generate deserr calls

**Files:**

- Modify: `crates/modular_derive/src/module_attr.rs:716-726`

**Step 1: Update install_params_deserializer**

```rust
// Before:
fn install_params_deserializer(map: &mut std::collections::HashMap<String, crate::params::ParamsDeserializer>) {
    fn deserializer(params: serde_json::Value) -> napi::Result<crate::params::CachedParams> {
        let parsed: #params_struct_name = serde_json::from_value(params)?;
        let channel_count = #channel_count_fn_name(&parsed);
        Ok(crate::params::CachedParams {
            params: Box::new(parsed),
            channel_count,
        })
    }
    map.insert(#module_name.into(), deserializer as crate::params::ParamsDeserializer);
}

// After:
fn install_params_deserializer(map: &mut std::collections::HashMap<String, crate::params::ParamsDeserializer>) {
    fn deserializer(params: serde_json::Value) -> Result<crate::params::CachedParams, crate::param_errors::ModuleParamErrors> {
        let parsed: #params_struct_name = deserr::deserialize(params)?;
        let channel_count = #channel_count_fn_name(&parsed);
        Ok(crate::params::CachedParams {
            params: Box::new(parsed),
            channel_count,
        })
    }
    map.insert(#module_name.into(), deserializer as crate::params::ParamsDeserializer);
}
```

**Step 2: Commit**

```bash
git add crates/modular_derive/src/module_attr.rs
git commit -m "refactor: generate deserr deserialize calls in proc macro"
```

---

### Task 2.3: Update params_cache.rs

**Files:**

- Modify: `crates/modular/src/params_cache.rs`

**Step 1: Update deserialize_params to handle new error type**

```rust
// The function signature changes from napi::Result<DeserializedParams>
// to Result<DeserializedParams, ModuleParamErrors>
// Update all error handling accordingly.
```

**Step 2: Commit**

```bash
git add crates/modular/src/params_cache.rs
git commit -m "refactor: update params cache for ModuleParamErrors"
```

---

### Task 2.4: Update derive_channel_count in lib.rs

**Files:**

- Modify: `crates/modular/src/lib.rs:1178-1200`

**Step 1: Update error handling**

Replace the serde error parsing with direct use of `ModuleParamErrors`:

```rust
#[napi]
pub fn derive_channel_count(
  module_type: String,
  params: serde_json::Value,
) -> DeriveChannelCountResult {
  match deserialize_params(&module_type, params, true) {
    Ok(d) => DeriveChannelCountResult {
      channel_count: Some(d.channel_count as u32),
      errors: None,
    },
    Err(errors) => {
      // Convert ModuleParamErrors to DeriveChannelCountError format
      // Include source line from spans
      let error_messages = errors.into_errors();
      DeriveChannelCountResult {
        channel_count: None,
        errors: Some(vec![DeriveChannelCountError {
          message: error_messages.iter().map(|e| e.message.clone()).join("; "),
          params: error_messages.iter().map(|e| e.field.clone()).collect(),
        }]),
      }
    }
  }
}
```

**Step 2: Commit**

```bash
git add crates/modular/src/lib.rs
git commit -m "refactor: update derive_channel_count for ModuleParamErrors"
```

---

### Task 2.5: Update apply_patch in audio.rs

**Files:**

- Modify: `crates/modular/src/audio.rs:1002-1105`

**Step 1: Update error handling for deserialize_params call**

Replace napi error with ModuleParamErrors handling.

**Step 2: Commit**

```bash
git add crates/modular/src/audio.rs
git commit -m "refactor: update apply_patch for ModuleParamErrors"
```

---

### Task 2.6: Update TypeScript error handling

**Files:**

- Modify: `src/main/dsl/factories.ts:275-319`
- Modify: `src/shared/ipcTypes.ts`

**Step 1: Update factories.ts to handle batched errors**

```typescript
// Before: single error, first missing param
// After: multiple errors, all issues reported

if (deriveResult.errors) {
    // Collect all errors
    const allMessages = deriveResult.errors.map((e) => e.message);
    throw new Error(
        `${schema.name} at line ${sourceLocation.line}: ${allMessages.join('; ')}`,
    );
}
```

**Step 2: Commit**

```bash
git add src/main/dsl/factories.ts src/shared/ipcTypes.ts
git commit -m "refactor: update TypeScript error handling for batched errors"
```

---

## Phase 3: Cleanup

### Task 3.1: Remove serde::Deserialize from params structs

**Files:**

- Modify: All params structs in `crates/modular_core/src/dsp/*/`

**Step 1: Remove `Deserialize` from derives**

```rust
// Before:
#[derive(Clone, Deserialize, JsonSchema, Connect, ChannelCount, SignalParams)]

// After:
#[derive(Clone, JsonSchema, Connect, ChannelCount, SignalParams)]
```

**Step 2: Commit**

```bash
git add crates/modular_core/src/dsp/
git commit -m "refactor: remove Deserialize derive from params structs"
```

---

### Task 3.2: Remove error parsing functions from lib.rs

**Files:**

- Modify: `crates/modular/src/lib.rs:1086-1171`

**Step 1: Remove parse_deserialization_error and translate_serde_error functions**

These are no longer needed — deserr provides structured errors directly.

**Step 2: Commit**

```bash
git add crates/modular/src/lib.rs
git commit -m "refactor: remove obsolete serde error parsing functions"
```

---

### Task 3.3: Simplify validation.rs

**Files:**

- Modify: `crates/modular/src/validation.rs:268-445`

**Step 1: Remove param-level validation**

The loop at lines 338-378 validates individual params — this is now handled by deserr. Keep:

- Module type existence check
- Cable reference validation (if still needed separately)
- Scope validation

```rust
// Remove the param validation loop (lines 338-378)
// Keep the module existence check and scope validation
```

**Step 2: Commit**

```bash
git add crates/modular/src/validation.rs
git commit -m "refactor: remove param-level validation from validation.rs"
```

---

### Task 3.4: Run full test suite

**Step 1: Run Rust tests**

```bash
cargo test -p modular_core
cargo test -p modular
```

**Step 2: Run TypeScript tests**

```bash
yarn test:unit
yarn test:e2e
```

**Step 3: Run lint and typecheck**

```bash
yarn lint
yarn typecheck
```

**Step 4: Commit final cleanup**

```bash
git add -A
git commit -m "feat: complete deserr migration"
```

---

## Verification Commands

| Phase | Command                       | Expected                     |
| ----- | ----------------------------- | ---------------------------- |
| 1     | `cargo check -p modular_core` | No errors                    |
| 2     | `cargo check -p modular`      | No errors                    |
| 2     | `yarn generate-lib`           | TypeScript types regenerated |
| 3     | `yarn lint && yarn typecheck` | All pass                     |
| 3     | `yarn test:unit`              | All pass                     |
| 3     | `yarn test:e2e`               | All pass                     |

---

## Plan Complete

**Saved to:** `docs/plans/2026-03-15-deserr-migration.md`

**Next step:** Execute this plan task-by-task using `superpowers:executing-plans` skill.
