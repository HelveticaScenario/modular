extern crate quote;
extern crate syn;

extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Attribute, Field, FieldsNamed, LitStr, Token, parse::Parser, punctuated::Punctuated,
    spanned::Spanned,
};
use syn::{Data, DeriveInput, Fields};

/// Parsed output attribute data
struct OutputAttr {
    name: LitStr,
    description: Option<LitStr>,
    is_default: bool,
}

#[proc_macro_derive(Params, attributes(name, description, param))]
pub fn params_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // // Build the trait implementation
    impl_params_macro(&ast)
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

fn map_name_description<F, B>(fields: &FieldsNamed, ident: &str, mut closure: F) -> Vec<B>
where
    F: FnMut(&Field, Option<Ident>, Option<LitStr>, Option<LitStr>) -> B,
{
    fields
        .named
        .iter()
        .filter(|f| {
            f.attrs
                .iter()
                .filter(|attr| attr.path().is_ident(ident))
                .count()
                > 0
        })
        .map(|f| {
            let f_name = &f.ident;
            let (name, description) = unwrap_name_description(&f.attrs, ident);
            closure(f, f_name.clone(), name, description)
        })
        .collect()
    // .map(|f| {
    //     let f_name = &f.ident;
    //     let (name, description) = unwrap_name_description(&f.attrs, "param");
    //     (
    //         quote_spanned! {f.span()=>
    //             state.insert(#name.to_owned(), self.#f_name.to_param());
    //         },
    //         quote_spanned! {f.span()=>
    //             #name => {
    //                 self.#f_name = new_param;
    //                 Ok(())
    //             }
    //         },
    //         quote_spanned! {f.span()=>
    //             crate::types::PortSchema {
    //                 name: #name,
    //                 description: #description,
    //             },
    //         },
    //     )
    // })
}

fn impl_params_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (inserts, updates, schemas) = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let v = map_name_description(fields, "param", |f, f_name, name, description| {
                    (
                        quote_spanned! {f.span()=>
                            state.insert(#name.to_owned(), self.#f_name.to_param());
                        },
                        quote_spanned! {f.span()=>
                            #name => {
                                if self.#f_name != *new_param {
                                    self.#f_name = new_param.clone();
                                }
                                Ok(())
                            }
                        },
                        quote_spanned! {f.span()=>
                            crate::types::ParamSchema {
                                name: #name.to_string(),
                                description: #description.to_string(),
                            },
                        },
                    )
                });
                let insert_iter = v.iter().map(|(insert, _, _)| insert);
                let update_iter = v.iter().map(|(_, update, _)| update);
                let schema_iter = v.iter().map(|(_, _, schema)| schema);
                (
                    quote! {
                        #(#insert_iter)*
                    },
                    quote! {
                        #(#update_iter)*
                    },
                    quote! {
                        #(#schema_iter)*
                    },
                )
            }
            Fields::Unnamed(_) | Fields::Unit => {
                unimplemented!()
            }
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    };

    let generated = quote! {
        impl crate::types::Params for #name {
            fn get_params_state(&self) -> std::collections::HashMap<String, crate::types::Param>{
                let mut state = std::collections::HashMap::new();
                #inserts
                state
            }
            fn update_param(&mut self, param_name: &String, new_param: &crate::types::InternalParam, module_name: &str) -> Result<()> {
                match param_name.as_str() {
                    #updates
                    _ => Err(anyhow!(
                        "{} is not a valid param name for {}",
                        param_name,
                        module_name
                    )),
                }
            }
            fn get_schema() -> Vec<crate::types::ParamSchema> {
                vec![
                    #schemas
                ]
            }
        }
    };
    generated.into()
}

#[proc_macro_derive(Module, attributes(output, module))]
pub fn module_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // // Build the trait implementation
    impl_module_macro(&ast)
}

fn impl_module_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (module_name, module_description) = unwrap_name_description(&ast.attrs, "module");

    let outputs: Vec<_> = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields
                .named
                .iter()
                .filter(|f| unwrap_attr(&f.attrs, "output").is_some())
                .map(|f| {
                    let field_name = f.ident.clone();
                    let output_attr = unwrap_attr(&f.attrs, "output")
                        .map(|tokens| parse_output_attr(tokens))
                        .expect("Failed to parse output attribute");

                    let output_name = &output_attr.name;
                    let output_name_string = output_name.value();
                    let description = output_attr.description.as_ref()
                        .map(|d| quote!(#d.to_string()))
                        .unwrap_or(quote!("".to_string()));
                    let is_default = output_attr.is_default;

                    (
                        field_name.clone().unwrap(),
                        is_default,
                        output_name_string,
                        quote! {
                            outputs.#field_name.copy_from_slice(&module.#field_name);
                        },
                        quote! {
                            #output_name => {
                                let guard = self.outputs.try_read_for(core::time::Duration::from_millis(10)).unwrap();
                                buffer.copy_from_slice(&guard.#field_name);
                                Ok(())
                            },
                        },
                        quote! {
                            crate::types::OutputSchema {
                                name: #output_name.to_string(),
                                description: #description,
                                default: #is_default,
                            },
                        },
                    )
                })
                .collect(),
            Fields::Unnamed(_) | Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    };
    // Validate that at most one output is marked as default
    let default_count = outputs
        .iter()
        .filter(|(_, is_default, _, _, _, _)| *is_default)
        .count();
    if default_count > 1 {
        let error_msg = format!(
            "Module '{}' has {} outputs marked as default, but only one is allowed",
            module_name
                .as_ref()
                .map(|n| n.value())
                .unwrap_or_else(|| "unknown".to_string()),
            default_count
        );
        return syn::Error::new(Span::call_site(), error_msg)
            .to_compile_error()
            .into();
    }

    let output_names = outputs.iter().map(|(idents, _, _, _, _, _)| idents);
    let output_assignments = outputs.iter().map(|(_, _, _, assignment, _, _)| assignment);
    let output_retrievals = outputs.iter().map(|(_, _, _, _, retrieval, _)| retrieval);
    let output_schemas = outputs.iter().map(|(_, _, _, _, _, schema)| schema);
    let struct_name = format_ident!("{}Sampleable", name);
    let output_struct_name = format_ident!("{}Outputs", name);
    let constructor_name = format_ident!("{}Constructor", name)
        .to_string()
        .to_case(Case::Snake);
    let constructor_name = Ident::new(&constructor_name, Span::call_site());
    let params_struct_name = format_ident!("{}Params", name);
    let generated = quote! {

        #[derive(Default, Clone, Copy)]
        struct #output_struct_name {
            #(#output_names: crate::types::ChannelBuffer,)*
        }

        #[derive(Default)]
        struct #struct_name {
            id: String,
            outputs: parking_lot::RwLock<#output_struct_name>,
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
                    #(#output_assignments)*
                }
            }

            fn get_sample(&self, port: &String, buffer: &mut crate::types::ChannelBuffer) -> Result<()> {
                self.update();
                match port.as_str() {
                    #(#output_retrievals)*
                    _ => Err(anyhow!(
                        "{} with id {} does not have port {}",
                        #module_name,
                        &self.id,
                        port
                    ))
                }
            }

            fn get_state(&self) -> crate::types::ModuleState {
                use crate::types::Params;
                crate::types::ModuleState {
                    module_type: #module_name.to_owned(),
                    id: self.id.clone(),
                    params: self.module.lock().params.get_params_state(),
                }
            }

            fn update_param(&self, param_name: &String, new_param: &crate::types::InternalParam) -> Result<()> {
                use crate::types::Params;
                self.module.lock().params.update_param(param_name, new_param, #module_name)
            }

            fn get_id(&self) -> &String {
                &self.id
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
            fn get_schema() -> crate::types::ModuleSchema {
                use crate::types::Params;

                // Validate that parameter names and output names don't overlap
                let param_schemas = #params_struct_name::get_schema();
                let output_schemas = vec![
                    #(#output_schemas)*
                ];

                // Check for name collisions
                for param in &param_schemas {
                    for output in &output_schemas {
                        if param.name == output.name {
                            panic!(
                                "Module '{}' has parameter and output with the same name '{}'. Parameters and outputs must have unique names.",
                                #module_name,
                                param.name
                            );
                        }
                    }
                }

                crate::types::ModuleSchema {
                    name: #module_name.to_string(),
                    description: #module_description.to_string(),
                    params: param_schemas,
                    outputs: output_schemas,
                }
            }
        }
    };
    generated.into()
}
