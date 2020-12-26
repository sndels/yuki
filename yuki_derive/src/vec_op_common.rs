use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

// Giving other implements a vec-vec operation, vec-scalar otherwise
pub fn impl_vec_op_tokens(
    trait_ident: Ident,
    trait_fn_ident: Ident,
    type_ident: &Ident,
    other_ident: Option<&Ident>,
    output_ident: &Ident,
    opped_components: TokenStream,
) -> TokenStream {
    let other = if let Some(other) = other_ident {
        quote! {#other<T>}
    } else {
        quote! { T }
    };

    quote! {
        impl<T> #trait_ident<#other> for #type_ident<T>
        where
            T: yuki_common::ValueType
        {
            type Output = #output_ident<T>;

            fn #trait_fn_ident(self, other: #other) -> Self::Output {
                #output_ident::new( #opped_components )
            }
        }
    }
}

// Giving other implements a vec-vec operation, vec-scalar otherwise
pub fn impl_vec_assign_op_tokens(
    trait_ident: Ident,
    trait_fn_ident: Ident,
    op_ident: Ident,
    type_ident: &Ident,
    other_ident: Option<&Ident>,
) -> TokenStream {
    let other = if let Some(other) = other_ident {
        quote! {#other<T>}
    } else {
        quote! { T }
    };

    quote! {
        impl<T> #trait_ident<#other> for #type_ident<T>
        where
            T: yuki_common::ValueType
        {
            fn #trait_fn_ident(&mut self, other: #other) {
                debug_assert!(!self.has_nans());
                *self = self.#op_ident(other);
                debug_assert!(!self.has_nans());
            }
        }
    }
}
