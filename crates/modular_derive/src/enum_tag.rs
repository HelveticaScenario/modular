use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Fields, LitStr};

fn parse_enum_tag_name(attrs: &[Attribute], default_ident: Ident) -> syn::Result<Ident> {
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

pub fn impl_enum_tag_macro(ast: &DeriveInput) -> TokenStream {
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
