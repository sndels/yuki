use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, Field, Ident};

use crate::common::{
    add_trait_bound, combined_error, impl_vec_op_tokens, parse_generics, per_component_tokens,
    TraitInfo,
};

pub fn vec_op(
    op_trait: Ident,
    other: Ident,
    output: Option<&Ident>,
    item: &DeriveInput,
) -> TokenStream {
    let TraitInfo {
        ident: trait_ident,
        trait_fn_ident,
        op_ident,
        is_scalar_op,
        is_assign_op,
    } = TraitInfo::new(&op_trait.to_string());

    if is_scalar_op {
        return syn::Error::new(op_trait.span(), "Scalar ops not supported").to_compile_error();
    }
    assert!(is_assign_op == output.is_none());

    let generics = add_trait_bound(&item.generics, quote!(#trait_ident));

    let (_, impl_generics, type_generics, where_clause) = match parse_generics(&generics) {
        Ok((g, i, t, w)) => (g, i, t, w),
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
        trait_fn_ident,
        op_ident,
        &type_ident,
        other_tokens,
        output,
        impl_generics,
        type_generics,
        where_clause,
        false,
    )
}

pub fn abs_diff_eq(item: &DeriveInput, value_type: &Ident) -> TokenStream {
    let vec_type = &item.ident;

    let default_epsilon_tokens = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #value_type::default_epsilon()),
        &|recurse| quote!(#(#recurse),*),
    );

    let abs_diff_eq_tokens = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| {
            quote_spanned! { f.span() =>
                self.#c.abs_diff_eq(&other.#c, epsilon.#c)
            }
        },
        &|recurse| quote!(#(#recurse)&&*),
    );

    quote! {
        impl approx::AbsDiffEq for #vec_type<#value_type>
        {
            type Epsilon = Self;

            fn default_epsilon() -> Self::Epsilon {
                #vec_type {
                    #default_epsilon_tokens
                }
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
                #abs_diff_eq_tokens
            }
        }
    }
}

pub fn relative_eq(item: &DeriveInput, value_type: &Ident) -> TokenStream {
    let vec_type = &item.ident;

    let default_max_relative_tokens = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #value_type::default_max_relative()),
        &|recurse| quote!(#(#recurse),*),
    );

    let relative_eq_tokens = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| {
            quote_spanned! { f.span() =>
                self.#c.relative_eq(&other.#c, epsilon.#c, max_relative.#c)
            }
        },
        &|recurse| quote!(#(#recurse)&&*),
    );

    quote! {
        impl approx::RelativeEq for #vec_type<#value_type>
        {
            fn default_max_relative() -> Self::Epsilon {
                #vec_type {
                    #default_max_relative_tokens
                }
            }

            fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
                #relative_eq_tokens
            }
        }
    }
}
