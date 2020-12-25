use proc_macro;
use syn::{parse_macro_input, DeriveInput};

mod math_op;

macro_rules! derive {
    ($tr:ident $fn_name:ident $module:ident) => {
        #[proc_macro_derive($tr)]
        pub fn $fn_name(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            // Parse the input tokens into a syntax tree.
            let input = parse_macro_input!(input as DeriveInput);
            proc_macro::TokenStream::from($module::derive(input, stringify!($tr)))
        }
    };
}

derive!(Add add math_op);
derive!(Sub sub math_op);
