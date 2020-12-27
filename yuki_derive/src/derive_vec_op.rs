use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, GenericParam, Generics, Ident, TypeGenerics, WhereClause};

use crate::vec_op_common::{impl_vec_assign_op_tokens, impl_vec_op_tokens};

pub fn vec_op(input: DeriveInput, trait_name: &str) -> TokenStream {
    // *Scalar is our own indicator
    // Check if its present and get the real trait to implement
    let (trait_name, is_scalar_op) = if trait_name.ends_with("Scalar") {
        (trait_name.trim_end_matches("Scalar"), true)
    } else {
        (trait_name, false)
    };
    let trait_ident = Ident::new(&trait_name, Span::call_site());

    // The underlying component op is different from trait op for assign ops
    let (trait_fn_ident, op_ident, is_assign_op) = if trait_name.ends_with("Assign") {
        let component_op = trait_name
            .trim_end_matches("Assign")
            .to_lowercase()
            .to_string();
        (
            Ident::new(&(component_op.clone() + "_assign"), Span::call_site()),
            Ident::new(&component_op, Span::call_site()),
            true,
        )
    } else {
        (
            Ident::new(&trait_name.to_lowercase(), Span::call_site()),
            Ident::new(&trait_name.to_lowercase(), Span::call_site()),
            false,
        )
    };

    let (generic_param, type_generics, where_clause) = match parse_generics(&input.generics) {
        Ok((g, t, w)) => (g, t, w),
        Err(errors) => {
            return errors
                .iter()
                .map(|&(err, span)| {
                    syn::Error::new(
                        if let Some(span) = span {
                            span
                        } else {
                            input.ident.span()
                        },
                        format!("Derive '{}': {}", trait_name, err),
                    )
                })
                .fold_first(|mut acc, err| {
                    acc.combine(err);
                    acc
                })
                .unwrap()
                .to_compile_error();
        }
    };

    // All derive ops operate on the given type
    let type_ident = input.ident;
    // Scalar ops default use other: T
    let other_ident = if is_scalar_op {
        quote! {#generic_param}
    } else {
        quote! {#type_ident #type_generics}
    };

    if is_assign_op {
        impl_vec_assign_op_tokens(
            trait_ident,
            trait_fn_ident,
            op_ident,
            &type_ident,
            other_ident,
            type_generics,
            where_clause,
        )
    } else {
        let opped_components = opped_components_tokens(input.data, &op_ident, !is_scalar_op);

        impl_vec_op_tokens(
            trait_ident,
            trait_fn_ident,
            &type_ident,
            other_ident,
            &type_ident,
            opped_components,
            type_generics,
            where_clause,
        )
    }
}

fn parse_generics<'a>(
    generics: &'a Generics,
) -> Result<(Ident, TypeGenerics, Option<&'a WhereClause>), Vec<(&str, Option<Span>)>> {
    // We expect a struct with the form
    // struct Type<T>
    // where
    //      T: Bounds
    let mut generic_param = None;
    let mut errors = vec![];
    for g in generics.params.iter() {
        match g {
            GenericParam::Type(t) => {
                if !t.bounds.is_empty() {
                    errors.push(("Bounds should be in a where clause", Some(t.bounds.span())));
                }
                if generic_param.is_some() {
                    errors.push(("Only one generic type param supported", Some(t.span())));
                }
                generic_param = Some(t.ident.clone());
            }
            GenericParam::Lifetime(l) => {
                errors.push(("Lifetimes not supported", Some(l.span())));
            }
            GenericParam::Const(c) => {
                errors.push(("Consts not supported", Some(c.span())));
            }
        }
    }
    if generic_param.is_none() {
        errors.push(("A single generic param expected", None));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let generic_param = generic_param.unwrap();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    // With our strict and previously validated form, this should always be true
    assert!(
        impl_generics.to_token_stream().to_string() == type_generics.to_token_stream().to_string()
    );

    Ok((generic_param, type_generics, where_clause))
}

fn opped_components_tokens(
    data: Data,
    op_ident: &Ident,
    other_has_components: bool,
) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Use correct field span to get potential error on correct line
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        if other_has_components {
                            quote_spanned! {f.span() =>
                                self.#name.#op_ident(other.#name)
                            }
                        } else {
                            quote_spanned! {f.span() =>
                                self.#name.#op_ident(other)
                            }
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
