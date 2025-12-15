# Data Parameters: Derive Macro Implementation Guide

This document provides detailed specifications for implementing the `#[derive(DataParams)]` procedural macro and integrating it with the existing `#[derive(Module)]` macro. **This is planning documentation only** - no code should be implemented yet.

## Table of Contents

1. [Macro Architecture](#macro-architecture)
2. [Attribute Syntax](#attribute-syntax)
3. [Code Generation](#code-generation)
4. [Integration with Module Macro](#integration-with-module-macro)
5. [Error Handling](#error-handling)
6. [Edge Cases](#edge-cases)

## Macro Architecture

### Overview

The `DataParams` derive macro follows the same pattern as the existing `Params` macro but generates different code:

```
#[derive(DataParams)]     →     impl DataParams trait
     ↓                                    ↓
Parse attributes                  get_data_state()
     ↓                            update_data()
Generate code                     get_data_schema()
```

### Files to Modify

- `modular_derive/src/lib.rs` - Add new macro and helper functions
- `modular_core/src/types.rs` - Add `DataParams` trait definition

## Attribute Syntax

### Basic Syntax

```rust
#[data_param("name", "description", type_constraint)]
field_name: RustType,
```

### Type Constraints

Each type constraint maps to both:
1. Rust type validation (compile-time)
2. `ParamType` enum variant (runtime validation)

| Constraint | Rust Type | ParamType | Validation |
|------------|-----------|-----------|------------|
| `string` | `String` | `String` | None |
| `int(min, max)` | `i64` | `Int { min, max }` | Range check |
| `float(min, max)` | `f64` | `Float { min, max }` | Range check |
| `bool` | `bool` | `Bool` | None |
| `enum("a", "b")` | `String` | `Enum { variants }` | Variant check |
| `array(inner, min, max)` | `Vec<T>` | `Array { element_type, min_length, max_length }` | Length + element validation |
| `object` | Custom struct | `Object { fields }` | Field validation |

### Full Attribute Examples

```rust
// String (no constraints)
#[data_param("name", "User-friendly name", string)]
name: String,

// String with optional flag
#[data_param("label", "Optional label", string, optional)]
label: String,

// String with default value
#[data_param("mode", "Operating mode", string, default = "normal")]
mode: String,

// Integer with bounds
#[data_param("count", "Number of items", int(1, 100))]
count: i64,

// Integer with only minimum
#[data_param("steps", "Step count", int(1, ))]
steps: i64,

// Float with bounds
#[data_param("ratio", "Frequency ratio", float(0.0, 10.0))]
ratio: f64,

// Boolean
#[data_param("enabled", "Enable processing", bool)]
enabled: bool,

// Boolean with default
#[data_param("loop", "Loop playback", bool, default = true)]
loop_mode: bool,

// Enum with variants
#[data_param("type", "Filter type", enum("lowpass", "highpass", "bandpass", "notch"))]
filter_type: String,

// Array of floats
#[data_param("values", "Sequence values", array(float(-10.0, 10.0), 1, 32))]
values: Vec<f64>,

// Array of enums
#[data_param("gates", "Gate pattern", array(bool, 1, 16))]
gates: Vec<bool>,

// Nested array (array of arrays)
#[data_param("matrix", "2D data", array(array(float(0.0, 1.0), 2, 2), 2, 2))]
matrix: Vec<Vec<f64>>,
```

## Code Generation

### Trait Definition

First, define the trait in `modular_core/src/types.rs`:

```rust
pub trait DataParams {
    /// Serialize all data parameters to HashMap
    fn get_data_state(&self) -> HashMap<String, DataParam>;
    
    /// Update a single data parameter from a DataParam value
    fn update_data(&mut self, param_name: &str, value: &DataParam) -> Result<()>;
    
    /// Get schema information for all data parameters
    fn get_data_schema() -> Vec<ParamSchema>;
}
```

### Generated Code Structure

For this input:

```rust
#[derive(Default, DataParams)]
struct MyModuleData {
    #[data_param("mode", "Operating mode", enum("forward", "reverse"))]
    mode: String,
    
    #[data_param("speed", "Speed multiplier", float(0.1, 10.0))]
    speed: f64,
    
    #[data_param("enabled", "Enable processing", bool)]
    enabled: bool,
}
```

Generate this implementation:

```rust
impl crate::types::DataParams for MyModuleData {
    fn get_data_state(&self) -> std::collections::HashMap<String, crate::types::DataParam> {
        let mut state = std::collections::HashMap::new();
        
        state.insert(
            "mode".to_owned(),
            crate::types::DataParam::Enum { variant: self.mode.clone() }
        );
        
        state.insert(
            "speed".to_owned(),
            crate::types::DataParam::Float { value: self.speed }
        );
        
        state.insert(
            "enabled".to_owned(),
            crate::types::DataParam::Bool { value: self.enabled }
        );
        
        state
    }
    
    fn update_data(
        &mut self,
        param_name: &str,
        value: &crate::types::DataParam
    ) -> Result<()> {
        match param_name {
            "mode" => {
                if let crate::types::DataParam::Enum { variant } = value {
                    // Validate enum variant
                    match variant.as_str() {
                        "forward" | "reverse" => {
                            self.mode = variant.clone();
                            Ok(())
                        }
                        _ => Err(anyhow!(
                            "Invalid variant '{}' for param 'mode'. Must be one of: forward, reverse",
                            variant
                        ))
                    }
                } else {
                    Err(anyhow!(
                        "Type mismatch for param 'mode': expected Enum, got {:?}",
                        value
                    ))
                }
            }
            
            "speed" => {
                if let crate::types::DataParam::Float { value: v } = value {
                    // Validate bounds
                    if *v < 0.1 || *v > 10.0 {
                        return Err(anyhow!(
                            "Value {} for param 'speed' is out of bounds [0.1, 10.0]",
                            v
                        ));
                    }
                    self.speed = *v;
                    Ok(())
                } else {
                    Err(anyhow!(
                        "Type mismatch for param 'speed': expected Float, got {:?}",
                        value
                    ))
                }
            }
            
            "enabled" => {
                if let crate::types::DataParam::Bool { value: v } = value {
                    self.enabled = *v;
                    Ok(())
                } else {
                    Err(anyhow!(
                        "Type mismatch for param 'enabled': expected Bool, got {:?}",
                        value
                    ))
                }
            }
            
            _ => Err(anyhow!(
                "'{}' is not a valid data param name for MyModuleData",
                param_name
            ))
        }
    }
    
    fn get_data_schema() -> Vec<crate::types::ParamSchema> {
        vec![
            crate::types::ParamSchema {
                name: "mode".to_string(),
                description: "Operating mode".to_string(),
                param_type: crate::types::ParamType::Enum {
                    variants: vec!["forward".to_string(), "reverse".to_string()],
                },
                optional: false,
                default: None,
            },
            crate::types::ParamSchema {
                name: "speed".to_string(),
                description: "Speed multiplier".to_string(),
                param_type: crate::types::ParamType::Float {
                    min: Some(0.1),
                    max: Some(10.0),
                },
                optional: false,
                default: None,
            },
            crate::types::ParamSchema {
                name: "enabled".to_string(),
                description: "Enable processing".to_string(),
                param_type: crate::types::ParamType::Bool,
                optional: false,
                default: None,
            },
        ]
    }
}
```

### Parsing Attributes

The attribute parsing needs to handle nested syntax:

```rust
fn parse_data_param_attr(attr: &Attribute) -> DataParamAttr {
    // Parse: #[data_param("name", "description", type_constraint, ...flags)]
    
    struct DataParamAttr {
        name: LitStr,
        description: LitStr,
        type_constraint: TypeConstraint,
        optional: bool,
        default: Option<TokenStream2>,
    }
    
    enum TypeConstraint {
        String,
        Int { min: Option<i64>, max: Option<i64> },
        Float { min: Option<f64>, max: Option<f64> },
        Bool,
        Enum { variants: Vec<String> },
        Array {
            element_type: Box<TypeConstraint>,
            min_length: Option<usize>,
            max_length: Option<usize>,
        },
        Object,
    }
}
```

**Parsing Strategy**:

1. Extract tokens from attribute
2. Parse name and description as first two string literals
3. Parse third token as type constraint identifier (`string`, `int`, `float`, etc.)
4. If type constraint has parentheses, parse arguments
5. Parse remaining flags (`optional`, `default = value`)

**Example parsing**:

```rust
// Input: #[data_param("freq", "Frequency", float(20.0, 20000.0), default = 440.0)]

// Step 1: Extract tokens
tokens = ["freq", "Frequency", float(20.0, 20000.0), default = 440.0]

// Step 2: Parse name and description
name = "freq"
description = "Frequency"

// Step 3: Parse type constraint
type_ident = "float"
type_args = [20.0, 20000.0]
type_constraint = Float { min: Some(20.0), max: Some(20000.0) }

// Step 4: Parse flags
has_default = true
default_value = 440.0
```

### Type Mapping

Map Rust types to DataParam constructors:

| Rust Type | DataParam Constructor |
|-----------|-----------------------|
| `String` | `DataParam::String { value: self.field.clone() }` |
| `i64` | `DataParam::Int { value: self.field }` |
| `f64` | `DataParam::Float { value: self.field }` |
| `bool` | `DataParam::Bool { value: self.field }` |
| `String` (enum) | `DataParam::Enum { variant: self.field.clone() }` |
| `Vec<T>` | `DataParam::Array { values: self.field.iter().map(...).collect() }` |

**Challenge: Detecting Enum vs String**

Since both enum constraints and string use `String` as the Rust type, the distinction comes from the attribute:

- `#[data_param("x", "y", string)]` → `DataParam::String`
- `#[data_param("x", "y", enum("a", "b"))]` → `DataParam::Enum`

The macro must track this distinction and generate appropriate code.

## Integration with Module Macro

### Module Macro Changes

The existing `#[derive(Module)]` macro needs to be updated to:

1. Detect if module struct has a field with type that implements `DataParams`
2. Call `get_data_schema()` and merge with audio param schema
3. Handle data param updates during patch application

**Detection Strategy**:

```rust
// In impl_module_macro()

let has_data_field = match ast.data {
    Data::Struct(ref data) => match data.fields {
        Fields::Named(ref fields) => {
            fields.named.iter().any(|f| {
                // Check if field name is "data"
                f.ident.as_ref().map(|i| i == "data").unwrap_or(false)
            })
        }
        _ => false,
    }
    _ => false,
};

if has_data_field {
    // Generate data-aware code
}
```

### Generated Module Schema

With data params, the schema generation changes:

```rust
impl crate::types::Module for MyModule {
    fn get_schema() -> crate::types::ModuleSchema {
        use crate::types::Params;
        use crate::types::DataParams;
        
        let mut param_schemas = MyModuleParams::get_schema();
        let mut data_schemas = MyModuleData::get_data_schema();
        
        // Merge schemas
        param_schemas.append(&mut data_schemas);
        
        // Validate no name collisions between params and data
        // (already done for params vs outputs)
        let param_names: HashSet<_> = MyModuleParams::get_schema()
            .iter()
            .map(|p| &p.name)
            .collect();
        let data_names: HashSet<_> = MyModuleData::get_data_schema()
            .iter()
            .map(|p| &p.name)
            .collect();
        
        for name in param_names.intersection(&data_names) {
            panic!(
                "Module '{}' has both audio param and data param named '{}'. Names must be unique.",
                "module-name",
                name
            );
        }
        
        crate::types::ModuleSchema {
            name: "module-name".to_string(),
            description: "Module description".to_string(),
            params: param_schemas,
            outputs: output_schemas,
        }
    }
}
```

### Module Construction with Data Params

When creating a module, data params must be initialized:

```rust
// Current constructor signature
fn module_constructor(
    id: &String,
    sample_rate: f32
) -> Result<Arc<Box<dyn Sampleable>>> {
    // ...
}

// With data params, signature could be:
fn module_constructor_with_data(
    id: &String,
    sample_rate: f32,
    data: HashMap<String, DataParam>
) -> Result<Arc<Box<dyn Sampleable>>> {
    // Parse data params into MyModuleData struct
    let mut module_data = MyModuleData::default();
    for (name, value) in data {
        module_data.update_data(&name, &value)?;
    }
    
    // Create module with data
    Ok(Arc::new(Box::new(MyModuleSampleable {
        id: id.clone(),
        sample_rate,
        module: Mutex::new(MyModule {
            data: module_data,
            // ...
        }),
        // ...
    })))
}
```

**Alternative**: Keep constructor simple, update data after creation:

```rust
// 1. Create with defaults
let module = module_constructor(id, sample_rate)?;

// 2. Update data params
for (name, value) in data_params {
    module.update_data_param(&name, &value)?;
}
```

This requires adding `update_data_param()` to the `Sampleable` trait (or a new trait).

## Error Handling

### Compile-Time Errors

The macro should provide helpful error messages:

```rust
// Error: Type mismatch
#[data_param("count", "Count", int(1, 10))]
count: String,  // ❌ Expected i64

// Error message:
// error: data_param with constraint 'int' must have type i64, found String
//   --> src/module.rs:10:5
//    |
// 10 |     count: String,
//    |     ^^^^^^^^^^^^

// Error: Missing required parts
#[data_param("name")]  // ❌ Missing description and type
name: String,

// Error message:
// error: data_param attribute requires at least 3 arguments: name, description, and type constraint
//   --> src/module.rs:8:5
//    |
// 8  |     #[data_param("name")]
//    |     ^^^^^^^^^^^^^^^^^^^^^

// Error: Invalid enum variants (empty)
#[data_param("mode", "Mode", enum())]  // ❌ No variants
mode: String,

// Error message:
// error: enum constraint must have at least one variant
//   --> src/module.rs:12:5
//    |
// 12 |     #[data_param("mode", "Mode", enum())]
//    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

// Error: Invalid bounds
#[data_param("value", "Value", float(10.0, 1.0))]  // ❌ min > max
value: f64,

// Error message:
// error: float constraint min value (10.0) cannot be greater than max value (1.0)
//   --> src/module.rs:14:5
//    |
// 14 |     #[data_param("value", "Value", float(10.0, 1.0))]
//    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

### Runtime Errors

Validation errors during `update_data()`:

```rust
// Type mismatch
let result = data.update_data("speed", &DataParam::String { value: "fast".to_string() });
// Error: Type mismatch for param 'speed': expected Float, got String { value: "fast" }

// Out of bounds
let result = data.update_data("speed", &DataParam::Float { value: 100.0 });
// Error: Value 100.0 for param 'speed' is out of bounds [0.1, 10.0]

// Invalid enum variant
let result = data.update_data("mode", &DataParam::Enum { variant: "invalid".to_string() });
// Error: Invalid variant 'invalid' for param 'mode'. Must be one of: forward, reverse

// Array length
let result = data.update_data("values", &DataParam::Array { values: vec![] });
// Error: Array length 0 for param 'values' is below minimum 1
```

## Edge Cases

### Optional Parameters

Optional params can be omitted from `get_data_state()` if they have no value:

```rust
#[data_param("label", "Optional label", string, optional)]
label: Option<String>,

// get_data_state() implementation:
if let Some(ref label_value) = self.label {
    state.insert("label".to_owned(), DataParam::String { value: label_value.clone() });
}
// If None, don't insert into map
```

**Rust Type Mapping**:
- Optional param → `Option<T>` type
- Non-optional param → `T` type

### Default Values

Default values affect both the schema and the Rust struct:

```rust
#[data_param("speed", "Speed", float(0.1, 10.0), default = 1.0)]
speed: f64,

// In Default impl:
impl Default for MyModuleData {
    fn default() -> Self {
        Self {
            speed: 1.0,  // Use default from attribute
            // ...
        }
    }
}

// In get_data_schema():
crate::types::ParamSchema {
    name: "speed".to_string(),
    // ...
    default: Some(crate::types::DataParam::Float { value: 1.0 }),
}
```

**Macro Challenge**: Parse default value from attribute and:
1. Include in `Default` impl generation
2. Include in schema generation
3. Validate type matches constraint

### Array Nesting

Arrays can be nested to arbitrary depth:

```rust
// 2D array
#[data_param("matrix", "2D matrix", array(array(float(0.0, 1.0), 2, 4), 2, 4))]
matrix: Vec<Vec<f64>>,

// Generated validation code must recurse:
fn validate_array_element(elem: &DataParam, constraint: &TypeConstraint) -> Result<()> {
    match (elem, constraint) {
        (DataParam::Array { values }, TypeConstraint::Array { element_type, .. }) => {
            for v in values {
                validate_array_element(v, element_type)?;
            }
            Ok(())
        }
        (DataParam::Float { value }, TypeConstraint::Float { min, max }) => {
            // Validate float
            Ok(())
        }
        _ => Err(anyhow!("Type mismatch"))
    }
}
```

### Object Types

Object types require nested struct definitions:

```rust
#[derive(Default, DataParams)]
struct ReverbConfig {
    #[data_param("roomSize", "Room size", float(0.0, 1.0))]
    room_size: f64,
    
    #[data_param("damping", "High frequency damping", float(0.0, 1.0))]
    damping: f64,
}

#[derive(Default, DataParams)]
struct EffectData {
    // Reference nested struct
    #[data_param("reverb", "Reverb configuration", object)]
    reverb: ReverbConfig,
}
```

**Generated Code**:

```rust
// For parent struct
state.insert(
    "reverb".to_owned(),
    DataParam::Object {
        fields: self.reverb.get_data_state()  // Recursively get nested state
    }
);

// Schema generation
ParamSchema {
    name: "reverb".to_string(),
    param_type: ParamType::Object {
        fields: ReverbConfig::get_data_schema()
            .into_iter()
            .map(|s| (s.name.clone(), s.param_type))
            .collect()
    },
    // ...
}
```

### Empty Structs

Edge case: struct with no data params:

```rust
#[derive(Default, DataParams)]
struct EmptyData {
    // No fields
}

// Should generate valid but empty implementations:
impl DataParams for EmptyData {
    fn get_data_state(&self) -> HashMap<String, DataParam> {
        HashMap::new()
    }
    
    fn update_data(&mut self, param_name: &str, value: &DataParam) -> Result<()> {
        Err(anyhow!("EmptyData has no data parameters"))
    }
    
    fn get_data_schema() -> Vec<ParamSchema> {
        Vec::new()
    }
}
```

## Testing Strategy

### Macro Tests

Test the macro with various inputs:

```rust
#[test]
fn test_derive_data_params_simple() {
    #[derive(Default, DataParams)]
    struct TestData {
        #[data_param("value", "A value", int(0, 10))]
        value: i64,
    }
    
    let data = TestData::default();
    let state = data.get_data_state();
    assert!(state.contains_key("value"));
    
    let schema = TestData::get_data_schema();
    assert_eq!(schema.len(), 1);
    assert_eq!(schema[0].name, "value");
}

#[test]
fn test_derive_data_params_enum() {
    #[derive(Default, DataParams)]
    struct TestData {
        #[data_param("mode", "Mode", enum("a", "b", "c"))]
        mode: String,
    }
    
    let mut data = TestData { mode: "a".to_string() };
    
    // Valid update
    assert!(data.update_data("mode", &DataParam::Enum { variant: "b".to_string() }).is_ok());
    assert_eq!(data.mode, "b");
    
    // Invalid variant
    assert!(data.update_data("mode", &DataParam::Enum { variant: "x".to_string() }).is_err());
}

#[test]
fn test_derive_data_params_bounds() {
    #[derive(Default, DataParams)]
    struct TestData {
        #[data_param("ratio", "Ratio", float(0.5, 2.0))]
        ratio: f64,
    }
    
    let mut data = TestData::default();
    
    // Within bounds
    assert!(data.update_data("ratio", &DataParam::Float { value: 1.0 }).is_ok());
    
    // Below min
    assert!(data.update_data("ratio", &DataParam::Float { value: 0.1 }).is_err());
    
    // Above max
    assert!(data.update_data("ratio", &DataParam::Float { value: 10.0 }).is_err());
}
```

### Integration Tests

Test with real module definitions:

```rust
#[test]
fn test_module_with_data_params() {
    #[derive(Default, Params)]
    struct TestParams {
        #[param("input", "Input signal")]
        input: InternalParam,
    }
    
    #[derive(Default, DataParams)]
    struct TestData {
        #[data_param("gain", "Gain factor", float(0.0, 2.0))]
        gain: f64,
    }
    
    #[derive(Module)]
    #[module("test-module", "Test module")]
    struct TestModule {
        #[output("output", "Output signal", default)]
        output: ChannelBuffer,
        params: TestParams,
        data: TestData,
    }
    
    let schema = TestModule::get_schema();
    
    // Should have both audio param and data param
    assert_eq!(schema.params.len(), 2);
    assert!(schema.params.iter().any(|p| p.name == "input"));
    assert!(schema.params.iter().any(|p| p.name == "gain"));
}
```

## Implementation Phases

### Phase 1: Basic Types
- [ ] Implement parsing for `string`, `int`, `float`, `bool`
- [ ] Generate `get_data_state()` for basic types
- [ ] Generate `update_data()` with type checking
- [ ] Generate `get_data_schema()` for basic types
- [ ] Test with simple modules

### Phase 2: Constraints
- [ ] Add bounds validation for `int` and `float`
- [ ] Implement `enum` with variant validation
- [ ] Add `optional` flag support
- [ ] Add `default` value support
- [ ] Test constraint enforcement

### Phase 3: Complex Types
- [ ] Implement `array` with element type and length constraints
- [ ] Add recursive validation for nested arrays
- [ ] Implement `object` with nested structs
- [ ] Test complex type combinations

### Phase 4: Module Integration
- [ ] Update `Module` macro to detect data fields
- [ ] Merge audio param and data param schemas
- [ ] Add name collision detection
- [ ] Update constructor to handle data params
- [ ] Test with complete modules

### Phase 5: Error Handling
- [ ] Improve compile-time error messages
- [ ] Add detailed runtime validation errors
- [ ] Test error cases comprehensively
- [ ] Document error scenarios

## Conclusion

The `DataParams` derive macro provides:

- ✅ **Type-safe** Rust code generation
- ✅ **Compile-time** validation of attribute syntax
- ✅ **Runtime** validation of parameter values
- ✅ **Schema generation** for TypeScript integration
- ✅ **Seamless integration** with existing `Module` macro

By following this implementation guide, the macro will enable clean, ergonomic data parameter definitions while maintaining the safety and performance characteristics of the existing system.
