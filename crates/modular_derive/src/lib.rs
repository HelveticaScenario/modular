extern crate quote;
extern crate syn;

extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{Attribute, LitStr, Token, parse::Parser, punctuated::Punctuated, spanned::Spanned};
use syn::{Data, DeriveInput, Fields, Type};

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

fn unwrap_attr(attrs: &Vec<Attribute>, ident: &str) -> Option<TokenStream2> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident(ident))
        .next()
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

/// Parse module attribute tokens into ModuleAttr
/// Supports:
/// - #[module("name", "description")]
/// - #[module("name", "description", channels = N)]
/// - #[module("name", "description", channels_param = "paramName", channels_param_default = N)]
/// - #[module("name", "description", channels_derive = my_derive_fn)]
fn parse_module_attr(attrs: &Vec<Attribute>) -> ModuleAttr {
    use syn::Result as SynResult;
    use syn::parse::{Parse, ParseStream};

    struct ModuleAttrParser {
        name: LitStr,
        description: Option<LitStr>,
        channels: Option<u8>,
        channels_param: Option<LitStr>,
        channels_param_default: Option<u8>,
        channels_derive: Option<syn::Path>,
    }

    impl Parse for ModuleAttrParser {
        fn parse(input: ParseStream) -> SynResult<Self> {
            // Parse first string literal (name)
            let name: LitStr = input.parse()?;

            let mut description: Option<LitStr> = None;
            let mut channels: Option<u8> = None;
            let mut channels_param: Option<LitStr> = None;
            let mut channels_param_default: Option<u8> = None;
            let mut channels_derive: Option<syn::Path> = None;

            // Parse remaining optional elements
            while input.peek(Token![,]) {
                input.parse::<Token![,]>()?;

                if input.is_empty() {
                    break;
                }

                // Check if it's a string literal (description) or identifier (attribute)
                if input.peek(LitStr) {
                    description = Some(input.parse()?);
                } else {
                    // Try to parse an identifier
                    let ident: Ident = input.parse()?;

                    if ident == "channels" {
                        input.parse::<Token![=]>()?;
                        let lit: syn::LitInt = input.parse()?;
                        channels = Some(lit.base10_parse()?);
                    } else if ident == "channels_param" {
                        input.parse::<Token![=]>()?;
                        let lit: LitStr = input.parse()?;
                        channels_param = Some(lit);
                    } else if ident == "channels_param_default" {
                        input.parse::<Token![=]>()?;
                        let lit: syn::LitInt = input.parse()?;
                        channels_param_default = Some(lit.base10_parse()?);
                    } else if ident == "channels_derive" {
                        input.parse::<Token![=]>()?;
                        let path: syn::Path = input.parse()?;
                        channels_derive = Some(path);
                    } else {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!(
                                "Unknown module attribute '{}'. Expected 'channels', 'channels_param', 'channels_param_default', or 'channels_derive'",
                                ident
                            ),
                        ));
                    }
                }
            }

            Ok(ModuleAttrParser {
                name,
                description,
                channels,
                channels_param,
                channels_param_default,
                channels_derive,
            })
        }
    }

    let tokens = unwrap_attr(attrs, "module").expect("Missing #[module(...)] attribute");
    let parsed = syn::parse2::<ModuleAttrParser>(tokens).expect("Failed to parse module attribute");

    ModuleAttr {
        name: parsed.name,
        description: parsed.description,
        channels: parsed.channels,
        channels_param: parsed.channels_param,
        channels_param_default: parsed.channels_param_default,
        channels_derive: parsed.channels_derive,
    }
}

fn unwrap_name_description(
    attrs: &Vec<Attribute>,
    ident: &str,
) -> (Option<LitStr>, Option<LitStr>) {
    let attr = unwrap_attr(attrs, ident)
        .map(|tokens| {
            Punctuated::<LitStr, Token![,]>::parse_terminated
                .parse2(tokens)
                .unwrap()
        })
        .unwrap_or_default();
    let mut iter = attr.iter();
    let name = iter.next().map(|lit| lit.clone());
    let description = iter.next().map(|lit| lit.clone());
    (name, description)
}

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
                        name_value,
                        RESERVED_OUTPUT_NAMES
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

#[proc_macro_derive(Module, attributes(output, module, args, stateful, patch_update))]
pub fn module_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // // Build the trait implementation
    impl_module_macro(&ast)
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

    let module =
        module.ok_or_else(|| syn::Error::new(attr.span(), "missing `module` in default_connection"))?;
    let port =
        port.ok_or_else(|| syn::Error::new(attr.span(), "missing `port` in default_connection"))?;
    if channels.is_empty() {
        return Err(syn::Error::new(
            attr.span(),
            "missing `channel` or `channels` in default_connection",
        ));
    }

    Ok(DefaultConnectionAttr { module, port, channels })
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
                                            println!("Checking default connection for field: {}", stringify!(#field_ident));
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
                                            println!("Checking default connection for field: {}", stringify!(#field_ident));
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
                        println!("Connecting field: {}", stringify!(#field_ident));
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
                println!("Connecting module outputs for {}", stringify!(#name));
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

fn impl_module_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let module_attr = parse_module_attr(&ast.attrs);
    let module_name = module_attr.name;
    let module_description = module_attr.description;

    // Store channels info for channel_count generation
    let hardcoded_channels = module_attr.channels;
    let channels_param_name = module_attr.channels_param.clone();
    let channels_param_default_val = module_attr.channels_param_default;
    let channels_derive_fn = module_attr.channels_derive;

    let module_channels = match module_attr.channels {
        Some(n) => quote! { Some(#n) },
        None => quote! { None },
    };
    let module_channels_param = match &module_attr.channels_param {
        Some(s) => quote! { Some(#s.to_string()) },
        None => quote! { None },
    };
    let module_channels_param_default = match module_attr.channels_param_default {
        Some(n) => quote! { Some(#n) },
        None => quote! { None },
    };

    let args_tokens = unwrap_attr(&ast.attrs, "args");
    let positional_args_exprs = if let Some(tokens) = args_tokens {
        let args = Punctuated::<ArgAttr, Token![,]>::parse_terminated
            .parse2(tokens)
            .expect("Failed to parse args attribute");

        args.into_iter()
            .map(|arg| {
                let name = arg.name.to_string();
                let optional = arg.optional;
                quote! {
                    crate::types::PositionalArg {
                        name: #name.to_string(),
                        optional: #optional,
                    }
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    // New convention: the module struct contains a single `outputs` field.
    // The outputs type itself must `#[derive(Outputs)]` which implements `crate::types::OutputStruct`.
    let outputs_ty: Type = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                // Disallow legacy per-field #[output] annotations on the module struct.
                if fields
                    .named
                    .iter()
                    .any(|f| unwrap_attr(&f.attrs, "output").is_some())
                {
                    return syn::Error::new(
                        Span::call_site(),
                        "#[derive(Module)] now expects an `outputs` field (a struct that derives Outputs); do not annotate module fields with #[output(...)]",
                    )
                    .to_compile_error()
                    .into();
                }

                let outputs_field = fields
                    .named
                    .iter()
                    .find(|f| f.ident.as_ref().map(|i| i == "outputs").unwrap_or(false));

                match outputs_field {
                    Some(f) => f.ty.clone(),
                    None => {
                        return syn::Error::new(
                            Span::call_site(),
                            "#[derive(Module)] requires a field named `outputs` whose type derives Outputs",
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
            Fields::Unnamed(_) | Fields::Unit => {
                return syn::Error::new(
                    Span::call_site(),
                    "Module can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return syn::Error::new(Span::call_site(), "Module can only be derived for structs")
                .to_compile_error()
                .into();
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

    let is_stateful = ast
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("stateful"));
    let get_state_impl = if is_stateful {
        quote! {
            use crate::types::StatefulModule;
            // SAFETY: Audio thread has exclusive access. See crate::types module documentation.
            let module = unsafe { &*self.module.get() };
            module.get_state()
        }
    } else {
        quote! { None }
    };

    // Check for #[patch_update] attribute - if present, call the module's on_patch_update
    let has_patch_update = ast
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("patch_update"));
    let on_patch_update_impl = if has_patch_update {
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
    let channel_count_fn_name = format_ident!("__{}_derive_channel_count", name.to_string().to_case(Case::Snake));

    // Generate the core channel count implementation that works with typed params.
    // If channels_derive is specified, use that custom function.
    // Otherwise, generate a default implementation based on schema attributes.
    let channel_count_fn_impl = if let Some(ref custom_fn) = channels_derive_fn {
        // Custom function provided - just call it directly
        // The custom function must have signature: fn(&ParamsStruct) -> usize
        quote! {
            /// Core channel count derivation function for this module.
            /// Called by both `channel_count(&self)` and `derive_channel_count(json)`.
            #[inline]
            fn #channel_count_fn_name(params: &#params_struct_name) -> usize {
                #custom_fn(params)
            }
        }
    } else {
        // Generate default implementation based on schema attributes
        match (
            hardcoded_channels,
            &channels_param_name,
            channels_param_default_val,
        ) {
            // 1. Hardcoded channel count from #[module(..., channels = N)]
            (Some(n), _, _) => {
                let n = n as usize;
                quote! {
                    #[inline]
                    fn #channel_count_fn_name(_params: &#params_struct_name) -> usize {
                        #n
                    }
                }
            }
            // 2. channels_param specified - read from params field, with optional default
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
            // 3. Infer from PolySignal inputs
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
        // Generated core channel count function that both self.channel_count() and 
        // derive_channel_count(json) call.
        #channel_count_fn_impl

        // Generated channel_count method for the module
        impl #impl_generics #name #ty_generics #where_clause {
            /// Get the channel count for this module.
            /// Priority: 1. hardcoded channels, 2. channels_param value/default, 3. max of PolySignal inputs
            #[inline]
            pub fn channel_count(&self) -> usize {
                #channel_count_fn_name(&self.params)
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
            sample_rate: f32
        }

        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    id: String::new(),
                    outputs: std::cell::UnsafeCell::new(Default::default()),
                    module: std::cell::UnsafeCell::new(Default::default()),
                    processed: core::sync::atomic::AtomicBool::new(false),
                    sample_rate: 0.0,
                }
            }
        }

        // SAFETY: This type is only accessed from the audio thread after construction.
        // The audio thread has exclusive ownership of all modules in the Patch.
        // See the struct-level documentation and crates/modular/src/audio.rs AudioProcessor.
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
                    // SAFETY: Audio thread has exclusive access to modules.
                    // The processed flag prevents re-entrant calls within the same frame.
                    // See struct-level documentation for full safety invariants.
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
                // SAFETY: Audio thread has exclusive access. Reading outputs after
                // update() is complete (processed flag ensures no concurrent mutation).
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

            fn try_update_params(&self, params: serde_json::Value) -> napi::Result<()> {
                // SAFETY: Audio thread has exclusive access. See struct-level documentation.
                let module = unsafe { &mut *self.module.get() };
                module.params = serde_json::from_value(params)?;
                Ok(())
            }

            fn get_id(&self) -> &str {
                &self.id
            }

            fn connect(&self, patch: &crate::Patch) {
                // SAFETY: Audio thread has exclusive access. See struct-level documentation.
                let module = unsafe { &mut *self.module.get() };
                crate::types::Connect::connect(&mut module.params, patch);
            }

            #on_patch_update_impl

            fn get_state(&self) -> Option<serde_json::Value> {
                #get_state_impl
            }
        }

        fn #constructor_name(id: &String, sample_rate: f32) -> napi::Result<std::sync::Arc<Box<dyn crate::types::Sampleable>>> {
            Ok(std::sync::Arc::new(Box::new(#struct_name {
                id: id.clone(),
                sample_rate,
                ..#struct_name::default()
            })))
        }

        impl #impl_generics crate::types::Module for #name #ty_generics #where_clause {
            fn install_constructor(map: &mut std::collections::HashMap<String, crate::types::SampleableConstructor>) {
                map.insert(#module_name.into(), Box::new(#constructor_name));
            }

            fn install_params_validator(map: &mut std::collections::HashMap<String, crate::types::ParamsValidator>) {
                map.insert(#module_name.into(), Self::validate_params_json as crate::types::ParamsValidator);
            }

            fn validate_params_json(params: &serde_json::Value) -> napi::Result<()> {
                // Attempt to deserialize the JSON params object into the module's concrete
                // `*Params` struct. If this fails, the patch's params shape is incompatible
                // with what the DSP module expects.
                let _parsed: #params_struct_name = serde_json::from_value(params.clone())?;
                Ok(())
            }

            fn derive_channel_count(params: &serde_json::Value) -> Option<usize> {
                // Deserialize JSON to typed params struct and call the shared implementation.
                let parsed: #params_struct_name = serde_json::from_value(params.clone()).ok()?;
                Some(#channel_count_fn_name(&parsed))
            }

            fn get_schema() -> crate::types::ModuleSchema {

                // Derive JSON Schemas directly from the Rust param/output types.
                // These are forwarded to the frontend for schema-driven editing/validation.
                let params_schema = schemars::schema_for!(#params_struct_name);

                // Validate that parameter names and output names don't overlap.
                // (This is a runtime panic to keep schema generation deterministic and testable.)
                let mut param_names: std::collections::HashSet<String> = std::collections::HashSet::new();
                if let Some(obj) = params_schema.as_object() {
                    // schemars has produced both "properties" (direct schema) and
                    // {"schema": {"properties": ...}} shapes across versions; tolerate both.
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
                            // Check for reserved parameter names
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
    generated.into()
}
