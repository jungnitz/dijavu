use std::{any::type_name, borrow::Cow};

use crate::{
    Error, Injectable, Target, UnsizeFrom,
    build::{
        BuildState, Registered, RegisteredValues,
        dependency_graph::{DependencyType, NodeIndex},
        injectable_context,
        prepare::{Preparations, Prepared},
        start::Starts,
        value_per_injectable::ValuePerInjectable,
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

    /// Returns the name of the injectable being assembled, e.g. for errors.
    pub(crate) fn injectable_name(&self) -> &Cow<'static, str> {
        self.state.dependency_graph.get(self.node_index).name()
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

    /// Returns a reference to the injectable `To`, referenced as `B` (e.g. a trait object), which
    /// will be enabled once the build has finished.
    ///
    /// # Panics
    ///
    /// Panics if `To` has not been added during the declare step.
    pub(crate) fn get_ref_as<B: ?Sized + UnsizeFrom<To>, To: Injectable>(&mut self) -> Ref<B> {
        let to_index = self.state.dependency_graph.get_index::<To>();
        self.state
            .dependency_graph
            .add_dependency(self.node_index, to_index, DependencyType::Soft);
        Ref::unsized_from::<To>()
    }

    /// Returns references to the providers of `D`, in the order in which they were added to the
    /// build.
    pub(crate) fn get_provided_refs<D: ?Sized + Send + Sync + 'static>(&mut self) -> Vec<Ref<D>> {
        let refs: Vec<_> = self.state.providers.get_refs_and_indices::<D>().collect();
        refs.into_iter()
            .map(|(reference, to_index)| {
                self.state.dependency_graph.add_dependency(
                    self.node_index,
                    to_index,
                    DependencyType::Soft,
                );
                reference
            })
            .collect()
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
    /// The `Start` of each injectable
    starts: ValuePerInjectable,
    /// The nodes in dependency order
    order: Vec<NodeIndex>,
}

impl BuildState<AssembleStep> {
    pub(super) fn new(state: BuildState<Prepared>) -> Self {
        state.map_step(|prepared, _| AssembleStep {
            injectables: Injectables::with_capacity(prepared.preparations.len()),
            starts: ValuePerInjectable::with_capacity(prepared.preparations.len()),
            preparations: prepared.preparations,
            order: prepared.order,
        })
    }

    /// Assembles all injectables and returns them, along with what is needed to start them.
    pub(super) async fn assemble(mut self) -> Result<(Injectables, Starts), Error> {
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
        let order = self
            .step
            .order
            .iter()
            .map(|&node_index| (node_index, self.dependency_graph.get(node_index).start))
            .collect();
        let starts = Starts {
            order,
            values: self.step.starts,
        };
        Ok((self.step.injectables, starts))
    }

    pub(super) async fn assemble_one<I: Injectable>(
        &mut self,
        index: NodeIndex,
    ) -> Result<(), Error> {
        let preparation = self.step.preparations.remove::<I>(index);
        let (injectable, start) = I::assemble(
            preparation,
            AssembleContext::new(FieldAssembleContext {
                state: self,
                node_index: index,
            }),
        )
        .await?;
        self.step.injectables.set::<I>(injectable);
        self.step.starts.insert::<I::Start>(index, start);
        Ok(())
    }
}
