#![feature(iterator_fold_self)]

use proc_macro;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, DeriveInput, Ident};

mod common;
mod derive_math_op;
mod derive_trait;
mod impl_normal;
mod impl_point;
mod impl_vec;
mod impl_vec_like;
mod impl_vec_op;

use impl_normal as normal_impl;
use impl_point as point_impl;
use impl_vec as vec_impl;

#[proc_macro_attribute]
/// Doesn't expect attributes
pub fn impl_point(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as DeriveInput);

    let impl_tokens = point_impl::point_impl(&item);
    let tokens = quote! {
        #item
        #impl_tokens
    };

    // Can be used to print the tokens
    // panic!(impl_tokens.to_string());
    // panic!(tokens.to_string());

    proc_macro::TokenStream::from(tokens)
}

#[proc_macro_attribute]
/// Doesn't expect attributes
pub fn impl_vec(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as DeriveInput);

    let impl_tokens = vec_impl::vec_impl(&item);
    let tokens = quote! {
        #item
        #impl_tokens
    };

    // Can be used to print the tokens
    // panic!(impl_tokens.to_string());
    // panic!(tokens.to_string());

    proc_macro::TokenStream::from(tokens)
}

#[proc_macro_attribute]
/// Doesn't expect attributes
pub fn impl_normal(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as DeriveInput);

    let impl_tokens = normal_impl::normal_impl(&item);
    let tokens = quote! {
        #item
        #impl_tokens
    };

    // Can be used to print the tokens
    // panic!(impl_tokens.to_string());
    // panic!(tokens.to_string());

    proc_macro::TokenStream::from(tokens)
}

struct ApproxAttr {
    value_types: Vec<Ident>,
}

impl Parse for ApproxAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut value_types = vec![input.parse()?];
        while !input.is_empty() {
            input.parse::<syn::token::Comma>()?;
            value_types.push(input.parse()?);
        }
        Ok(ApproxAttr { value_types })
    }
}

#[proc_macro_attribute]
/// Expects a list of value types, e.g. (f32, f64)
pub fn impl_abs_diff_eq(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ApproxAttr { value_types } = parse_macro_input!(attr as ApproxAttr);
    let item = parse_macro_input!(item as DeriveInput);

    let mut tokens = quote! {#item};
    for value_type in value_types {
        let impl_tokens = impl_vec_op::abs_diff_eq(&item, &value_type);
        tokens = quote! {
            #tokens
            #impl_tokens

        };
    }

    // Can be used to print the tokens
    // panic!(tokens.to_string());

    proc_macro::TokenStream::from(tokens)
}

#[proc_macro_attribute]
/// Expects a list of value types, e.g. (f32, f64)
pub fn impl_relative_eq(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ApproxAttr { value_types } = parse_macro_input!(attr as ApproxAttr);
    let item = parse_macro_input!(item as DeriveInput);

    let mut tokens = quote! {#item};
    for value_type in value_types {
        let impl_tokens = impl_vec_op::relative_eq(&item, &value_type);
        tokens = quote! {
            #tokens
            #impl_tokens

        };
    }

    // Can be used to print the tokens
    // panic!(tokens.to_string());

    proc_macro::TokenStream::from(tokens)
}
struct VecOpAttr {
    op_trait: Ident,
    other: Ident,
    output: Ident,
}

impl Parse for VecOpAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(VecOpAttr {
            op_trait: input.parse()?,
            other: input.parse()?,
            output: input.parse()?,
        })
    }
}

#[proc_macro_attribute]
/// Expects attrs ('Trait' 'Other' 'Output')
pub fn vec_op(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let VecOpAttr {
        op_trait,
        other,
        output,
    } = parse_macro_input!(attr as VecOpAttr);
    let item = parse_macro_input!(item as DeriveInput);

    let impl_tokens = impl_vec_op::vec_op(op_trait, other, Some(&output), &item);
    let tokens = quote! {
        #item
        #impl_tokens
    };

    // Can be used to print the tokens
    // panic!(impl_tokens.to_string());
    // panic!(tokens.to_string());

    proc_macro::TokenStream::from(tokens)
}

struct VecAssignOpAttr {
    op_trait: Ident,
    other: Ident,
}

impl Parse for VecAssignOpAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(VecAssignOpAttr {
            op_trait: input.parse()?,
            other: input.parse()?,
        })
    }
}

#[proc_macro_attribute]
/// Expects attrs ('Trait' 'Other')
pub fn vec_assign_op(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let VecAssignOpAttr { op_trait, other } = parse_macro_input!(attr as VecAssignOpAttr);
    let item = parse_macro_input!(item as DeriveInput);

    let impl_tokens = impl_vec_op::vec_op(op_trait, other, None, &item);
    let tokens = quote! {
        #item
        #impl_tokens
    };

    // Can be used to print the tokens
    // panic!(impl_tokens.to_string());
    // panic!(tokens.to_string());

    proc_macro::TokenStream::from(tokens)
}

macro_rules! derive {
    ($tr:ident $fn_name:ident $derive_func:path) => {
        #[proc_macro_derive($tr)]
        pub fn $fn_name(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            // Parse the input tokens into a syntax tree.
            let input = parse_macro_input!(input as DeriveInput);

            let tokens = $derive_func(input, stringify!($tr));

            // Can be used to print the tokens
            // panic!(tokens.to_string());

            proc_macro::TokenStream::from(tokens)
        }
    };
}

// These implement the op assuming the type is a "vector" with separate components of type 'T'
// where T: yuki_common::ValueType
// They also require a construction with the form new(c0, c1...)
// These are basically Op<"Vector"<T>> for "Vector"<T>
derive!(Add add derive_math_op::vec_op);
derive!(Sub sub derive_math_op::vec_op);
derive!(AddAssign add_assign derive_math_op::vec_op);
derive!(SubAssign sub_assign derive_math_op::vec_op);
// These are basically Op<T> for "Vector"<T>
derive!(AddScalar add_scalar derive_math_op::vec_op);
derive!(SubScalar sub_scalar derive_math_op::vec_op);
derive!(MulScalar mul_scalar derive_math_op::vec_op);
derive!(DivScalar div_scalar derive_math_op::vec_op);
derive!(AddAssignScalar add_assign_scalar derive_math_op::vec_op);
derive!(SubAssignScalar sub_assign_scalar derive_math_op::vec_op);
derive!(MulAssignScalar mul_assign_scalar derive_math_op::vec_op);
derive!(DivAssignScalar div_assign_scalar derive_math_op::vec_op);
derive!(Index index derive_trait::index);
derive!(IndexMut index_mut derive_trait::index);
derive!(Neg neg derive_math_op::neg);
