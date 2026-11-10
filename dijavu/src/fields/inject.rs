use std::{marker::PhantomData, ops::Deref};

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, Target, Visitable, build::DependencyType, fields::DependToken, refs::Ref,
};

/// A token that indicates that an [`Injectable`](crate::Injectable) will be built as a result of the current
/// initialization process.
///
/// You can obtain one of these tokens by adding a [`Inject`] field for `T` to your [`Injectable`](crate::Injectable),
/// via [`FieldDeclareContext::inject`], or for the injectable itself via its context, like
/// [`PrepareContext::inject_token`](crate::PrepareContext::inject_token).
// Implementation Note: this token makes absolutely no **guarantees** about anything. It's just a
// hint to the user on what he needs to do (add an `Inject` field) to achieve certain things. Do not
// rely on it for any safety reasoning etc.
pub struct InjectToken<T: ?Sized + Target>(PhantomData<T>);

impl<T: ?Sized + Target> InjectToken<T> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: ?Sized + Target> Clone for InjectToken<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Target> Copy for InjectToken<T> {}

/// A reference to another injectable that may only be used once the build has finished.
///
/// In contrast to [`Depend`](super::Depend), this does not impose any order on building the
/// injectables, so cyclic references are possible. Dereferencing panics until the build has
/// finished.
///
/// `T` can also be a trait object that is bound to an injectable (see [`Target`]).
pub struct Inject<T: ?Sized + Target>(Ref<T>);

impl<T: ?Sized + Target> InjectableField for Inject<T> {
    type Declaration = InjectToken<T>;
    type Preparation = InjectToken<T>;
    type Start = ();

    fn declare(mut init: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        T::declare(&mut init, DependencyType::Soft);
        Ok(InjectToken::new())
    }

    async fn prepare(
        declaration: Self::Declaration,
        _init: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration)
    }

    async fn assemble(
        _init: Self::Preparation,
        mut builder: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        Ok((Inject(T::get_ref(&mut builder)), ()))
    }
}

// SAFETY: the reference can only be moved out of `Inject` through `&mut self`
unsafe impl<T: ?Sized + Target> Visitable for Inject<T> {
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
        visitor.visit_ref(&mut self.0);
    }
}

impl<T: ?Sized + Target> Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized + Target> From<DependToken<T>> for InjectToken<T> {
    fn from(_value: DependToken<T>) -> Self {
        InjectToken::new()
    }
}
