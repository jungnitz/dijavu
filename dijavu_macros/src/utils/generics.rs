use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{GenericParam, Generics};

use crate::utils::{Either, PunctuatedIter};

pub struct GenericsHelper {
    pub generics: Generics,
    where_clause: TokenStream,
}

impl GenericsHelper {
    pub fn from_generics(generics: Generics) -> Self {
        Self {
            where_clause: generics
                .where_clause
                .as_ref()
                .map(ToTokens::to_token_stream)
                .unwrap_or_else(|| quote!(where)),
            generics,
        }
    }

    pub fn split_for_impl(&self) -> (impl ToTokens, impl ToTokens, impl ToTokens) {
        (
            PunctuatedIter::comma(self.generics.params.iter()),
            PunctuatedIter::comma(self.generics.params.iter().map(|param| match param {
                GenericParam::Const(param) => Either::Left(&param.ident),
                GenericParam::Lifetime(param) => Either::Right(&param.lifetime),
                GenericParam::Type(param) => Either::Left(&param.ident),
            })),
            &self.where_clause,
        )
    }
}
