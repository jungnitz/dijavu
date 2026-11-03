use std::{iter::FusedIterator, slice};

use futures::future::BoxFuture;

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, UnsizeFrom, Visitable,
};

/// The state of a [`Fields`] after the declare step, to which fields are added via
/// [`add`](Self::add).
pub struct FieldsDeclaration<T: ?Sized>(Vec<Box<PrepareFn<T>>>);

/// Prepares one of the added fields and returns the function assembling it.
type PrepareFn<T> = dyn for<'ctx> FnOnce(
        FieldPrepareContext<'ctx>,
    ) -> BoxFuture<'ctx, Result<Box<AssembleFn<T>>, Error>>
    + Send;

/// Returns the function preparing the field `F` from its declaration.
fn prepare_fn<T, F>(declaration: F::Declaration) -> Box<PrepareFn<T>>
where
    T: ?Sized + UnsizeFrom<F>,
    F: InjectableField,
{
    Box::new(|ctx| {
        Box::pin(async move {
            let preparation = F::prepare(declaration, ctx).await?;
            Ok(assemble_fn::<T, F>(preparation))
        })
    })
}

/// The state of a [`Fields`] after the prepare step.
pub struct FieldsPreparation<T: ?Sized>(Vec<Box<AssembleFn<T>>>);

/// Assembles one of the added fields, converted to `T`.
type AssembleFn<T> = dyn for<'ctx> FnOnce(FieldAssembleContext<'ctx>) -> BoxFuture<'ctx, Result<Box<T>, Error>>
    + Send;

/// Returns the function assembling the field `F` from its preparation.
fn assemble_fn<T, F>(preparation: F::Preparation) -> Box<AssembleFn<T>>
where
    T: ?Sized + UnsizeFrom<F>,
    F: InjectableField,
{
    Box::new(|ctx| {
        Box::pin(async move {
            let (field, _) = F::assemble(preparation, ctx).await?;
            Ok(T::coerce_box(Box::new(field)))
        })
    })
}

/// Any number of fields of possibly different types, which are all accessed as `T` (typically a
/// trait object). They are added during the build via [`FieldsDeclaration::add`].
///
/// Each added field takes part in the prepare and assemble steps of the injectable containing this
/// field, in the order in which the fields were added. Their [`Start`](InjectableField::Start) is
/// dropped after the assemble step.
///
/// Trait objects can be used here via [`unsizable!`](crate::unsizable). To visit the references
/// within the added fields, `T` must implement [`Visitable`], so for a trait object, the trait
/// needs [`Visitable`] as a supertrait, like `trait Plugin: Visitable + Send + Sync`.
pub struct Fields<T: ?Sized>(Vec<Box<T>>);

impl<T: Send + Sync + ?Sized> FieldsDeclaration<T> {
    /// Adds the field `F`, after all previously added fields, given its declaration.
    ///
    /// The declaration is typically created via [`InjectableField::declare`] in a `declare` hook,
    /// so that `F` declares what it references. It can also be put together from tokens that are
    /// already available, e.g. an [`InjectToken`](crate::fields::InjectToken) for an
    /// [`Inject`](crate::fields::Inject) field.
    pub fn add<F>(&mut self, declaration: F::Declaration)
    where
        F: InjectableField,
        T: UnsizeFrom<F>,
    {
        self.0.push(prepare_fn::<T, F>(declaration));
    }
}

impl<T: ?Sized> Fields<T> {
    /// Returns an iterator over the fields, in the order in which they were added.
    pub fn iter(&self) -> FieldsIter<'_, T> {
        FieldsIter(self.0.iter())
    }

    /// Returns the field at `index`, or `None` if it is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index).map(|field| &**field)
    }

    /// Returns the number of fields.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether there are no fields.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: Send + Sync + ?Sized + Visitable + 'static> InjectableField for Fields<T> {
    type Declaration = FieldsDeclaration<T>;
    type Preparation = FieldsPreparation<T>;
    type Start = ();

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(FieldsDeclaration(Vec::new()))
    }

    async fn prepare(
        declaration: Self::Declaration,
        mut ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        let mut assemble_fns = Vec::with_capacity(declaration.0.len());
        for declaration in declaration.0 {
            assemble_fns.push(declaration(ctx.reborrow()).await?);
        }
        Ok(FieldsPreparation(assemble_fns))
    }

    async fn assemble(
        preparation: Self::Preparation,
        mut ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        let mut values = Vec::with_capacity(preparation.0.len());
        for assemble_fn in preparation.0 {
            values.push(assemble_fn(ctx.reborrow()).await?);
        }
        Ok((Fields(values), ()))
    }
}

// SAFETY: the fields can only be moved out of `Fields` through `&mut self`, and each of them is
// `Visitable` itself
unsafe impl<T: ?Sized + Visitable> Visitable for Fields<T> {
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
        for field in &mut self.0 {
            field.visit_refs(visitor);
        }
    }
}

impl<'a, T: ?Sized> IntoIterator for &'a Fields<T> {
    type Item = &'a T;
    type IntoIter = FieldsIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the fields of a [`Fields`], see [`Fields::iter`].
pub struct FieldsIter<'a, T: ?Sized>(slice::Iter<'a, Box<T>>);

impl<'a, T: ?Sized> Iterator for FieldsIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|field| &**field)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T: ?Sized> DoubleEndedIterator for FieldsIter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|field| &**field)
    }
}

impl<T: ?Sized> ExactSizeIterator for FieldsIter<'_, T> {}

impl<T: ?Sized> FusedIterator for FieldsIter<'_, T> {}
