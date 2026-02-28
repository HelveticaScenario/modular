# Signal Param Metadata Design

## Problem

Module params that are signal inputs (PolySignal/MonoSignal) have no way to declare metadata about themselves. Outputs already have rich metadata via `#[output("name", "desc", default, range = (-5.0, 5.0))]`, but params lack equivalents. The editor needs this metadata to auto-generate sliders with the correct label, initial value, min, and max.

## Solution

Add a `#[signal()]` field attribute to params struct fields. The metadata is:

- **type**: `pitch | gate | trig | control` — label only, no DSP behavior change
- **default**: signal default value in volts
- **range**: `(min, max)` in volts

## Rust-Side Declaration

```rust
#[derive(Clone, Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct LowpassFilterParams {
    /// signal input
    input: PolySignal,  // unannotated → control, 0.0, (-5.0, 5.0)

    /// cutoff frequency in V/Oct (0V = C4)
    #[signal(type = pitch, default = 0.0, range = (-5.0, 5.0))]
    cutoff: PolySignal,

    /// filter resonance (0-5)
    #[signal(type = control, default = 0.0, range = (0.0, 5.0))]
    resonance: PolySignal,
}
```

All three sub-attributes are optional. Omitted values use defaults.

## Defaults for Unannotated Signal Params

| Field           | Default     |
| --------------- | ----------- |
| `signal_type`   | `"control"` |
| `default_value` | `0.0`       |
| `min_value`     | `-5.0`      |
| `max_value`     | `5.0`       |

Non-signal fields (numbers, enums, bools) get no SignalParamSchema entry.

## Schema Changes (Rust)

Extend `SignalParamSchema` in `types.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct SignalParamSchema {
    pub name: String,
    pub description: String,
    pub signal_type: String,    // "pitch" | "gate" | "trig" | "control"
    pub default_value: f64,
    pub min_value: f64,
    pub max_value: f64,
}
```

Add to `ModuleSchema`:

```rust
pub signal_params: Vec<SignalParamSchema>,
```

## Proc Macro Changes

### New trait: `SignalParamMeta`

```rust
pub trait SignalParamMeta {
    fn signal_param_schemas() -> Vec<SignalParamSchema> where Self: Sized;
}
```

### New derive: extend `Connect` derive (or add `SignalParams` derive)

The macro:

1. Iterates fields in the params struct
2. For each field whose type contains `PolySignal` or `MonoSignal`:
    - Parse any `#[signal()]` attribute
    - Extract doc comments for description
    - Generate a `SignalParamSchema` entry with declared or default values
3. Generates `SignalParamMeta` impl returning the collected schemas

### `#[module]` macro changes

In `get_schema()`, call `<ParamsStruct as SignalParamMeta>::signal_param_schemas()` and include the result in the `ModuleSchema`.

## Frontend Changes (TypeScript)

Extend `ParamDescriptor` in `paramsSchema.ts`:

```typescript
export type SignalType = 'pitch' | 'gate' | 'trig' | 'control';

export type ParamDescriptor = {
    name: string;
    kind: ParamKind;
    description?: string;
    optional: boolean;
    enumValues?: string[];
    signalType?: SignalType;
    defaultValue?: number;
    minValue?: number;
    maxValue?: number;
};
```

In `processModuleSchema()`, merge `schema.signalParams` entries into the `ParamDescriptor` list by matching on `name`.

## Scope

- Metadata is label-only — no DSP behavior changes
- Only applies to PolySignal and MonoSignal fields
- Primary use case: editor auto-generates $slider with correct label/initial/min/max
