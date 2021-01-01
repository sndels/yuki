use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, Field, Ident};

use crate::common::{
    add_trait_bound, combined_error, impl_vec_op_tokens, parse_generics, per_component_tokens,
    TraitInfo,
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

pub fn neg(input: DeriveInput, _: &str) -> TokenStream {
    let generics = add_trait_bound(&input.generics, quote!(num::traits::Signed));

    let (_, impl_generics, type_generics, where_clause) = match parse_generics(&generics) {
        Ok((g, i, t, w)) => (g, i, t, w),
        Err(errors) => {
            return combined_error("Derive 'Neg'", input.ident.span(), errors).to_compile_error();
        }
    };
    let type_ident = &input.ident;

    let negated_components = per_component_tokens(
        &input.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned! {f.span() => #c: -self.#c },
        &|recurse| quote!(#(#recurse,)*),
    );

    quote! {
        impl #impl_generics Neg for #type_ident #type_generics
        #where_clause
        {
            type Output = Self;

            fn neg(self) -> Self {
                debug_assert!(!self.has_nans());

                Self {
                    #negated_components
                }
            }
        }
    }
}
