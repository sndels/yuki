use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields};

use crate::vec_op_common::{combined_error, parse_generics, TraitInfo};

pub fn index(input: DeriveInput, name: &str) -> TokenStream {
    let TraitInfo {
        ident: trait_ident,
        trait_fn_ident,
        ..
    } = TraitInfo::new(name);

    let (generic_param, impl_generics, type_generics, where_clause) =
        match parse_generics(&input.generics) {
            Ok((g, i, t, w)) => (g, i, t, w),
            Err(errors) => {
                return combined_error("Derive'Index'", input.ident.span(), errors)
                    .to_compile_error();
            }
        };
    let type_ident = &input.ident;

    let is_mutable_index = name.ends_with("Mut");

    let self_ref_tokens = if is_mutable_index {
        quote! {&mut self}
    } else {
        quote! {&self}
    };

    let match_tokens = index_match_tokens(&input.data, &self_ref_tokens);

    // IndexMut uses the output type defined by Index
    let trait_output_tokens = if is_mutable_index {
        None
    } else {
        Some(quote! {
            type Output = #generic_param;
        })
    };

    let fn_output_tokens = if is_mutable_index {
        quote! {
            &mut Self::Output
        }
    } else {
        quote! {
            &Self::Output
        }
    };

    // We only support impl_generics == type_generics
    quote! {
        impl #impl_generics #trait_ident <usize> for #type_ident #type_generics
        #where_clause
        {
            #trait_output_tokens

            fn #trait_fn_ident(#self_ref_tokens, component: usize) -> #fn_output_tokens {
                match component {
                    #match_tokens
                    _ => {
                        panic!("Component {} is out of bounds", component);
                    }
                }
            }
        }
    }
}

fn index_match_tokens(data: &Data, self_ref_tokens: &TokenStream) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().enumerate().map(|(i, f)| {
                        let name = &f.ident;
                        // Use correct field span to get potential error on correct line
                        quote_spanned! {f.span() =>
                            #i => #self_ref_tokens.#name
                        }
                    });
                    quote! {
                        #(#recurse,)*
                    }
                }
                _ => unimplemented!(),
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
