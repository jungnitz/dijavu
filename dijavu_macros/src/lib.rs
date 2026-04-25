mod initializable;
mod injectable;
mod utils;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Initializable, attributes(initializable))]
pub fn derive_initializable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    initializable::derive_initializable(input)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

#[proc_macro_derive(Injectable, attributes(inject))]
pub fn derive_injectable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    injectable::derive_injectable(input)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}
