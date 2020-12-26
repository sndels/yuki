use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn impl_vec_op_vec_tokens(
    trait_ident: Ident,
    other_ident: &Ident,
    type_ident: &Ident,
    output_ident: &Ident,
    op_ident: Ident,
    opped_components: TokenStream,
) -> TokenStream {
    impl_vec_op_tokens(
        trait_ident,
        quote! { #other_ident<T> },
        type_ident,
        output_ident,
        op_ident,
        opped_components,
    )
}

pub fn impl_vec_op_scalar_tokens(
    trait_ident: Ident,
    type_ident: &Ident,
    output_ident: &Ident,
    op_ident: Ident,
    opped_components: TokenStream,
) -> TokenStream {
    impl_vec_op_tokens(
        trait_ident,
        quote! { T },
        type_ident,
        output_ident,
        op_ident,
        opped_components,
    )
}

pub fn impl_vec_op_tokens(
    trait_ident: Ident,
    other: TokenStream,
    type_ident: &Ident,
    output_ident: &Ident,
    op_ident: Ident,
    opped_components: TokenStream,
) -> TokenStream {
    quote! {
        impl<T> #trait_ident<#other> for #type_ident<T>
        where
        T: yuki_common::ValueType
        {
            type Output = #output_ident<T>;

            fn #op_ident(self, other: #other) -> Self::Output {
                #output_ident::new( #opped_components )
            }
        }
    }
}
