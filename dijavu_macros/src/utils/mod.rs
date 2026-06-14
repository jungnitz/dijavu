use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{Expr, Lit, Meta};

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

pub fn doc_hidden(present: bool) -> TokenStream {
    if present {
        quote!(#[doc(hidden)])
    } else {
        quote!()
    }
}

pub struct WithSpan<T> {
    pub span: Span,
    pub inner: T,
}

impl<T: FromMeta> FromMeta for WithSpan<T> {
    fn from_nested_meta(item: &NestedMeta) -> darling::Result<Self> {
        Ok(Self {
            span: item.span(),
            inner: T::from_nested_meta(item)?,
        })
    }

    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Ok(Self {
            span: item.span(),
            inner: T::from_meta(item)?,
        })
    }

    fn from_none() -> Option<Self> {
        Some(Self {
            span: Span::call_site(),
            inner: T::from_none()?,
        })
    }

    fn from_word() -> darling::Result<Self> {
        Ok(Self {
            span: Span::call_site(),
            inner: T::from_word()?,
        })
    }

    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        Ok(Self {
            span: Span::call_site(),
            inner: T::from_list(items)?,
        })
    }

    fn from_value(value: &Lit) -> darling::Result<Self> {
        Ok(Self {
            span: value.span(),
            inner: T::from_value(value)?,
        })
    }

    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        Ok(Self {
            span: expr.span(),
            inner: T::from_expr(expr)?,
        })
    }

    fn from_char(value: char) -> darling::Result<Self> {
        Ok(Self {
            span: Span::call_site(),
            inner: T::from_char(value)?,
        })
    }

    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(Self {
            span: Span::call_site(),
            inner: T::from_string(value)?,
        })
    }

    fn from_bool(value: bool) -> darling::Result<Self> {
        Ok(Self {
            span: Span::call_site(),
            inner: T::from_bool(value)?,
        })
    }
}

impl<T: Default> Default for WithSpan<T> {
    fn default() -> Self {
        Self {
            span: Span::call_site(),
            inner: T::default(),
        }
    }
}
