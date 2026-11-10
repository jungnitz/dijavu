use std::{iter::FusedIterator, slice};

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, Injectable,
    InjectableField, RefVisitor, UnsizeFrom, Visitable, fields::InjectToken, refs::Ref,
};

/// References to any number of other injectables implementing `D`, which are added during the build
/// via [`InjectVecState::add`].
///
/// Unlike [`InjectAll`](super::InjectAll), this does not include the providers of `D`, so the
/// injectable containing it decides about the implementations, their order and whether one is
/// referenced multiple times.
///
/// Trait objects can be used here via [`unsizable!`](crate::unsizable), or via
/// [`bindable!`](crate::bindable) if they should be bindable as well.
///
/// The implementations are kept in the order in which they were added. While building, the state of
/// the injectable holds an [`InjectVecState`]. Accessing the implementations panics until the build
/// has finished.
///
/// # Example
///
/// ```
/// use dijavu::{*, fields::{Inject, InjectVec}, state::Declare};
///
/// pub trait Handler: Send + Sync {
///     fn name(&self) -> &str;
/// }
/// dijavu::unsizable!(Handler);
///
/// #[derive(Injectable)]
/// struct Logging;
///
/// impl Handler for Logging {
///     fn name(&self) -> &str {
///         "logging"
///     }
/// }
///
/// #[derive(Injectable)]
/// struct Metrics;
///
/// impl Handler for Metrics {
///     fn name(&self) -> &str {
///         "metrics"
///     }
/// }
///
/// #[derive(Injectable)]
/// #[injectable(prepare = Self::add_handlers)]
/// struct Server {
///     handlers: InjectVec<dyn Handler>,
///     logging: Inject<Logging>,
///     metrics: Inject<Metrics>,
/// }
///
/// impl Server {
///     async fn add_handlers(
///         state: &mut ServerState<Declare>,
///         _ctx: PrepareContext<'_, Self>,
///     ) -> Result<(), Error> {
///         state.handlers.add(state.logging);
///         state.handlers.add(state.metrics);
///         Ok(())
///     }
/// }
///
/// let mut builder = Builder::new();
/// builder.add::<Server>();
/// let injector = futures::executor::block_on(builder.build()).unwrap();
/// let names: Vec<_> = injector.get::<Server>().handlers.iter().map(|h| h.name()).collect();
/// assert_eq!(names, ["logging", "metrics"]);
/// ```
pub struct InjectVec<D: ?Sized + Send + Sync + 'static>(Vec<Ref<D>>);

/// The state of an [`InjectVec`] during the build.
pub struct InjectVecState<D: ?Sized + Send + Sync + 'static> {
    /// Create the references to the added implementations.
    get_refs: Vec<fn(&mut FieldAssembleContext<'_>) -> Ref<D>>,
}

impl<D: ?Sized + Send + Sync + 'static> InjectVecState<D> {
    /// Adds the injectable `T` as an implementation, after all previously added ones.
    ///
    /// Adding the same injectable multiple times references it multiple times.
    ///
    /// The token is obtained by adding an [`Inject`](super::Inject) field for `T`, or for the
    /// injectable itself via its context, like
    /// [`PrepareContext::inject_token`](crate::PrepareContext::inject_token).
    ///
    /// # Panics
    ///
    /// The assemble step panics if `T` is not part of the build, which can only happen if the token
    /// was obtained during another build.
    pub fn add<T: Injectable>(&mut self, _token: InjectToken<T>)
    where
        D: UnsizeFrom<T>,
    {
        self.get_refs.push(|ctx| ctx.get_ref_as::<D, T>());
    }
}

impl<D: ?Sized + Send + Sync + 'static> InjectVec<D> {
    /// Returns an iterator over the implementations, in the order in which they were added.
    ///
    /// # Panics
    ///
    /// Advancing the iterator panics until the build has finished.
    pub fn iter(&self) -> InjectVecIter<'_, D> {
        InjectVecIter(self.0.iter())
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

impl<D: ?Sized + Send + Sync + 'static> InjectableField for InjectVec<D> {
    type Declaration = InjectVecState<D>;
    type Preparation = InjectVecState<D>;
    type Start = ();

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(InjectVecState {
            get_refs: Vec::new(),
        })
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration)
    }

    async fn assemble(
        preparation: Self::Preparation,
        mut ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        let refs = preparation
            .get_refs
            .into_iter()
            .map(|get_ref| get_ref(&mut ctx))
            .collect();
        Ok((Self(refs), ()))
    }
}

// SAFETY: the references can only be moved out of `InjectVec` through `&mut self`
unsafe impl<D: ?Sized + Send + Sync + 'static> Visitable for InjectVec<D> {
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
        for reference in &mut self.0 {
            visitor.visit_ref(reference);
        }
    }
}

impl<'a, D: ?Sized + Send + Sync + 'static> IntoIterator for &'a InjectVec<D> {
    type Item = &'a D;
    type IntoIter = InjectVecIter<'a, D>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the implementations of an [`InjectVec`], see [`InjectVec::iter`].
pub struct InjectVecIter<'a, D: ?Sized + Send + Sync + 'static>(slice::Iter<'a, Ref<D>>);

impl<'a, D: ?Sized + Send + Sync + 'static> Iterator for InjectVecIter<'a, D> {
    type Item = &'a D;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|reference| &**reference)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<D: ?Sized + Send + Sync + 'static> DoubleEndedIterator for InjectVecIter<'_, D> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|reference| &**reference)
    }
}

impl<D: ?Sized + Send + Sync + 'static> ExactSizeIterator for InjectVecIter<'_, D> {}

impl<D: ?Sized + Send + Sync + 'static> FusedIterator for InjectVecIter<'_, D> {}
