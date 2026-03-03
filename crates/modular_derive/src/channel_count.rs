use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields};

use crate::utils::is_poly_signal_type;

pub fn impl_channel_count_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let poly_signal_field_refs: Vec<TokenStream2> = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|field| {
                    let field_ident = field.ident.as_ref()?;
                    if is_poly_signal_type(&field.ty) {
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
