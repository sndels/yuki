use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, Field, Ident};

use crate::common::{combined_error, parse_generics, per_component_tokens};
use crate::impl_vec_like::vec_like_impl;

pub fn vec_impl(item: &DeriveInput) -> TokenStream {
    let vec_type = &item.ident;

    let (generic_param, impl_generics, type_generics, where_clause) =
        match parse_generics(&item.generics) {
            Ok((g, i, t, w)) => (g, i, t, w),
            Err(errors) => {
                return combined_error("Impl Vec", item.ident.span(), errors).to_compile_error();
            }
        };

    let dot_ret = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c * other.#c),
        &|recurse| quote!( #generic_param::zero() #(+ #recurse)*),
    );

    let member_ops = quote! {
        /// Returns the dot product of the two vectors.
        #[inline]
        pub fn dot(&self, other: Self) -> #generic_param {
            debug_assert!(!self.has_nans());
            debug_assert!(!other.has_nans());

            #dot_ret
        }

        /// Returns the vector's squared length.
        #[inline]
        pub fn len_sqr(&self) -> #generic_param {
            debug_assert!(!self.has_nans());

            self.dot(*self)
        }

        /// Returns the vector's length.
        #[inline]
        pub fn len(&self) -> #generic_param {
            debug_assert!(!self.has_nans());

            #generic_param::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
        }

        /// Returns the normalized vector.
        #[inline]
        pub fn normalized(&self) -> Self {
            debug_assert!(!self.has_nans());

            *self / self.len()
        }

    };

    let from_args = per_component_tokens(
        &item.data,
        &|_c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => v),
        &|recurse| quote!(#(#recurse),*),
    );

    let post_impl = quote! {
        // I don't really like that this trait gets generated from the impl macro,
        // though deriving From<T> with a derive macro seems as cryptic.
        // Then again, this whole thing is an exercise in rubegoldberging and should
        // only be used through the generated docs...
        impl #impl_generics From #type_generics for #vec_type #type_generics
        #where_clause
        {
            fn from(v: #generic_param) -> Self {
                Self::new(#from_args)
            }
        }
    };

    vec_like_impl(
        &item.data,
        vec_type,
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
        Some(member_ops),
        Some(post_impl),
    )
}
