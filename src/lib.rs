#![recursion_limit = "128"]

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn lapack(_attr: TokenStream, _func: TokenStream) -> TokenStream {
    todo!()
}
