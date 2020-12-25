use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident, ImplGenerics, TypeGenerics, WhereClause};

pub fn compile_vec_vec(
    impl_generics: ImplGenerics,
    tr_name: &Ident,
    other_name: &Ident,
    ty_name: &Ident,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
    output_name: &Ident,
    op_name: Ident,
    component_sums: TokenStream,
) -> TokenStream {
    let expanded = quote! {
        impl #impl_generics #tr_name < #other_name #ty_generics > for #ty_name #ty_generics
        #where_clause
        {
            type Output = #output_name #ty_generics;

            fn #op_name (self, other: #other_name #ty_generics) -> #output_name #ty_generics{
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                #output_name {
                    #component_sums
                }
            }
        }
    };

    // Can be used to print the tokens
    // panic!(expanded.to_string());

    proc_macro2::TokenStream::from(expanded)
}

pub fn derive(input: DeriveInput, trait_name: &str) -> TokenStream {
    let tr_name = Ident::new(trait_name, Span::call_site());
    let op_name = Ident::new(&trait_name.to_lowercase(), Span::call_site());
    let ty_name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let component_sums = compile_component_ops(&input.data, &op_name, true);

    compile_vec_vec(
        impl_generics,
        &tr_name,
        &ty_name,
        &ty_name,
        &ty_generics,
        where_clause,
        &ty_name,
        op_name,
        component_sums,
    )
}

fn compile_component_ops(data: &Data, op_name: &Ident, other_has_components: bool) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Use correct field span to get potential error on correct line
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        if other_has_components {
                            quote_spanned! {f.span() =>
                                #name: self.#name.#op_name(other.#name)
                            }
                        } else {
                            quote_spanned! {f.span() =>
                                #name: self.#name.#op_name(other)
                            }
                        }
                    });
                    quote! {
                        #(#recurse,)*
                    }
                }
                _ => unimplemented!(),
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
