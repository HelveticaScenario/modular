# Design: `$clamp` update + `$wrap` new module

Date: 2026-03-04

## Summary

Update the existing `$clamp` module and add a new `$wrap` module to the utilities category.
Both modules take three `PolySignal` params (`input`, `min`, `max`) and produce a single `PolyOutput`.
Both swap `min` and `max` automatically when `max < min`.

---

## `$clamp` — update to existing module

**File:** `crates/modular_core/src/dsp/utilities/clamp.rs`

**Change:** add min/max swap. When both `min` and `max` are connected and `max_val < min_val`,
swap them before clamping. Existing optional behavior (no bound applied when a side is disconnected)
is preserved. Positional arg mapping (`args(input)`) is unchanged.

**Pseudocode per channel:**

```
val = input.get_value(i)
if has_min && has_max:
    (lo, hi) = if max < min: (max, min) else: (min, max)
    val = clamp(val, lo, hi)
else if has_min:
    val = max(val, min)
else if has_max:
    val = min(val, max)
```

---

## `$wrap` — new module

**File:** `crates/modular_core/src/dsp/utilities/wrap.rs`

**Params:**

| Name    | Type         | Default | Positional |
| ------- | ------------ | ------- | ---------- |
| `input` | `PolySignal` | —       | 1st        |
| `min`   | `PolySignal` | 0.0     | 2nd        |
| `max`   | `PolySignal` | 5.0     | 3rd        |

**Output:** single `PolyOutput` (default output).

**Registration:** same 4-line pattern in `utilities/mod.rs` (`pub mod`, + 4 install calls).

**Algorithm (per channel):**

```
min = min.get_value_or(i, 0.0)
max = max.get_value_or(i, 5.0)
(min, max) = if max < min: (max, min) else: (min, max)
if max == min: output min; continue

span = max - min
offset = (val - min) % span
if offset < 0: offset += span
output = min + offset
```

Modulo-based, O(1) per channel regardless of distance from range.
Degenerate zero-width range: output `min` directly to avoid division by zero.

**DSL usage:**

```js
$wrap(ramp, 0, 5); // positional: wrap into 0–5 V
$wrap(lfo, { min: -1, max: 1 }); // named: wrap into -1–1 V
$wrap(signal); // defaults: wrap into 0–5 V
```

**No TypeScript changes needed** — the schema-driven factory auto-generates the DSL wrapper.

---

## Out of scope

- No changes to `src/main/dsl/factories.ts`
- No changes to N-API bindings beyond the normal `yarn build-native` regeneration
- No new proc-macro patterns — follows the identical `clamp.rs` structure
