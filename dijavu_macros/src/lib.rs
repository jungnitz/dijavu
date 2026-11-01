//! The derive macros of `dijavu`, which re-exports and documents them.

mod injectable;
mod visitable;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Injectable, attributes(injectable, visitable))]
pub fn derive_injectable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    injectable::derive_injectable(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(InjectableField, attributes(injectable, visitable))]
pub fn derive_injectable_field(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    injectable::derive_injectable_field(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Visitable, attributes(visitable))]
pub fn derive_visitable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    visitable::derive_visitable(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
