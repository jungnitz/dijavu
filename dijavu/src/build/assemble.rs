use std::any::type_name;

use crate::{
    Error, Injectable, Target,
    build::{
        BuildState, Registered, RegisteredValues,
        dependency_graph::{DependencyType, NodeIndex},
        injectable_context,
        prepare::Preparations,
    },
    error::MissingInputError,
    fields::Input,
    refs::Injectables,
    refs::Ref,
};

injectable_context!(AssembleContext, FieldAssembleContext, "assemble");

/// Gives fields access to the build during the assemble step.
pub struct FieldAssembleContext<'a> {
    state: &'a mut BuildState<AssembleStep>,
    node_index: NodeIndex,
}

impl<'ctx> FieldAssembleContext<'ctx> {
    /// Returns an owned context borrowing from this one, e.g. to pass it to multiple fields.
    pub fn reborrow(&mut self) -> FieldAssembleContext<'_> {
        FieldAssembleContext {
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
            AssembleContext::new(FieldAssembleContext {
                state: self,
                node_index: index,
            }),
        )
        .await?;
        self.step.injectables.set::<I>(injectable);
        Ok(())
    }
}
