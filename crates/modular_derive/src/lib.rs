extern crate quote;
extern crate syn;

extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{Attribute, LitStr, Token, punctuated::Punctuated, spanned::Spanned};
use syn::{Data, DeriveInput, Fields, Type};

/// Key used for internal metadata field storing argument source spans.
/// Must match modular_core::types::ARGUMENT_SPANS_KEY.
const _ARGUMENT_SPANS_KEY: &str = "__argument_spans";

#[proc_macro]
pub fn message_handlers(input: TokenStream) -> TokenStream {
    struct Arm {
        variant: Ident,
        bindings: Vec<Ident>,
        handler: syn::Path,
    }

    struct Input {
        ty: Ident,
        arms: Vec<Arm>,
    }

    impl syn::parse::Parse for Arm {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let variant: Ident = input.parse()?;

            let bindings = if input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in input);
                let parsed: Punctuated<Ident, Token![,]> =
                    content.parse_terminated(Ident::parse, Token![,])?;
                parsed.into_iter().collect()
            } else {
                Vec::new()
            };

            input.parse::<Token![=>]>()?;
            let handler: syn::Path = input.parse()?;

            Ok(Self {
                variant,
                bindings,
                handler,
            })
        }
    }

    impl syn::parse::Parse for Input {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            input.parse::<Token![impl]>()?;
            let ty: Ident = input.parse()?;

            let content;
            syn::braced!(content in input);

            let mut arms = Vec::new();
            while !content.is_empty() {
                let arm: Arm = content.parse()?;
                arms.push(arm);
                let _ = content.parse::<Token![,]>();
            }

            Ok(Self { ty, arms })
        }
    }

    let parsed: Input = match syn::parse(input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let ty = parsed.ty;
    let wrapper = format_ident!("{}Sampleable", ty);

    if parsed.arms.is_empty() {
        return quote! {
            impl crate::types::MessageHandler for #wrapper {}
        }
        .into();
    }

    // Deduplicate tags while preserving order.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut tag_variants: Vec<Ident> = Vec::new();
    for arm in &parsed.arms {
        if seen.insert(arm.variant.to_string()) {
            tag_variants.push(arm.variant.clone());
        }
    }

    let tag_exprs: Vec<TokenStream2> = tag_variants
        .iter()
        .map(|v| quote!(crate::types::MessageTag::#v))
        .collect();

    let mut match_arms: Vec<TokenStream2> = Vec::new();
    for arm in &parsed.arms {
        let variant = &arm.variant;
        let handler = &arm.handler;

        let bindings = &arm.bindings;

        if bindings.is_empty() {
            // Unit variant - no bindings, handler takes only &mut self
            match_arms.push(quote! {
                crate::types::Message::#variant => #handler(&mut *module)
            });
        } else {
            // Tuple variant - destructure and pass bindings to handler
            match_arms.push(quote! {
                crate::types::Message::#variant( #( #bindings ),* ) => #handler(&mut *module, #( #bindings ),* )
            });
        }
    }

    quote! {
        impl crate::types::MessageHandler for #wrapper {
            fn handled_message_tags(&self) -> &'static [crate::types::MessageTag] {
                &[ #( #tag_exprs ),* ]
            }

            fn handle_message(&self, message: &crate::types::Message) -> napi::Result<()> {
                // SAFETY: Audio thread has exclusive access during message dispatch.
                // Messages are only dispatched from AudioProcessor::process_commands().
                // See crate::types module documentation for full safety invariants.
                let module = unsafe { &mut *self.module.get() };
                match message {
                    #( #match_arms, )*
                    _ => Ok(()),
                }
            }
        }
    }
    .into()
}

/// Parsed output attribute data
struct OutputAttr {
    name: LitStr,
    description: Option<LitStr>,
    is_default: bool,
    range: Option<(f64, f64)>,
}

#[proc_macro_derive(Outputs, attributes(output))]
pub fn outputs_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    impl_outputs_macro(&ast)
}

#[proc_macro_derive(EnumTag, attributes(enum_tag))]
pub fn enum_tag_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    impl_enum_tag_macro(&ast)
}

fn parse_enum_tag_name(attrs: &Vec<Attribute>, default_ident: Ident) -> syn::Result<Ident> {
    let mut found: Option<Ident> = None;

    for attr in attrs.iter().filter(|a| a.path().is_ident("enum_tag")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value: LitStr = meta.value()?.parse()?;
                let name_str = value.value();
                let ident = syn::parse_str::<Ident>(&name_str).map_err(|_| {
                    syn::Error::new(
                        value.span(),
                        "enum_tag name must be a valid Rust identifier",
                    )
                })?;
                found = Some(ident);
                Ok(())
            } else {
                Err(meta.error("unsupported enum_tag attribute; expected `name = \"...\"`"))
            }
        })?;
    }

    Ok(found.unwrap_or(default_ident))
}

fn impl_enum_tag_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let vis = &ast.vis;

    let default_tag_name = format_ident!("{}Tag", name);
    let tag_name = match parse_enum_tag_name(&ast.attrs, default_tag_name) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let data_enum = match &ast.data {
        Data::Enum(e) => e,
        Data::Struct(_) | Data::Union(_) => {
            return syn::Error::new(Span::call_site(), "EnumTag can only be derived for enums")
                .to_compile_error()
                .into();
        }
    };

    let mut tag_variants: Vec<TokenStream2> = Vec::new();
    let mut match_arms: Vec<TokenStream2> = Vec::new();
    for v in &data_enum.variants {
        let v_ident = &v.ident;
        tag_variants.push(quote!(#v_ident));

        let pat = match &v.fields {
            Fields::Unit => quote!(Self::#v_ident),
            Fields::Unnamed(_) => quote!(Self::#v_ident(..)),
            Fields::Named(_) => quote!(Self::#v_ident { .. }),
        };
        match_arms.push(quote!(#pat => #tag_name::#v_ident));
    }

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let generated = quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #vis enum #tag_name {
            #( #tag_variants, )*
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #vis fn tag(&self) -> #tag_name {
                match self {
                    #( #match_arms, )*
                }
            }
        }
    };

    generated.into()
}

/// Extract `///` doc comments from a list of attributes.
/// In Rust, `/// text` desugars to `#[doc = "text"]`.
fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        return Some(s.value());
                    }
                }
            }
            None
        })
        .collect();
    if docs.is_empty() {
        None
    } else {
        // Each doc comment line has a leading space (" text") — trim it.
        // Join with newlines to preserve paragraph structure.
        Some(
            docs.iter()
                .map(|line| {
                    if let Some(stripped) = line.strip_prefix(' ') {
                        stripped.to_string()
                    } else {
                        line.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

fn unwrap_attr(attrs: &Vec<Attribute>, ident: &str) -> Option<TokenStream2> {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident(ident))
        .and_then(|attr| {
            if let syn::Meta::List(list) = &attr.meta {
                Some(list.tokens.clone())
            } else {
                None
            }
        })
}

/// Parsed module attribute data
struct ModuleAttr {
    name: LitStr,
    description: Option<LitStr>,
    channels: Option<u8>,
    channels_param: Option<LitStr>,
    channels_param_default: Option<u8>,
    /// Custom function to derive channel count from params struct.
    /// The function must have signature: fn(&ParamsStruct) -> Option<usize>
    channels_derive: Option<syn::Path>,
}

// ---------------------------------------------------------------------------
// Attribute-macro argument parser
// ---------------------------------------------------------------------------

/// All configuration parsed from `#[module(...)]` attribute arguments.
///
/// Idiomatic key=value syntax:
/// ```text
/// #[module(
///     name = "$sine",
///     description = "A sine wave oscillator",
///     channels = 2,
///     args(freq, engine?),
///     stateful,
///     patch_update,
///     has_init,
/// )]
/// ```
struct ModuleAttrArgs {
    module: ModuleAttr,
    args: Vec<ArgAttr>,
    /// Whether the `args(...)` keyword was present at all (even if empty).
    has_args: bool,
    stateful: bool,
    patch_update: bool,
    has_init: bool,
}

impl syn::parse::Parse for ModuleAttrArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut description: Option<LitStr> = None;
        let mut channels: Option<u8> = None;
        let mut channels_param: Option<LitStr> = None;
        let mut channels_param_default: Option<u8> = None;
        let mut channels_derive: Option<syn::Path> = None;
        let mut args: Vec<ArgAttr> = Vec::new();
        let mut has_args = false;
        let mut stateful = false;
        let mut patch_update = false;
        let mut has_init = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    name = Some(input.parse()?);
                }
                "description" => {
                    input.parse::<Token![=]>()?;
                    description = Some(input.parse()?);
                }
                "channels" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitInt = input.parse()?;
                    channels = Some(lit.base10_parse()?);
                }
                "channels_param" => {
                    input.parse::<Token![=]>()?;
                    channels_param = Some(input.parse()?);
                }
                "channels_param_default" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitInt = input.parse()?;
                    channels_param_default = Some(lit.base10_parse()?);
                }
                "channels_derive" => {
                    input.parse::<Token![=]>()?;
                    channels_derive = Some(input.parse()?);
                }
                "args" => {
                    has_args = true;
                    let content;
                    syn::parenthesized!(content in input);
                    let parsed: Punctuated<ArgAttr, Token![,]> =
                        Punctuated::parse_terminated(&content)?;
                    args = parsed.into_iter().collect();
                }
                "stateful" => {
                    stateful = true;
                }
                "patch_update" => {
                    patch_update = true;
                }
                "has_init" => {
                    has_init = true;
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "Unknown module attribute '{other}'. Expected one of: \
                             name, description, channels, channels_param, \
                             channels_param_default, channels_derive, args, \
                             stateful, patch_update, has_init"
                        ),
                    ));
                }
            }

            // Consume trailing comma if present
            let _ = input.parse::<Token![,]>();
        }

        let name = name.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing `name` in #[module(...)]",
            )
        })?;

        Ok(ModuleAttrArgs {
            module: ModuleAttr {
                name,
                description,
                channels,
                channels_param,
                channels_param_default,
                channels_derive,
            },
            args,
            has_args,
            stateful,
            patch_update,
            has_init,
        })
    }
}

// ---------------------------------------------------------------------------
// Legacy derive-macro helpers (kept for `Outputs`, `Connect`, `ChannelCount`)
// ---------------------------------------------------------------------------

/// Parse output attribute tokens into OutputAttr
/// Supports:
/// - #[output("name", "description")]
/// - #[output("name", "description", default)]
/// - #[output("name", "description", range = (-1.0, 1.0))]
/// - #[output("name", "description", default, range = (-1.0, 1.0))]
fn parse_output_attr(tokens: TokenStream2) -> OutputAttr {
    use syn::Result as SynResult;
    use syn::parse::{Parse, ParseStream};

    /// Reserved output names that conflict with ModuleOutput, Collection, or CollectionWithRange
    /// methods/properties in the TypeScript DSL. These names cannot be used as output names.
    ///
    /// IMPORTANT: When adding new methods to any type that a factory function could return
    /// (ModuleOutput, ModuleOutputWithRange, BaseCollection, Collection, CollectionWithRange),
    /// the method name MUST be added to this list. Keep in sync with:
    /// - src/dsl/factories.ts (RESERVED_OUTPUT_NAMES)
    /// - src/dsl/typescriptLibGen.ts (RESERVED_OUTPUT_NAMES)
    const RESERVED_OUTPUT_NAMES: &[&str] = &[
        // ModuleOutput properties
        "builder",
        "moduleId",
        "module_id",
        "portName",
        "port_name",
        "channel",
        // ModuleOutput methods
        "gain",
        "shift",
        "scope",
        "out",
        "outMono",
        "out_mono",
        "o",
        "toString",
        "to_string",
        // ModuleOutputWithRange properties
        "minValue",
        "min_value",
        "maxValue",
        "max_value",
        "range",
        // Collection/CollectionWithRange properties
        "items",
        "length",
        // DeferredModuleOutput/DeferredCollection methods
        "set",
        // JavaScript built-ins
        "constructor",
        "prototype",
        "__proto__",
    ];

    struct OutputAttrParser {
        name: LitStr,
        description: Option<LitStr>,
        is_default: bool,
        range: Option<(f64, f64)>,
    }

    impl Parse for OutputAttrParser {
        fn parse(input: ParseStream) -> SynResult<Self> {
            // Parse first string literal (name)
            let name: LitStr = input.parse()?;
            let name_value = name.value();

            // Check against reserved names
            if RESERVED_OUTPUT_NAMES.contains(&name_value.as_str()) {
                return Err(syn::Error::new(
                    name.span(),
                    format!(
                        "Output name '{}' is reserved. Reserved names are: {:?}",
                        name_value, RESERVED_OUTPUT_NAMES
                    ),
                ));
            }

            input.parse::<Token![,]>()?;

            // Parse description string
            let description: LitStr = input.parse()?;

            // Parse optional attributes (default, range)
            let mut is_default = false;
            let mut range: Option<(f64, f64)> = None;
            while input.peek(Token![,]) {
                input.parse::<Token![,]>()?;

                if input.is_empty() {
                    break;
                }

                // Check for `default` keyword
                if input.peek(syn::Ident) {
                    let ident: Ident = input.parse()?;
                    if ident == "default" {
                        is_default = true;
                    } else if ident == "range" {
                        input.parse::<Token![=]>()?;
                        let content;
                        syn::parenthesized!(content in input);
                        let min: syn::LitFloat = content.parse()?;
                        content.parse::<Token![,]>()?;
                        let max: syn::LitFloat = content.parse()?;
                        range = Some((min.base10_parse()?, max.base10_parse()?));
                    } else {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!(
                                "Unknown output attribute '{}'. Expected 'default' or 'range'",
                                ident
                            ),
                        ));
                    }
                }
            }

            Ok(OutputAttrParser {
                name,
                description: Some(description),
                is_default,
                range,
            })
        }
    }

    let parsed = syn::parse2::<OutputAttrParser>(tokens).expect("Failed to parse output attribute");

    OutputAttr {
        name: parsed.name,
        description: parsed.description,
        is_default: parsed.is_default,
        range: parsed.range,
    }
}

/// Attribute-style proc macro for declaring audio modules.
///
/// # Syntax
///
/// ```rust,ignore
/// #[module(
///     name = "$sine",
///     description = "A sine wave oscillator",
///     // Channel count configuration (at most one):
///     // channels = 2,                         // hardcoded
///     // channels_param = "channels",           // read from param field
///     // channels_param_default = 1,            // default when param absent
///     // channels_derive = my_derive_fn,        // custom function
///     //
///     // Positional DSL arguments (optional):
///     // args(freq, engine?),
///     //
///     // Flags (optional):
///     // stateful,      // implements StatefulModule
///     // patch_update,  // implements PatchUpdateHandler
///     // has_init,      // has fn init(&mut self, sample_rate: f32)
/// )]
/// pub struct MyModule { ... }
/// ```
///
/// The struct **must** have a field named `outputs` whose type derives `Outputs`,
/// and a field named `params` whose type derives `Deserialize`, `JsonSchema`,
/// `Connect`, and `ChannelCount`.
///
/// **Important**: If the struct derives `Default`, the `#[derive(Default)]`
/// attribute must come *after* `#[module(...)]`, not before it. This is because
/// `#[module]` injects a `_channel_count` field, and if `#[derive(Default)]`
/// precedes `#[module]`, the derive expands on the original struct (without
/// the injected field) and produces a broken `Default` impl.
#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = syn::parse_macro_input!(attr as ModuleAttrArgs);
    let mut ast: DeriveInput = syn::parse_macro_input!(item as DeriveInput);

    // Strip any leftover helper attributes that we've absorbed (safety net for migration)
    ast.attrs.retain(|a| {
        !a.path().is_ident("module")
            && !a.path().is_ident("args")
            && !a.path().is_ident("stateful")
            && !a.path().is_ident("patch_update")
            && !a.path().is_ident("has_init")
    });

    // Inject `_channel_count: usize` field into the struct so that
    // `self.channel_count()` can return a precomputed value set by the
    // main thread via `try_update_params`.
    if let Data::Struct(ref mut data_struct) = ast.data {
        if let Fields::Named(ref mut fields) = data_struct.fields {
            let field: syn::Field = syn::parse_quote! {
                pub _channel_count: usize
            };
            fields.named.push(field);
        }
    }

    match impl_module_macro_attr(&ast, &attr_args) {
        Ok(generated) => {
            let mut output = quote!(#ast);
            output.extend(generated);
            output.into()
        }
        Err(e) => e.to_compile_error().into(),
    }
}

/// Precision type for output fields
#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputPrecision {
    F32,
    PolySignal,
}

/// Parsed output field data
struct OutputField {
    field_name: Ident,
    output_name: LitStr,
    precision: OutputPrecision,
    description: TokenStream2,
    is_default: bool,
    range: Option<(f64, f64)>,
}

fn impl_outputs_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let outputs: Vec<OutputField> = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mut out = Vec::new();
                for f in fields.named.iter() {
                    let field_name = f
                        .ident
                        .clone()
                        .expect("Expected named field in Outputs struct");

                    let output_attr_tokens = match unwrap_attr(&f.attrs, "output") {
                        Some(t) => t,
                        None => {
                            return syn::Error::new(
                                f.span(),
                                "Every field in an Outputs struct must be annotated with #[output(...)]",
                            )
                            .to_compile_error()
                            .into();
                        }
                    };

                    // Detect field precision (f32 or PolySignal)
                    let precision = match &f.ty {
                        Type::Path(tp) => {
                            let type_name =
                                tp.path.segments.last().map(|seg| seg.ident.to_string());
                            match type_name.as_deref() {
                                Some("f32") => OutputPrecision::F32,
                                Some("PolyOutput") => OutputPrecision::PolySignal,
                                _ => {
                                    return syn::Error::new(
                                        f.ty.span(),
                                        "Output fields must have type f32 or PolyOutput",
                                    )
                                    .to_compile_error()
                                    .into();
                                }
                            }
                        }
                        _ => {
                            return syn::Error::new(
                                f.ty.span(),
                                "Output fields must have type f32, or PolyOutput",
                            )
                            .to_compile_error()
                            .into();
                        }
                    };

                    let output_attr = parse_output_attr(output_attr_tokens);
                    let output_name = output_attr.name;
                    let description = output_attr
                        .description
                        .as_ref()
                        .map(|d| quote!(#d.to_string()))
                        .unwrap_or(quote!("".to_string()));

                    out.push(OutputField {
                        field_name,
                        output_name,
                        precision,
                        description,
                        is_default: output_attr.is_default,
                        range: output_attr.range,
                    });
                }
                out
            }
            Fields::Unnamed(_) | Fields::Unit => {
                return syn::Error::new(
                    Span::call_site(),
                    "Outputs can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return syn::Error::new(Span::call_site(), "Outputs can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    // Validate that exactly one output is marked as default
    let default_outputs: Vec<_> = outputs.iter().filter(|o| o.is_default).collect();
    if default_outputs.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            format!(
                "Outputs struct '{}' must have exactly one output marked as `default`. \
                 Add `default` to one of the #[output(...)] attributes.",
                name
            ),
        )
        .to_compile_error()
        .into();
    }
    if default_outputs.len() > 1 {
        let names: Vec<_> = default_outputs
            .iter()
            .map(|o| o.output_name.value())
            .collect();
        return syn::Error::new(
            Span::call_site(),
            format!(
                "Outputs struct '{}' has {} outputs marked as `default` ({:?}), but only one is allowed.",
                name,
                default_outputs.len(),
                names
            ),
        )
        .to_compile_error()
        .into();
    }

    let _field_idents: Vec<_> = outputs.iter().map(|o| &o.field_name).collect();

    // Generate default value expressions for each field type
    let field_defaults: Vec<_> = outputs
        .iter()
        .map(|o| {
            let field_name = &o.field_name;
            match o.precision {
                OutputPrecision::F32 => quote! { #field_name: 0.0 },
                OutputPrecision::PolySignal => {
                    quote! { #field_name: crate::poly::PolyOutput::default() }
                }
            }
        })
        .collect();

    // Generate get_poly_sample match arms (returns PolyOutput)
    let poly_sample_match_arms: Vec<_> = outputs
        .iter()
        .map(|o| {
            let output_name = &o.output_name;
            let field_name = &o.field_name;
            match o.precision {
                OutputPrecision::F32 => quote! {
                    #output_name => Some(crate::poly::PolyOutput::mono(self.#field_name)),
                },
                OutputPrecision::PolySignal => quote! {
                    #output_name => Some(self.#field_name),
                },
            }
        })
        .collect();

    let schema_exprs: Vec<_> = outputs
        .iter()
        .map(|o| {
            let output_name = &o.output_name;
            let description = &o.description;
            let is_polyphonic = o.precision == OutputPrecision::PolySignal;
            let is_default = o.is_default;
            let min_value = match o.range {
                Some((min, _)) => quote! { Some(#min) },
                None => quote! { None },
            };
            let max_value = match o.range {
                Some((_, max)) => quote! { Some(#max) },
                None => quote! { None },
            };
            quote! {
                crate::types::OutputSchema {
                    name: #output_name.to_string(),
                    description: #description,
                    polyphonic: #is_polyphonic,
                    default: #is_default,
                    min_value: #min_value,
                    max_value: #max_value,
                }
            }
        })
        .collect();

    let copy_stmts: Vec<_> = outputs
        .iter()
        .map(|o| {
            let field_name = &o.field_name;
            quote! {
                self.#field_name = other.#field_name;
            }
        })
        .collect();

    let set_channels_stmts: Vec<_> = outputs
        .iter()
        .filter(|o| o.precision == OutputPrecision::PolySignal)
        .map(|o| {
            let field_name = &o.field_name;
            quote! {
                self.#field_name.set_channels(channels);
            }
        })
        .collect();

    let generated = quote! {
        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#field_defaults,)*
                }
            }
        }

        impl crate::types::OutputStruct for #name {
            fn copy_from(&mut self, other: &Self) {
                #(#copy_stmts)*
            }

            fn get_poly_sample(&self, port: &str) -> Option<crate::poly::PolyOutput> {
                match port {
                    #(#poly_sample_match_arms)*
                    _ => None,
                }
            }

            fn set_all_channels(&mut self, channels: usize) {
                #(#set_channels_stmts)*
            }

            fn schemas() -> Vec<crate::types::OutputSchema> {
                vec![
                    #(#schema_exprs,)*
                ]
            }
        }
    };

    generated.into()
}

#[proc_macro_derive(Connect, attributes(default_connection))]
pub fn connect_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    impl_connect_macro(&ast)
}

/// Parsed `#[default_connection(...)]` attribute data
struct DefaultConnectionAttr {
    module: Ident,
    port: String,
    /// For Signal: single channel. For PolySignal: multiple channels.
    channels: Vec<usize>,
}

/// Parse `#[default_connection(id = "...", port = "...", channel = N)]` for Signal
/// or `#[default_connection(id = "...", port = "...", channels = [N, M, ...])]` for PolySignal
fn parse_default_connection_attr(attr: &Attribute) -> syn::Result<DefaultConnectionAttr> {
    let mut module: Option<Ident> = None;
    let mut port: Option<String> = None;
    let mut channels: Vec<usize> = Vec::new();

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("module") {
            let value: Ident = meta.value()?.parse()?;
            module = Some(value);
            Ok(())
        } else if meta.path.is_ident("port") {
            let value: LitStr = meta.value()?.parse()?;
            port = Some(value.value());
            Ok(())
        } else if meta.path.is_ident("channel") {
            let value: syn::LitInt = meta.value()?.parse()?;
            channels = vec![value.base10_parse()?];
            Ok(())
        } else if meta.path.is_ident("channels") {
            meta.value()?;
            let content;
            syn::bracketed!(content in meta.input);
            let parsed: Punctuated<syn::LitInt, Token![,]> =
                Punctuated::parse_terminated(&content)?;
            channels = parsed
                .into_iter()
                .map(|lit| lit.base10_parse())
                .collect::<syn::Result<Vec<usize>>>()?;
            Ok(())
        } else {
            Err(meta.error("expected `module`, `port`, `channel`, or `channels`"))
        }
    })?;

    let module = module
        .ok_or_else(|| syn::Error::new(attr.span(), "missing `module` in default_connection"))?;
    let port =
        port.ok_or_else(|| syn::Error::new(attr.span(), "missing `port` in default_connection"))?;
    if channels.is_empty() {
        return Err(syn::Error::new(
            attr.span(),
            "missing `channel` or `channels` in default_connection",
        ));
    }

    Ok(DefaultConnectionAttr {
        module,
        port,
        channels,
    })
}

/// Check if a type is exactly PolySignal (for default_connection code generation)
fn is_poly_signal_type(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|seg| seg.ident == "PolySignal")
            .unwrap_or(false),
        _ => false,
    }
}

fn impl_connect_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (default_connection_stmts, connect_body) = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let mut default_stmts = TokenStream2::new();
                let mut connect_stmts = TokenStream2::new();

                for field in fields.named.iter() {
                    let Some(field_ident) = &field.ident else {
                        continue;
                    };

                    // Check for #[default_connection(...)] attribute
                    for attr in &field.attrs {
                        if attr.path().is_ident("default_connection") {
                            match parse_default_connection_attr(attr) {
                                Ok(dc) => {
                                    let module = &dc.module;
                                    let port = &dc.port;
                                    let is_poly = is_poly_signal_type(&field.ty);

                                    if is_poly {
                                        // Generate PolySignal default
                                        let cable_exprs: Vec<TokenStream2> = dc
                                            .channels
                                            .iter()
                                            .map(|ch| {
                                                quote! {
                                                    crate::types::WellKnownModule::#module.to_cable(#ch, #port)
                                                }
                                            })
                                            .collect();
                                        default_stmts.extend(quote_spanned! {field.span()=>
                                            if self.#field_ident.is_disconnected() {
                                                self.#field_ident = crate::poly::PolySignal::poly(&[
                                                    #(#cable_exprs),*
                                                ]);
                                            }
                                        });
                                    } else {
                                        // Generate Signal default (single channel)
                                        let ch = dc.channels.first().copied().unwrap_or(0);
                                        default_stmts.extend(quote_spanned! {field.span()=>
                                            if self.#field_ident.is_disconnected() {
                                                self.#field_ident = crate::types::WellKnownModule::#module.to_cable(#ch, #port);
                                            }
                                        });
                                    }
                                }
                                Err(e) => return e.to_compile_error().into(),
                            }
                        }
                    }

                    // Always call connect on every field (no-op impls handle primitives)
                    connect_stmts.extend(quote_spanned! {field.span()=>
                        crate::types::Connect::connect(&mut self.#field_ident, patch);
                    });
                }

                (default_stmts, connect_stmts)
            }
            Fields::Unnamed(_) | Fields::Unit => {
                return syn::Error::new(
                    ast.span(),
                    "#[derive(Connect)] only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return syn::Error::new(ast.span(), "#[derive(Connect)] only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let generated = quote! {
        impl crate::types::Connect for #name {
            fn connect(&mut self, patch: &crate::Patch) {
                // Apply default connections for disconnected inputs
                #default_connection_stmts
                // Connect all fields
                #connect_body
            }
        }
    };

    generated.into()
}

#[proc_macro_derive(ChannelCount)]
pub fn channel_count_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    impl_channel_count_macro(&ast)
}

fn impl_channel_count_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let poly_signal_field_refs: Vec<TokenStream2> = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|field| {
                    let field_ident = field.ident.as_ref()?;
                    if is_poly_signal(&field.ty) {
                        Some(quote! { &self.#field_ident })
                    } else {
                        None
                    }
                })
                .collect(),
            Fields::Unnamed(_) | Fields::Unit => {
                return syn::Error::new(
                    ast.span(),
                    "#[derive(ChannelCount)] only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return syn::Error::new(ast.span(), "#[derive(ChannelCount)] only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let generated = quote! {
        impl crate::types::PolySignalFields for #name {
            fn poly_signal_fields(&self) -> Vec<&crate::poly::PolySignal> {
                vec![#(#poly_signal_field_refs),*]
            }
        }
    };

    generated.into()
}

struct ArgAttr {
    name: Ident,
    optional: bool,
}

impl syn::parse::Parse for ArgAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let optional = if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            true
        } else {
            false
        };
        Ok(ArgAttr { name, optional })
    }
}

/// Check if a type is exactly PolySignal (not nested in Option, Vec, etc.)
fn is_poly_signal(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => {
            let last = match tp.path.segments.last() {
                Some(seg) => seg,
                None => return false,
            };
            last.ident == "PolySignal"
        }
        _ => false,
    }
}

fn impl_module_macro_attr(
    ast: &DeriveInput,
    attr_args: &ModuleAttrArgs,
) -> syn::Result<TokenStream2> {
    let name = &ast.ident;
    let module_name = &attr_args.module.name;
    let module_description = &attr_args.module.description;

    // Extract /// doc comments from the module struct for detailed documentation
    let module_documentation = extract_doc_comments(&ast.attrs);
    let module_documentation_token = match &module_documentation {
        Some(doc) => quote! { Some(#doc.to_string()) },
        None => quote! { None },
    };

    // Store channels info for channel_count generation
    let hardcoded_channels = attr_args.module.channels;
    let channels_param_name = attr_args.module.channels_param.clone();
    let channels_param_default_val = attr_args.module.channels_param_default;
    let channels_derive_fn = &attr_args.module.channels_derive;

    let module_channels = match attr_args.module.channels {
        Some(n) => quote! { Some(#n) },
        None => quote! { None },
    };
    let module_channels_param = match &attr_args.module.channels_param {
        Some(s) => quote! { Some(#s.to_string()) },
        None => quote! { None },
    };
    let module_channels_param_default = match attr_args.module.channels_param_default {
        Some(n) => quote! { Some(#n) },
        None => quote! { None },
    };

    let has_args = attr_args.has_args;
    let positional_args_exprs: Vec<TokenStream2> = attr_args
        .args
        .iter()
        .map(|arg| {
            let arg_name = arg.name.to_string();
            let optional = arg.optional;
            quote! {
                crate::types::PositionalArg {
                    name: #arg_name.to_string(),
                    optional: #optional,
                }
            }
        })
        .collect();

    // The module struct must contain a field named `outputs`.
    let outputs_ty: Type = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                // Disallow legacy per-field #[output] annotations on the module struct.
                if fields
                    .named
                    .iter()
                    .any(|f| unwrap_attr(&f.attrs, "output").is_some())
                {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "#[module] expects an `outputs` field (a struct that derives Outputs); do not annotate module fields with #[output(...)]",
                    ));
                }

                let outputs_field = fields
                    .named
                    .iter()
                    .find(|f| f.ident.as_ref().map(|i| i == "outputs").unwrap_or(false));

                match outputs_field {
                    Some(f) => f.ty.clone(),
                    None => {
                        return Err(syn::Error::new(
                            Span::call_site(),
                            "#[module] requires a field named `outputs` whose type derives Outputs",
                        ));
                    }
                }
            }
            Fields::Unnamed(_) | Fields::Unit => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "#[module] can only be applied to structs with named fields",
                ));
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return Err(syn::Error::new(
                Span::call_site(),
                "#[module] can only be applied to structs",
            ));
        }
    };

    let struct_name = format_ident!("{}Sampleable", name);
    let constructor_name = format_ident!("{}Constructor", name)
        .to_string()
        .to_case(Case::Snake);
    let constructor_name = Ident::new(&constructor_name, Span::call_site());
    let params_struct_name = format_ident!("{}Params", name);

    // Extract generics for proper impl blocks
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // For the wrapper struct, we need to replace all lifetime parameters with 'static
    // since Sampleable requires 'static. Build a static version of ty_generics.
    let static_ty_generics = {
        let params = ast
            .generics
            .params
            .iter()
            .map(|p| match p {
                syn::GenericParam::Lifetime(_) => quote!('static),
                syn::GenericParam::Type(t) => {
                    let ident = &t.ident;
                    quote!(#ident)
                }
                syn::GenericParam::Const(c) => {
                    let ident = &c.ident;
                    quote!(#ident)
                }
            })
            .collect::<Vec<_>>();
        if params.is_empty() {
            quote!()
        } else {
            quote!(<#(#params),*>)
        }
    };

    let is_stateful = attr_args.stateful;

    let get_state_impl = if is_stateful {
        if has_args {
            // Stateful module with positional args - merge argument_spans into state
            quote! {
                use crate::types::StatefulModule;
                // SAFETY: Audio thread has exclusive access. See crate::types module documentation.
                let module = unsafe { &*self.module.get() };
                let argument_spans = unsafe { &*self.argument_spans.get() };

                // Get base state from module's StatefulModule impl
                let state = module.get_state();

                // If we have argument spans, merge them into the state
                if argument_spans.is_empty() {
                    state
                } else {
                    match (state, serde_json::to_value(argument_spans).ok()) {
                        (Some(serde_json::Value::Object(mut obj)), Some(spans)) => {
                            obj.insert("argument_spans".to_string(), spans);
                            Some(serde_json::Value::Object(obj))
                        }
                        (Some(state_val), Some(spans)) => {
                            // State exists but isn't an object - wrap it
                            Some(serde_json::json!({
                                "_state": state_val,
                                "argument_spans": spans
                            }))
                        }
                        (None, Some(spans)) => {
                            // No base state, create one with just argument_spans
                            Some(serde_json::json!({
                                "argument_spans": spans
                            }))
                        }
                        (state, None) => state,
                    }
                }
            }
        } else {
            // Stateful module without args - just return module state
            quote! {
                use crate::types::StatefulModule;
                // SAFETY: Audio thread has exclusive access. See crate::types module documentation.
                let module = unsafe { &*self.module.get() };
                module.get_state()
            }
        }
    } else if has_args {
        // Non-stateful module with args - return argument_spans only if present
        quote! {
            let argument_spans = unsafe { &*self.argument_spans.get() };
            if !argument_spans.is_empty() {
                serde_json::to_value(std::collections::HashMap::from([
                    ("argument_spans".to_string(), argument_spans.clone())
                ])).ok()
            } else {
                None
            }
        }
    } else {
        quote! { None }
    };

    // Check for has_init flag
    let has_init_call = if attr_args.has_init {
        quote! {
            // SAFETY: We just created sampleable, no one else has access yet.
            unsafe { (*sampleable.module.get()).init(sample_rate); }
        }
    } else {
        quote! {}
    };

    // Check for patch_update flag
    let on_patch_update_impl = if attr_args.patch_update {
        quote! {
            fn on_patch_update(&self) {
                use crate::types::PatchUpdateHandler;
                // SAFETY: Audio thread has exclusive access. See crate::types module documentation.
                let module = unsafe { &mut *self.module.get() };
                PatchUpdateHandler::on_patch_update(module);
            }
        }
    } else {
        quote! {
            fn on_patch_update(&self) {}
        }
    };

    // Generate the channel count derivation function name
    let channel_count_fn_name = format_ident!(
        "__{}_derive_channel_count",
        name.to_string().to_case(Case::Snake)
    );

    // Generate the core channel count implementation that works with typed params.
    let channel_count_fn_impl = if let Some(custom_fn) = channels_derive_fn {
        quote! {
            #[inline]
            fn #channel_count_fn_name(params: &#params_struct_name) -> usize {
                #custom_fn(params)
            }
        }
    } else {
        match (
            hardcoded_channels,
            &channels_param_name,
            channels_param_default_val,
        ) {
            (Some(n), _, _) => {
                let n = n as usize;
                quote! {
                    #[inline]
                    fn #channel_count_fn_name(_params: &#params_struct_name) -> usize {
                        #n
                    }
                }
            }
            (None, Some(param_name), default_val) => {
                let param_ident = Ident::new(&param_name.value(), param_name.span());
                match default_val {
                    Some(default) => {
                        let default = default as usize;
                        quote! {
                            #[inline]
                            fn #channel_count_fn_name(params: &#params_struct_name) -> usize {
                                let param_value = params.#param_ident;
                                if param_value > 0 {
                                    param_value.clamp(1, crate::poly::PORT_MAX_CHANNELS)
                                } else {
                                    #default
                                }
                            }
                        }
                    }
                    None => {
                        quote! {
                            #[inline]
                            fn #channel_count_fn_name(params: &#params_struct_name) -> usize {
                                params.#param_ident.clamp(1, crate::poly::PORT_MAX_CHANNELS)
                            }
                        }
                    }
                }
            }
            (None, None, _) => {
                quote! {
                    #[inline]
                    fn #channel_count_fn_name(params: &#params_struct_name) -> usize {
                        use crate::types::PolySignalFields;
                        let fields = params.poly_signal_fields();
                        let refs: Vec<&crate::poly::PolySignal> = fields.into_iter().collect();
                        crate::poly::PolySignal::max_channels(&refs).max(1) as usize
                    }
                }
            }
        }
    };

    let generated = quote! {
        // Generated core channel count function (used by derive_channel_count and initial default)
        // IMPORTANT: This function should never be called within the audio thread.
        // It may be computationally expensive. It should only be called in non-audio-thread contexts.
        #channel_count_fn_impl

        impl #impl_generics #name #ty_generics #where_clause {
            /// Returns the precomputed channel count injected by `try_update_params`.
            #[inline]
            pub fn channel_count(&self) -> usize {
                self._channel_count
            }
        }

        /// Generated wrapper struct for audio-thread-only module access.
        ///
        /// # Safety Model (UnsafeCell)
        ///
        /// This struct uses `UnsafeCell` instead of `Mutex`/`RwLock` for interior mutability.
        /// This is safe because:
        ///
        /// 1. **Exclusive Audio Thread Ownership**: After construction, all modules live in
        ///    `AudioProcessor::patch` which is owned exclusively by the audio thread closure.
        ///    See `crates/modular/src/audio.rs` `make_stream()`.
        ///
        /// 2. **Command Queue Isolation**: The main thread communicates via `PatchUpdate`
        ///    commands through an `rtrb` SPSC queue. It never directly accesses module state.
        ///
        /// 3. **No Escaping References**: Module `Arc`s are stored in `Patch::sampleables` and
        ///    are never cloned or sent to other threads after being added to the patch.
        ///
        /// ## Invariants (DO NOT VIOLATE)
        ///
        /// - **NEVER** call Sampleable trait methods from the main thread
        /// - **NEVER** clone module Arcs and send them across threads
        /// - **NEVER** access Patch::sampleables from outside AudioProcessor
        /// - **ALWAYS** use the command queue for main→audio communication
        ///
        /// Violating these invariants will cause undefined behavior (data races).
        struct #struct_name {
            id: String,
            outputs: std::cell::UnsafeCell<#outputs_ty>,
            module: std::cell::UnsafeCell<#name #static_ty_generics>,
            processed: core::sync::atomic::AtomicBool,
            sample_rate: f32,
            argument_spans: std::cell::UnsafeCell<std::collections::HashMap<String, crate::types::ArgumentSpan>>,
        }

        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    id: String::new(),
                    outputs: std::cell::UnsafeCell::new(Default::default()),
                    module: std::cell::UnsafeCell::new(Default::default()),
                    processed: core::sync::atomic::AtomicBool::new(false),
                    sample_rate: 0.0,
                    argument_spans: std::cell::UnsafeCell::new(std::collections::HashMap::new()),
                }
            }
        }

        // SAFETY: This type is only accessed from the audio thread after construction.
        unsafe impl Send for #struct_name {}
        unsafe impl Sync for #struct_name {}

        impl crate::types::Sampleable for #struct_name {
            fn tick(&self) -> () {
                self.processed.store(false, core::sync::atomic::Ordering::Release);
            }

            fn update(&self) -> () {
                if let Ok(_) = self.processed.compare_exchange(
                    false,
                    true,
                    core::sync::atomic::Ordering::Acquire,
                    core::sync::atomic::Ordering::Relaxed,
                ) {
                    unsafe {
                        let module = &mut *self.module.get();
                        module.update(self.sample_rate);
                        let outputs = &mut *self.outputs.get();
                        crate::types::OutputStruct::copy_from(outputs, &module.outputs);
                    }
                }
            }

            fn get_poly_sample(&self, port: &str) -> napi::Result<crate::poly::PolyOutput> {
                self.update();
                let outputs = unsafe { &*self.outputs.get() };
                crate::types::OutputStruct::get_poly_sample(outputs, port).ok_or_else(|| {
                    napi::Error::from_reason(
                        format!(
                            "{} with id {} does not have port {}",
                            #module_name,
                            &self.id,
                            port
                        )
                    )
                })
            }

            fn get_module_type(&self) -> &str {
                #module_name
            }

            fn try_update_params(&self, params: serde_json::Value, channel_count: usize) -> napi::Result<()> {
                let module = unsafe { &mut *self.module.get() };
                let argument_spans = unsafe { &mut *self.argument_spans.get() };

                let params_to_deserialize = if params.is_object() {
                    let mut obj = match params {
                        serde_json::Value::Object(o) => o,
                        _ => unreachable!(),
                    };
                    if let Some(spans_value) = obj.remove("__argument_spans") {
                        if let serde_json::Value::Object(spans_obj) = spans_value {
                            argument_spans.clear();
                            for (key, value) in spans_obj {
                                if let Ok(span) = serde_json::from_value::<crate::types::ArgumentSpan>(value) {
                                    argument_spans.insert(key, span);
                                }
                            }
                        }
                    }
                    serde_json::Value::Object(obj)
                } else {
                    params
                };
                module.params = serde_json::from_value(params_to_deserialize)?;
                module._channel_count = channel_count;
                crate::types::OutputStruct::set_all_channels(&mut module.outputs, channel_count);
                Ok(())
            }

            fn get_id(&self) -> &str {
                &self.id
            }

            fn connect(&self, patch: &crate::Patch) {
                let module = unsafe { &mut *self.module.get() };
                crate::types::Connect::connect(&mut module.params, patch);
            }

            #on_patch_update_impl

            fn get_state(&self) -> Option<serde_json::Value> {
                #get_state_impl
            }
        }

        fn #constructor_name(id: &String, sample_rate: f32) -> napi::Result<std::sync::Arc<Box<dyn crate::types::Sampleable>>> {
            let sampleable = #struct_name {
                id: id.clone(),
                sample_rate,
                ..#struct_name::default()
            };
            #has_init_call
            Ok(std::sync::Arc::new(Box::new(sampleable)))
        }

        impl #impl_generics crate::types::Module for #name #ty_generics #where_clause {
            fn install_constructor(map: &mut std::collections::HashMap<String, crate::types::SampleableConstructor>) {
                map.insert(#module_name.into(), Box::new(#constructor_name));
            }

            fn install_params_validator(map: &mut std::collections::HashMap<String, crate::types::ParamsValidator>) {
                map.insert(#module_name.into(), Self::validate_params_json as crate::types::ParamsValidator);
            }

            fn validate_params_json(params: &serde_json::Value) -> napi::Result<()> {
                let params_to_validate = if params.is_object() {
                    let mut obj = params.as_object().unwrap().clone();
                    obj.remove("__argument_spans");
                    serde_json::Value::Object(obj)
                } else {
                    params.clone()
                };
                let _parsed: #params_struct_name = serde_json::from_value(params_to_validate)?;
                Ok(())
            }

            fn derive_channel_count(params: &serde_json::Value) -> Option<usize> {
                let params_to_parse = if params.is_object() {
                    let mut obj = params.as_object().unwrap().clone();
                    obj.remove("__argument_spans");
                    serde_json::Value::Object(obj)
                } else {
                    params.clone()
                };
                let parsed: #params_struct_name = serde_json::from_value(params_to_parse).ok()?;
                Some(#channel_count_fn_name(&parsed))
            }

            fn get_schema() -> crate::types::ModuleSchema {
                let params_schema = schemars::schema_for!(#params_struct_name);

                let mut param_names: std::collections::HashSet<String> = std::collections::HashSet::new();
                if let Some(obj) = params_schema.as_object() {
                    let props = obj
                        .get("properties")
                        .and_then(|v| v.as_object())
                        .or_else(|| {
                            obj.get("schema")
                                .and_then(|s| s.as_object())
                                .and_then(|s| s.get("properties"))
                                .and_then(|v| v.as_object())
                        });
                    if let Some(props) = props {
                        for key in props.keys() {
                            if key == "o" || key == "out" {
                                panic!("Parameter name '{}' is reserved and cannot be used", key);
                            }
                            param_names.insert(key.clone());
                        }
                    }
                }

                let output_schemas = <#outputs_ty as crate::types::OutputStruct>::schemas();
                if output_schemas.iter().any(|o| param_names.contains(&o.name)) {
                    panic!("Parameters and outputs must have unique names");
                }

                crate::types::ModuleSchema {
                    name: #module_name.to_string(),
                    description: #module_description.to_string(),
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
            }
        }
    };
    Ok(generated)
}
