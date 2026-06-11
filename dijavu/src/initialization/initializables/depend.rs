use crate::initializables::{Inject, InjectInit};
use crate::{InitInjector, Initializable, Injectable, InjectorBuilder, NewInitValue};
use std::ops::Deref;

/// Guarantees initialization of an injectable before its initialization.
///
/// Effectively, this is simply a slim wrapper around [`Inject`] that also calls
/// [`InitInjector::get`] for `I` in its [`NewInitValue`] implementation.
/// Should two injectables depend on one another, this will result in a stack overflow and therefore
/// a failing initialization or build phase.
pub struct Depend<I: 'static>(Inject<I>);

pub struct DependInit<I: 'static>(InjectInit<I>);

impl<I> NewInitValue for Depend<I>
where
    I: Injectable,
{
    type Error = I::Error;

    async fn new_init(injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        injector.get::<I>().await?;
        Ok(DependInit(Inject::<I>::new_init(injector).await?))
    }
}

impl<I> Initializable for Depend<I>
where
    I: Injectable,
{
    type Init = DependInit<I>;

    async fn build(init: Self::Init, builder: &mut InjectorBuilder) -> crate::Result<Self> {
        Ok(Self(Inject::<I>::build(init.0, builder).await?))
    }
}

impl<I: 'static> Deref for Depend<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
