# `$clamp` + `$wrap` Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Update `$clamp` to swap bounds when `max < min`, and add a new `$wrap` module that folds a signal into a range.

**Architecture:** Both are single-file Rust DSP modules in `crates/modular_core/src/dsp/utilities/`. Each uses the proc-macro pattern (`#[module(...)]`, `#[derive(Outputs)]`, `message_handlers!`) and is registered in `utilities/mod.rs`. No TypeScript changes needed — the schema-driven factory picks them up automatically.

**Tech Stack:** Rust (edition 2024), `modular_core` proc-macros (`Connect`, `ChannelCount`, `SignalParams`, `Outputs`, `module`), `PolySignal`/`PolyOutput` from `crate::poly`.

---

## Task 1: Update `$clamp` — add min/max swap + tests

**Files:**

- Modify: `crates/modular_core/src/dsp/utilities/clamp.rs`

### Step 1: Write the failing tests (add to bottom of `clamp.rs`)

Append this `#[cfg(test)]` block to the end of `clamp.rs` (after line 71, after `message_handlers!`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::PolySignal;

    fn run_clamp(input: f32, min: Option<f32>, max: Option<f32>) -> f32 {
        let mut module = Clamp::default();
        module.params.input = PolySignal::from_constant(input);
        if let Some(v) = min {
            module.params.min = PolySignal::from_constant(v);
        }
        if let Some(v) = max {
            module.params.max = PolySignal::from_constant(v);
        }
        module.update(44100.0);
        module.outputs.sample.get(0)
    }

    #[test]
    fn clamp_within_range() {
        assert_eq!(run_clamp(2.5, Some(0.0), Some(5.0)), 2.5);
    }

    #[test]
    fn clamp_below_min() {
        assert_eq!(run_clamp(-1.0, Some(0.0), Some(5.0)), 0.0);
    }

    #[test]
    fn clamp_above_max() {
        assert_eq!(run_clamp(7.0, Some(0.0), Some(5.0)), 5.0);
    }

    #[test]
    fn clamp_swaps_when_max_less_than_min() {
        // max=0, min=5 → swapped to [0, 5]
        assert_eq!(run_clamp(7.0, Some(5.0), Some(0.0)), 5.0);
        assert_eq!(run_clamp(-1.0, Some(5.0), Some(0.0)), 0.0);
        assert_eq!(run_clamp(3.0, Some(5.0), Some(0.0)), 3.0);
    }

    #[test]
    fn clamp_no_min_unclamped_below() {
        assert_eq!(run_clamp(-100.0, None, Some(5.0)), -100.0);
    }

    #[test]
    fn clamp_no_max_unclamped_above() {
        assert_eq!(run_clamp(100.0, Some(0.0), None), 100.0);
    }
}
```

**Note:** `PolySignal::from_constant(v)` and `PolyOutput::get(channel)` may not exist yet — check `crates/modular_core/src/poly.rs` for the actual test helper API. If they don't exist, look at how existing test files (e.g. `quantizer.rs`) construct `PolySignal` values in tests and adapt accordingly.

### Step 2: Run to confirm tests fail

```bash
cargo test -p modular_core clamp 2>&1 | tail -30
```

Expected: compilation errors or test failures (the swap test should fail since swap logic doesn't exist yet).

### Step 3: Rewrite the `update` method in `clamp.rs`

Replace the entire `impl Clamp { fn update ... }` block (lines 43–69) with:

```rust
impl Clamp {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();
        let has_min = !self.params.min.is_disconnected();
        let has_max = !self.params.max.is_disconnected();

        for i in 0..channels as usize {
            let mut val = self.params.input.get_value(i);

            match (has_min, has_max) {
                (true, true) => {
                    let a = self.params.min.get_value(i);
                    let b = self.params.max.get_value(i);
                    let (lo, hi) = if b < a { (b, a) } else { (a, b) };
                    val = val.clamp(lo, hi);
                }
                (true, false) => {
                    let min_val = self.params.min.get_value(i);
                    if val < min_val {
                        val = min_val;
                    }
                }
                (false, true) => {
                    let max_val = self.params.max.get_value(i);
                    if val > max_val {
                        val = max_val;
                    }
                }
                (false, false) => {}
            }

            self.outputs.sample.set(i, val);
        }
    }
}
```

### Step 4: Run tests — confirm they all pass

```bash
cargo test -p modular_core clamp 2>&1 | tail -20
```

Expected: all tests pass.

### Step 5: Commit

```bash
git add crates/modular_core/src/dsp/utilities/clamp.rs
git commit -m "feat: \$clamp swaps min/max when max < min"
```

---

## Task 2: Create `$wrap` module

**Files:**

- Create: `crates/modular_core/src/dsp/utilities/wrap.rs`
- Modify: `crates/modular_core/src/dsp/utilities/mod.rs`

### Step 1: Write the failing tests first

Create `crates/modular_core/src/dsp/utilities/wrap.rs` with tests only (no implementation yet):

````rust
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolyOutput, PolySignal};

#[derive(Clone, Deserialize, Default, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(default, rename_all = "camelCase")]
struct WrapParams {
    /// signal to wrap
    input: PolySignal,
    /// lower bound of the wrap range
    #[signal(default = 0.0)]
    min: PolySignal,
    /// upper bound of the wrap range
    #[signal(default = 5.0)]
    max: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct WrapOutputs {
    #[output("output", "wrapped signal output", default)]
    sample: PolyOutput,
}

/// Folds a signal into a range by wrapping values that exceed the boundaries
/// back from the opposite side — like a phase accumulator.
///
/// Both **min** and **max** accept polyphonic signals. If **max** < **min**
/// the bounds are swapped automatically.
///
/// ```js
/// // wrap a ramp into 0–5 V
/// $wrap(ramp, 0, 5)
///
/// // wrap with default 0–5 V range
/// $wrap(signal)
/// ```
#[module(name = "$wrap", args(input, min?, max?))]
#[derive(Default)]
pub struct Wrap {
    outputs: WrapOutputs,
    params: WrapParams,
}

impl Wrap {
    fn update(&mut self, _sample_rate: f32) {
        todo!("implement wrap")
    }
}

message_handlers!(impl Wrap {});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::PolySignal;

    fn run_wrap(input: f32, min: f32, max: f32) -> f32 {
        let mut module = Wrap::default();
        module.params.input = PolySignal::from_constant(input);
        module.params.min = PolySignal::from_constant(min);
        module.params.max = PolySignal::from_constant(max);
        module.update(44100.0);
        module.outputs.sample.get(0)
    }

    #[test]
    fn wrap_within_range_unchanged() {
        let result = run_wrap(2.5, 0.0, 5.0);
        assert!((result - 2.5).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_above_max_folds_back() {
        // 6.0 in [0, 5] → 1.0
        let result = run_wrap(6.0, 0.0, 5.0);
        assert!((result - 1.0).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_below_min_folds_forward() {
        // -1.0 in [0, 5] → 4.0
        let result = run_wrap(-1.0, 0.0, 5.0);
        assert!((result - 4.0).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_exactly_at_max_wraps_to_min() {
        // 5.0 in [0, 5] → 0.0 (exclusive upper bound)
        let result = run_wrap(5.0, 0.0, 5.0);
        assert!((result - 0.0).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_exactly_at_min_stays() {
        let result = run_wrap(0.0, 0.0, 5.0);
        assert!((result - 0.0).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_swaps_when_max_less_than_min() {
        // max=0, min=5 → treated as [0, 5]; 6 → 1
        let result = run_wrap(6.0, 5.0, 0.0);
        assert!((result - 1.0).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_degenerate_zero_width_outputs_min() {
        let result = run_wrap(3.0, 2.0, 2.0);
        assert!((result - 2.0).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_negative_range() {
        // 0.5 in [-1, 1] → 0.5
        let result = run_wrap(0.5, -1.0, 1.0);
        assert!((result - 0.5).abs() < 1e-5, "got {result}");
    }

    #[test]
    fn wrap_far_above_range_multiple_cycles() {
        // 11.0 in [0, 5] → 1.0 (two full cycles above)
        let result = run_wrap(11.0, 0.0, 5.0);
        assert!((result - 1.0).abs() < 1e-5, "got {result}");
    }
}
````

**Same note as Task 1:** check `poly.rs` for `PolySignal::from_constant` and `PolyOutput::get` — adapt if the API differs.

### Step 2: Register `wrap` in `utilities/mod.rs` (compile only, no run yet)

In `crates/modular_core/src/dsp/utilities/mod.rs` make 5 additions (insert alphabetically among existing entries):

**Line 7 area — add module declaration** (after `pub mod clamp;`… before `pub mod clock_divider;` — or at the end of the `pub mod` block, doesn't matter):

```rust
pub mod wrap;
```

**`install_constructors` function** — after the `clamp` line:

```rust
wrap::Wrap::install_constructor(map);
```

**`install_param_validators` function** — after the `clamp` line:

```rust
wrap::Wrap::install_params_validator(map);
```

**`install_params_deserializers` function** — after the `clamp` line:

```rust
wrap::Wrap::install_params_deserializer(map);
```

**`schemas()` function** — after the `clamp` line:

```rust
wrap::Wrap::get_schema(),
```

### Step 3: Run tests — confirm they compile but the wrap tests fail (todo! panics)

```bash
cargo test -p modular_core wrap 2>&1 | tail -30
```

Expected: tests exist, all fail with "not yet implemented".

### Step 4: Implement `update` in `wrap.rs`

Replace the `todo!` stub with the real implementation:

```rust
impl Wrap {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();

        for i in 0..channels as usize {
            let val = self.params.input.get_value(i);
            let a = self.params.min.get_value_or(i, 0.0);
            let b = self.params.max.get_value_or(i, 5.0);
            let (min, max) = if b < a { (b, a) } else { (a, b) };

            let output = if (max - min).abs() < f32::EPSILON {
                min
            } else {
                let span = max - min;
                let mut offset = (val - min) % span;
                if offset < 0.0 {
                    offset += span;
                }
                min + offset
            };

            self.outputs.sample.set(i, output);
        }
    }
}
```

### Step 5: Run tests — confirm all pass

```bash
cargo test -p modular_core wrap 2>&1 | tail -20
```

Expected: all 9 tests pass.

### Step 6: Run all modular_core tests to check nothing is broken

```bash
cargo test -p modular_core 2>&1 | tail -20
```

Expected: all tests pass.

### Step 7: Commit

```bash
git add crates/modular_core/src/dsp/utilities/wrap.rs \
        crates/modular_core/src/dsp/utilities/mod.rs
git commit -m "feat: add \$wrap module — folds signal into range with min/max swap"
```

---

## Task 3: Rebuild native and verify DSL

**Files:** none (build step only)

### Step 1: Build the native Rust module

```bash
yarn build-native 2>&1 | tail -20
```

Expected: successful build, no errors.

### Step 2: Check the generated TypeScript types include both modules

```bash
grep -E '\$clamp|\$wrap' crates/modular/index.d.ts
```

Expected: both `$clamp` and `$wrap` entries appear in the generated types file.

### Step 3: Commit the regenerated types

```bash
git add crates/modular/index.d.ts
git commit -m "chore: regenerate N-API types after \$clamp/\$wrap changes"
```

---

## Task 4: Run full test suite

### Step 1: Run all tests

```bash
yarn test:all 2>&1 | tail -30
```

Expected: all tests pass (Rust unit, JS unit, E2E).

If E2E tests fail unrelated to this change, note them and proceed. Only block on failures directly related to `$clamp` or `$wrap`.
