use crate::data::DataItem;
use crate::init::InitAppContainer;
use crate::{AppContainer, AppContainerBuilder, Data, Result};
use dijavu::{DataKey, Error};
use std::marker::PhantomData;
use std::ops::Deref;

/// A value that participates in the initialization → build → runtime lifecycle
///
/// `Initializable` abstracts how a value:
///
/// - is created during initialization
/// - is moved into the final [`AppContainer`]
/// - is retrieved at runtime
///
/// It is primarily used by derive macros to implement field-level initialization without requiring
/// any verbose manual definitions.
///
/// ## Implementations
///
/// Currently, there is one builtin implementation:
/// [`Value`], which initially generates the default value of its generic parameter on
/// initialization, which can then be modified. During runtime, a reference to the final value
/// is provided.
///
/// ## Namespacing
///
/// The generic parameter `U` on methods is used to namespace stored values.
/// Typically, you will want to use it as a generic parameter to a [`DataKey`] type, which avoids
/// key collisions when the same `Initializable` type is used in multiple contexts.
pub trait Initializable: Sized {
    type Error;
    /// The intermediate initialization value.
    ///
    /// This is created during initialization and consumed during build.
    type Init;

    /// Creates the initial value during initialization.
    ///
    /// This is typically called lazily when the value is first requested.
    fn new_init<U: 'static>(container: &mut InitAppContainer) -> Result<Self::Init, Self::Error>;

    /// Consumes the initialization value and inserts the runtime representation.
    ///
    /// Implementations should most likely insert data into the [`AppContainerBuilder`].
    ///
    /// The type parameter `U` is used to namespace stored values.
    fn on_build<U: 'static>(
        init: Self::Init,
        data: &mut Data,
        builder: &mut AppContainerBuilder,
    ) -> Result<()>;

    /// Retrieves the value from the [`AppContainer`] at runtime.
    ///
    /// This must match what was inserted during [`on_build`](Self::on_build).
    fn get<U: 'static>(container: AppContainer) -> Result<Self, Self::Error>;
}

/// A simple, initializable value
///
/// `Value<T>` is a lightweight [`Initializable`] implementation that allows modifying the `T`
/// during initialization.
/// The initial value is the default of `T`.
/// During the build phase, the `T` is then stored in the application data and referenced during
/// runtime.
/// If the value was not initialized, an error is returned when attempting to retrieve the value
/// during runtime.
#[derive(Debug)]
pub struct Value<T: 'static>(&'static T);

struct ValueKey<U, T>(U, T);

impl<U: 'static, T: DataItem + 'static> DataKey for ValueKey<U, T> {
    type Item = T;
}

impl<T> Initializable for Value<T>
where
    T: DataItem + Default,
{
    type Error = Error;
    type Init = T;

    fn new_init<U: 'static>(_container: &mut InitAppContainer) -> Result<Self::Init, Self::Error> {
        Ok(T::default())
    }

    fn on_build<U: 'static>(
        init: Self::Init,
        _data: &mut Data,
        builder: &mut AppContainerBuilder,
    ) -> Result<()> {
        builder.insert_app_data::<ValueKey<U, T>>(init)?;
        Ok(())
    }

    fn get<U: 'static>(container: AppContainer) -> Result<Self, Self::Error> {
        Ok(Self(
            container
                .data()
                .get::<ValueKey<U, T>>()
                .ok_or_else(|| Error::msg("uninitialized"))?,
        ))
    }
}

impl<T: 'static> Deref for Value<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> Clone for Value<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Value<T> {}

/// A value that is stored in the start data after initialization
///
/// `StartValue<T, D>` allows modifying the `T` during initialization.
/// The initial value is the default of `T`.
/// During the build phase, the `T` is then stored in the start data and can be retrieved from it
/// via [`remove_from_start_data`](Self::remove_from_start_data).
///
/// Note that in contrast to some other `Initializable`s, you must only ever initialize a single
/// `StartValue` for any given `T`.
/// Otherwise, the build phase will fail because of duplicate keys in the start data.
/// If you fear that this may happen, wrap your `T` in a local new-type struct to prevent others
/// from using it.
pub struct StartValue<T>(PhantomData<T>);

impl<T> Initializable for StartValue<T>
where
    T: DataItem + Default,
{
    type Error = Error;
    type Init = T;

    fn new_init<U: 'static>(_container: &mut InitAppContainer) -> Result<Self::Init, Self::Error> {
        Ok(T::default())
    }

    fn on_build<U: 'static>(
        init: Self::Init,
        _data: &mut Data,
        builder: &mut AppContainerBuilder,
    ) -> Result<()> {
        builder.insert_start_data::<StartValueKey<T>>(init)?;
        Ok(())
    }

    fn get<U: 'static>(_container: AppContainer) -> Result<Self, Self::Error> {
        Ok(Self(PhantomData))
    }
}

impl<T: DataItem> StartValue<T> {
    /// Removes the instance of `T` added to the start data by `StartValue<T>`.
    pub fn remove_from_start_data(start_data: &mut Data) -> Option<T> {
        start_data.remove::<StartValueKey<T>>()
    }
}

struct StartValueKey<T>(PhantomData<T>);

impl<T: DataItem> DataKey for StartValueKey<T> {
    type Item = T;
}

impl<T> Clone for StartValue<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for StartValue<T> {}
