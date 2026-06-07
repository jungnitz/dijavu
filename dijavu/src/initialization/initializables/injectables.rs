use crate::initializables::{Initializables, InitializablesInit, InitializablesIter, Inject};
use crate::{InitInjector, Initializable, Injectable, InjectorBuilder, NewInitValue};
use std::convert::Infallible;

/// Collection of injectable types
///
/// This is essentially a convenient wrapper around [`Initializables<T>`](Initializables) with the
/// only difference being that it collects injectables and not initializables, which is less
/// powerful, but vastly more convenient.
/// See the documentation of [`Initializables`] for more information on how this type is intended to
/// be used.
pub struct Injectables<T>(Initializables<InitializableInjectableWrapper<T>>);

impl<T> Injectables<T> {
    pub fn iter(&self) -> InjectablesIter<'_, T> {
        self.into_iter()
    }
}

pub struct InjectablesInit<T>(InitializablesInit<InitializableInjectableWrapper<T>>);

struct InitializableInjectableWrapper<T>(T);

impl<T, I> From<Inject<I>> for InitializableInjectableWrapper<T>
where
    I: Injectable,
    &'static I: Into<T>,
{
    fn from(value: Inject<I>) -> Self {
        Self(value.to_static_ref().into())
    }
}

impl<T> InjectablesInit<T> {
    pub fn add<I>(&mut self)
    where
        I: Injectable,
        &'static I: Into<T>,
    {
        self.0.add::<Inject<I>>(());
    }
}

impl<T> Initializable for Injectables<T>
where
    T: Send + 'static,
{
    type Init = InjectablesInit<T>;

    async fn build(init: Self::Init, builder: &mut InjectorBuilder) -> crate::Result<Self> {
        Ok(Self(Initializables::build(init.0, builder).await?))
    }
}

impl<T> NewInitValue for Injectables<T>
where
    T: Send + 'static,
{
    type Error = Infallible;

    async fn new_init(injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(InjectablesInit(
            Initializables::<InitializableInjectableWrapper<T>>::new_init(injector).await?,
        ))
    }
}

/// Iterator over the items stored in [`Injectables`]
pub struct InjectablesIter<'a, T>(InitializablesIter<'a, InitializableInjectableWrapper<T>>);

impl<'a, T> Iterator for InjectablesIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        Some(&self.0.next()?.0)
    }
}

impl<'a, T> IntoIterator for &'a Injectables<T> {
    type Item = &'a T;
    type IntoIter = InjectablesIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        InjectablesIter(self.0.iter())
    }
}
