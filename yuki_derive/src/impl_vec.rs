use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput, Field, Ident};

use crate::{
    common::{abs_impl, combined_error, parse_generics, per_component_tokens},
    impl_vec_like::{vec_like_impl, vec_normal_members_impl},
};

pub fn vec_impl(item: &DeriveInput) -> TokenStream {
    let vec_type = &item.ident;

    let (generic_param, impl_generics, type_generics, where_clause) =
        match parse_generics(&item.generics) {
            Ok((g, i, t, w)) => (g, i, t, w),
            Err(errors) => {
                return combined_error("Impl Vec", item.ident.span(), errors).to_compile_error();
            }
        };

    let member_ops = vec_normal_members_impl(&item.data, &vec_type, &generic_param);

    let from_args = per_component_tokens(
        &item.data,
        &|_c: &Option<Ident>, f: &Field| quote_spanned!(f.span() => v),
        &|recurse| quote!(#(#recurse),*),
    );

    let signed_abs_impl = abs_impl(vec_type, item);

    let post_impl = quote! {
        #signed_abs_impl

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
