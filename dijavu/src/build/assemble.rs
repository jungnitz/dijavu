use std::{any::type_name, marker::PhantomData};

use crate::{
    Error, Injectable, Target,
    build::{
        BuildState, Registered, RegisteredValues,
        dependency_graph::{DependencyType, NodeIndex},
        prepare::Preparations,
    },
    error::MissingInputError,
    fields::Input,
    refs::Injectables,
    refs::Ref,
};

/// Gives access to the build during the assemble step.
pub struct AssembleContext<'a, I: Injectable> {
    state: &'a mut BuildState<AssembleStep>,
    node_index: NodeIndex,
    _i: PhantomData<I>,
}

impl<'ctx, I: Injectable> AssembleContext<'ctx, I> {
    /// Returns an owned context borrowing from this one, e.g. to pass it to multiple fields.
    pub fn as_owned(&mut self) -> AssembleContext<'_, I> {
        AssembleContext {
            state: self.state,
            node_index: self.node_index,
            _i: PhantomData,
        }
    }

    /// Returns the values of type `R` that were registered via [`DeclareContext::register`], in
    /// the order of their registration, which is empty if there are none.
    ///
    /// The returned list does not borrow from this context and can be kept in the injectable.
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

    /// Returns a reference to `To`, which will be enabled once the build has finished.
    ///
    /// # Panics
    ///
    /// Panics if `To` has not been added during the declare step.
    pub(crate) fn get_ref<To: Injectable>(&mut self) -> Ref<To> {
        let to_index = self.state.dependency_graph.get_index::<To>();
        self.state
            .dependency_graph
            .add_dependency(self.node_index, to_index, DependencyType::Soft);
        Ref::new()
    }

    /// Returns a reference to the injectable `To` is bound to, which will be enabled once the
    /// build has finished.
    ///
    /// # Panics
    ///
    /// Panics if `To` has not been added as a bound dependency during the declare step.
    pub(crate) fn get_bound_ref<To: ?Sized + Target>(&mut self) -> Ref<To> {
        let Some((reference, to_index)) = self.state.bindings.get_ref_and_index::<To>() else {
            // this is checked at the end of the declare step
            panic!("`{}` should have been bound", type_name::<To>())
        };
        self.state
            .dependency_graph
            .add_dependency(self.node_index, to_index, DependencyType::Soft);
        reference
    }
}

pub(super) struct AssembleStep {
    preparations: Preparations,
    injectables: Injectables,
}

impl BuildState<AssembleStep> {
    pub(super) fn new(state: BuildState<Preparations>) -> Self {
        state.map_step(|preparations, _| AssembleStep {
            injectables: Injectables::with_capacity(preparations.len()),
            preparations,
        })
    }

    pub(super) async fn assemble(mut self) -> Result<Injectables, Error> {
        // from here on no nodes are added to the dependency graph, so we can iterate over the
        // nodes sequentially via indices
        for node_index in 0..self.dependency_graph.node_count() {
            let node_index = NodeIndex::from_usize(node_index);
            let node = self.dependency_graph.get(node_index);
            (node.assemble)(&mut self, node_index).await?;
        }
        assert_eq!(self.step.preparations.len(), 0);
        assert_eq!(
            self.step.injectables.len(),
            self.dependency_graph.node_count()
        );
        self.step.injectables.enable_refs();
        Ok(self.step.injectables)
    }

    pub(super) async fn assemble_one<I: Injectable>(
        &mut self,
        index: NodeIndex,
    ) -> Result<(), Error> {
        let preparation = self.step.preparations.remove::<I>(index);
        let injectable = I::assemble(
            preparation,
            AssembleContext {
                state: self,
                node_index: index,
                _i: PhantomData,
            },
        )
        .await?;
        self.step.injectables.set::<I>(injectable);
        Ok(())
    }
}
