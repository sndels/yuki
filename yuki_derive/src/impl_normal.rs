use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::{
    common::{combined_error, parse_generics},
    impl_vec_like::{vec_like_impl, vec_normal_members_impl},
};

pub fn normal_impl(item: &DeriveInput) -> TokenStream {
    let vec_type = &item.ident;

    let parsed_generics = match parse_generics(&item.generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error("Impl Normal", item.ident.span(), errors).to_compile_error();
        }
    };

    let member_ops = vec_normal_members_impl(&item.data, vec_type, &parsed_generics.generic_param);

    vec_like_impl(
        &item.data,
        vec_type,
        parsed_generics,
        Some(member_ops),
        None,
    )
}
