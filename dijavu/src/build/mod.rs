mod assemble;
mod bindings;
mod declare;
mod dependency_graph;
mod inputs;
mod prepare;
mod registry;
mod start;
mod value_per_injectable;

pub use self::assemble::{AssembleContext, FieldAssembleContext};
pub use self::declare::{DeclareContext, FieldDeclareContext};
pub(crate) use self::dependency_graph::DependencyType;
pub use self::prepare::{FieldPrepareContext, PrepareContext, PreparingHandle};
pub use self::registry::{Registered, RegisteredValues};
pub use self::start::StartContext;

use crate::{
    Bindable, Error, Injectable, Injector,
    build::{
        assemble::AssembleStep, bindings::Bindings, declare::DeclareStep,
        dependency_graph::DependencyGraph, inputs::Inputs, prepare::PrepareStep,
        registry::Registry,
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

    /// Sets the value that [`Input<T>`](crate::fields::Input) provides, replacing any previous
    /// value of the same type.
    pub fn set_input<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.state.inputs.set(value);
        self
    }

    /// Runs all build steps, then starts the injectables (see [`Injectable::start`]) and returns
    /// the resulting [`Injector`].
    pub async fn build(self) -> Result<Injector, Error> {
        let state = self.state.declare()?;
        let state = BuildState::<PrepareStep>::new(state);
        let state = state.prepare().await?;
        let state = BuildState::<AssembleStep>::new(state);
        let (injectables, starts) = state.assemble().await?;
        let injector = Injector::new(injectables);
        starts.start(&injector).await?;
        Ok(injector)
    }
}

#[derive(Default)]
pub struct BuildState<Step> {
    dependency_graph: DependencyGraph,
    bindings: Bindings,
    registry: Registry,
    inputs: Inputs,
    step: Step,
}

impl<Step> BuildState<Step> {
    fn with_step<NewStep>(self, new_step: NewStep) -> (BuildState<NewStep>, Step) {
        (
            BuildState {
                dependency_graph: self.dependency_graph,
                bindings: self.bindings,
                registry: self.registry,
                inputs: self.inputs,
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

/// Defines the context that gives an injectable access to the build during one build step, based on
/// the context that its fields receive during that step.
macro_rules! injectable_context {
    ($name:ident, $field:ident, $step:literal) => {
        #[doc = concat!(
            "Gives the injectable `I` access to the build during the ", $step, " step.\n\n",
            "Everything provided by [`", stringify!($field), "`] is available through deref. There ",
            "is no public way to create this type."
        )]
        pub struct $name<'ctx, I: crate::Injectable> {
            field: $field<'ctx>,
            injectable: std::marker::PhantomData<fn() -> I>,
        }

        impl<'ctx, I: crate::Injectable> $name<'ctx, I> {
            pub(super) fn new(field: $field<'ctx>) -> Self {
                Self {
                    field,
                    injectable: std::marker::PhantomData,
                }
            }

            /// Returns a token for the injectable `I` itself.
            pub fn inject_token(&self) -> crate::fields::InjectToken<I> {
                crate::fields::InjectToken::new()
            }

            /// Returns an owned context borrowing from this one, e.g. to pass it to a hook.
            pub fn reborrow(&mut self) -> $name<'_, I> {
                $name::new(self.field.reborrow())
            }

            /// Returns the context for calling the functions of a field.
            pub fn field(&mut self) -> $field<'_> {
                self.field.reborrow()
            }
        }

        impl<'ctx, I: crate::Injectable> std::ops::Deref for $name<'ctx, I> {
            type Target = $field<'ctx>;

            fn deref(&self) -> &Self::Target {
                &self.field
            }
        }

        impl<'ctx, I: crate::Injectable> std::ops::DerefMut for $name<'ctx, I> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.field
            }
        }
    };
}
use injectable_context;
