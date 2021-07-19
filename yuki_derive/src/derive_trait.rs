use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned, Data, DeriveInput, Field, Fields, Ident, ImplGenerics, TypeGenerics,
    WhereClause,
};

use crate::common::{
    add_trait_bound, combined_error, parse_generics, per_component_tokens, ParsedGenerics,
    TraitInfo,
};

pub fn index(input: DeriveInput, name: &str) -> TokenStream {
    let TraitInfo {
        ident: trait_ident,
        op_ident,
        ..
    } = TraitInfo::new(name);

    let ParsedGenerics {
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
    } = match parse_generics(&input.generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error("Derive'Index'", input.ident.span(), errors).to_compile_error();
        }
    };
    let type_ident = &input.ident;

    let is_mutable_index = name.ends_with("Mut");

    let self_ref_tokens = if is_mutable_index {
        quote! {&mut self}
    } else {
        quote! {&self}
    };

    let match_tokens = index_match_tokens(&input.data, &self_ref_tokens);

    // IndexMut uses the output type defined by Index
    let trait_output_tokens = if is_mutable_index {
        None
    } else {
        let type_name = type_ident.to_string();
        if type_name.starts_with("Bounds") {
            // We should have already checked this is fine back in bounds_impl
            let component_count = type_name.chars().last().unwrap().to_digit(10).unwrap();
            let point_ident = Ident::new(&format!("Point{}", component_count), Span::call_site());
            Some(quote! {
                type Output = #point_ident #type_generics;
            })
        } else {
            Some(quote! {
                type Output = #generic_param;
            })
        }
    };

    let fn_output_tokens = if is_mutable_index {
        quote! {
            &mut Self::Output
        }
    } else {
        quote! {
            &Self::Output
        }
    };

    quote! {
        impl #impl_generics #trait_ident <usize> for #type_ident #type_generics
        #where_clause
        {
            #trait_output_tokens

            fn #op_ident(#self_ref_tokens, index: usize) -> #fn_output_tokens {
                match index {
                    #match_tokens
                    _ => {
                        panic!("Index {} is out of bounds", index);
                    }
                }
            }
        }
    }
}

fn index_match_tokens(data: &Data, self_ref_tokens: &TokenStream) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().enumerate().map(|(i, f)| {
                        let name = &f.ident;
                        // Use correct field span to get potential error on correct line
                        quote_spanned! {f.span() =>
                            #i => #self_ref_tokens.#name
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

pub fn approx(input: DeriveInput, name: &str) -> TokenStream {
    let trait_ident = Ident::new(name, Span::call_site());

    let ParsedGenerics { generic_param, .. } = match parse_generics(&input.generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error(&format!("Derive '{}'", name), input.ident.span(), errors)
                .to_compile_error();
        }
    };

    let generics = add_trait_bound(&input.generics, quote! { #trait_ident });
    let generics = add_trait_bound(&generics, quote! { AbsDiffEq<Epsilon = #generic_param> });
    let ParsedGenerics {
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
    } = match parse_generics(&generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error(&format!("Derive '{}'", name), input.ident.span(), errors)
                .to_compile_error();
        }
    };

    match name {
        "AbsDiffEq" => abs_diff_eq(
            input.data,
            &input.ident,
            &generic_param,
            impl_generics,
            type_generics,
            where_clause,
        ),
        "RelativeEq" => relative_eq(
            input.data,
            &input.ident,
            &generic_param,
            impl_generics,
            type_generics,
            where_clause,
        ),
        _ => unimplemented!(),
    }
}

fn abs_diff_eq(
    data: Data,
    vec_type: &Ident,
    generic_param: &Ident,
    impl_generics: ImplGenerics,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> TokenStream {
    let abs_diff_eq_tokens = per_component_tokens(
        &data,
        &|c: &Option<Ident>, f: &Field| {
            quote_spanned! { f.span() =>
                self.#c.abs_diff_eq(&other.#c, epsilon)
            }
        },
        &|recurse| quote!(#(#recurse)&&*),
    );

    quote! {
        impl #impl_generics approx::AbsDiffEq for #vec_type #type_generics
        #where_clause
        {
            type Epsilon = #generic_param::Epsilon;

            fn default_epsilon() -> Self::Epsilon {
                #generic_param::default_epsilon()
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
                #abs_diff_eq_tokens
            }
        }
    }
}

pub fn relative_eq(
    data: Data,
    vec_type: &Ident,
    generic_param: &Ident,
    impl_generics: ImplGenerics,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> TokenStream {
    let relative_eq_tokens = per_component_tokens(
        &data,
        &|c: &Option<Ident>, f: &Field| {
            quote_spanned! { f.span() =>
                self.#c.relative_eq(&other.#c, epsilon, max_relative)
            }
        },
        &|recurse| quote!(#(#recurse)&&*),
    );

    quote! {
        impl #impl_generics approx::RelativeEq for #vec_type #type_generics
        #where_clause
        {
            fn default_max_relative() -> Self::Epsilon {
                #generic_param::default_max_relative()
            }

            fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
                #relative_eq_tokens
            }
        }
    }
}
