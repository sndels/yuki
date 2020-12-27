#![feature(iterator_fold_self)]

use proc_macro;
use syn::{parse_macro_input, DeriveInput};

mod derive_vec_op;
mod vec_op_common;

macro_rules! derive_vec_op {
    ($tr:ident $fn_name:ident) => {
        #[proc_macro_derive($tr)]
        pub fn $fn_name(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            // Parse the input tokens into a syntax tree.
            let input = parse_macro_input!(input as DeriveInput);

            let tokens = derive_vec_op::vec_op(input, stringify!($tr));

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
derive_vec_op!(Add add);
derive_vec_op!(Sub sub);
derive_vec_op!(AddAssign add_assign);
derive_vec_op!(SubAssign sub_assign);
// These are basically Op<T> for "Vector"<T>
derive_vec_op!(AddScalar add_scalar);
derive_vec_op!(SubScalar sub_scalar);
derive_vec_op!(MulScalar mul_scalar);
derive_vec_op!(DivScalar div_scalar);
derive_vec_op!(AddAssignScalar add_assign_scalar);
derive_vec_op!(SubAssignScalar sub_assign_scalar);
derive_vec_op!(MulAssignScalar mul_assign_scalar);
derive_vec_op!(DivAssignScalar div_assign_scalar);
