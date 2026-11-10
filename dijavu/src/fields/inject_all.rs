use std::{iter::FusedIterator, slice};

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, Visitable, refs::Ref,
};

/// References to all providers of `D` in the build, see [`FieldDeclareContext::provide`].
///
/// Trait objects can be used here via [`unsizable!`](crate::unsizable), or via
/// [`bindable!`](crate::bindable) if they should be bindable as well.
///
/// The providers are kept in the order in which they were added to the build. Providers that are
/// not part of the build are not referenced, so they still have to be added, e.g. via an
/// [`Inject`](super::Inject) field. To decide about the implementations instead, use
/// [`InjectVec`](super::InjectVec). Accessing the implementations panics until the build has
/// finished.
///
/// # Example
///
/// ```
/// use dijavu::{*, fields::InjectAll};
///
/// pub trait Handler: Send + Sync {
///     fn name(&self) -> &str;
/// }
/// dijavu::unsizable!(Handler);
///
/// #[derive(Injectable)]
/// #[injectable(provide(dyn Handler))]
/// struct Logging;
///
/// impl Handler for Logging {
///     fn name(&self) -> &str {
///         "logging"
///     }
/// }
///
/// #[derive(Injectable)]
/// #[injectable(provide(dyn Handler))]
/// struct Metrics;
///
/// impl Handler for Metrics {
///     fn name(&self) -> &str {
///         "metrics"
///     }
/// }
///
/// #[derive(Injectable)]
/// struct Server {
///     handlers: InjectAll<dyn Handler>,
/// }
///
/// let mut builder = Builder::new();
/// builder.add::<Server>().add::<Logging>().add::<Metrics>();
/// let injector = futures::executor::block_on(builder.build()).unwrap();
/// let names: Vec<_> = injector.get::<Server>().handlers.iter().map(|h| h.name()).collect();
/// assert_eq!(names, ["logging", "metrics"]);
/// ```
pub struct InjectAll<D: ?Sized + Send + Sync + 'static>(Vec<Ref<D>>);

impl<D: ?Sized + Send + Sync + 'static> InjectAll<D> {
    /// Returns an iterator over the implementations, in the order in which they were added to the
    /// build.
    ///
    /// # Panics
    ///
    /// Advancing the iterator panics until the build has finished.
    pub fn iter(&self) -> InjectAllIter<'_, D> {
        InjectAllIter(self.0.iter())
    }

    /// Returns the implementation at `index`, or `None` if it is out of bounds.
    ///
    /// # Panics
    ///
    /// Panics if the index is in bounds but the build has not finished yet.
    pub fn get(&self, index: usize) -> Option<&D> {
        self.0.get(index).map(|reference| &**reference)
    }

    /// Returns the number of implementations.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether there are no implementations.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<D: ?Sized + Send + Sync + 'static> InjectableField for InjectAll<D> {
    type Declaration = ();
    type Preparation = ();
    type Start = ();

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(())
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration)
    }

    async fn assemble(
        _preparation: Self::Preparation,
        mut ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        Ok((Self(ctx.get_provided_refs::<D>()), ()))
    }
}

// SAFETY: the references can only be moved out of `InjectAll` through `&mut self`
unsafe impl<D: ?Sized + Send + Sync + 'static> Visitable for InjectAll<D> {
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
        for reference in &mut self.0 {
            visitor.visit_ref(reference);
        }
    }
}

impl<'a, D: ?Sized + Send + Sync + 'static> IntoIterator for &'a InjectAll<D> {
    type Item = &'a D;
    type IntoIter = InjectAllIter<'a, D>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the implementations of an [`InjectAll`], see [`InjectAll::iter`].
pub struct InjectAllIter<'a, D: ?Sized + Send + Sync + 'static>(slice::Iter<'a, Ref<D>>);

impl<'a, D: ?Sized + Send + Sync + 'static> Iterator for InjectAllIter<'a, D> {
    type Item = &'a D;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|reference| &**reference)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<D: ?Sized + Send + Sync + 'static> DoubleEndedIterator for InjectAllIter<'_, D> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|reference| &**reference)
    }
}

impl<D: ?Sized + Send + Sync + 'static> ExactSizeIterator for InjectAllIter<'_, D> {}

impl<D: ?Sized + Send + Sync + 'static> FusedIterator for InjectAllIter<'_, D> {}
