use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, TypeGenerics, WhereClause};

// Other should be follow the form `Vec<T>` or `T`
pub fn impl_vec_op_tokens(
    trait_ident: Ident,
    trait_fn_ident: Ident,
    type_ident: &Ident,
    other: TokenStream,
    output_ident: &Ident,
    opped_components: TokenStream,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> TokenStream {
    // We only support impl_generics == type_generics
    quote! {
        impl #type_generics #trait_ident<#other> for #type_ident #type_generics
        #where_clause
        {
            type Output = #output_ident #type_generics;

            fn #trait_fn_ident(self, other: #other) -> Self::Output {
                #output_ident::new( #opped_components )
            }
        }
    }
}

// Other should be form `Vec<T>` or `T`
pub fn impl_vec_assign_op_tokens(
    trait_ident: Ident,
    trait_fn_ident: Ident,
    op_ident: Ident,
    type_ident: &Ident,
    other: TokenStream,
    type_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> TokenStream {
    // We only support impl_generics == type_generics
    quote! {
        impl #type_generics #trait_ident<#other> for #type_ident #type_generics
        #where_clause
        {
            fn #trait_fn_ident(&mut self, other: #other) {
                debug_assert!(!self.has_nans());
                *self = self.#op_ident(other);
                debug_assert!(!self.has_nans());
            }
        }
    }
}
