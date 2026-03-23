use crate::container::{AppContainer, ScopeContainer};
use std::convert::Infallible;

/// A type that can be constructed from an [`AppContainer`]
///
/// `Injectable` defines how a type is retrieved or built from the runtime container.
/// This enables composable, type-safe dependency injection.
pub trait Injectable: Sized + 'static {
    /// The error type returned if injection fails.
    type Error;

    /// Retrieves or constructs `Self` from the [`AppContainer`].
    fn get(container: AppContainer) -> Result<Self, Self::Error>;
}

/// A type that can be constructed from a [`ScopeContainer`]
///
/// `ScopeInjectable` is the scoped counterpart to [`Injectable`].
/// It allows constructing values using global application data (via [`AppContainer`]) and mutable,
/// scope-local data
pub trait ScopeInjectable<'a>: Sized {
    type Error;

    fn get(container: &'a mut ScopeContainer) -> Result<Self, Self::Error>;
}

impl<'a, I> ScopeInjectable<'a> for I
where
    I: Injectable,
{
    type Error = I::Error;

    fn get(container: &'a mut ScopeContainer) -> Result<Self, Self::Error> {
        I::get(container.app())
    }
}

impl Injectable for () {
    type Error = Infallible;

    fn get(_container: AppContainer) -> Result<Self, Self::Error> {
        Ok(())
    }
}
