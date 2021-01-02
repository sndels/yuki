use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, Field, Ident};

use crate::common::{combined_error, parse_generics, per_component_tokens};

pub fn vec_impl(item: &DeriveInput) -> TokenStream {
    let vec_type = &item.ident;
    let shorthand = Ident::new(&vec_type.to_string().to_lowercase(), Span::call_site());

    let (generic_param, impl_generics, type_generics, where_clause) =
        match parse_generics(&item.generics) {
            Ok((g, i, t, w)) => (g, i, t, w),
            Err(errors) => {
                return combined_error("Impl Vec", item.ident.span(), errors).to_compile_error();
            }
        };

    let new_args = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #generic_param),
        &|recurse| quote!(#(#recurse),*),
    );
    let new_init = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c),
        &|recurse| quote!(#(#recurse),*),
    );
    let zeros_init = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #generic_param::zero()),
        &|recurse| quote!(#(#recurse,)*),
    );
    let ones_init = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #generic_param::one()),
        &|recurse| quote!(#(#recurse,)*),
    );
    // Not all T have is_nan() so use NaN != NaN
    let has_nans_pred = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c != self.#c),
        &|recurse| quote!(#(#recurse)||*),
    );
    let dot_ret = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c * other.#c),
        &|recurse| quote!( #generic_param::zero() #(+ #recurse)*),
    );
    let min_init = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: self.#c.mini(other.#c)),
        &|recurse| quote!(#(#recurse,)*),
    );
    let max_init = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: self.#c.maxi(other.#c)),
        &|recurse| quote!(#(#recurse,)*),
    );
    let permuted_args = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: usize),
        &|recurse| quote!(#(, #recurse)*),
    );
    let permuted_init = per_component_tokens(
        &item.data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: self[#c]),
        &|recurse| quote!(#(#recurse,)*),
    );
    let from_args = per_component_tokens(
        &item.data,
        &|_c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => v),
        &|recurse| quote!(#(#recurse),*),
    );

    quote! {
        impl #impl_generics #vec_type #type_generics
        #where_clause
        {
            /// Constructs a new vector.
            ///
            /// Has a debug assert that checks for NaNs.
            #[inline]
            pub fn new(#new_args) -> Self {
                let v = Self{ #new_init };
                debug_assert!(!v.has_nans());
                v
            }

            /// Constructs a new vector of 0s.
            #[inline]
            pub fn zeros() -> Self {
                Self {
                    #zeros_init
                }
            }

            /// Constructs a new vector of 1s.
            #[inline]
            pub fn ones() -> Self {
                Self {
                    #ones_init
                }
            }

            /// Returns `true` if any component is NaN.
            #[inline]
            pub fn has_nans(&self) -> bool {
                #has_nans_pred
            }

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

            /// Returns the component-wise minimum of the two vectors.
            #[inline]
            pub fn min(&self, other: Self) -> Self {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                Self {
                    #min_init
                }
            }

            /// Returns the component-wise maximum of the two vectors.
            #[inline]
            pub fn max(&self, other: Self) -> Self {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                Self {
                    #max_init
                }
            }

            /// Returns the vector permutation defined by the indices.
            #[inline]
            pub fn permuted(&self #permuted_args) -> Self {
                debug_assert!(!self.has_nans());

                Self {
                    #permuted_init
                }
            }
        }

        /// Shorthand constructor
        #[inline]
        pub fn #shorthand #type_generics (#new_args) -> #vec_type #type_generics
        #where_clause
        {
            // Use new() to catch NANs
            #vec_type::new(#new_init)
        }

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
    }
}
