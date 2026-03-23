use proc_macro2::TokenStream;
use quote::ToTokens;

mod generics;
pub use generics::GenericsHelper;

mod punctuated;
pub use punctuated::PunctuatedIter;

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> ToTokens for Either<L, R>
where
    L: ToTokens,
    R: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Either::Left(left) => left.to_tokens(tokens),
            Either::Right(right) => right.to_tokens(tokens),
        }
    }
}
