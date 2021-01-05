use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::common::{combined_error, parse_generics};
use crate::impl_vec_like::{vec_like_impl, vec_normal_members_impl};

pub fn normal_impl(item: &DeriveInput) -> TokenStream {
    let vec_type = &item.ident;

    let (generic_param, impl_generics, type_generics, where_clause) =
        match parse_generics(&item.generics) {
            Ok((g, i, t, w)) => (g, i, t, w),
            Err(errors) => {
                return combined_error("Impl Normal", item.ident.span(), errors).to_compile_error();
            }
        };

    let member_ops = vec_normal_members_impl(&item.data, &generic_param);

    vec_like_impl(
        &item.data,
        vec_type,
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
        Some(member_ops),
        None,
    )
}
