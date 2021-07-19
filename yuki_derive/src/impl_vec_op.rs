use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

use crate::common::{
    add_trait_bound, combined_error, impl_vec_op_tokens, parse_generics, ParsedGenerics, TraitInfo,
};

pub fn vec_op(
    op_trait: Ident,
    other: Ident,
    output: Option<&Ident>,
    item: &DeriveInput,
) -> TokenStream {
    let TraitInfo {
        ident: trait_ident,
        op_ident,
        is_scalar_op,
        is_assign_op,
    } = TraitInfo::new(&op_trait.to_string());

    if is_scalar_op {
        return syn::Error::new(op_trait.span(), "Scalar ops not supported").to_compile_error();
    }
    assert!(is_assign_op == output.is_none());

    let generics = add_trait_bound(&item.generics, quote!(#trait_ident));

    let ParsedGenerics {
        impl_generics,
        type_generics,
        where_clause,
        ..
    } = match parse_generics(&generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error(&format!("vec_op '{}'", op_trait), item.ident.span(), errors)
                .to_compile_error();
        }
    };
    let type_ident = &item.ident;

    // We always have other since the ops vec-vec
    let other_tokens = quote! {#other #type_generics};

    impl_vec_op_tokens(
        &item.data,
        trait_ident,
        op_ident,
        type_ident,
        other_tokens,
        output,
        impl_generics,
        type_generics,
        where_clause,
        false,
    )
}
