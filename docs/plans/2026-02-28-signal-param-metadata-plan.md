# Signal Param Metadata Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add metadata (signal type, default, range) to signal params so the frontend can auto-generate sliders with correct labels, initial values, and ranges.

**Architecture:** A `#[signal()]` field attribute on PolySignal/MonoSignal params fields, parsed by a new `SignalParams` derive macro, which generates a `SignalParamMeta` trait impl. The `#[module]` macro references this to populate a new `signal_params` field on `ModuleSchema`. The TypeScript side merges this into `ParamDescriptor`.

**Tech Stack:** Rust proc macros (syn/quote), napi-rs, TypeScript/Vitest

---

### Task 1: Extend `SignalParamSchema` and `ModuleSchema` in Rust

**Files:**

- Modify: `crates/modular_core/src/types.rs:825-830` (SignalParamSchema)
- Modify: `crates/modular_core/src/types.rs:909-929` (ModuleSchema)

**Step 1: Extend `SignalParamSchema`**

Replace the existing struct at line 825-830:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct SignalParamSchema {
    pub name: String,
    pub description: String,
    pub signal_type: String,
    pub default_value: f64,
    pub min_value: f64,
    pub max_value: f64,
}
```

Note: This replaces the old struct which lacked `#[napi(object)]` and the new metadata fields.

**Step 2: Add `SignalParamMeta` trait**

Add after `PolySignalFields` trait (around line 433):

```rust
/// Trait for params structs to expose signal parameter metadata.
/// Auto-derived by the `SignalParams` derive macro.
pub trait SignalParamMeta {
    fn signal_param_schemas() -> Vec<SignalParamSchema>
    where
        Self: Sized,
    {
        vec![]
    }
}
```

**Step 3: Add `signal_params` to `ModuleSchema`**

Add a new field to the `ModuleSchema` struct:

```rust
pub signal_params: Vec<SignalParamSchema>,
```

**Step 4: Verify it compiles**

Run: `cargo build -p modular_core 2>&1 | head -20`

Expected: Compile errors in the proc macro crate where `ModuleSchema` is constructed without the new field. That's expected and will be fixed in Task 3.

**Step 5: Commit**

```bash
git add crates/modular_core/src/types.rs
git commit -m "feat: extend SignalParamSchema and ModuleSchema with signal param metadata"
```

---

### Task 2: Add `SignalParams` derive macro

**Files:**

- Modify: `crates/modular_derive/src/lib.rs`

This task adds a new derive macro `SignalParams` that:

1. Iterates named fields in a params struct
2. For fields typed `PolySignal` or `MonoSignal`, collects `#[signal()]` attributes
3. Generates a `SignalParamMeta` trait impl

**Step 1: Add `SignalAttr` struct and parser**

Add after the `is_poly_signal` function (around line 1148), before `impl_module_macro_attr`:

```rust
/// Parsed `#[signal(...)]` attribute data for signal param metadata.
struct SignalAttr {
    signal_type: String,
    default_value: f64,
    min_value: f64,
    max_value: f64,
}

impl Default for SignalAttr {
    fn default() -> Self {
        Self {
            signal_type: "control".to_string(),
            default_value: 0.0,
            min_value: -5.0,
            max_value: 5.0,
        }
    }
}

fn parse_signal_attr(attr: &Attribute) -> syn::Result<SignalAttr> {
    let mut result = SignalAttr::default();

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("type") {
            let value: Ident = meta.value()?.parse()?;
            let type_str = value.to_string();
            match type_str.as_str() {
                "pitch" | "gate" | "trig" | "control" => {
                    result.signal_type = type_str;
                }
                other => {
                    return Err(meta.error(format!(
                        "Unknown signal type '{}'. Expected: pitch, gate, trig, control",
                        other
                    )));
                }
            }
            Ok(())
        } else if meta.path.is_ident("default") {
            let value: syn::LitFloat = meta.value()?.parse()?;
            result.default_value = value.base10_parse()?;
            Ok(())
        } else if meta.path.is_ident("range") {
            meta.value()?;
            let content;
            syn::parenthesized!(content in meta.input);
            let min: syn::LitFloat = content.parse()?;
            content.parse::<Token![,]>()?;
            let max: syn::LitFloat = content.parse()?;
            result.min_value = min.base10_parse()?;
            result.max_value = max.base10_parse()?;
            Ok(())
        } else {
            Err(meta.error("expected `type`, `default`, or `range`"))
        }
    })?;

    Ok(result)
}
```

**Step 2: Add the derive macro entry point**

Add after the `ChannelCount` derive macro (around line 1072):

```rust
#[proc_macro_derive(SignalParams, attributes(signal))]
pub fn signal_params_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    impl_signal_params_macro(&ast)
}
```

**Step 3: Implement `impl_signal_params_macro`**

Add after the entry point:

```rust
fn impl_signal_params_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let schema_exprs: Vec<TokenStream2> = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|field| {
                    let field_ident = field.ident.as_ref()?;
                    let is_signal = is_poly_signal_type(&field.ty) || is_mono_signal_type(&field.ty);
                    if !is_signal {
                        return None;
                    }

                    // Parse #[signal(...)] attribute if present, otherwise use defaults
                    let signal_attr = field
                        .attrs
                        .iter()
                        .find(|a| a.path().is_ident("signal"))
                        .map(|a| parse_signal_attr(a));

                    let attr = match signal_attr {
                        Some(Ok(a)) => a,
                        Some(Err(e)) => return Some(Err(e)),
                        None => SignalAttr::default(),
                    };

                    // Extract doc comments for description
                    let description = extract_doc_comments(&field.attrs)
                        .unwrap_or_default();

                    // Use serde rename_all camelCase convention for the field name
                    let field_name = field_ident.to_string();
                    let signal_type = &attr.signal_type;
                    let default_value = attr.default_value;
                    let min_value = attr.min_value;
                    let max_value = attr.max_value;

                    Some(Ok(quote! {
                        crate::types::SignalParamSchema {
                            name: #field_name.to_string(),
                            description: #description.to_string(),
                            signal_type: #signal_type.to_string(),
                            default_value: #default_value,
                            min_value: #min_value,
                            max_value: #max_value,
                        }
                    }))
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_else(|e| {
                    vec![e.to_compile_error()]
                }),
            Fields::Unnamed(_) | Fields::Unit => {
                return syn::Error::new(
                    ast.span(),
                    "#[derive(SignalParams)] only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return syn::Error::new(ast.span(), "#[derive(SignalParams)] only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let generated = quote! {
        impl crate::types::SignalParamMeta for #name {
            fn signal_param_schemas() -> Vec<crate::types::SignalParamSchema> {
                vec![#(#schema_exprs,)*]
            }
        }
    };

    generated.into()
}
```

**Step 4: Verify macro crate compiles**

Run: `cargo build -p modular_derive`

Expected: Should compile (the macro doesn't depend on the types crate at compile time; it generates code that references `crate::types`).

**Step 5: Commit**

```bash
git add crates/modular_derive/src/lib.rs
git commit -m "feat: add SignalParams derive macro for signal param metadata"
```

---

### Task 3: Wire up `SignalParams` derive in modules and `#[module]` macro

**Files:**

- Modify: `crates/modular_derive/src/lib.rs:1624-1667` (`get_schema()` in `impl_module_macro_attr`)
- Modify: `crates/modular_core/src/dsp/filters/lowpass.rs` (add `SignalParams` to derives, add `#[signal()]` annotations)

**Step 1: Update `get_schema()` in `impl_module_macro_attr`**

In the `get_schema()` method (around line 1653), add the `signal_params` field to the `ModuleSchema` construction:

Change:

```rust
crate::types::ModuleSchema {
    name: #module_name.to_string(),
    documentation: #module_documentation_token,
    params_schema: crate::types::SchemaContainer {
        schema: params_schema,
    },
    outputs: output_schemas,
    positional_args: vec![
        #(#positional_args_exprs),*
    ],
    channels: #module_channels,
    channels_param: #module_channels_param,
    channels_param_default: #module_channels_param_default,
}
```

To:

```rust
crate::types::ModuleSchema {
    name: #module_name.to_string(),
    documentation: #module_documentation_token,
    params_schema: crate::types::SchemaContainer {
        schema: params_schema,
    },
    outputs: output_schemas,
    signal_params: <#params_struct_name as crate::types::SignalParamMeta>::signal_param_schemas(),
    positional_args: vec![
        #(#positional_args_exprs),*
    ],
    channels: #module_channels,
    channels_param: #module_channels_param,
    channels_param_default: #module_channels_param_default,
}
```

**Step 2: Add `SignalParams` derive to a test module**

In `crates/modular_core/src/dsp/filters/lowpass.rs`, add `SignalParams` to the derives and `#[signal()]` annotations:

Change:

```rust
#[derive(Clone, Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct LowpassFilterParams {
    /// signal input
    input: PolySignal,
    /// cutoff frequency in V/Oct (0V = C4)
    cutoff: PolySignal,
    /// filter resonance (0-5)
    resonance: PolySignal,
}
```

To:

```rust
#[derive(Clone, Deserialize, Default, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(default, rename_all = "camelCase")]
struct LowpassFilterParams {
    /// signal input
    input: PolySignal,
    /// cutoff frequency in V/Oct (0V = C4)
    #[signal(type = pitch, default = 0.0, range = (-5.0, 5.0))]
    cutoff: PolySignal,
    /// filter resonance (0-5)
    #[signal(type = control, default = 0.0, range = (0.0, 5.0))]
    resonance: PolySignal,
}
```

**Step 3: Add `SignalParams` derive to ALL other modules**

Every params struct that derives `Connect` and `ChannelCount` must also derive `SignalParams`. For now, just add the derive — don't add `#[signal()]` annotations. The unannotated defaults (control, 0.0, -5..5) will apply.

Find all params structs by searching for `derive(Clone, Deserialize, Default, JsonSchema, Connect, ChannelCount)` across `crates/modular_core/src/dsp/`. Add `, SignalParams` to each derive list.

**Step 4: Build and verify**

Run: `cargo build -p modular_core`

Expected: Clean compile. All modules now have `SignalParamMeta` impls and `ModuleSchema` includes `signal_params`.

**Step 5: Run tests**

Run: `cargo test`

Expected: All existing tests pass.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: wire SignalParams derive into all modules and module schema"
```

---

### Task 4: Extend TypeScript types and schema processing

**Files:**

- Modify: `src/main/dsl/paramsSchema.ts:41-56` (ParamDescriptor)
- Modify: `src/main/dsl/paramsSchema.ts:356-389` (processModuleSchema)

**Step 1: Add signal metadata types to `ParamDescriptor`**

Add the `SignalType` type and extend `ParamDescriptor`:

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

**Step 2: Update `processModuleSchema` to merge signal param metadata**

After building the base `params` array, merge in `schema.signalParams`:

```typescript
export function processModuleSchema(
    schema: ModuleSchema,
): ProcessedModuleSchema {
    const root = asJsonSchema(schema.paramsSchema) ?? {};
    const rootResolved = resolveAndMerge(root, root);

    const obj = rootResolved.type === 'object' ? rootResolved : null;
    const properties = obj?.properties ?? {};
    const required = new Set(obj?.required ?? []);

    // Build signal param lookup from schema.signalParams
    const signalParamsByName = new Map<
        string,
        ModuleSchema['signalParams'][number]
    >();
    if (schema.signalParams) {
        for (const sp of schema.signalParams) {
            signalParamsByName.set(sp.name, sp);
        }
    }

    const params: ParamDescriptor[] = Object.entries(properties).map(
        ([name, s]) => {
            const resolved = resolveAndMerge(root, s);
            const enumValues = extractStringEnum(resolved);
            const inferedKind = inferKind(root, s);

            const signalMeta = signalParamsByName.get(name);

            return {
                name,
                kind: inferedKind,
                description: resolved.description,
                optional: !required.has(name),
                enumValues,
                ...(signalMeta && {
                    signalType: signalMeta.signalType as SignalType,
                    defaultValue: signalMeta.defaultValue,
                    minValue: signalMeta.minValue,
                    maxValue: signalMeta.maxValue,
                }),
            };
        },
    );

    const paramsByName: Record<string, ParamDescriptor> = {};
    for (const p of params) paramsByName[p.name] = p;

    return {
        ...schema,
        params,
        paramsByName,
    };
}
```

**Step 3: Build TypeScript**

Run: `yarn build-native && yarn typecheck`

Expected: Clean compile. The `ModuleSchema` type from the native module now includes `signalParams`.

**Step 4: Commit**

```bash
git add src/main/dsl/paramsSchema.ts
git commit -m "feat: extend ParamDescriptor with signal metadata from module schema"
```

---

### Task 5: Add N-API integration test for signal param metadata

**Files:**

- Modify: `crates/modular/__test__/napi.test.ts`

**Step 1: Add test for signal_params in schema**

Add to the `getSchemas` describe block:

```typescript
test('schemas include signalParams for modules with signal inputs', () => {
    const schemas = getSchemas();
    const lpf = schemas.find((s) => s.name === '$lpf');
    expect(lpf).toBeDefined();
    expect(lpf!.signalParams).toBeDefined();
    expect(lpf!.signalParams.length).toBeGreaterThan(0);

    // Check that cutoff has pitch type with correct range
    const cutoff = lpf!.signalParams.find((p: any) => p.name === 'cutoff');
    expect(cutoff).toBeDefined();
    expect(cutoff!.signalType).toBe('pitch');
    expect(cutoff!.defaultValue).toBe(0.0);
    expect(cutoff!.minValue).toBe(-5.0);
    expect(cutoff!.maxValue).toBe(5.0);

    // Check that resonance has control type
    const resonance = lpf!.signalParams.find(
        (p: any) => p.name === 'resonance',
    );
    expect(resonance).toBeDefined();
    expect(resonance!.signalType).toBe('control');
    expect(resonance!.minValue).toBe(0.0);
    expect(resonance!.maxValue).toBe(5.0);

    // Check that unannotated signal params get defaults
    const input = lpf!.signalParams.find((p: any) => p.name === 'input');
    expect(input).toBeDefined();
    expect(input!.signalType).toBe('control');
    expect(input!.defaultValue).toBe(0.0);
    expect(input!.minValue).toBe(-5.0);
    expect(input!.maxValue).toBe(5.0);
});
```

**Step 2: Run the test**

Run: `yarn test:unit`

Expected: All tests pass including the new one.

**Step 3: Commit**

```bash
git add crates/modular/__test__/napi.test.ts
git commit -m "test: add integration test for signal param metadata in schemas"
```

---

### Task 6: Annotate key modules with `#[signal()]` metadata

**Files:**

- Modify: Various module files in `crates/modular_core/src/dsp/`

This is the incremental annotation pass. Add `#[signal()]` attributes to the most important modules. Focus on oscillators, filters, and utilities where the signal type distinction matters most.

**Step 1: Annotate oscillator modules**

For each oscillator, `freq` params are `pitch`:

- `dsp/oscillators/sine.rs`: `#[signal(type = pitch)]` on `freq`
- `dsp/oscillators/saw.rs`: `#[signal(type = pitch)]` on `freq`, `#[signal(type = control, default = 0.0, range = (0.0, 1.0))]` on `shape` if it's a signal
- Other oscillators: same pattern for frequency params

**Step 2: Annotate filter modules**

Already done for lowpass in Task 3. Apply similar to:

- `dsp/filters/` — `cutoff` is `pitch`, `resonance`/`q` is `control`

**Step 3: Annotate envelope/gate modules**

Look for gate/trig inputs:

- Any `gate` param: `#[signal(type = gate, default = 0.0, range = (0.0, 10.0))]`
- Any `trig`/`trigger` param: `#[signal(type = trig, default = 0.0, range = (0.0, 10.0))]`

**Step 4: Build and test**

Run: `cargo build && cargo test`

Expected: Clean build, all tests pass.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: annotate key modules with signal param metadata"
```
