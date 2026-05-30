use crate::utils::{Either, PunctuatedIter};
use darling::FromGenerics;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{GenericParam, Generics};

pub struct GenericsHelper {
    pub generics: Generics,
    where_clause: TokenStream,
}

impl GenericsHelper {
    pub fn is_empty(&self) -> bool {
        self.generics.params.is_empty()
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

impl FromGenerics for GenericsHelper {
    fn from_generics(generics: &Generics) -> darling::Result<Self> {
        Ok(Self {
            where_clause: generics
                .where_clause
                .as_ref()
                .map_or_else(|| quote!(where), ToTokens::to_token_stream),
            generics: generics.clone(),
        })
    }
}
