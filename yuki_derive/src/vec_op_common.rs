use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, GenericParam, Generics, Ident, TypeGenerics, WhereClause};

pub struct TraitInfo {
    pub ident: Ident,
    pub trait_fn_ident: Ident,
    pub op_ident: Ident,
    pub is_scalar_op: bool,
    pub is_assign_op: bool,
}

impl TraitInfo {
    pub fn new(full_name: &str) -> Self {
        // *Scalar is our own indicator
        // Check if its present and get the real trait to implement
        let (trait_name, is_scalar_op) = if full_name.ends_with("Scalar") {
            (full_name.trim_end_matches("Scalar"), true)
        } else {
            (full_name, false)
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

        Self {
            ident: trait_ident,
            trait_fn_ident,
            op_ident,
            is_scalar_op,
            is_assign_op,
        }
    }
}

pub struct TypeInfo<'a> {
    pub type_ident: &'a Ident,
    pub generic_param: Ident,
    pub type_generics: TypeGenerics<'a>,
    pub where_clause: Option<&'a WhereClause>,
}

impl<'a> TypeInfo<'a> {
    pub fn new(input: &'a DeriveInput) -> Result<Self, Vec<(&str, Option<Span>)>> {
        let (generic_param, type_generics, where_clause) = match parse_generics(&input.generics) {
            Ok((g, t, w)) => (g, t, w),
            Err(errors) => {
                return Err(errors);
            }
        };
        Ok(Self {
            type_ident: &input.ident,
            generic_param,
            type_generics,
            where_clause,
        })
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

pub fn combined_error(
    prefix: &str,
    default_span: Span,
    errors: Vec<(&str, Option<Span>)>,
) -> syn::Error {
    return errors
        .iter()
        .map(|&(err, span)| {
            syn::Error::new(span.unwrap_or(default_span), format!("{}: {}", prefix, err))
        })
        .fold_first(|mut acc, err| {
            acc.combine(err);
            acc
        })
        .unwrap();
}

pub fn opped_components_tokens(
    data: &Data,
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

pub fn impl_vec_op_tokens(
    item_data: &Data,
    trait_ident: Ident,
    trait_fn_ident: Ident,
    op_ident: Ident,
    type_ident: &Ident,
    other_tokens: TokenStream,
    output_ident: Option<&Ident>,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
    is_scalar_op: bool,
) -> TokenStream {
    if output_ident.is_some() {
        let opped_components = opped_components_tokens(item_data, &trait_fn_ident, !is_scalar_op);

        // We only support impl_generics == type_generics
        quote! {
            impl #type_generics #trait_ident<#other_tokens> for #type_ident #type_generics
            #where_clause
            {
                type Output = #output_ident #type_generics;

                fn #trait_fn_ident(self, other: #other_tokens) -> Self::Output {
                    #output_ident::new( #opped_components )
                }
            }
        }
    } else {
        // We only support impl_generics == type_generics
        quote! {
            impl #type_generics #trait_ident<#other_tokens> for #type_ident #type_generics
            #where_clause
            {
                fn #trait_fn_ident(&mut self, other: #other_tokens) {
                    debug_assert!(!self.has_nans());
                    *self = self.#op_ident(other);
                    debug_assert!(!self.has_nans());
                }
            }
        }
    }
}
