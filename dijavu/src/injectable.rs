#[cfg(doc)]
use crate::AppContainer;
use crate::container::ScopeContainer;
use crate::{Error, InitAppContainer, RuntimeData};
use std::{convert::Infallible, marker::PhantomData};

/// A type that can be constructed from an [`AppContainer`]
///
/// `Injectable` defines how a type is retrieved or built from the runtime container.
/// This enables composable, type-safe dependency injection.
pub trait Injectable: Sized + 'static {
    /// The error type returned if injection fails.
    type Error: Into<Error>;
    /// Initialization state of this type
    ///
    /// This type may borrow mutably from a [`InitAppContainer`] with lifetime `'a`.
    type Init<'a>;

    /// Retrieves or constructs the initialization value from the container.
    fn get_init(container: &mut InitAppContainer) -> Result<Self::Init<'_>, Self::Error>;

    /// Retrieves or constructs `Self` from the [`RuntimeData`].
    fn get(data: &RuntimeData) -> Result<Self, Self::Error>;
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
        I::get(container.app().data())
    }
}

impl Injectable for () {
    type Error = Infallible;
    type Init<'a> = ();

    fn get_init(_container: &mut InitAppContainer) -> Result<Self::Init<'_>, Self::Error> {
        Ok(())
    }

    fn get(_data: &RuntimeData) -> Result<Self, Self::Error> {
        Ok(())
    }
}

impl<T: 'static> Injectable for PhantomData<T> {
    type Error = Infallible;
    type Init<'a> = ();

    fn get_init(_container: &mut InitAppContainer) -> Result<Self::Init<'_>, Self::Error> {
        Ok(())
    }

    fn get(_data: &RuntimeData) -> Result<Self, Self::Error> {
        Ok(PhantomData)
    }
}
