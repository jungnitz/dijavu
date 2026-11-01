use std::ops::Deref;

use crate::{
    AssembleContext, DeclareContext, Error, Injectable, InjectableField, PrepareContext,
    RefVisitor, Visitable,
};

/// A simple value that can be modified during the build.
///
/// `Value<T>` is a slim wrapper around `T` that starts out as `T::default()`. While building, the
/// state of the injectable holds the `T` directly, afterwards a reference to it can be accessed.
#[derive(Debug)]
pub struct Value<T>(T);

impl<T> Value<T> {
    /// Creates a value holding `t`.
    pub fn new(t: T) -> Self {
        Self(t)
    }

    /// Returns the `T` held by this value.
    pub fn into_inner(this: Value<T>) -> T {
        this.0
    }
}

impl<I: Injectable, T: Default + Send + Sync + 'static> InjectableField<I> for Value<T> {
    type Declaration = T;
    type Preparation = T;

    fn declare(_ctx: DeclareContext<'_, I>) -> Result<Self::Declaration, Error> {
        Ok(T::default())
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: PrepareContext<'_, I>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration)
    }

    async fn assemble(
        preparation: Self::Preparation,
        _ctx: AssembleContext<'_, I>,
    ) -> Result<Self, Error> {
        Ok(Self(preparation))
    }
}

// SAFETY: does not visit anything
unsafe impl<T> Visitable for Value<T> {
    fn visit_refs(&mut self, _visitor: &mut RefVisitor<'_>) {}
}

impl<T> Deref for Value<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
