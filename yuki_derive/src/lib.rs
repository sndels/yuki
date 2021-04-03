use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    {parse_macro_input, DeriveInput, Ident},
};

mod common;
mod derive_math_op;
mod derive_trait;
mod impl_bounds;
mod impl_normal;
mod impl_point;
mod impl_vec;
mod impl_vec_like;
mod impl_vec_op;

use impl_bounds as bounds_impl;
use impl_normal as normal_impl;
use impl_point as point_impl;
use impl_vec as vec_impl;

macro_rules! impl_t {
    ($fn_name:ident $impl_fn:path) => {
        #[proc_macro_attribute]
        /// Doesn't expect attributes
        pub fn $fn_name(
            _attr: proc_macro::TokenStream,
            item: proc_macro::TokenStream,
        ) -> proc_macro::TokenStream {
            let item = parse_macro_input!(item as DeriveInput);

            let impl_tokens = $impl_fn(&item);
            let tokens = quote! {
                #item
                #impl_tokens
            };

            // Can be used to print the tokens
            // panic!(impl_tokens.to_string());
            // panic!(tokens.to_string());

            proc_macro::TokenStream::from(tokens)
        }
    };
}

impl_t!(impl_bounds bounds_impl::bounds_impl);
impl_t!(impl_normal normal_impl::normal_impl);
impl_t!(impl_point point_impl::point_impl);
impl_t!(impl_vec vec_impl::vec_impl);

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
derive!(AbsDiffEq abs_diff_eq derive_trait::approx);
derive!(RelativeEq relative_eq derive_trait::approx);
derive!(Neg neg derive_math_op::neg);
