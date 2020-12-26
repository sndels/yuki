use proc_macro;
use syn::{parse_macro_input, DeriveInput};

mod derive_vec_op;
mod vec_op_common;

use derive_vec_op::derive_vec_op;

macro_rules! derive {
    ($tr:ident $fn_name:ident $derive_fn:ident) => {
        #[proc_macro_derive($tr)]
        pub fn $fn_name(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            // Parse the input tokens into a syntax tree.
            let input = parse_macro_input!(input as DeriveInput);
            proc_macro::TokenStream::from($derive_fn(input, stringify!($tr)))
        }
    };
}

// These implement the op assuming the type is a "vector" with separate components of type T
// These are basically Op<"Vector"<T>> for "Vector"<T> where T: ValueType
derive!(Add add derive_vec_op);
derive!(Sub sub derive_vec_op);
// These are basically Op<T> for "Vector"<T> where T: ValueType
derive!(AddScalar add_scalar derive_vec_op);
derive!(SubScalar sub_scalar derive_vec_op);
derive!(MulScalar mul_scalar derive_vec_op);
derive!(DivScalar div_scalar derive_vec_op);
