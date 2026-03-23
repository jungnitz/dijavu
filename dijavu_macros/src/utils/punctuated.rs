use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Token;

pub struct PunctuatedIter<I, D>(I, D);

impl<I> PunctuatedIter<I, Token![,]> {
    pub fn comma(iter: I) -> Self {
        Self(iter, Default::default())
    }
}

impl<T, I, D> ToTokens for PunctuatedIter<I, D>
where
    T: ToTokens,
    D: ToTokens,
    I: Iterator<Item = T> + Clone,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for v in self.0.clone() {
            v.to_tokens(tokens);
            self.1.to_tokens(tokens);
        }
    }
}
