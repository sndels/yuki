use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, Field, Ident};

use crate::common::{add_trait_bound, combined_error, parse_generics, per_component_tokens};
use crate::impl_vec_like::vec_like_impl;

pub fn point_impl(item: &DeriveInput) -> TokenStream {
    let point_type = &item.ident;

    let (generic_param, impl_generics, type_generics, where_clause) =
        match parse_generics(&item.generics) {
            Ok((g, i, t, w)) => (g, i, t, w),
            Err(errors) => {
                return combined_error("Impl Point", item.ident.span(), errors).to_compile_error();
            }
        };
    let member_ops = quote! {
            /// Returns the distance between the points.
            #[inline]
            pub fn dist(&self, other: Self ) -> T {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                (*self - other).len()
            }

            /// Returns the distance between the points.
            #[inline]
            pub fn dist_sqr(&self, other: Self ) -> T {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                (*self - other).len_sqr()
            }

            /// Linearly interpolates between the points by factor t
            #[inline]
            pub fn lerp(&self, other: Self , t: #generic_param) -> Self {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                *self * (#generic_param::one() - t) + other * t
            }
    };

    let float_floor_ceil_impl = point_floor_ceil_impl(point_type, item);
    let signed_abs_impl = point_abs_impl(point_type, item);
    let post_impl = quote! {
        #float_floor_ceil_impl
        #signed_abs_impl
    };

    vec_like_impl(
        &item.data,
        point_type,
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
        Some(member_ops),
        Some(post_impl),
    )
}

fn point_floor_ceil_impl(point_type: &Ident, item: &DeriveInput) -> TokenStream {
    let generics = add_trait_bound(&item.generics, quote! {num::Float});

    let (_, impl_generics, type_generics, where_clause) = match parse_generics(&generics) {
        Ok((g, i, t, w)) => (g, i, t, w),
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

    quote! {
        impl #impl_generics #point_type #type_generics
        #where_clause
        {
            /// Returns the vector with each of the components rounded down
            #[inline]
            pub fn floor(&self) -> Self {
                #floor_ret
            }

            /// Returns the vector with each of the components rounded up
            #[inline]
            pub fn ceil(&self) -> Self {
                #ceil_ret
            }
        }
    }
}

fn point_abs_impl(point_type: &Ident, item: &DeriveInput) -> TokenStream {
    let generics = add_trait_bound(&item.generics, quote! {num::traits::Signed});

    let (_, impl_generics, type_generics, where_clause) = match parse_generics(&generics) {
        Ok((g, i, t, w)) => (g, i, t, w),
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

    quote! {
        impl #impl_generics #point_type #type_generics
        #where_clause
        {
            /// Returns the vector with the absolute value for each component
            #[inline]
            pub fn abs(&self) -> Self {
                #abs_ret
            }
        }
    }
}
