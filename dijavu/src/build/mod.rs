mod slot;
pub(crate) use self::slot::Slot;
use self::slot::SlotKey;

#[cfg(doc)]
use crate::Initializable;
use crate::on_start::{DynOnStart, OnStart};
use crate::{Data, InitData, Injectable, InjectableKey, InjectablesData, Injector, Restricted};
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, TryStreamExt};
use std::marker::PhantomData;

/// Manages the state of the build phase.
///
/// `InjectorBuilder` is used during the _build phase_ to assemble the [`Injector`] from
/// initialization data.
/// It provides two main interfaces to be used from [`Injectable::build`] or
/// [`Initializable::build`]:
/// - [`init_data_mut`](Self::init_data_mut) gives access to the final initialization data
/// - [`on_start`](Self::on_start) allows registering hooks that are executed right after the
///   `Injector` was built
pub struct InjectorBuilder {
    init_data: InitData,
    slots: Data,
    injectable_builders: Vec<Box<dyn InjectableBuilder>>,
    on_start: Vec<Box<dyn DynOnStart>>,
}

impl InjectorBuilder {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            init_data: InitData::default(),
            slots: Data::default(),
            injectable_builders: Vec::default(),
            on_start: Vec::default(),
        }
    }

    /// Returns the current initialization data.
    pub fn init_data_mut(&mut self) -> &mut InitData {
        &mut self.init_data
    }

    /// Enqueues an [`Injectable`] to be built and returns a slot pointing to the memory location of
    /// its (future) instance.
    pub(crate) fn enqueue<I: Injectable>(&mut self) -> Slot<I> {
        *self.slots.entry::<SlotKey<I>>().or_insert_with(|| {
            self.injectable_builders
                .push(Box::new(InjectableInitializerImpl::<I>::default()));
            Slot::uninit()
        })
    }

    /// Registers a hook that is executed right after the [`Injector`] was constructed.
    ///
    /// Note that there is no guarantee on the order in which the hooks are executed.
    pub fn on_start(&mut self, on_start: impl OnStart) {
        self.on_start.push(Box::new(on_start));
    }

    /// See [`crate::InitInjector::build`]
    pub(crate) async fn build(mut self) -> crate::Result<Injector> {
        let mut injectables = InjectablesData::default();
        while let Some(initializer) = self.injectable_builders.pop() {
            initializer.initialize(&mut self, &mut injectables).await?;
        }

        let injector = Injector::new(Box::leak(Box::new(injectables)));

        // concurrently run start hooks
        self.on_start
            .into_iter()
            .map(|on_start| on_start.on_start(injector))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<()>()
            .await?;

        Ok(injector)
    }
}

impl Default for InjectorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

trait InjectableBuilder: Send + Sync {
    fn initialize<'a>(
        self: Box<Self>,
        builder: &'a mut InjectorBuilder,
        injectables_data: &'a mut InjectablesData,
    ) -> BoxFuture<'a, crate::Result<()>>;
}

struct InjectableInitializerImpl<I>(PhantomData<I>);

impl<I: Injectable> InjectableBuilder for InjectableInitializerImpl<I> {
    fn initialize<'a>(
        self: Box<Self>,
        builder: &'a mut InjectorBuilder,
        injectables_data: &'a mut InjectablesData,
    ) -> BoxFuture<'a, crate::Result<()>> {
        async move {
            let slot = *builder
                .slots
                .get::<SlotKey<I>>()
                .expect("slot should exist");
            slot.set(I::build(builder, Restricted(())).await?);
            assert!(
                injectables_data
                    .insert::<InjectableKey<I>>(slot.get().expect("slot should be set"))
                    .is_none(),
                "injectable entry already present"
            );
            Ok(())
        }
        .boxed()
    }
}

impl<I> Default for InjectableInitializerImpl<I> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
