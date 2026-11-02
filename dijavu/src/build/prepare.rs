use std::ops::{Deref, DerefMut};

use crate::{
    Error, Injectable,
    build::{
        BuildState, Registered, RegisteredValues, declare::Declarations,
        dependency_graph::NodeIndex, injectable_context, value_per_injectable::ValuePerInjectable,
    },
    error::MissingInputError,
    fields::{DependToken, Input},
    injectable::Preparing,
};

injectable_context!(PrepareContext, FieldPrepareContext, "prepare");

/// Gives fields access to the build during the prepare step.
pub struct FieldPrepareContext<'ctx> {
    state: &'ctx mut BuildState<PrepareStep>,
    node_index: NodeIndex,
}

impl<'ctx> FieldPrepareContext<'ctx> {
    /// Returns an owned context borrowing from this one, e.g. to pass it to multiple fields.
    pub fn reborrow(&mut self) -> FieldPrepareContext<'_> {
        FieldPrepareContext {
            state: self.state,
            node_index: self.node_index,
        }
    }

    /// Returns the values of type `R` that were registered via [`FieldDeclareContext::register`],
    /// in the order of their registration.
    ///
    /// [`FieldDeclareContext::register`]: crate::FieldDeclareContext::register
    pub fn registered<R: Registered>(&self) -> RegisteredValues<R> {
        self.state.registry.get()
    }

    /// Returns the value set via [`Builder::set_input`](crate::Builder::set_input).
    ///
    /// Fails if the input has not been set.
    pub fn input<T: Send + Sync + 'static>(&self) -> Result<Input<T>, MissingInputError> {
        self.state
            .inputs
            .get(self.state.dependency_graph.get(self.node_index))
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
            PrepareContext::new(FieldPrepareContext {
                state: self.state,
                node_index,
            }),
        );
        PreparingHandle {
            preparing: Some(preparing),
            node_index,
        }
    }
}

/// The [`Preparing`] state of an injectable, returned by [`FieldPrepareContext::get`].
///
/// Dropping this stores the changes back into the build.
pub struct PreparingHandle<'ctx, I: Injectable> {
    preparing: Option<I::Preparing<'ctx>>,
    node_index: NodeIndex,
}

impl<'ctx, I: Injectable> Drop for PreparingHandle<'ctx, I> {
    fn drop(&mut self) {
        let (ctx, preparation) = self.preparing.take().unwrap().into_parts();
        ctx.field
            .state
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
            PrepareContext::new(FieldPrepareContext {
                state: self,
                node_index: index,
            }),
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
