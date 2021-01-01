use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_quote, Data, Fields, GenericParam, Generics, Ident, ImplGenerics, TypeGenerics,
    WhereClause,
};

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

        // This could be much cleaner but hey, it works
        let snake_case_op: String = trait_name.chars().fold(String::new(), |mut acc, c| {
            if c.is_uppercase() {
                acc.push('_');
            }
            acc.push_str(&c.to_lowercase().collect::<String>());
            acc
        })[1..]
            .into();

        // The underlying component op is different from trait op for assign ops
        let (trait_fn_ident, op_ident, is_assign_op) = if snake_case_op.ends_with("_assign") {
            let component_op = snake_case_op.trim_end_matches("_assign");
            (
                Ident::new(&snake_case_op, Span::call_site()),
                Ident::new(&component_op, Span::call_site()),
                true,
            )
        } else if trait_name.ends_with("_mut") {
            let component_op = snake_case_op.trim_end_matches("_mut");
            (
                Ident::new(&snake_case_op, Span::call_site()),
                Ident::new(&component_op, Span::call_site()),
                false,
            )
        } else {
            (
                Ident::new(&snake_case_op, Span::call_site()),
                Ident::new(&snake_case_op, Span::call_site()),
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

pub fn add_trait_bound(generics: &Generics, trait_ident: &Ident) -> Generics {
    let mut ret = generics.clone();
    for param in &mut ret.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote! {
                #trait_ident
            });
        }
    }
    ret.make_where_clause();
    ret
}

pub fn parse_generics<'a>(
    generics: &'a Generics,
) -> Result<(Ident, ImplGenerics, TypeGenerics, Option<&'a WhereClause>), Vec<(&str, Option<Span>)>>
{
    // We expect a struct with the form
    // struct Type<T>
    // where
    //      T: Bounds
    let mut generic_param = None;
    let mut errors = vec![];
    for g in generics.params.iter() {
        match g {
            GenericParam::Type(t) => {
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

    Ok((generic_param, impl_generics, type_generics, where_clause))
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
    impl_generics: ImplGenerics,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
    is_scalar_op: bool,
) -> TokenStream {
    if output_ident.is_some() {
        let opped_components = opped_components_tokens(item_data, &trait_fn_ident, !is_scalar_op);

        quote! {
            impl #impl_generics #trait_ident<#other_tokens> for #type_ident #type_generics
            #where_clause
            {
                type Output = #output_ident #type_generics;

                fn #trait_fn_ident(self, other: #other_tokens) -> Self::Output {
                    #output_ident::new( #opped_components )
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #trait_ident<#other_tokens> for #type_ident #type_generics
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
