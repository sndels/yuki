use proc_macro;
use syn::{parse_macro_input, DeriveInput};

mod derive_vec_op;
mod vec_op_common;

use derive_vec_op::derive_vec_op_vec;

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

derive!(Add add derive_vec_op_vec);
derive!(Sub sub derive_vec_op_vec);
