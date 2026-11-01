mod assemble;
mod bindings;
mod declare;
mod dependency_graph;
mod prepare;
mod registry;
mod value_per_injectable;

pub use self::assemble::AssembleContext;
pub use self::declare::DeclareContext;
pub(crate) use self::dependency_graph::DependencyType;
pub use self::prepare::{PrepareContext, PreparingHandle};
pub use self::registry::{Registered, RegisteredValues};

use crate::{
    Bindable, Error, Injectable, Injector,
    build::{
        assemble::AssembleStep, bindings::Bindings, declare::DeclareStep,
        dependency_graph::DependencyGraph, prepare::PrepareStep, registry::Registry,
    },
    error::BindingConflictError,
};

/// Collects the injectables and bindings that an [`Injector`] is built from.
#[derive(Default)]
pub struct Builder {
    state: BuildState<DeclareStep>,
}

impl Builder {
    /// Creates a builder without any injectables.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the injectable `I`, which also adds everything it references.
    pub fn add<I: Injectable>(&mut self) -> &mut Self {
        self.state.add::<I>();
        self
    }

    /// Binds `B` (e.g. a trait object) to the injectable `I`, so that `Inject<B>` and `Depend<B>`
    /// reference `I`.
    ///
    /// Fails if `B` is already bound to another injectable.
    pub fn bind<B: ?Sized + Bindable<I>, I: Injectable>(
        &mut self,
    ) -> Result<&mut Self, BindingConflictError> {
        self.state.bind::<B, I>()?;
        Ok(self)
    }

    /// Runs all build steps and returns the resulting [`Injector`].
    pub async fn build(self) -> Result<Injector, Error> {
        let state = self.state.declare()?;
        let state = BuildState::<PrepareStep>::new(state);
        let state = state.prepare().await?;
        let state = BuildState::<AssembleStep>::new(state);
        let injectables = state.assemble().await?;
        Ok(Injector::new(injectables))
    }
}

#[derive(Default)]
pub struct BuildState<Step> {
    dependency_graph: DependencyGraph,
    bindings: Bindings,
    registry: Registry,
    step: Step,
}

impl<Step> BuildState<Step> {
    fn with_step<NewStep>(self, new_step: NewStep) -> (BuildState<NewStep>, Step) {
        (
            BuildState {
                dependency_graph: self.dependency_graph,
                bindings: self.bindings,
                registry: self.registry,
                step: new_step,
            },
            self.step,
        )
    }

    fn map_step<NewStep>(
        self,
        map: impl FnOnce(Step, &mut BuildState<()>) -> NewStep,
    ) -> BuildState<NewStep> {
        let (mut state, step) = self.with_step(());
        let step = map(step, &mut state);
        state.with_step(step).0
    }
}
