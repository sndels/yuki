use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput, Field, Ident};

use crate::{
    common::{combined_error, parse_generics, per_component_tokens},
    impl_vec_like::vec_like_impl,
};

pub fn spectrum_impl(item: &DeriveInput) -> TokenStream {
    let vec_type = &item.ident;

    let parsed_generics = match parse_generics(&item.generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error("Impl Spectrum", item.ident.span(), errors).to_compile_error();
        }
    };

    let member_ops = {
        let data = &item.data;
        let generic_param = &parsed_generics.generic_param;

        let is_black_ret = per_component_tokens(
            data,
            &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c == #generic_param::zero()),
            &|recurse| quote!(#(#recurse)&&*),
        );

        let str_type = vec_type.to_string();
        let is_black_doc = format!("Returns `true` if this `{0}` is black.", str_type);

        quote! {
            #[doc = #is_black_doc]
            #[inline]
            pub fn is_black(&self) -> bool {
                debug_assert!(!self.has_nans());

                #is_black_ret
            }
        }
    };

    vec_like_impl(
        &item.data,
        vec_type,
        parsed_generics,
        Some(member_ops),
        None,
    )
}
