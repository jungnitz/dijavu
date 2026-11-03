use std::{marker::PhantomData, ops::Deref};

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, Target, Visitable, build::DependencyType, refs::Ref,
};

/// Allows accessing the preparation of `T` via [`FieldPrepareContext::get`].
///
/// You can obtain one of these tokens by adding a [`Depend`] field for `T` to your
/// [`Injectable`](crate::Injectable).
pub struct DependToken<T: ?Sized>(PhantomData<T>);

/// A reference to another injectable that is prepared before the injectable containing it.
///
/// Dereferencing panics until the build has finished.
///
/// `T` can also be a trait object that is bound to an injectable (see [`Target`]). In this case,
/// the bound injectable is still prepared first, but its preparation cannot be accessed via
/// [`FieldPrepareContext::get`].
pub struct Depend<T: ?Sized + Target>(Ref<T>);

impl<T: ?Sized + Target> InjectableField for Depend<T> {
    type Declaration = DependToken<T>;
    type Preparation = DependToken<T>;
    type Start = ();

    fn declare(mut init: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        T::declare(&mut init, DependencyType::Hard);
        Ok(DependToken(PhantomData))
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
        Ok((Depend(T::get_ref(&mut builder)), ()))
    }
}

// SAFETY: the reference can only be moved out of `Depend` through `&mut self`
unsafe impl<T: ?Sized + Target> Visitable for Depend<T> {
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
        visitor.visit_ref(&mut self.0);
    }
}

impl<T: ?Sized + Target> Deref for Depend<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
