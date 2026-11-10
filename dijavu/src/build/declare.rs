use std::any::{TypeId, type_name};

use crate::{
    Bindable, Error, Injectable, Target, UnsizeFrom,
    build::{
        BuildState, Registered, dependency_graph::NodeIndex, injectable_context,
        value_per_injectable::ValuePerInjectable,
    },
    error::{BindingConflictError, MissingInputError, UnboundError},
    fields::{DependToken, InjectToken, Input},
};

use super::dependency_graph::DependencyType;

injectable_context!(DeclareContext, FieldDeclareContext, "declare");

/// Gives fields access to the build during the declare step.
pub struct FieldDeclareContext<'a> {
    state: &'a mut BuildState<DeclareStep>,
    node_index: NodeIndex,
}

impl<'a> FieldDeclareContext<'a> {
    /// Returns an owned context borrowing from this one, e.g. to pass it to multiple fields.
    pub fn reborrow(&mut self) -> FieldDeclareContext<'_> {
        FieldDeclareContext {
            state: self.state,
            node_index: self.node_index,
        }
    }

    pub(crate) fn add<Other: Injectable>(&mut self) {
        self.state.get_or_add::<Other>();
    }

    pub(crate) fn add_hard_dependency<To: Injectable>(&mut self) {
        let to_index = self.state.get_or_add::<To>();
        self.state
            .dependency_graph
            .add_dependency(self.node_index, to_index, DependencyType::Hard);
    }

    /// Adds a dependency on the injectable that `To` is bound to.
    pub(crate) fn add_bound_dependency<To: ?Sized + Target>(&mut self, typ: DependencyType) {
        self.state.step.bound_dependencies.push(BoundDependency {
            from_idx: self.node_index,
            to_type_id: TypeId::of::<To>(),
            to_name: type_name::<To>(),
            typ,
        });
    }

    /// Binds `B` (e.g. a trait object) to the injectable `Impl`, so that `Inject<B>` and
    /// `Depend<B>` reference `Impl`.
    ///
    /// Fails if `B` is already bound to another injectable.
    pub fn bind<B: ?Sized + Bindable<Impl>, Impl: Injectable>(
        &mut self,
    ) -> Result<(), BindingConflictError> {
        self.state.bind::<B, Impl>()
    }

    /// Adds the injectable `Impl` as a provider of `D` (e.g. a trait object), so that every
    /// [`InjectAll<D>`](crate::fields::InjectAll) references it.
    ///
    /// Unlike [`bind`](Self::bind), any number of injectables can provide the same `D`. Providing
    /// adds `Impl` to the build, and providing the same `D` multiple times from the same `Impl` has
    /// no effect.
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
    /// struct Server {
    ///     handlers: InjectAll<dyn Handler>,
    /// }
    ///
    /// let mut builder = Builder::new();
    /// builder.add::<Server>().add::<Logging>();
    /// let injector = futures::executor::block_on(builder.build()).unwrap();
    /// let names: Vec<_> = injector.get::<Server>().handlers.iter().map(|h| h.name()).collect();
    /// assert_eq!(names, ["logging"]);
    /// ```
    pub fn provide<D: ?Sized + UnsizeFrom<Impl>, Impl: Injectable>(&mut self) {
        self.state.provide::<D, Impl>();
    }

    /// Adds `T` to the build, like an [`Inject`](crate::fields::Inject) field for it.
    ///
    /// `T` may be an injectable or a bound target (see [`Target`]).
    pub fn inject<T: ?Sized + Target>(&mut self) -> InjectToken<T> {
        T::declare(self, DependencyType::Soft);
        InjectToken::new()
    }

    /// Adds `T` to the build and depends on it, like a [`Depend`](crate::fields::Depend) field for
    /// it.
    ///
    /// `T` may be an injectable or a bound target (see [`Target`]).
    pub fn depend<T: ?Sized + Target>(&mut self) -> DependToken<T> {
        T::declare(self, DependencyType::Hard);
        DependToken::new()
    }

    /// Registers a value that can be read via [`FieldPrepareContext::registered`] and
    /// [`FieldAssembleContext::registered`], by any injectable.
    ///
    /// Values are grouped by their type `R`, which has to implement [`Registered`]. To register
    /// different types implementing a trait, use a trait object as `R`, for example
    /// `Box<dyn Plugin>`.
    ///
    /// This also adds [`Registered::Consumer`] to the build.
    ///
    /// Registered values cannot be read while declaring, as not all of them are known yet. The
    /// injectable registering a value still has to be part of the build, so it must be referenced
    /// or added to the [`Builder`](crate::Builder).
    ///
    /// [`FieldPrepareContext::registered`]: crate::FieldPrepareContext::registered
    /// [`FieldAssembleContext::registered`]: crate::FieldAssembleContext::registered
    ///
    /// # Example
    ///
    /// ```
    /// use dijavu::{*, fields::Value, state::Declare};
    ///
    /// pub trait Plugin: Send + Sync {
    ///     fn name(&self) -> &str;
    /// }
    ///
    /// impl Registered for Box<dyn Plugin> {
    ///     type Consumer = PluginNames;
    /// }
    ///
    /// struct Logging;
    ///
    /// impl Plugin for Logging {
    ///     fn name(&self) -> &str {
    ///         "logging"
    ///     }
    /// }
    ///
    /// #[derive(Injectable)]
    /// #[injectable(declare = Self::add_plugin)]
    /// struct LoggingModule;
    ///
    /// impl LoggingModule {
    ///     fn add_plugin(
    ///         _state: &mut LoggingModuleState<Declare>,
    ///         mut ctx: DeclareContext<'_, Self>,
    ///     ) -> Result<(), Error> {
    ///         ctx.register::<Box<dyn Plugin>>(Box::new(Logging));
    ///         Ok(())
    ///     }
    /// }
    ///
    /// #[derive(Injectable)]
    /// #[injectable(prepare = Self::collect_names)]
    /// pub struct PluginNames {
    ///     names: Value<Vec<String>>,
    /// }
    ///
    /// impl PluginNames {
    ///     async fn collect_names(
    ///         state: &mut PluginNamesState<Declare>,
    ///         ctx: PrepareContext<'_, Self>,
    ///     ) -> Result<(), Error> {
    ///         let plugins = ctx.registered::<Box<dyn Plugin>>();
    ///         state.names = plugins.iter().map(|plugin| plugin.name().to_owned()).collect();
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut builder = Builder::new();
    /// builder.add::<LoggingModule>();
    /// let injector = futures::executor::block_on(builder.build()).unwrap();
    /// assert_eq!(*injector.get::<PluginNames>().names, ["logging"]);
    /// ```
    pub fn register<R: Registered>(&mut self, value: R) {
        self.add::<R::Consumer>();
        self.state.registry.register(value);
    }

    /// Returns the value set via [`Builder::set_input`](crate::Builder::set_input).
    ///
    /// Fails if the input has not been set.
    pub fn input<T: Send + Sync + 'static>(&self) -> Result<Input<T>, MissingInputError> {
        self.state
            .inputs
            .get(self.state.dependency_graph.get(self.node_index))
    }
}

type DeclareOneFn = dyn FnOnce(&mut BuildState<DeclareStep>) -> Result<(), Error> + Send;

#[derive(Default)]
pub(super) struct DeclareStep {
    pending: Vec<Box<DeclareOneFn>>,
    declarations: Declarations,
    bound_dependencies: Vec<BoundDependency>,
}

/// A dependency on a bound [`Target`].
struct BoundDependency {
    from_idx: NodeIndex,
    to_type_id: TypeId,
    to_name: &'static str,
    typ: DependencyType,
}

impl BuildState<DeclareStep> {
    fn get_or_add<I: Injectable>(&mut self) -> NodeIndex {
        let (node_index, created) = self.dependency_graph.get_or_add::<I>();
        if created {
            self.step
                .pending
                .push(Box::new(move |state| state.declare_one::<I>(node_index)));
        }
        node_index
    }

    pub(super) fn add<I: Injectable>(&mut self) {
        self.get_or_add::<I>();
    }

    pub(super) fn provide<D: ?Sized + UnsizeFrom<I>, I: Injectable>(&mut self) {
        let node_index = self.get_or_add::<I>();
        self.providers.provide::<D, I>(node_index);
    }

    pub(super) fn bind<B: ?Sized + Bindable<I>, I: Injectable>(
        &mut self,
    ) -> Result<(), BindingConflictError> {
        // check first, a rejected injectable must not be added
        self.bindings.check_conflict::<B, I>()?;
        let node_index = self.get_or_add::<I>();
        self.bindings.bind::<B, I>(node_index);
        Ok(())
    }

    pub(super) fn declare(mut self) -> Result<BuildState<Declarations>, Error> {
        while let Some(pending) = self.step.pending.pop() {
            pending(&mut self)?;
        }
        // there should exist a declaration for every injectable
        assert_eq!(
            self.dependency_graph.node_count(),
            self.step.declarations.len()
        );
        // all bindings exist now, so create the necessary hard dependencies
        for dependency in &self.step.bound_dependencies {
            let Some(to_index) = self.bindings.get_index_by_type_id(dependency.to_type_id) else {
                let from_name = self
                    .dependency_graph
                    .get(dependency.from_idx)
                    .name()
                    .clone();
                return Err(UnboundError::new(dependency.to_name, from_name).into());
            };
            // soft dependencies are added when assembling, like for injectables
            if let DependencyType::Hard = dependency.typ {
                self.dependency_graph.add_dependency(
                    dependency.from_idx,
                    to_index,
                    DependencyType::Hard,
                );
            }
        }
        Ok(self.map_step(|step, _| step.declarations))
    }

    fn declare_one<I: Injectable>(&mut self, node_index: NodeIndex) -> Result<(), Error> {
        let declaration = I::declare(DeclareContext::new(FieldDeclareContext {
            state: self,
            node_index,
        }))?;
        self.step.declarations.insert::<I>(node_index, declaration);
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Declarations(ValuePerInjectable);

impl Declarations {
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn insert<T: Injectable>(&mut self, index: NodeIndex, declaration: T::Declaration) {
        self.0.insert::<T::Declaration>(index, declaration);
    }

    pub(super) fn remove<T: Injectable>(&mut self, index: NodeIndex) -> T::Declaration {
        self.0.remove::<T::Declaration>(index)
    }
}
