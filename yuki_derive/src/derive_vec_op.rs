use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident};

use crate::vec_op_common::{emit_vec_op_scalar_impl, emit_vec_op_vec_impl};

pub fn derive_vec_op(input: DeriveInput, trait_name: &str) -> TokenStream {
    let (trait_name, scalar_op) = if trait_name.ends_with("Scalar") {
        (trait_name.strip_suffix("Scalar").unwrap(), true)
    } else {
        (trait_name, false)
    };
    let trait_ident = Ident::new(&trait_name, Span::call_site());
    let op_ident = Ident::new(&trait_name.to_lowercase(), Span::call_site());
    let type_ident = input.ident;

    if scalar_op {
        let component_sums = derive_component_ops(input.data, &op_ident, false);

        emit_vec_op_scalar_impl(
            trait_ident,
            &type_ident,
            &type_ident,
            op_ident,
            component_sums,
        )
    } else {
        let component_sums = derive_component_ops(input.data, &op_ident, true);

        emit_vec_op_vec_impl(
            trait_ident,
            &type_ident,
            &type_ident,
            &type_ident,
            op_ident,
            component_sums,
        )
    }
}

fn derive_component_ops(data: Data, op_ident: &Ident, other_has_components: bool) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Use correct field span to get potential error on correct line
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        if other_has_components {
                            quote_spanned! {f.span() =>
                                self.#name.#op_ident(other.#name)
                            }
                        } else {
                            quote_spanned! {f.span() =>
                                self.#name.#op_ident(other)
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
