use std::{marker::PhantomData, ops::Deref};

use futures::future::BoxFuture;

use crate::{
    Error, Injectable, Injector,
    build::{dependency_graph::NodeIndex, value_per_injectable::ValuePerInjectable},
};

/// Gives the injectable `I` access to the [`Injector`] while it is started, see
/// [`Injectable::start`].
///
/// The injector is available through deref. There is no public way to create this type.
pub struct StartContext<'ctx, I: Injectable> {
    injector: &'ctx Injector,
    injectable: PhantomData<fn() -> I>,
}

impl<'ctx, I: Injectable> StartContext<'ctx, I> {
    fn new(injector: &'ctx Injector) -> Self {
        Self {
            injector,
            injectable: PhantomData,
        }
    }

    /// Returns a clone of the injector, e.g. to keep it in a spawned task.
    ///
    /// Storing it within an injectable creates a cycle, so the injectables are never dropped.
    pub fn injector(&self) -> Injector {
        self.injector.clone()
    }
}

impl<I: Injectable> Clone for StartContext<'_, I> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I: Injectable> Copy for StartContext<'_, I> {}

impl<I: Injectable> Deref for StartContext<'_, I> {
    type Target = Injector;

    fn deref(&self) -> &Self::Target {
        self.injector
    }
}

/// Starts the injectable belonging to a node, taking its `Start` from the given values.
pub(super) type StartFn = for<'a> fn(
    &'a Injector,
    &mut ValuePerInjectable,
    NodeIndex,
) -> BoxFuture<'a, Result<(), Error>>;

pub(super) fn start_fn<I: Injectable>() -> StartFn {
    |injector, values, index| {
        let start = values.remove::<I::Start>(index);
        let ctx = StartContext::new(injector);
        Box::pin(injector.get::<I>().start(start, ctx))
    }
}

/// The injectables to start, in dependency order, along with their `Start`.
pub(super) struct Starts {
    pub(super) order: Vec<(NodeIndex, StartFn)>,
    pub(super) values: ValuePerInjectable,
}

impl Starts {
    pub(super) async fn start(mut self, injector: &Injector) -> Result<(), Error> {
        for (index, start) in self.order {
            start(injector, &mut self.values, index).await?;
        }
        assert_eq!(self.values.len(), 0);
        Ok(())
    }
}
