extern crate quote;
extern crate syn;

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Add)]
pub fn add(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let component_sums = compile_component_ops(&input.data);

    let expanded = quote! {
        // The generated impl.
        impl #impl_generics Add for #name #ty_generics
        #where_clause
        {
            type Output = Self;

            fn add (self, other: Self) -> Self {
                debug_assert!(!self.has_nans());
                debug_assert!(!other.has_nans());

                #name {
                    #component_sums
                }
            }
        }
    };

    // panic!(expanded.to_string());

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

//
fn compile_component_ops(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Use correct field span to get potential error on correct line
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! {f.span() =>
                            #name: self.#name + other.#name
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
