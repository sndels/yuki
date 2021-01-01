use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident};

use crate::vec_op_common::{
    add_trait_bound, combined_error, impl_vec_op_tokens, parse_generics, TraitInfo,
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

    let generics = add_trait_bound(&item.generics, &trait_ident);

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

    let default_epsilon_tokens = type_op_init_tokens(
        &item.data,
        value_type,
        &Ident::new("default_epsilon", Span::call_site()),
    );

    let abs_diff_eq_tokens = abs_diff_eq_and_tokens(&item.data);

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

fn type_op_init_tokens(data: &Data, value_type: &Ident, op: &Ident) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        // Use correct field span to get potential error on correct line
                        quote_spanned! { f.span() =>
                            #name: #value_type::#op()
                        }
                    });
                    quote! {
                        #(#recurse),*
                    }
                }
                _ => unimplemented!(),
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}

fn abs_diff_eq_and_tokens(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        // Use correct field span to get potential error on correct line
                        quote_spanned! { f.span() =>
                            self.#name.abs_diff_eq(&other.#name, epsilon.#name)
                        }
                    });
                    quote! {
                        #(#recurse)&&*
                    }
                }
                _ => unimplemented!(),
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}

pub fn relative_eq(item: &DeriveInput, value_type: &Ident) -> TokenStream {
    let vec_type = &item.ident;

    let default_max_relative_tokens = type_op_init_tokens(
        &item.data,
        value_type,
        &Ident::new("default_max_relative", Span::call_site()),
    );

    let relative_eq_tokens = relative_eq_and_tokens(&item.data);

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

fn relative_eq_and_tokens(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        // Use correct field span to get potential error on correct line
                        quote_spanned! { f.span() =>
                            self.#name.relative_eq(&other.#name, epsilon.#name, max_relative.#name)
                        }
                    });
                    quote! {
                        #(#recurse)&&*
                    }
                }
                _ => unimplemented!(),
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
