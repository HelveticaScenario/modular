# $curve Module + .gain() / .exp() Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add perceptual amplitude control via a `$curve` DSP module and `.gain()` / `.exp()` DSL convenience methods.

**Architecture:** New Rust DSP module `$curve` applies `sign(x) * 5 * (|x|/5)^exp` per channel. Two TypeScript DSL methods chain it: `.gain(level)` curves the level then feeds it to `$scaleAndShift`, `.exp(factor=3)` wraps a signal directly with `$curve`.

**Tech Stack:** Rust (modular_core DSP), TypeScript (DSL GraphBuilder, type generation, docs)

**Design doc:** `docs/plans/2026-03-02-curve-module-design.md`

---

### Task 1: Create the `$curve` Rust DSP module

**Files:**

- Create: `crates/modular_core/src/dsp/utilities/curve.rs`

**Step 1: Write the module**

````rust
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolyOutput, PolySignal, PORT_MAX_CHANNELS};

#[derive(Clone, Deserialize, Default, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(default, rename_all = "camelCase")]
struct CurveParams {
    /// signal to apply curve to
    input: PolySignal,
    /// exponent for the power curve (0 = step, 1 = linear, >1 = audio taper)
    #[signal(range = (0.0, 10.0))]
    exp: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CurveOutputs {
    #[output("output", "curved signal output", default)]
    sample: PolyOutput,
}

/// Applies a power curve to a signal, normalised at ±5 V.
///
/// Formula: `sign(x) × 5 × (|x| / 5) ^ exp`
///
/// - **exp = 1** — linear pass-through
/// - **exp > 1** — pushes midrange toward zero (audio taper)
/// - **0 < exp < 1** — pushes midrange toward ±5 V
/// - **exp = 0** — step function (any nonzero → ±5 V)
///
/// ```js
/// $curve(lfo, 2)       // quadratic curve
/// $curve(signal, 3)    // cubic curve (audio taper)
/// ```
#[module(name = "$curve", args(input, exp))]
#[derive(Default)]
pub struct Curve {
    outputs: CurveOutputs,
    params: CurveParams,
}

impl Curve {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();

        for i in 0..channels as usize {
            let x = self.params.input.get_value(i);
            let exp = self.params.exp.get_value(i).max(0.0);

            let normalized = (x.abs() / 5.0).max(0.0);
            let curved = x.signum() * 5.0 * normalized.powf(exp);

            self.outputs.sample.set(i, curved);
        }
    }
}

message_handlers!(impl Curve {});
````

**Step 2: Verify it compiles in isolation**

No standalone compile step needed — will verify after registration in Task 2.

---

### Task 2: Register `$curve` in the utilities module

**Files:**

- Modify: `crates/modular_core/src/dsp/utilities/mod.rs`

**Step 1: Add the module declaration and registration**

Add `pub mod curve;` to the module declarations (after `clamp`, alphabetically).

Add these four lines to the four registration functions:

- `install_constructors`: `curve::Curve::install_constructor(map);`
- `install_param_validators`: `curve::Curve::install_params_validator(map);`
- `install_params_deserializers`: `curve::Curve::install_params_deserializer(map);`
- `schemas`: `curve::Curve::get_schema(),`

**Step 2: Verify it compiles**

Run: `cargo build -p modular_core 2>&1 | tail -5`
Expected: successful build

---

### Task 3: Add Rust tests for `$curve`

**Files:**

- Modify: `crates/modular_core/tests/dsp_fresh_tests.rs`

**Step 1: Write unit tests**

Add these tests to `dsp_fresh_tests.rs`:

```rust
#[test]
fn curve_linear_passthrough() {
    // exp=1 should be linear: output ≈ input
    let m = make_module("$curve", "curve-1");
    set_params(&**m, json!({ "input": 3.0, "exp": 1.0 }), 1);
    for _ in 0..500 {
        step(&**m);
    }
    let sample = m.get_poly_sample(DEFAULT_PORT).unwrap().get(0);
    assert!(approx_eq(sample, 3.0, 0.1), "exp=1 should pass through, got {sample}");
}

#[test]
fn curve_unity_at_5v() {
    // At 5V input, output should be 5V regardless of exponent
    let m = make_module("$curve", "curve-2");
    set_params(&**m, json!({ "input": 5.0, "exp": 3.0 }), 1);
    for _ in 0..500 {
        step(&**m);
    }
    let sample = m.get_poly_sample(DEFAULT_PORT).unwrap().get(0);
    assert!(approx_eq(sample, 5.0, 0.1), "5V should stay 5V, got {sample}");
}

#[test]
fn curve_cubic_midpoint() {
    // exp=3, input=2.5: output = 5 * (2.5/5)^3 = 5 * 0.125 = 0.625
    let m = make_module("$curve", "curve-3");
    set_params(&**m, json!({ "input": 2.5, "exp": 3.0 }), 1);
    for _ in 0..500 {
        step(&**m);
    }
    let sample = m.get_poly_sample(DEFAULT_PORT).unwrap().get(0);
    assert!(approx_eq(sample, 0.625, 0.1), "expected ~0.625, got {sample}");
}

#[test]
fn curve_preserves_sign() {
    // Negative input should produce negative output
    let m = make_module("$curve", "curve-4");
    set_params(&**m, json!({ "input": -2.5, "exp": 2.0 }), 1);
    for _ in 0..500 {
        step(&**m);
    }
    let sample = m.get_poly_sample(DEFAULT_PORT).unwrap().get(0);
    // sign(-2.5) * 5 * (2.5/5)^2 = -1 * 5 * 0.25 = -1.25
    assert!(approx_eq(sample, -1.25, 0.1), "expected ~-1.25, got {sample}");
}

#[test]
fn curve_zero_input() {
    // Zero input should produce zero output
    let m = make_module("$curve", "curve-5");
    set_params(&**m, json!({ "input": 0.0, "exp": 3.0 }), 1);
    for _ in 0..500 {
        step(&**m);
    }
    let sample = m.get_poly_sample(DEFAULT_PORT).unwrap().get(0);
    assert!(approx_eq(sample, 0.0, 0.01), "0V input should produce 0V, got {sample}");
}

#[test]
fn curve_exp_zero_step_function() {
    // exp=0: any nonzero input → ±5V
    let m = make_module("$curve", "curve-6");
    set_params(&**m, json!({ "input": 1.0, "exp": 0.0 }), 1);
    for _ in 0..500 {
        step(&**m);
    }
    let sample = m.get_poly_sample(DEFAULT_PORT).unwrap().get(0);
    assert!(approx_eq(sample, 5.0, 0.1), "exp=0 nonzero input should → 5V, got {sample}");
}
```

**Step 2: Run the tests**

Run: `cargo test -p modular_core --test dsp_fresh_tests curve 2>&1 | tail -20`
Expected: all 6 tests pass

---

### Task 4: Add `.gain()` and `.exp()` to `GraphBuilder.ts`

**Files:**

- Modify: `src/main/dsl/GraphBuilder.ts`

**Step 1: Add the `GAIN_CURVE_EXP` constant**

Near the top of the file (after `PORT_MAX_CHANNELS`), add:

```typescript
/** Exponent used by .gain() for perceptual amplitude curve */
const GAIN_CURVE_EXP = 3;
```

**Step 2: Add methods to `BaseCollection`**

Add after the existing `shift()` method (around line 136):

```typescript
    /**
     * Scale all outputs by a factor with a perceptual (audio taper) curve.
     * Chains $curve → $scaleAndShift with exponent 3.
     */
    gain(level: PolySignal): Collection {
        if (this.items.length === 0) return new Collection();
        const curveFactory = this.items[0].builder.getFactory('$curve');
        const scaleFactory = this.items[0].builder.getFactory('$scaleAndShift');
        if (!curveFactory || !scaleFactory) {
            throw new Error('Factory for $curve or $scaleAndShift not registered');
        }
        const curvedLevel = curveFactory(level, GAIN_CURVE_EXP);
        return scaleFactory(this.items, curvedLevel) as Collection;
    }

    /**
     * Apply a power curve to all outputs. Creates a $curve module internally.
     */
    exp(factor: PolySignal = GAIN_CURVE_EXP): Collection {
        if (this.items.length === 0) return new Collection();
        const factory = this.items[0].builder.getFactory('$curve');
        if (!factory) {
            throw new Error('Factory for $curve not registered');
        }
        return factory(this.items, factor) as Collection;
    }
```

**Step 3: Add methods to `ModuleOutput`**

Add after the existing `shift()` method (around line 960):

```typescript
    /**
     * Scale this output by a factor with a perceptual (audio taper) curve.
     * Chains $curve → $scaleAndShift with exponent 3.
     */
    gain(level: Value): Collection {
        const curveFactory = this.builder.getFactory('$curve');
        const scaleFactory = this.builder.getFactory('$scaleAndShift');
        if (!curveFactory || !scaleFactory) {
            throw new Error('Factory for $curve or $scaleAndShift not registered');
        }
        const curvedLevel = curveFactory(level, GAIN_CURVE_EXP);
        return scaleFactory(this, curvedLevel) as Collection;
    }

    /**
     * Apply a power curve to this output. Creates a $curve module internally.
     */
    exp(factor: Value = GAIN_CURVE_EXP): Collection {
        const factory = this.builder.getFactory('$curve');
        if (!factory) {
            throw new Error('Factory for $curve not registered');
        }
        return factory(this, factor) as Collection;
    }
```

---

### Task 5: Add reserved output names

**Files:**

- Modify: `crates/reserved_output_names.rs`

**Step 1: Add `gain` and `exp` to the reserved names list**

Add under the `// ModuleOutput methods` section:

```rust
    "gain",
    "exp",
```

---

### Task 6: Add type declarations in `typescriptLibGen.ts`

**Files:**

- Modify: `src/main/dsl/typescriptLibGen.ts`

**Step 1: Add to `ModuleOutput` interface**

After the `shift()` declaration (around line 345), add:

```typescript
  /**
   * Scale the signal by a factor with a perceptual (audio taper) curve.
   * Chains \\$curve → \\$scaleAndShift with exponent 3.
   * @param level - Amplitude level as {@link Poly<Signal>}
   * @returns The scaled {@link Collection} for chaining
   * @example osc.gain(2.5)  // Perceptual half volume
   */
  gain(level: Poly<Signal>): Collection;

  /**
   * Apply a power curve to this signal. Creates a \\$curve module internally.
   * @param factor - Exponent for the curve (default 3)
   * @returns The curved {@link Collection} for chaining
   * @example lfo.exp(2)  // Quadratic curve
   */
  exp(factor?: Poly<Signal>): Collection;
```

**Step 2: Add to `BaseCollection` class**

After the `shift()` declaration (around line 493), add:

```typescript
  /**
   * Scale all signals by a factor with a perceptual (audio taper) curve.
   * @param level - Amplitude level as {@link Poly<Signal>}
   * @see {@link ModuleOutput.gain}
   */
  gain(level: Poly<Signal>): Collection;

  /**
   * Apply a power curve to all signals. Creates a \\$curve module internally.
   * @param factor - Exponent for the curve (default 3)
   * @see {@link ModuleOutput.exp}
   */
  exp(factor?: Poly<Signal>): Collection;
```

---

### Task 7: Add method documentation in `typeDocs.ts`

**Files:**

- Modify: `src/shared/dsl/typeDocs.ts`

**Step 1: Add to `ModuleOutput` methods array**

After the `shift` method entry (around line 136), add:

```typescript
            {
                name: 'gain',
                signature: 'gain(level: Poly<Signal>): ModuleOutput',
                description:
                    'Scale the signal with a perceptual (audio taper) curve. Chains $curve and $scaleAndShift internally with exponent 3.',
                example: 'osc.gain(2.5)  // Perceptual half volume',
            },
            {
                name: 'exp',
                signature: 'exp(factor?: Poly<Signal>): ModuleOutput',
                description:
                    'Apply a power curve to the signal. Creates a $curve module internally. Default exponent is 3.',
                example: 'lfo.exp(2)  // Quadratic curve',
            },
```

**Step 2: Add to `Collection` methods array**

After the `shift` method entry (around line 243), add:

```typescript
            {
                name: 'gain',
                signature: 'gain(level: Poly<Signal>): Collection',
                description:
                    'Scale all signals with a perceptual (audio taper) curve.',
                example: '$c(osc1, osc2).gain(2.5)',
            },
            {
                name: 'exp',
                signature: 'exp(factor?: Poly<Signal>): Collection',
                description:
                    'Apply a power curve to all signals in the collection. Default exponent is 3.',
                example: '$c(lfo1, lfo2).exp(2)',
            },
```

---

### Task 8: Add TypeScript executor tests

**Files:**

- Modify: `src/main/dsl/__tests__/executor.test.ts`

**Step 1: Add tests for `.gain()`, `.exp()`, and `$curve`**

In the `chaining methods` describe block (after the `.shift()` test around line 285), add:

```typescript
test('.gain() creates curve and scaleAndShift modules', () => {
    const patch = execPatch('$sine("C4").gain(2.5).out()');
    expect(findModules(patch, '$sine').length).toBe(1);
    expect(findModules(patch, '$curve').length).toBeGreaterThan(0);
    expect(findModules(patch, '$scaleAndShift').length).toBeGreaterThan(0);
});

test('.exp() creates a curve module', () => {
    const patch = execPatch('$sine("C4").exp(2).out()');
    expect(findModules(patch, '$sine').length).toBe(1);
    expect(findModules(patch, '$curve').length).toBeGreaterThan(0);
});

test('.exp() with default factor creates a curve module', () => {
    const patch = execPatch('$sine("C4").exp().out()');
    expect(findModules(patch, '$sine').length).toBe(1);
    expect(findModules(patch, '$curve').length).toBeGreaterThan(0);
});
```

In the utilities describe block (after the `$scaleAndShift` test around line 372), add:

```typescript
test('$curve', () => {
    const patch = execPatch('$curve($sine("C4"), 2).out()');
    expect(findModules(patch, '$curve').length).toBeGreaterThan(0);
});
```

**Step 2: Run the tests**

Run: `npx vitest run src/main/dsl/__tests__/executor.test.ts 2>&1 | tail -20`
Expected: all tests pass

---

### Task 9: Build verification

**Step 1: Run the full Rust test suite**

Run: `cargo test -p modular_core 2>&1 | tail -10`
Expected: all tests pass

**Step 2: Run the full TypeScript test suite**

Run: `npx vitest run 2>&1 | tail -20`
Expected: all tests pass

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add \$curve module with .gain() and .exp() DSL methods for perceptual amplitude control"
```
