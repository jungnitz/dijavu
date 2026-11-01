use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    Error, Injectable,
    build::{
        BuildState, Registered, RegisteredValues, declare::Declarations,
        dependency_graph::NodeIndex, value_per_injectable::ValuePerInjectable,
    },
    error::MissingInputError,
    fields::{DependToken, Input},
    injectable::Preparing,
};

/// Gives access to the build during the prepare step.
pub struct PrepareContext<'ctx, I: Injectable> {
    state: &'ctx mut BuildState<PrepareStep>,
    _i: PhantomData<I>,
}

impl<'ctx, I: Injectable> PrepareContext<'ctx, I> {
    /// Returns an owned context borrowing from this one, e.g. to pass it to multiple fields.
    pub fn as_owned(&mut self) -> PrepareContext<'_, I> {
        PrepareContext {
            state: self.state,
            _i: PhantomData,
        }
    }

    /// Returns the values of type `R` that were registered via [`DeclareContext::register`], in
    /// the order of their registration, which is empty if there are none.
    ///
    /// All values are available regardless of the dependencies between injectables. The returned
    /// list does not borrow from this context and can be kept in the state of the injectable.
    ///
    /// [`DeclareContext::register`]: crate::DeclareContext::register
    pub fn registered<R: Registered>(&self) -> RegisteredValues<R> {
        self.state.registry.get()
    }

    /// Returns the value set via [`Builder::set_input`](crate::Builder::set_input).
    ///
    /// Fails if the input has not been set.
    pub fn input<T: Send + Sync + 'static>(&self) -> Result<Input<T>, MissingInputError> {
        self.state.inputs.get::<I, T>()
    }

    /// Returns the [`Preparing`] state of `Other`, which allows configuring it.
    ///
    /// The token is obtained by adding a [`Depend`](crate::fields::Depend) field for `Other`, which
    /// ensures that `Other` has already been prepared.
    pub fn get<Other: Injectable>(
        &mut self,
        _token: &mut DependToken<Other>,
    ) -> PreparingHandle<'_, Other> {
        let node_index = self.state.dependency_graph.get_index::<Other>();
        let preparation = self.state.step.preparations.remove::<Other>(node_index);
        let preparing = Other::make_preparing(
            preparation,
            PrepareContext {
                state: self.state,
                _i: PhantomData,
            },
        );
        PreparingHandle {
            preparing: Some(preparing),
            node_index,
        }
    }
}

/// The [`Preparing`] state of an injectable, returned by [`PrepareContext::get`].
///
/// Dropping this stores the changes back into the build.
pub struct PreparingHandle<'ctx, I: Injectable> {
    preparing: Option<I::Preparing<'ctx>>,
    node_index: NodeIndex,
}

impl<'ctx, I: Injectable> Drop for PreparingHandle<'ctx, I> {
    fn drop(&mut self) {
        let (ctx, preparation) = self.preparing.take().unwrap().into_parts();
        ctx.state
            .step
            .preparations
            .insert::<I>(self.node_index, preparation);
    }
}

impl<'ctx, I: Injectable> Deref for PreparingHandle<'ctx, I> {
    type Target = I::Preparing<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.preparing.as_ref().unwrap()
    }
}

impl<'ctx, I: Injectable> DerefMut for PreparingHandle<'ctx, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.preparing.as_mut().unwrap()
    }
}

pub(super) struct PrepareStep {
    declarations: Declarations,
    preparations: Preparations,
}

impl BuildState<PrepareStep> {
    pub(super) fn new(state: BuildState<Declarations>) -> Self {
        state.map_step(|declarations, _| PrepareStep {
            preparations: Preparations::new(declarations.len()),
            declarations,
        })
    }

    pub(super) async fn prepare(mut self) -> Result<BuildState<Preparations>, Error> {
        let order = self.dependency_graph.topological_sort()?;
        for node_index in order {
            let node = self.dependency_graph.get(node_index);
            (node.prepare)(&mut self, node_index).await?;
        }
        assert_eq!(self.step.declarations.len(), 0);
        assert_eq!(
            self.step.preparations.len(),
            self.dependency_graph.node_count()
        );
        Ok(self.map_step(|step, _| step.preparations))
    }

    pub(super) async fn prepare_one<I: Injectable>(
        &mut self,
        index: NodeIndex,
    ) -> Result<(), Error> {
        let declaration = self.step.declarations.remove::<I>(index);
        let preparation = I::prepare(
            declaration,
            PrepareContext::<I> {
                state: self,
                _i: PhantomData,
            },
        )
        .await?;
        self.step.preparations.insert::<I>(index, preparation);
        Ok(())
    }
}

pub(super) struct Preparations(ValuePerInjectable);

impl Preparations {
    fn new(capacity: usize) -> Self {
        Self(ValuePerInjectable::with_capacity(capacity))
    }

    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn insert<T: Injectable>(&mut self, index: NodeIndex, preparation: T::Preparation) {
        self.0.insert::<T::Preparation>(index, preparation);
    }

    pub(super) fn remove<T: Injectable>(&mut self, index: NodeIndex) -> T::Preparation {
        self.0.remove::<T::Preparation>(index)
    }
}
