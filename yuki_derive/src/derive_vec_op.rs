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
        Err((err, span)) => {
            return syn::Error::new(span, format!("Derive '{}': {}", trait_name, err))
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
) -> Result<(Ident, TypeGenerics, Option<&'a WhereClause>), (&str, Span)> {
    // We expect a struct with the form
    // struct Type<T>
    // where
    //      T: Bounds
    let mut generic_param = None;
    for g in generics.params.iter() {
        match g {
            GenericParam::Type(t) => {
                if !t.bounds.is_empty() {
                    return Err(("Bounds should be in a where clause", t.bounds.span()));
                }
                if generic_param.is_some() {
                    return Err(("Only one generic type param supported", t.span()));
                }
                generic_param = Some(t.ident.clone());
            }
            GenericParam::Lifetime(l) => {
                return Err(("No lifetimes supported", l.span()));
            }
            GenericParam::Const(c) => {
                return Err(("No const supported", c.span()));
            }
        }
    }
    if generic_param.is_none() {
        return Err((
            "No generic type param. The derive expects a type of form Type<T>.",
            generics.span(),
        ));
    }
    let generic_param = generic_param.unwrap();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    // These should be the same unless we change them ourselves
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
