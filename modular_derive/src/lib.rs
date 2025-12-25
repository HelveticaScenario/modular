extern crate quote;
extern crate syn;

extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{Attribute, LitStr, Token, parse::Parser, punctuated::Punctuated, spanned::Spanned};
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};

/// Parsed output attribute data
struct OutputAttr {
    name: LitStr,
    description: Option<LitStr>,
    is_default: bool,
}

#[proc_macro_derive(Outputs, attributes(output))]
pub fn outputs_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    impl_outputs_macro(&ast)
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
fn parse_output_attr(tokens: TokenStream2) -> OutputAttr {
    use syn::Result as SynResult;
    use syn::parse::{Parse, ParseStream};

    struct OutputAttrParser {
        name: LitStr,
        description: Option<LitStr>,
        is_default: bool,
    }

    impl Parse for OutputAttrParser {
        fn parse(input: ParseStream) -> SynResult<Self> {
            // Parse first string literal (name)
            let name: LitStr = input.parse()?;
            if name.value() == "o" {
                return Err(syn::Error::new(
                    name.span(),
                    "Output name cannot be 'o' as it is a reserved keyword",
                ));
            }

            input.parse::<Token![,]>()?;

            // Try to parse second element - description (string)
            // It's a description string
            let description: LitStr = input.parse()?;

            // Check if there's another comma
            if !input.peek(Token![,]) {
                return Ok(OutputAttrParser {
                    name,
                    description: Some(description),
                    is_default: false,
                });
            }
            input.parse::<Token![,]>()?;

            // Parse the 'default' keyword
            let default_ident: Ident = input.parse()?;
            if default_ident != "default" {
                return Err(syn::Error::new(
                    default_ident.span(),
                    format!("Expected 'default', found '{}'", default_ident),
                ));
            }

            Ok(OutputAttrParser {
                name,
                description: Some(description),
                is_default: true,
            })
        }
    }

    let parsed = syn::parse2::<OutputAttrParser>(tokens).expect("Failed to parse output attribute");

    OutputAttr {
        name: parsed.name,
        description: parsed.description,
        is_default: parsed.is_default,
    }
}

#[proc_macro_derive(Module, attributes(output, module))]
pub fn module_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // // Build the trait implementation
    impl_module_macro(&ast)
}

fn impl_outputs_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let outputs: Vec<(
        Ident,
        bool,
        LitStr,
        TokenStream2,
        TokenStream2,
        TokenStream2,
    )> = match ast.data {
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

                    // Enforce f32 outputs (engine expects f32 samples).
                    let is_f32 = match &f.ty {
                        Type::Path(tp) => tp
                            .path
                            .segments
                            .last()
                            .map(|seg| seg.ident == "f32")
                            .unwrap_or(false),
                        _ => false,
                    };
                    if !is_f32 {
                        return syn::Error::new(f.ty.span(), "Output fields must have type f32")
                            .to_compile_error()
                            .into();
                    }

                    let output_attr = parse_output_attr(output_attr_tokens);
                    let output_name = output_attr.name;
                    let description = output_attr
                        .description
                        .as_ref()
                        .map(|d| quote!(#d.to_string()))
                        .unwrap_or(quote!("".to_string()));
                    let is_default = output_attr.is_default;

                    out.push((
                        field_name.clone(),
                        is_default,
                        output_name.clone(),
                        quote! {
                            #output_name => Some(self.#field_name),
                        },
                        quote! {
                            crate::types::OutputSchema {
                                name: #output_name.to_string(),
                                description: #description,
                                default: #is_default,
                            }
                        },
                        quote! {
                            self.#field_name = other.#field_name;
                        },
                    ));
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

    // Validate that at most one output is marked as default.
    let default_count = outputs
        .iter()
        .filter(|(_, is_default, _, _, _, _)| *is_default)
        .count();
    if default_count > 1 {
        let error_msg = format!(
            "Outputs struct '{}' has {} outputs marked as default, but only one is allowed",
            name, default_count
        );
        return syn::Error::new(Span::call_site(), error_msg)
            .to_compile_error()
            .into();
    }

    let field_idents = outputs.iter().map(|(field, _, _, _, _, _)| field);
    let sample_match_arms = outputs.iter().map(|(_, _, _, arm, _, _)| arm);
    let schema_exprs = outputs.iter().map(|(_, _, _, _, schema, _)| schema);
    let copy_stmts = outputs.iter().map(|(_, _, _, _, _, copy)| copy);

    let generated = quote! {
        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#field_idents: 0.0,)*
                }
            }
        }

        impl crate::types::OutputStruct for #name {
            fn copy_from(&mut self, other: &Self) {
                #(#copy_stmts)*
            }

            fn get_sample(&self, port: &str) -> Option<f32> {
                match port {
                    #(#sample_match_arms)*
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

#[proc_macro_derive(Connect)]
pub fn connect_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    impl_connect_macro(&ast)
}

fn contains_signal(ty: &Type) -> bool {
    match ty {
        Type::Paren(p) => contains_signal(&p.elem),
        Type::Group(g) => contains_signal(&g.elem),
        Type::Reference(r) => contains_signal(&r.elem),
        Type::Array(a) => contains_signal(&a.elem),
        Type::Slice(s) => contains_signal(&s.elem),
        Type::Path(tp) => {
            let last = match tp.path.segments.last() {
                Some(seg) => seg,
                None => return false,
            };

            if last.ident == "Signal" {
                return true;
            }

            if let PathArguments::AngleBracketed(args) = &last.arguments {
                return args.args.iter().any(|arg| match arg {
                    GenericArgument::Type(inner_ty) => contains_signal(inner_ty),
                    _ => false,
                });
            }

            false
        }
        Type::Tuple(tt) => tt.elems.iter().any(contains_signal),
        _ => false,
    }
}

fn first_type_arg(_span: Span, last: &syn::PathSegment) -> Option<&Type> {
    match &last.arguments {
        PathArguments::AngleBracketed(args) => args.args.iter().find_map(|arg| match arg {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        }),
        _ => None,
    }
}

fn nth_type_arg(_span: Span, last: &syn::PathSegment, idx: usize) -> Option<&Type> {
    match &last.arguments {
        PathArguments::AngleBracketed(args) => args
            .args
            .iter()
            .filter_map(|arg| match arg {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .nth(idx),
        _ => None,
    }
}

fn gen_connect_stmts(
    ty: &Type,
    place_expr: TokenStream2,
    depth: usize,
    span: Span,
) -> TokenStream2 {
    match ty {
        Type::Paren(p) => gen_connect_stmts(&p.elem, place_expr, depth, span),
        Type::Group(g) => gen_connect_stmts(&g.elem, place_expr, depth, span),
        Type::Reference(r) => gen_connect_stmts(&r.elem, quote! { *#place_expr }, depth, span),
        Type::Array(a) => {
            if !contains_signal(&a.elem) {
                return quote! {};
            }
            let item_ident = format_ident!("__connect_item{}", depth);
            let inner_place = quote! { *#item_ident };
            let inner_body = gen_connect_stmts(&a.elem, inner_place, depth + 1, span);
            quote_spanned! {span=>
                for #item_ident in (#place_expr).iter_mut() {
                    #inner_body
                }
            }
        }
        Type::Slice(s) => {
            if !contains_signal(&s.elem) {
                return quote! {};
            }
            let item_ident = format_ident!("__connect_item{}", depth);
            let inner_place = quote! { *#item_ident };
            let inner_body = gen_connect_stmts(&s.elem, inner_place, depth + 1, span);
            quote_spanned! {span=>
                for #item_ident in (#place_expr).iter_mut() {
                    #inner_body
                }
            }
        }
        Type::Path(tp) => {
            let last = match tp.path.segments.last() {
                Some(seg) => seg,
                None => return quote! {},
            };

            if last.ident == "Signal" {
                return quote_spanned! {span=>
                    crate::types::Connect::connect(&mut #place_expr, patch);
                };
            }

            if last.ident == "Vec" {
                let Some(inner_ty) = first_type_arg(span, last) else {
                    return quote! {};
                };
                if !contains_signal(inner_ty) {
                    return quote! {};
                }
                let item_ident = format_ident!("__connect_item{}", depth);
                let inner_place = quote! { *#item_ident };
                let inner_body = gen_connect_stmts(inner_ty, inner_place, depth + 1, span);
                return quote_spanned! {span=>
                    for #item_ident in (#place_expr).iter_mut() {
                        #inner_body
                    }
                };
            }

            if last.ident == "Option" {
                let Some(inner_ty) = first_type_arg(span, last) else {
                    return quote! {};
                };
                if !contains_signal(inner_ty) {
                    return quote! {};
                }
                let item_ident = format_ident!("__connect_item{}", depth);
                let inner_place = quote! { *#item_ident };
                let inner_body = gen_connect_stmts(inner_ty, inner_place, depth + 1, span);
                return quote_spanned! {span=>
                    if let Some(#item_ident) = (#place_expr).as_mut() {
                        #inner_body
                    }
                };
            }

            if last.ident == "Box" {
                let Some(inner_ty) = first_type_arg(span, last) else {
                    return quote! {};
                };
                if !contains_signal(inner_ty) {
                    return quote! {};
                }
                let inner_place = quote! { **(#place_expr) };
                return gen_connect_stmts(inner_ty, inner_place, depth + 1, span);
            }

            if last.ident == "HashMap" || last.ident == "BTreeMap" {
                let Some(value_ty) = nth_type_arg(span, last, 1) else {
                    return quote! {};
                };
                if !contains_signal(value_ty) {
                    return quote! {};
                }
                let key_ident = format_ident!("__connect_key{}", depth);
                let val_ident = format_ident!("__connect_val{}", depth);
                let inner_place = quote! { *#val_ident };
                let inner_body = gen_connect_stmts(value_ty, inner_place, depth + 1, span);
                return quote_spanned! {span=>
                    for (#key_ident, #val_ident) in (#place_expr).iter_mut() {
                        let _ = #key_ident;
                        #inner_body
                    }
                };
            }

            quote! {}
        }
        Type::Tuple(tt) => {
            let mut out = TokenStream2::new();
            for (idx, elem_ty) in tt.elems.iter().enumerate() {
                if !contains_signal(elem_ty) {
                    continue;
                }
                let index = syn::Index::from(idx);
                let elem_place = quote! { (#place_expr).#index };
                out.extend(gen_connect_stmts(elem_ty, elem_place, depth + 1, span));
            }
            out
        }
        _ => quote! {},
    }
}

fn impl_connect_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let connect_body = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let mut stmts = TokenStream2::new();
                for field in fields.named.iter() {
                    let Some(field_ident) = &field.ident else {
                        continue;
                    };
                    if !contains_signal(&field.ty) {
                        continue;
                    }
                    let place_expr = quote! { self.#field_ident };
                    stmts.extend(gen_connect_stmts(&field.ty, place_expr, 0, field.span()));
                }
                stmts
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
                #connect_body
            }
        }
    };

    generated.into()
}

fn impl_module_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (module_name, module_description) = unwrap_name_description(&ast.attrs, "module");

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
    let generated = quote! {
        #[derive(Default)]
        struct #struct_name {
            id: String,
            outputs: parking_lot::RwLock<#outputs_ty>,
            module: parking_lot::Mutex<#name>,
            processed: core::sync::atomic::AtomicBool,
            sample_rate: f32
        }

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
                    let mut module = self.module.lock();
                    module.update(self.sample_rate);
                    let mut outputs = self.outputs.try_write_for(core::time::Duration::from_millis(10)).unwrap();
                    crate::types::OutputStruct::copy_from(&mut *outputs, &module.outputs);
                }
            }

            fn get_sample(&self, port: &String) -> Result<f32> {
                self.update();
                let outputs = self.outputs.try_read_for(core::time::Duration::from_millis(10)).unwrap();
                crate::types::OutputStruct::get_sample(&*outputs, port.as_str()).ok_or_else(|| {
                    anyhow!(
                        "{} with id {} does not have port {}",
                        #module_name,
                        &self.id,
                        port
                    )
                })
            }

            fn get_module_type(&self) -> String {
                #module_name.to_owned()
            }

            fn try_update_params(&self, params: serde_json::Value) -> Result<()> {
                let mut module = self.module.lock();
                module.params = serde_json::from_value(params)?;
                Ok(())
            }

            fn get_id(&self) -> &String {
                &self.id
            }

            fn connect(&self, patch: &crate::Patch) {
                let mut module = self.module.lock();
                crate::types::Connect::connect(&mut module.params, patch);
            }
        }

        fn #constructor_name(id: &String, sample_rate: f32) -> Result<std::sync::Arc<Box<dyn crate::types::Sampleable>>> {
            Ok(std::sync::Arc::new(Box::new(#struct_name {
                id: id.clone(),
                sample_rate,
                ..#struct_name::default()
            })))
        }

        impl crate::types::Module for #name {
            fn install_constructor(map: &mut std::collections::HashMap<String, crate::types::SampleableConstructor>) {
                map.insert(#module_name.into(), Box::new(#constructor_name));
            }

            fn install_params_validator(map: &mut std::collections::HashMap<String, crate::types::ParamsValidator>) {
                map.insert(#module_name.into(), Self::validate_params_json as crate::types::ParamsValidator);
            }

            fn validate_params_json(params: &serde_json::Value) -> anyhow::Result<()> {
                // Attempt to deserialize the JSON params object into the module's concrete
                // `*Params` struct. If this fails, the patch's params shape is incompatible
                // with what the DSP module expects.
                let _parsed: #params_struct_name = serde_json::from_value(params.clone())?;
                Ok(())
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
                    params_schema,
                    outputs: output_schemas,
                }
            }
        }
    };
    generated.into()
}
