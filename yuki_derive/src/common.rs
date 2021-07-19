use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::vec::IntoIter;
use syn::{
    parse_quote, spanned::Spanned, Data, DeriveInput, Field, Fields, GenericParam, Generics, Ident,
    ImplGenerics, TypeGenerics, WhereClause,
};

pub struct TraitInfo {
    pub ident: Ident,
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

        let is_assign_op = trait_name.ends_with("Assign");

        let trait_ident = Ident::new(trait_name, Span::call_site());

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
        let op_ident = Ident::new(&snake_case_op, Span::call_site());

        Self {
            ident: trait_ident,
            op_ident,
            is_scalar_op,
            is_assign_op,
        }
    }
}

pub fn add_trait_bound(generics: &Generics, trait_tokens: TokenStream) -> Generics {
    let mut ret = generics.clone();
    for param in &mut ret.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote! {
                #trait_tokens
            });
        }
    }
    ret.make_where_clause();
    ret
}

pub struct ParsedGenerics<'a> {
    pub generic_param: Ident,
    pub impl_generics: ImplGenerics<'a>,
    pub type_generics: TypeGenerics<'a>,
    pub where_clause: Option<&'a WhereClause>,
}

pub fn parse_generics(generics: &Generics) -> Result<ParsedGenerics, Vec<(&str, Option<Span>)>> {
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

    Ok(ParsedGenerics {
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
    })
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
        .reduce(|mut acc, err| {
            acc.combine(err);
            acc
        })
        .unwrap();
}

pub fn per_component_tokens(
    data: &Data,
    component_tokens: &dyn Fn(&Option<Ident>, &Field) -> TokenStream,
    meta_tokens: &dyn Fn(IntoIter<TokenStream>) -> TokenStream,
) -> TokenStream {
    match data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let component_streams = fields
                    .named
                    .iter()
                    .map(|f| {
                        let name = &f.ident;
                        // Use correct field span to get potential error on correct line
                        component_tokens(name, f)
                    })
                    .collect::<Vec<TokenStream>>();
                meta_tokens(component_streams.into_iter())
            }
            _ => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}

pub fn impl_vec_op_tokens(
    item_data: &Data,
    trait_ident: Ident,
    op_ident: Ident,
    type_ident: &Ident,
    other_tokens: TokenStream,
    output_ident: Option<&Ident>,
    impl_generics: ImplGenerics,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
    is_scalar_op: bool,
) -> TokenStream {
    let component_tokens = |c: &Option<Ident>, f: &Field| {
        if is_scalar_op {
            {
                quote_spanned! {f.span() =>
                    self.#c.#op_ident(other)
                }
            }
        } else {
            {
                quote_spanned! {f.span() =>
                    self.#c.#op_ident(other.#c)
                }
            }
        }
    };
    if output_ident.is_some() {
        // recurse gives result of each component, let's join with ',' for new-args
        let opped_components = per_component_tokens(
            item_data,
            &component_tokens,
            &|recurse| quote!(#(#recurse,)*),
        );

        quote! {
            impl #impl_generics #trait_ident<#other_tokens> for #type_ident #type_generics
            #where_clause
            {
                type Output = #output_ident #type_generics;

                fn #op_ident(self, other: #other_tokens) -> Self::Output {
                    #output_ident::new( #opped_components )
                }
            }
        }
    } else {
        // recurse gives assignment expr, just add ';' to complete
        let opped_components = per_component_tokens(
            item_data,
            &component_tokens,
            &|recurse| quote!(#(#recurse;)*),
        );
        quote! {
            impl #impl_generics #trait_ident<#other_tokens> for #type_ident #type_generics
            #where_clause
            {
                fn #op_ident(&mut self, other: #other_tokens) {
                    debug_assert!(!self.has_nans());
                    #opped_components
                    debug_assert!(!self.has_nans());
                }
            }
        }
    }
}

pub fn abs_impl(vec_type: &Ident, item: &DeriveInput) -> TokenStream {
    let generics = add_trait_bound(&item.generics, quote! {num::traits::Signed});

    let ParsedGenerics {
        impl_generics,
        type_generics,
        where_clause,
        ..
    } = match parse_generics(&generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error("Impl Point floor_ceil", item.ident.span(), errors)
                .to_compile_error();
        }
    };

    let abs_ret = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c.abs()),
        &|recurse| quote!(Self::new(#(#recurse),*)),
    );

    let str_type = vec_type.to_string();
    let abs_doc = format!(
        "Returns a new `{0}` with the absolute values of the components in this `{0}`.",
        str_type
    );

    quote! {
        impl #impl_generics #vec_type #type_generics
        #where_clause
        {
            #[doc = #abs_doc]
            #[inline]
            pub fn abs(&self) -> Self {
                #abs_ret
            }
        }
    }
}
