use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn emit_vec_op_vec_impl(
    trait_ident: Ident,
    other_ident: &Ident,
    type_ident: &Ident,
    output_ident: &Ident,
    op_ident: Ident,
    component_sums: TokenStream,
) -> TokenStream {
    let expanded = vec_op_impl(
        trait_ident,
        quote! { #other_ident<T> },
        type_ident,
        output_ident,
        op_ident,
        component_sums,
    );

    // Can be used to print the tokens
    // panic!(expanded.to_string());

    expanded
}

pub fn emit_vec_op_scalar_impl(
    trait_ident: Ident,
    type_ident: &Ident,
    output_ident: &Ident,
    op_ident: Ident,
    component_sums: TokenStream,
) -> TokenStream {
    let expanded = vec_op_impl(
        trait_ident,
        quote! { T },
        type_ident,
        output_ident,
        op_ident,
        component_sums,
    );

    // Can be used to print the tokens
    // panic!(expanded.to_string());

    expanded
}

pub fn vec_op_impl(
    trait_ident: Ident,
    other: TokenStream,
    type_ident: &Ident,
    output_ident: &Ident,
    op_ident: Ident,
    component_sums: TokenStream,
) -> TokenStream {
    quote! {
        impl<T> #trait_ident<#other> for #type_ident<T>
        where
        T: yuki_common::ValueType
        {
            type Output = #output_ident<T>;

            fn #op_ident (self, other: #other) -> #output_ident<T>{
                #output_ident::new(
                    #component_sums
                )
            }
        }
    }
}
