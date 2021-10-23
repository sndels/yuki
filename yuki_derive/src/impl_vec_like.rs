use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, Field, Ident};

use crate::common::{per_component_tokens, ParsedGenerics};

pub fn vec_like_impl(
    data: &Data,
    vec_type: &Ident,
    parsed_generics: ParsedGenerics,
    member_ops: Option<TokenStream>,
    post_impl: Option<TokenStream>,
) -> TokenStream {
    let ParsedGenerics {
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
    } = parsed_generics;

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
    let component_count = if str_type == "Normal" || str_type == "Spectrum" {
        3
    } else {
        match str_type.chars().last().unwrap().to_digit(10) {
            Some(c) => c as usize,
            None => {
                return syn::Error::new_spanned(
                    vec_type,
                    format!(
                        "Impl '{}': Expected component count at the end of name.",
                        str_type
                    ),
                )
                .to_compile_error()
            }
        }
    };

    let new_doc = format!("Creates a new `{0}`.", str_type);
    let zeros_doc = format!("Creates a new `{0}` filled with `0`s.", str_type);
    let ones_doc = format!("Creates a new `{0}` filled with `1`s.", str_type);
    let has_nans_doc = format!("Checks if this `{0}` contains NaNs.", str_type);
    let array_doc = format!("Returns a reference to this `{0}` as an array.", str_type);
    let array_mut_doc = format!(
        "Returns a mutable reference to this `{0}` as an array.",
        str_type
    );
    let min_doc = format!(
        "Returns a new `{0}` with the component-wise minimum of this `{0}` and another `{0}`.",
        str_type
    );
    let max_doc = format!(
        "Returns a new `{0}` with the component-wise maximum of this `{0}` and another `{0}`.",
        str_type
    );
    let permuted_doc = format!( "Returns a new `{0}` with a permutation of this `{0}`. The arguments define what index in this `{0}` to map for each component in the new `{0}`.", str_type);

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

            // This is generic so no is_nan() available
            #[allow(eq_op)]
            #[doc = #has_nans_doc]
            #[inline]
            pub fn has_nans(&self) -> bool {
                #has_nans_pred
            }

            #[doc = #array_doc]
            #[inline]
            pub fn array(&mut self) -> &[#generic_param] {
                self.array_mut()
            }

            #[doc = #array_mut_doc]
            #[inline]
            pub fn array_mut(&mut self) -> &mut [#generic_param; #component_count] {
                unsafe {
                    let slice_ref = std::slice::from_raw_parts_mut(self as *mut Self as *mut #generic_param, #component_count);
                    <&mut [#generic_param; #component_count]>::try_from(slice_ref).unwrap()
                }
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

        #post_impl
    }
}

pub fn vec_normal_members_impl(
    data: &Data,
    vec_type: &Ident,
    generic_param: &Ident,
) -> TokenStream {
    let dot_ret = per_component_tokens(
        data,
        &|c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => self.#c * other.#c),
        &|recurse| quote!( #generic_param::zero() #(+ #recurse)*),
    );

    let str_type = vec_type.to_string();
    let dot_doc = format!(
        "Calculates the dot product of this `{0}` and another `{0}`.",
        str_type
    );
    let len_sqr_doc = format!("Calculates the squared length of this `{0}`.", str_type);
    let len_doc = format!("Calculates the length of this `{0}`.", str_type);
    let normalized_doc = format!("Returns a new `{0}` with this `{0}` normalized.", str_type);

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
