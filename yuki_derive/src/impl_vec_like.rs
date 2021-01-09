use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, Field, Ident, ImplGenerics, TypeGenerics, WhereClause};

use crate::common::per_component_tokens;

pub fn vec_like_impl(
    data: &Data,
    vec_type: &Ident,
    generic_param: Ident,
    impl_generics: ImplGenerics,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
    member_ops: Option<TokenStream>,
    post_impl: Option<TokenStream>,
) -> TokenStream {
    let shorthand = Ident::new(&vec_type.to_string().to_lowercase(), Span::call_site());

    let new_args = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #generic_param),
        &|recurse| quote!(#(#recurse),*),
    );
    let new_init = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c),
        &|recurse| quote!(#(#recurse),*),
    );
    let zeros_init = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #generic_param::zero()),
        &|recurse| quote!(#(#recurse,)*),
    );
    let ones_init = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: #generic_param::one()),
        &|recurse| quote!(#(#recurse,)*),
    );
    // Not all T have is_nan() so use NaN != NaN
    let has_nans_pred = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c != self.#c),
        &|recurse| quote!(#(#recurse)||*),
    );
    let min_init = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: self.#c.mini(other.#c)),
        &|recurse| quote!(#(#recurse,)*),
    );
    let max_init = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: self.#c.maxi(other.#c)),
        &|recurse| quote!(#(#recurse,)*),
    );
    let permuted_args = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: usize),
        &|recurse| quote!(#(, #recurse)*),
    );
    let permuted_init = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => #c: self[#c]),
        &|recurse| quote!(#(#recurse,)*),
    );

    let str_type = vec_type.to_string();
    let new_doc = format! { "Creates a new `{0}`.", str_type};
    let zeros_doc = format! { "Creates a new `{0}` filled with `0`s.", str_type};
    let ones_doc = format! { "Creates a new `{0}` filled with `1`s.", str_type};
    let has_nans_doc = format! { "Checks if this `{0}` contains NaNs.", str_type};
    let min_doc = format! { "Returns a new `{0}` with the component-wise minimum of this `{0}` and another `{0}`.", str_type};
    let max_doc = format! { "Returns a new `{0}` with the component-wise maximum of this `{0}` and another `{0}`.", str_type};
    let permuted_doc = format! { "Returns a new `{0}` with a permutation of this `{0}`. The arguments define what index in this `{0}` to map for each component in the new `{0}`.", str_type};
    let shorthand_doc = format! { "A shorthand version of [{0}::new].", str_type};

    quote! {
        impl #impl_generics #vec_type #type_generics
        #where_clause
        {
            #[doc = #new_doc]
            #[inline]
            pub fn new(#new_args) -> Self {
                let v = Self{ #new_init };
                debug_assert!(!v.has_nans());
                v
            }

            #[doc = #zeros_doc]
            #[inline]
            pub fn zeros() -> Self {
                Self {
                    #zeros_init
                }
            }

            #[doc = #ones_doc]
            #[inline]
            pub fn ones() -> Self {
                Self {
                    #ones_init
                }
            }

            #[doc = #has_nans_doc]
            #[inline]
            pub fn has_nans(&self) -> bool {
                #has_nans_pred
            }

            #member_ops

            #[doc = #min_doc]
            #[inline]
            pub fn min(&self, other: Self) -> Self {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                Self {
                    #min_init
                }
            }

            #[doc = #max_doc]
            #[inline]
            pub fn max(&self, other: Self) -> Self {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                Self {
                    #max_init
                }
            }

            #[doc = #permuted_doc]
            #[inline]
            pub fn permuted(&self #permuted_args) -> Self {
                debug_assert!(!self.has_nans());

                Self {
                    #permuted_init
                }
            }
        }

        #[doc = #shorthand_doc]
        #[inline]
        pub fn #shorthand #type_generics (#new_args) -> #vec_type #type_generics
        #where_clause
        {
            // Use new() to catch NANs
            #vec_type::new(#new_init)
        }

        #post_impl
    }
}

pub fn vec_normal_members_impl(
    data: &Data,
    vec_type: &Ident,
    generic_param: &Ident,
) -> TokenStream {
    let dot_ret = per_component_tokens(
        &data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c * other.#c),
        &|recurse| quote!( #generic_param::zero() #(+ #recurse)*),
    );

    let str_type = vec_type.to_string();
    let dot_doc =
        format! { "Calculates the dot product of this `{0}` and another `{0}`.", str_type};
    let len_sqr_doc = format! { "Calculates the squared length of this `{0}`.", str_type};
    let len_doc = format! { "Calculates the length of this `{0}`.", str_type};
    let normalized_doc = format! { "Returns a new `{0}` with this `{0}` normalized.", str_type};

    quote! {
        #[doc = #dot_doc]
        #[inline]
        pub fn dot(&self, other: Self) -> #generic_param {
            debug_assert!(!self.has_nans());
            debug_assert!(!other.has_nans());

            #dot_ret
        }

        #[doc = #len_sqr_doc]
        #[inline]
        pub fn len_sqr(&self) -> #generic_param {
            debug_assert!(!self.has_nans());

            self.dot(*self)
        }

        #[doc = #len_doc]
        #[inline]
        pub fn len(&self) -> #generic_param {
            debug_assert!(!self.has_nans());

            #generic_param::from_f64(self.len_sqr().to_f64().unwrap().sqrt()).unwrap()
        }

        #[doc = #normalized_doc]
        #[inline]
        pub fn normalized(&self) -> Self {
            debug_assert!(!self.has_nans());

            *self / self.len()
        }

    }
}
