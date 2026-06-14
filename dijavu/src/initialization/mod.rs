use crate::build::Slot;
use crate::{InitData, Injectable, Injector, InjectorBuilder, OnStart, Result, hooks};
use futures::FutureExt;
use futures::future::BoxFuture;
use rustc_hash::FxHashMap;
use std::any::TypeId;
use std::marker::PhantomData;
use std::mem::take;

pub mod initializables;

mod initializable;
pub use initializable::{Initializable, NewInitValue};

mod injectable_init;
pub use injectable_init::InjectableInit;

/// Mutable container used during initialization
///
/// `InitInjector` contains the _initialization state_ of the application.
/// It provides mutable access this state, including structured access to the initialization state
/// of injectables.
///
/// ## Data access
///
/// You can access the underlying [`InitData`] directly using [`data_mut`](Self::data_mut).
/// This is typically only necessary if you implement a bespoke custom [`Injectable`].
/// In most cases, you should use [`get`](Self::get) for structured access to the initialization state
/// of other injectables:
///
/// ```
/// # use dijavu::{InitData, InitInjector};
/// # type Config = ();
/// # async fn f() -> dijavu::Result<()> {
/// # let mut init_injector = InitInjector::default();
/// let config = init_injector.get::<Config>().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct InitInjector {
    builder: InjectorBuilder,
    initializers: FxHashMap<TypeId, Box<dyn InjectableInitializer>>,
}

impl InitInjector {
    /// Returns mutable access to the underlying [`InitData`].
    ///
    /// This is primarily intended for low-level access in [`Injectable`] implementations or to
    /// insert external data at application startup.
    pub fn data_mut(&mut self) -> &mut InitData {
        self.builder.init_data_mut()
    }

    /// Returns access to the underlying [`InitData`].
    ///
    /// This is primarily intended for low-level access in [`Injectable`] implementations or to
    /// insert external data at application startup.
    pub fn data(&self) -> &InitData {
        self.builder.init_data()
    }

    /// Retrieves the initialization state of an [`Injectable`] from the container.
    /// This is the preferred way to access initialization data.
    ///
    /// # Errors
    ///
    /// This method performs a call to [`Injectable::new_init_data`] on `I` and returns any
    /// potential error of that call.
    pub async fn get<I>(&mut self) -> Result<I::Init<'_>, I::Error>
    where
        I: Injectable,
    {
        let init = InjectableInit::<I>::init(self).await?;
        Ok(I::new_init(init))
    }

    fn get_slot<I>(&mut self) -> Slot<I>
    where
        I: Injectable,
    {
        self.initializers
            .entry(TypeId::of::<I>())
            .or_insert_with(|| Box::new(InjectableInitializerImpl::<I>(PhantomData)));
        self.enqueue_assert_initialization::<I>()
    }

    /// Enqueue an injectable for building under the assumption that is has already been or will be
    /// initialized before the build phase.
    fn enqueue_assert_initialization<I>(&mut self) -> Slot<I>
    where
        I: Injectable,
    {
        self.builder.enqueue::<I>()
    }

    /// Registers a hook that is executed right after the [`Injector`] was constructed.
    ///
    /// Note that there is no guarantee on the order in which the hooks are executed.
    pub fn on_start(&mut self, on_start: impl OnStart) {
        self.builder.on_start(on_start);
    }

    /// Constructs an [`Injector`] from the current initialization state.
    ///
    /// This entails the following steps:
    /// - All global before build hooks are run
    /// - All [`Injectable`] types that were used during initialization are built, including those
    ///   that are used throughout the build process
    /// - The [`Injector`] is constructed from the injectables
    /// - The start hooks are executed
    ///
    /// # Errors
    ///
    /// This method returns an error if one occurs during any of the aforementioned steps.
    pub async fn build(mut self) -> Result<Injector> {
        hooks::run_global_before_build_hooks(&mut self).await?;
        while !self.initializers.is_empty() {
            for (_, initializer) in take(&mut self.initializers) {
                initializer.initialize(&mut self).await?;
            }
        }
        self.builder.build().await
    }
}

trait InjectableInitializer: Send + Sync + 'static {
    fn initialize<'a>(&'a self, injector: &'a mut InitInjector) -> BoxFuture<'a, Result<()>>;
}

struct InjectableInitializerImpl<I>(PhantomData<fn(I)>);

impl<I> InjectableInitializer for InjectableInitializerImpl<I>
where
    I: Injectable,
{
    fn initialize<'a>(&'a self, injector: &'a mut InitInjector) -> BoxFuture<'a, Result<()>> {
        async move {
            injector.get::<I>().await.map_err(Into::into)?;
            Ok(())
        }
        .boxed()
    }
}
