use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput, Field, Ident};

use crate::{
    common::{
        abs_impl, add_trait_bound, combined_error, parse_generics, per_component_tokens,
        ParsedGenerics,
    },
    impl_vec_like::vec_like_impl,
};

pub fn point_impl(item: &DeriveInput) -> TokenStream {
    let point_type = &item.ident;

    let parsed_generics = match parse_generics(&item.generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error("Impl Point", item.ident.span(), errors).to_compile_error();
        }
    };
    let ParsedGenerics {
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
    } = &parsed_generics;

    let str_type = point_type.to_string();
    let dist_doc = format!(
        "Calculates the distance between this `{0}` and another `{0}`.",
        str_type
    );
    let dist_sqr_doc = format!(
        "Calculates the squared distance between this `{0}` and another `{0}`.",
        str_type
    );
    let lerp_doc = format! ( "Returns a new `{0}` that was linearly interpolated between this `{0}` and another `{0}` by the factor `t`.", str_type);
    let lerp_components = per_component_tokens(
        &item.data,
        // We assume any Num type can be converted to and from f32
        &|c: &Option<Ident>, f: &Field| {
            quote_spanned! {
                f.span() =>
                   #c: #generic_param::from_f32(
                        (1.0 - t) * self.#c.to_f32().unwrap() + t * other.#c.to_f32().unwrap()
                    ).unwrap()
            }
        },
        &|recurse| quote!(#(#recurse,)*),
    );

    let member_ops = quote! {
            #[doc = #dist_doc]
            pub fn dist(&self, other: Self ) -> T {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                (*self - other).len()
            }

            #[doc = #dist_sqr_doc]
            pub fn dist_sqr(&self, other: Self ) -> T {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                (*self - other).len_sqr()
            }

            #[doc = #lerp_doc]
            pub fn lerp(&self, other: Self , t: f32) -> Self {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());
                debug_assert!(!t.is_nan());

                Self { #lerp_components }
            }
    };

    let float_floor_ceil_impl = point_floor_ceil_impl(point_type, item);
    let signed_abs_impl = abs_impl(point_type, item);

    let from_args = per_component_tokens(
        &item.data,
        &|_c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => v),
        &|recurse| quote!(#(#recurse),*),
    );

    let post_impl = quote! {
        #float_floor_ceil_impl
        #signed_abs_impl

        // I don't really like that this trait gets generated from the impl macro,
        // though deriving From<T> with a derive macro seems as cryptic.
        // Then again, this whole thing is an exercise in rubegoldberging and should
        // only be used through the generated docs...
        impl #impl_generics From #type_generics for #point_type #type_generics
        #where_clause
        {
            fn from(v: #generic_param) -> Self {
                Self::new(#from_args)
            }
        }
    };

    vec_like_impl(
        &item.data,
        point_type,
        parsed_generics,
        Some(member_ops),
        Some(post_impl),
    )
}

fn point_floor_ceil_impl(point_type: &Ident, item: &DeriveInput) -> TokenStream {
    let generics = add_trait_bound(&item.generics, quote! {num::Float});

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

    let floor_ret = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c.floor()),
        &|recurse| quote!(Self::new(#(#recurse),*)),
    );
    let ceil_ret = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c.ceil()),
        &|recurse| quote!(Self::new(#(#recurse),*)),
    );

    let str_type = point_type.to_string();
    let floor_doc = format!(
        "Returns a new `{0}` with the components of this `{0}` rounded down.",
        str_type
    );
    let ceil_doc = format!(
        "Returns a new `{0}` with the components of this `{0}` rounded up.",
        str_type
    );

    quote! {
        impl #impl_generics #point_type #type_generics
        #where_clause
        {
            #[doc = #floor_doc]
            pub fn floor(&self) -> Self {
                #floor_ret
            }

            #[doc = #ceil_doc]
            pub fn ceil(&self) -> Self {
                #ceil_ret
            }
        }
    }
}
