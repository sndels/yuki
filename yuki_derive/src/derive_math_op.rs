use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::common::{
    add_trait_bound, combined_error, impl_vec_op_tokens, parse_generics, TraitInfo,
};

pub fn vec_op(input: DeriveInput, full_name: &str) -> TokenStream {
    let TraitInfo {
        ident: trait_ident,
        trait_fn_ident,
        op_ident,
        is_scalar_op,
        is_assign_op,
    } = TraitInfo::new(full_name);

    let generics = add_trait_bound(&input.generics, quote!(#trait_ident));

    let (generic_param, impl_generics, type_generics, where_clause) =
        match parse_generics(&generics) {
            Ok((g, i, t, w)) => (g, i, t, w),
            Err(errors) => {
                return combined_error(
                    &format!("Derive '{}'", full_name),
                    input.ident.span(),
                    errors,
                )
                .to_compile_error();
            }
        };
    let type_ident = &input.ident;

    // Scalar ops default use other: T
    let other_tokens = if is_scalar_op {
        quote! {#generic_param}
    } else {
        quote! {#type_ident #type_generics}
    };

    // Assign ops have no output
    let output = if is_assign_op { None } else { Some(type_ident) };

    impl_vec_op_tokens(
        &input.data,
        trait_ident,
        trait_fn_ident,
        op_ident,
        &type_ident,
        other_tokens,
        output,
        impl_generics,
        type_generics,
        where_clause,
        is_scalar_op,
    )
}
