extern crate quote;
extern crate syn;

extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse::Parser, punctuated::Punctuated, spanned::Spanned, Attribute, Field, FieldsNamed, LitStr,
    Token,
};
use syn::{Data, DeriveInput, Fields};

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
        .filter(|attr| attr.path.is_ident(ident))
        .next()
        .map(|attr| {
            attr.tokens
                .clone()
                .into_iter()
                .map(|token| match token {
                    proc_macro2::TokenTree::Group(group) => group.stream(),
                    proc_macro2::TokenTree::Ident(_) => {
                        unimplemented!()
                    }
                    proc_macro2::TokenTree::Punct(_) => {
                        unimplemented!()
                    }
                    proc_macro2::TokenTree::Literal(_) => {
                        unimplemented!()
                    }
                })
                .next()
                .unwrap()
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
                .filter(|attr| attr.path.is_ident(ident))
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
    let (inserts, updates, schemas, regenerators) = match ast.data {
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
                            crate::types::PortSchema {
                                name: #name,
                                description: #description,
                            },
                        },
                        quote_spanned! {f.span() =>
                            if let crate::types::InternalParam::Cable {ref mut module, port: _} = self.#f_name {
                                if let Some(m) = sampleable_map.get(module.get_id()) {
                                    module.update_reference(m);
                                } else {
                                    self.#f_name = crate::types::InternalParam::Disconnected;
                                }
                            };
                        },
                    )
                });
                let insert_iter = v.iter().map(|(insert, _, _, _)| insert);
                let update_iter = v.iter().map(|(_, update, _, _)| update);
                let schema_iter = v.iter().map(|(_, _, schema, _)| schema);
                let regenerator_iter = v.iter().map(|(_, _, _, regenerator)| regenerator);
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
                    quote! {
                        #(#regenerator_iter)*
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
            fn update_param(&mut self, param_name: &str, new_param: &crate::types::InternalParam, module_name: &str) -> Result<()> {
                match param_name {
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
            fn regenerate_cables(&mut self, sampleable_map: &crate::types::SampleableMap) {
                use crate::types::HasId;
                #regenerators
            }
        }
    };
    gen.into()
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
                    let name = f.ident.clone();
                    let output = unwrap_attr(&f.attrs, "output")
                        .map(|tokens| {
                            Punctuated::<LitStr, Token![,]>::parse_terminated
                                .parse2(tokens)
                                .unwrap()
                        })
                        .unwrap_or_default();
                    let mut output_iter = output.iter();
                    let output_name = output_iter.next();
                    let description = output_iter.next();
                    (
                        name.clone().unwrap(),
                        quote! {
                            outputs.#name = module.#name;
                        },
                        quote! {
                            #output_name => Ok(self.outputs.try_read_for(core::time::Duration::from_millis(10)).unwrap().#name),
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
    let output_names = outputs.iter().map(|(idents, _, _, _)| idents);
    let output_assignments = outputs.iter().map(|(_, assignment, _, _)| assignment);
    let output_retrievals = outputs.iter().map(|(_, _, retrieval, _)| retrieval);
    let output_schemas = outputs.iter().map(|(_, _, _, schema)| schema);
    let struct_name = format_ident!("{}Sampleable", name);
    let output_struct_name = format_ident!("{}Outputs", name);
    let constructor_name = format_ident!("{}Constructor", name)
        .to_string()
        .to_case(Case::Snake);
    let constructor_name = Ident::new(&constructor_name, Span::call_site());
    let params_struct_name = format_ident!("{}Params", name);
    let gen = quote! {

        #[derive(Default)]
        struct #output_struct_name {
            #(#output_names: f32,)*
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

            fn get_sample(&self, port: &str) -> Result<f32> {
                self.update();
                match port {
                    #(#output_retrievals)*
                    _ => Err(anyhow!(
                        "{} with id {} does not have port {}",
                        #module_name,
                        self.id,
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

            fn update_param(&self, param_name: &str, new_param: &crate::types::InternalParam) -> Result<()> {
                use crate::types::Params;
                self.module.lock().params.update_param(param_name, new_param, #module_name)
            }

            fn regenerate_cables(&self, sampleable_map: &crate::types::SampleableMap) {
                use crate::types::Params;
                self.module.lock().params.regenerate_cables(sampleable_map);
            }
        }

        impl crate::types::HasId for #struct_name {
            fn get_id<'a>(&'a self) -> &'a str {
                &self.id
            }
        }

        fn #constructor_name(id: &str, sample_rate: f32) -> Result<std::sync::Arc<Box<dyn crate::types::Sampleable>>> {
            Ok(std::sync::Arc::new(Box::new(#struct_name {
                id: id.into(),
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
