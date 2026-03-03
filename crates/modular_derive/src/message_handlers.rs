use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Token, punctuated::Punctuated};

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

pub fn message_handlers_impl(input: TokenStream) -> TokenStream {
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
