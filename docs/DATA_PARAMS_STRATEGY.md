# Strategy for Arbitrary Data Parameters in Modules

## Executive Summary

This document outlines a strategy for extending the modular synthesizer's parameter system to support arbitrary data parameters alongside the existing `InternalParam` system. The goal is to enable modules to accept configuration data (strings, enums, arrays, objects) that don't require the real-time audio capabilities of regular parameters (cable routing, track automation, multichannel processing) while maintaining strong typing, validation, and DSL integration.

## Current Architecture Overview

### Existing Parameter System

The current system has two parallel representations of parameters:

1. **`InternalParam`** (Runtime, in `modular_core/src/types.rs`):
   ```rust
   pub enum InternalParam {
       Volts { value: ChannelBuffer },      // Static multichannel values
       Cable { module, port },               // Audio routing connections
       Track { track },                      // Automation references
       Disconnected,                         // No value
   }
   ```
   - Used during real-time audio processing
   - Supports 16-channel processing (`ChannelBuffer`)
   - Enables dynamic routing and automation
   - Must be lock-free and non-blocking in audio thread

2. **`Param`** (Serialization, in `modular_core/src/types.rs`):
   ```rust
   pub enum Param {
       Value { value: ChannelBuffer },
       Cable { module: String, port: String },
       Track { track: String },
       Disconnected,
   }
   ```
   - Serialized to/from JSON for DSL communication
   - TypeScript types auto-generated via `ts-rs`
   - Validated against module schemas before applying to patch

### Module Parameter Flow

1. **DSL (JavaScript)** → User writes: `sine('osc').freq(hz(440))`
2. **GraphBuilder (TS)** → Converts to `PatchGraph` JSON with `ModuleState.params: HashMap<String, Param>`
3. **Validation (Rust)** → Checks against `ModuleSchema.params` to ensure param names exist
4. **Conversion** → `Param::to_internal_param()` creates runtime `InternalParam`
5. **Module Update** → `Params::update_param()` writes to module's parameter struct
6. **Audio Processing** → `update()` method reads `InternalParam` values each frame

### Derive Macro System

Modules use proc macros to generate boilerplate:

```rust
#[derive(Default, Params)]
struct SineOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("phase", "the phase")]
    phase: InternalParam,
}

#[derive(Default, Module)]
#[module("sine", "A sine wave oscillator")]
pub struct SineOscillator {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    phase: ChannelBuffer,
    params: SineOscillatorParams,
}
```

The `Params` derive generates:
- `get_params_state()` - Serializes params to `HashMap<String, Param>`
- `update_param()` - Updates a single param from `InternalParam`
- `get_schema()` - Returns `Vec<ParamSchema>` for validation

## Limitations of Current System

The current `InternalParam` system has significant limitations for configuration data:

1. **No Native Data Types**: Cannot represent strings, enums, booleans, or complex objects
2. **Forced Multichannel**: All values use `ChannelBuffer` ([f32; 16]) even when single values needed
3. **Audio-Centric**: System designed for real-time audio signals, not static configuration
4. **Type Safety**: No compile-time checking of parameter value types
5. **Validation Gaps**: Schema only validates param name existence, not value types or constraints
6. **DSL Ergonomics**: Users must work around system (e.g., encoding enums as voltages)

### Use Cases for Data Parameters

Several module types would benefit from data parameters:

- **Wave Table Oscillator**: Waveform selection ("sine", "saw", "square", "triangle")
- **Sample Player**: File path string, loop mode enum
- **Sequencer**: Step data arrays, gate patterns
- **Filter**: Filter type enum ("lowpass", "highpass", "bandpass", "notch")
- **Quantizer**: Scale definition (array of note intervals)
- **Delay**: Buffer size integer, interpolation mode enum
- **Reverb**: Algorithm selection, preset name
- **Effects**: Dry/wet mode ("parallel", "series"), routing config

## Proposed Strategy

### Core Concept: Parallel Parameter Systems

Introduce a **parallel data parameter system** that coexists with `InternalParam` without replacing it:

- **Audio Parameters** (`InternalParam`): For real-time signals, cable routing, automation, multichannel
- **Data Parameters** (new `DataParam`): For static configuration, type-safe values, no real-time overhead

This separation maintains the performance characteristics of the audio thread while adding flexibility for configuration.

### 1. Type System Design

#### Rust Side: `DataParam` Enum

Add a new enum in `modular_core/src/types.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub enum DataParam {
    String { value: String },
    Int { value: i64 },
    Float { value: f64 },
    Bool { value: bool },
    Enum { variant: String },
    Array { values: Vec<DataParam> },
    Object { fields: HashMap<String, DataParam> },
}
```

**Design Rationale**:
- Tagged union for type safety and pattern matching
- Serialization-friendly (no Rust-specific types like `Weak` pointers)
- Composable (arrays and objects can nest)
- TypeScript-compatible via `ts-rs`
- No real-time audio concerns (no locks, no channels)

#### Schema Extension: `ParamType`

Extend `ParamSchema` to include type information:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum ParamType {
    Audio,      // Current InternalParam behavior
    String,
    Int { min: Option<i64>, max: Option<i64> },
    Float { min: Option<f64>, max: Option<f64> },
    Bool,
    Enum { variants: Vec<String> },
    Array { element_type: Box<ParamType>, min_length: Option<usize>, max_length: Option<usize> },
    Object { fields: HashMap<String, ParamType> },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ParamSchema {
    pub name: String,
    pub description: String,
    pub param_type: ParamType,  // New field
    #[serde(default)]
    pub optional: bool,
    pub default: Option<DataParam>,  // For data params only
}
```

**Design Rationale**:
- Explicit type declarations enable validation
- Constraints (min/max, variants) enforced at patch update
- Optional and default values improve ergonomics
- Backward compatible (add default value for existing audio params)

### 2. Module Structure Design

#### Separate Parameter Structs

Modules should declare audio and data parameters separately:

```rust
// Example: Wave table oscillator

#[derive(Default, Params)]
struct WaveTableParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("phase", "phase offset")]
    phase: InternalParam,
}

#[derive(Default, DataParams)]  // New derive macro
struct WaveTableData {
    #[data_param("waveform", "waveform type", enum("sine", "saw", "square", "triangle"))]
    waveform: String,
    #[data_param("octave", "octave shift", int(-4, 4))]
    octave: i64,
}

#[derive(Module)]
#[module("wavetable", "Wave table oscillator")]
pub struct WaveTableOscillator {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    phase: ChannelBuffer,
    params: WaveTableParams,
    data: WaveTableData,  // New field
}
```

**Design Rationale**:
- Clear separation of concerns (audio signals vs. configuration)
- Type-safe Rust fields (String, i64, bool directly)
- Attribute macros declare validation rules
- Generated code handles serialization/deserialization

#### Derive Macro: `DataParams`

Create a new proc macro `#[derive(DataParams)]` in `modular_derive/src/lib.rs`:

**Generated Methods**:
1. `get_data_state() -> HashMap<String, DataParam>` - Serialize to JSON
2. `update_data(&mut self, name: &str, value: &DataParam) -> Result<()>` - Deserialize from JSON
3. `get_data_schema() -> Vec<ParamSchema>` - Generate schema with types

**Attribute Syntax**:
- `#[data_param("name", "description", type_constraint)]`
- Type constraints: `string`, `int(min, max)`, `float(min, max)`, `bool`, `enum("a", "b")`, `array(inner_type)`, `object`
- Optional: `#[data_param("name", "desc", string, optional)]`
- Default: `#[data_param("name", "desc", string, default = "value")]`

### 3. Validation Strategy

#### Validation Flow

1. **DSL Build Time**: TypeScript type checking ensures correct types passed to param methods
2. **Patch Submission**: `validate_patch()` in `validation.rs` checks:
   - All param names exist in schema
   - Data param values match declared types
   - Enum values in allowed variants
   - Numeric values within min/max bounds
   - Array lengths within constraints
   - Required params are present
3. **Patch Application**: `update_data()` performs final type checks before updating module

#### Validation Implementation

Extend `validation.rs` with new function:

```rust
fn validate_data_param(
    param_name: &str,
    param_value: &DataParam,
    param_schema: &ParamSchema,
    location: &str,
) -> Result<(), ValidationError> {
    // Type matching
    // Constraint checking (bounds, enum variants, array length)
    // Nested validation for arrays/objects
}
```

**Key Validation Rules**:
- `DataParam::String` only valid if schema type is `ParamType::String`
- `DataParam::Enum` variant must be in schema's allowed variants
- `DataParam::Int` must be within min/max if specified
- `DataParam::Array` elements must all match schema's element type
- `DataParam::Object` fields must match schema's field definitions
- Audio params (`Param`) cannot be used for data param fields and vice versa

### 4. TypeScript Integration

#### Type Generation

Use `ts-rs` to generate TypeScript types automatically:

```typescript
// Generated in modular_web/src/types/generated/

export type DataParam =
  | { type: "string"; value: string }
  | { type: "int"; value: number }
  | { type: "float"; value: number }
  | { type: "bool"; value: boolean }
  | { type: "enum"; variant: string }
  | { type: "array"; values: DataParam[] }
  | { type: "object"; fields: Record<string, DataParam> };

export type ParamType =
  | { type: "audio" }
  | { type: "string" }
  | { type: "int"; min?: number; max?: number }
  | { type: "float"; min?: number; max?: number }
  | { type: "bool" }
  | { type: "enum"; variants: string[] }
  | { type: "array"; elementType: ParamType; minLength?: number; maxLength?: number }
  | { type: "object"; fields: Record<string, ParamType> };

export interface ParamSchema {
  name: string;
  description: string;
  paramType: ParamType;
  optional?: boolean;
  default?: DataParam;
}
```

#### DSL API Design

Extend `ModuleNode` class to support data parameters:

```typescript
class ModuleNode {
  // Existing audio param methods
  freq(value: ParamInput): this { ... }
  
  // New data param methods (type-safe based on schema)
  setData(name: string, value: string | number | boolean | DataParam): this {
    // Convert JS primitives to DataParam
    // Call builder.setDataParam(moduleId, name, dataParam)
    return this;
  }
}
```

**Type-Safe Factory Generation**:

Use schema information to generate strongly-typed factory functions:

```typescript
// Generated per module type
interface WaveTableOscillatorParams {
  waveform?: "sine" | "saw" | "square" | "triangle";  // Enum constraint from schema
  octave?: number;  // Int constraint from schema
}

function wavetable(
  id?: string,
  data?: WaveTableOscillatorParams
): ModuleNode {
  const node = context.getBuilder().addModule("wavetable", id);
  if (data) {
    if (data.waveform) node.setData("waveform", data.waveform);
    if (data.octave) node.setData("octave", data.octave);
  }
  return node;
}
```

**DSL Usage Examples**:

```javascript
// String data param
const osc = wavetable("osc1", { waveform: "saw", octave: 1 });
osc.freq(note("a4"));

// Enum via method (alternative API)
const filter = lowpass("filt1").filterType("butterworth");

// Array data param
const quantizer = quantize("q1", {
  scale: [0, 2, 4, 5, 7, 9, 11]  // Major scale in semitones
});

// Boolean data param
const delay = delay("d1", { feedback: true });
```

### 5. Serialization Format

#### ModuleState Extension

Extend `ModuleState` to include data parameters:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ModuleState {
    pub id: String,
    pub module_type: String,
    pub params: HashMap<String, Param>,        // Audio params (unchanged)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub data: HashMap<String, DataParam>,      // Data params (new)
}
```

**JSON Example**:

```json
{
  "modules": [
    {
      "id": "wavetable-1",
      "moduleType": "wavetable",
      "params": {
        "freq": { "type": "value", "value": [4.0, 4.0, ...] },
        "phase": { "type": "cable", "module": "lfo-1", "port": "output" }
      },
      "data": {
        "waveform": { "type": "enum", "variant": "saw" },
        "octave": { "type": "int", "value": 1 }
      }
    }
  ]
}
```

**Design Rationale**:
- Separate `params` and `data` fields make distinction clear
- Backward compatible: existing patches without `data` field still work
- `skip_serializing_if` keeps JSON clean for modules without data params

### 6. Performance Considerations

#### Audio Thread Safety

Data parameters are **read-only during audio processing**:

1. **Initialization**: Data params copied to module during construction
2. **Hot Reload**: Full module recreation when data params change (not in-place update)
3. **No Locking**: Audio thread only reads immutable data

**Why Full Recreation**:
- Data params often affect internal state (buffer sizes, lookup tables, algorithm choice)
- Simpler than implementing custom update logic per module
- Acceptable performance: data changes are rare compared to audio param changes
- Matches current behavior for module type changes

#### Memory Usage

- Data params stored once per module (not per-channel)
- String interning could be added later if many modules share same strings
- Objects/arrays allocated on heap, but not in audio-critical path

### 7. Migration Path

#### Phase 1: Core Infrastructure (No Breaking Changes)

1. Add `DataParam` enum to `types.rs`
2. Extend `ParamSchema` with `param_type` field (default to `Audio` for backward compat)
3. Add `data` field to `ModuleState` (optional, skipped when empty)
4. Generate TypeScript types
5. No module changes yet - system exists but unused

#### Phase 2: Derive Macro and Validation

1. Implement `#[derive(DataParams)]` macro
2. Extend `validate_patch()` with data param validation
3. Update module creation logic to handle data params
4. Test with simple module (e.g., test module with string param)

#### Phase 3: DSL Integration

1. Add `setData()` method to `ModuleNode`
2. Implement schema-based factory generation
3. Update type generation script
4. Add autocomplete support in editor

#### Phase 4: Module Ecosystem

1. Create utility modules that benefit from data params
2. Document patterns and best practices
3. Migrate existing "workaround" modules (if any)

#### Backward Compatibility

**Existing Patches**: Continue to work without modification
- `data` field optional in JSON
- Existing modules don't have `DataParams` struct (allowed)
- Validation permits empty data field

**Existing Modules**: Can be migrated incrementally
- Add `DataParams` struct without removing existing fields
- No changes to `InternalParam` usage
- Audio parameter behavior unchanged

### 8. Alternative Approaches Considered

#### Alternative A: Extend InternalParam

**Idea**: Add variants to `InternalParam` enum for data types
```rust
enum InternalParam {
    Volts { value: ChannelBuffer },
    Cable { ... },
    Track { ... },
    String { value: String },  // New
    Int { value: i64 },        // New
    // ...
}
```

**Rejected Because**:
- Pollutes audio processing code with non-audio concerns
- Every `get_value()` call must handle data types (performance overhead)
- Type confusion: string param could be misconnected to cable
- Multichannel system doesn't make sense for scalars

#### Alternative B: JSON Blobs in Volts

**Idea**: Encode data as JSON strings, store in voltage parameters

**Rejected Because**:
- No type safety
- Validation impossible
- DSL becomes unergonomic
- Parse overhead in audio thread
- Hacky and unmaintainable

#### Alternative C: Separate Module Config System

**Idea**: Data params live outside module graph in separate config structure

**Rejected Because**:
- Breaks conceptual model (params are properties of modules)
- Validation becomes disconnected from patch validation
- DSL would need separate API for config vs params
- Serialization more complex

### 9. Open Questions and Future Considerations

#### Hot-Reload Optimization

**Question**: Can we optimize data param updates to avoid full module recreation in some cases?

**Future Work**: 
- Add `fn update_data_param(&mut self, name: &str, value: &DataParam) -> Result<bool>` trait method
- Return `true` if hot-reload possible, `false` if recreation needed
- Modules implement logic based on their needs
- Falls back to recreation if not implemented

#### Schema Evolution

**Question**: How to handle schema changes between versions?

**Future Work**:
- Version field in `ModuleSchema`
- Migration functions for old patches
- Default values for new params
- Deprecation warnings

#### Complex Constraints

**Question**: How to express complex validation rules (e.g., "param A required if param B is true")?

**Future Work**:
- Add `constraints` field to `ModuleSchema` with expression language
- Custom validation functions registered per module type
- Start simple, add complexity as needed

#### Visual Editors

**Question**: How to provide UI for editing data params?

**Future Work**:
- Generate form widgets based on schema types
- Enum params → dropdown
- Int/Float with bounds → slider
- Bool → checkbox
- Arrays → list editor
- Integration with GraphBuilder for visual patching

## Implementation Checklist

When implementing this strategy:

### Core Types (modular_core)
- [ ] Add `DataParam` enum to `types.rs`
- [ ] Add `ParamType` enum to `types.rs`
- [ ] Extend `ParamSchema` with type information
- [ ] Add `data` field to `ModuleState`
- [ ] Update `Patch` to handle data params during construction

### Validation (modular_server)
- [ ] Implement `validate_data_param()` in `validation.rs`
- [ ] Extend `validate_patch()` to validate data params
- [ ] Add type checking for all `DataParam` variants
- [ ] Add constraint validation (bounds, enums, arrays)

### Derive Macros (modular_derive)
- [ ] Create `#[derive(DataParams)]` proc macro
- [ ] Implement attribute parsing for `#[data_param(...)]`
- [ ] Generate `get_data_state()` method
- [ ] Generate `update_data()` method
- [ ] Generate `get_data_schema()` method
- [ ] Update `#[derive(Module)]` to handle data params

### TypeScript Integration (modular_web)
- [ ] Generate TypeScript types for `DataParam`
- [ ] Generate TypeScript types for `ParamType`
- [ ] Update `ModuleState` type generation
- [ ] Add `setData()` method to `ModuleNode`
- [ ] Implement schema-based factory generation
- [ ] Update autocomplete to include data params

### Testing
- [ ] Unit tests for `DataParam` serialization
- [ ] Unit tests for validation logic
- [ ] Integration tests for patch with data params
- [ ] DSL tests for type safety
- [ ] Backward compatibility tests

### Documentation
- [ ] Update DSL guide with data param examples
- [ ] Document derive macro attributes
- [ ] Add migration guide for existing modules
- [ ] Create best practices document

## Conclusion

This strategy provides a path to supporting arbitrary data parameters while:

✅ **Maintaining Type Safety**: Rust enums + schema validation + TypeScript types
✅ **Preserving Performance**: No audio thread impact, immutable during processing
✅ **Enabling Validation**: Schema-based validation at patch submission
✅ **Supporting Strong DSL**: TypeScript factories with type constraints from schemas
✅ **Ensuring Compatibility**: Existing patches and modules unchanged
✅ **Allowing Incremental Adoption**: Modules can be migrated one at a time

The parallel parameter system (audio params vs. data params) clearly separates real-time signal processing concerns from static configuration, making the codebase easier to understand and maintain while adding powerful new capabilities.
