# Deserr Migration — Phase 2 & 3 Plan (Remaining Tasks)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the remaining tasks for the migration from `serde::Deserialize` to `deserr::Deserr` for module param deserialization. The core traits and proc-macro code generation have been updated. This plan covers wiring up the new error types through the cache layer, N-API entrypoints, the TypeScript frontend, and finally cleaning up the old serde code.

**Tech Stack:** Rust (deserr 0.6, serde, schemars), TypeScript, Electron IPC

**Branch:** `feature/required-params`

---

## Phase 2: Switch Deserialization Path (Continued)

### Task 2.3: Update cache layer error handling

**Files:**

- Modify: `crates/modular/src/params_cache.rs:48-88`

**Step 1: Update `deserialize_params` to use `ModuleParamErrors`**
Change the return type to `Result<DeserializedParams, modular_core::param_errors::ModuleParamErrors>`.

```rust
pub fn deserialize_params(
  module_type: &str,
  params: serde_json::Value,
  with_cache: bool,
) -> Result<DeserializedParams, modular_core::param_errors::ModuleParamErrors> {
```

**Step 2: Update error mapping on missing deserializer**

```rust
  // Cache miss — deserialize
  let deserializer = get_params_deserializers().get(module_type).ok_or_else(|| {
    let mut errors = modular_core::param_errors::ModuleParamErrors::default();
    errors.add(String::new(), format!("No params deserializer for module type: {}", module_type));
    errors
  })?;
```

**Step 3: Commit**

```bash
git commit -am "refactor: update params_cache to return ModuleParamErrors"
```

### Task 2.4: Update N-API entrypoints and error types

**Files:**

- Modify: `crates/modular/src/lib.rs`
- Modify: `crates/modular/src/audio.rs`

**Step 1: Update `derive_channel_count` in `lib.rs`**
Replace the string parsing logic with direct access to structured errors.

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
    Err(e) => {
      let param_errors = e.into_errors();
      DeriveChannelCountResult {
        channel_count: None,
        errors: Some(param_errors.into_iter().map(|err| {
          DeriveChannelCountError {
            message: err.message,
            params: if err.field.is_empty() { vec![] } else { vec![err.field] },
          }
        }).collect()),
      }
    }
  }
}
```

**Step 2: Update `apply_patch` in `audio.rs`**
Map `ModuleParamErrors` to `napi::Error`.

```rust
// In apply_patch around line 1069:
      let deserialized =
        crate::deserialize_params(&module_state.module_type, module_state.params, true)
          .map_err(|e| {
            napi::Error::from_reason(format!(
              "Failed to deserialize params for {}: {}",
              id, e
            ))
          })?;
```

**Step 3: Commit**

```bash
cargo check -p modular
git commit -am "refactor: wire deserr error types through N-API layer"
```

### Task 2.5: Remove obsolete serde error parsing

**Files:**

- Modify: `crates/modular/src/lib.rs`

**Step 1: Delete functions**
Delete `parse_deserialization_error` and `translate_serde_error` (lines ~1080-1171).

**Step 2: Commit**

```bash
git commit -am "refactor: remove brittle serde string parsing logic"
```

### Task 2.6: Update TypeScript Frontend

**Files:**

- Modify: `src/main/dsl/factories.ts:279-313`

**Step 1: Adjust error handling**
Since `deriveChannelCount` now returns all errors in a list rather than just the first one, we can present a more complete error message.

```typescript
// Check for errors from param deserialization
if (deriveResult.errors && deriveResult.errors.length > 0) {
    const messages = deriveResult.errors.map((e) => e.message).join('; ');
    const loc = sourceLocation ? ` at line ${sourceLocation.line}` : '';
    throw new Error(`${schema.name}${loc}: ${messages}`);
}
```

**Step 2: Rebuild NAPI to sync types**

```bash
yarn build-native
yarn generate-lib
```

**Step 3: Commit**

```bash
git commit -am "refactor(ts): update frontend error handling for batched deserr errors"
```

---

## Phase 3: Cleanup and Simplification

### Task 3.1: Remove validation.rs param-level checks

**Files:**

- Modify: `crates/modular/src/validation.rs:322-379`

**Step 1: Remove redundant validation**
Since deserr strictly checks param fields (`#[deserr(deny_unknown_fields)]`) and types, the manual validation in `validate_patch` that checks for unknown params and valid signal shapes is now redundant.

Remove lines 322-379 from `validate_patch`, leaving only:

- Module existence check
- Scope validation

_Note: Cable target validation may still be needed depending on whether deserr resolves cables or if that happens during `connect()`. Leave it for now if unsure._

**Step 2: Commit**

```bash
git commit -am "refactor: remove redundant param validation now handled by deserr"
```

### Task 3.2: Remove serde::Deserialize derives

**Files:**

- Modify: All params structs in `crates/modular_core/src/dsp/*/`

**Step 1: Strip Deserialize**
Run a global replace to remove `Deserialize` from the `#[derive(...)]` list on params structs, as well as any `#[serde(...)]` attributes if they are no longer needed. Note: Keep `Serialize` and `JsonSchema`.

**Step 2: Commit**

```bash
git commit -am "refactor: drop serde::Deserialize from param structs"
```

### Task 3.3: Final testing

**Step 1: Verify all tests pass**

```bash
cargo test -p modular_core
yarn lint
yarn typecheck
yarn test:unit
```

**Step 2: Fix 18 pre-existing errors (optional/separate PR)**
If tests fail with the known 18 pre-existing errors (Clock::default, Supersaw state, IntervalSeq::default), either ignore them or fix them as a follow-up.

---

## Plan Complete

**Saved to:** `docs/plans/2026-03-15-deserr-migration-phase2-remaining.md`
