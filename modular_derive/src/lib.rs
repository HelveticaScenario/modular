extern crate quote;
extern crate syn;

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, quote_spanned};
use syn::{AttributeArgs, Lit, LitStr, parenthesized, spanned::Spanned};
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Generics, Index,
};

#[proc_macro_derive(Params, attributes(name, description))]
pub fn params_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // // Build the trait implementation
    impl_params_macro(&ast)
}

fn impl_params_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (inserts, updates, schemas) = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let v = fields
                    .named
                    .iter()
                    .filter(|f| {
                        f.attrs
                            .iter()
                            .filter(|attr| attr.path.is_ident("description"))
                            .count()
                            > 0
                    })
                    .map(|f| {
                        let f_name = &f.ident;
                        let description = f
                            .attrs
                            .iter()
                            .filter(|attr| attr.path.is_ident("description"))
                            .map(|attr| attr.tokens.clone())
                            .next()
                            .unwrap_or(quote! {"TODO"})
                            .clone();
                        // let abc = f
                        //     .attrs
                        //     .iter()
                        //     .filter(|attr| attr.path.is_ident("name"))
                        //     .map(|attr| {
                        //         let args = attr.tokens.into();
                        //         parse_macro_input!(args as AttributeArgs)
                        //             .iter()
                        //             .map(|arg| match arg {
                        //                 syn::NestedMeta::Meta(_) => {
                        //                     unimplemented!()
                        //                 }
                        //                 syn::NestedMeta::Lit(l) => TokenStream::from(l.clone()),
                        //             })
                        //             .next()
                        //             .unwrap()
                        //     })
                        //     .next()
                        //     .unwrap()
                        //     .clone();
                        let name = f
                            .attrs
                            .iter()
                            .filter(|attr| attr.path.is_ident("name"))
                            .map(|attr| attr.tokens.clone())
                            .next()
                            .unwrap()
                            .clone();
                        (
                            quote_spanned! {f.span()=>
                                state.insert(#name.to_owned(), self.#f_name.to_param());
                            },
                            quote_spanned! {f.span()=>
                                #name => {
                                    self.#f_name = new_param;
                                    Ok(())
                                }
                            },
                            quote_spanned! {f.span()=>
                                crate::types::PortSchema {
                                    name: #name,
                                    description: #description,
                                },
                            },
                        )
                    })
                    .collect::<Vec<_>>();
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

    let gen = quote! {
        impl crate::types::Params for #name {
            fn get_params_state(&self) -> std::collections::HashMap<String, crate::types::Param>{
                let mut state = std::collections::HashMap::new();
                #inserts
                state
            }
            fn update_param(&mut self, param_name: &String, new_param: crate::types::InternalParam, module_name: &str) -> Result<()> {
                match param_name.as_str() {
                    #updates
                    _ => Err(anyhow!(
                        "{} is not a valid param name for {}",
                        param_name,
                        module_name
                    )),
                }
            }
            fn get_schema() -> &'static [crate::types::PortSchema] {
                &[
                    #schemas
                ]
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(Module, attributes(output, name, description))]
pub fn module_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    // // Build the trait implementation
    impl_module_macro(&ast)
}

fn impl_module_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let module_name = &ast
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident("name"))
        .map(|attr| attr.tokens.clone())
        .next()
        .unwrap_or(quote! {#name})
        .clone();

    let module_description = &ast
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident("description"))
        .map(|attr| {
            let tokens = &attr.tokens.clone();
            quote! {#tokens}
        })
        .collect::<Vec<_>>()
        .get(0)
        .unwrap_or(&quote! {"TODO"})
        .clone();

    let outputs: Vec<_> = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields
                .named
                .iter()
                .filter(|f| {
                    f.attrs
                        .iter()
                        .filter(|attr| attr.path.is_ident("output"))
                        .count()
                        > 0
                })
                .map(|f| {
                    let name = f.ident.clone();
                    let output_name = f
                        .attrs
                        .iter()
                        .filter(|attr| attr.path.is_ident("output"))
                        .next()
                        .map(|attr| attr.tokens.clone())
                        .unwrap()
                        .clone();
                    let description = f
                        .attrs
                        .iter()
                        .filter(|attr| attr.path.is_ident("description"))
                        .next()
                        .map(|attr| attr.tokens.clone())
                        .unwrap_or(quote! {"TODO"})
                        .clone();
                    (
                        name.clone().unwrap(),
                        quote! {
                            *self.#name.try_lock().unwrap() = self.module.try_lock().unwrap().#name;
                        },
                        quote! {
                            if port == #output_name {
                                return Ok(*self.#name.try_lock().unwrap());
                            }
                        },
                        quote! {
                            crate::types::PortSchema {
                                name: #output_name,
                                description: #description,
                            },
                        },
                    )
                })
                .collect(),
            Fields::Unnamed(_) | Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    };
    let output_names1 = outputs.iter().map(|(idents, _, _, _)| idents);
    let output_names2 = output_names1.clone();
    let output_names3 = output_names2.clone();
    let output_assignments = outputs.iter().map(|(_, assignment, _, _)| assignment);
    let output_retrievals = outputs.iter().map(|(_, _, retrieval, _)| retrieval);
    let output_schemas = outputs.iter().map(|(_, _, _, schema)| schema);
    let struct_name = format_ident!("{}Sampleable", name);
    let constuctor_name = format_ident!("{}_constructor", name);
    let params_struct_name = format_ident!("{}Params", name);
    let gen = quote! {
        #[derive(Default)]
        struct #struct_name {
            id: uuid::Uuid,
            #(#output_names1: std::sync::Mutex<f32>,)*
            module: std::sync::Mutex<#name>,
        }

        impl crate::types::Sampleable for #struct_name {
            fn tick(&self) -> () {
                #(#output_assignments)*
            }

            fn update(&self, sample_rate: f32) -> () {
                self.module.try_lock().unwrap().update(sample_rate);
            }

            fn get_sample(&self, port: &String) -> Result<f32> {
                #(#output_retrievals)*
                Err(anyhow!(
                    "{} with id {} does not have port {}",
                    #module_name,
                    self.id,
                    port
                ))
            }

            fn get_state(&self) -> crate::types::ModuleState {
                crate::types::ModuleState {
                    module_type: #module_name.to_owned(),
                    id: self.id.clone(),
                    params: self.module.lock().unwrap().params.get_params_state(),
                }
            }

            fn update_param(&self, param_name: &String, new_param: crate::types::InternalParam) -> Result<()> {
                self.module.lock().unwrap().params.update_param(param_name, new_param, #module_name)
            }

            fn get_id(&self) -> uuid::Uuid {
                self.id.clone()
            }
        }

        fn #constuctor_name(id: &uuid::Uuid) -> Result<std::sync::Arc<Box<dyn crate::types::Sampleable>>> {
            Ok(std::sync::Arc::new(Box::new(#struct_name {
                id: id.clone(),
                ..#struct_name::default()
            })))
        }

        impl crate::types::Module for #name {
            fn install_constructor(map: &mut std::collections::HashMap<String, crate::types::SampleableConstructor>) {
                map.insert(#module_name.into(), Box::new(#constuctor_name));
            }
            fn get_schema() -> crate::types::ModuleSchema {
                crate::types::ModuleSchema {
                    name: #module_name,
                    description: #module_description,
                    params: #params_struct_name::get_schema(),
                    outputs: &[
                        #(#output_schemas)*
                    ],
                }
            }
        }
    };
    gen.into()
}
