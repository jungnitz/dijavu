#[cfg(doc)]
use crate::InitInjectable;
use crate::data::DataItem;
use crate::init::InitAppContainer;
use crate::{AppContainer, AppContainerBuilder, Data, Result};
use dijavu::{DataKey, Error};
use std::marker::PhantomData;
use std::ops::Deref;

/// A value that can be used to build an [`InitInjectable`] type
///
/// `Initializable` defines the behaviour of a field when using the `InitInjectable` macro on a
/// type.
/// In particular, it defines
///
/// - the type of the field in the initialization type and how the initial value is constructed,
/// - the data required for constructing it during runtime and how this data is constructed from the
///   initialization value, and lastly
/// - how the actual value is constructed from the data available at runtime
///
/// ## Implementations
///
/// dijavu provides some simple, but useful implementations of the trait out of the box.
/// See e.g. [`Value`] or [`StartValue`] for examples.
pub trait Initializable: Sized {
    type Error;
    /// The type of the data that is modifiable during initialization
    type Init;
    /// The runtime state
    type Runtime;

    /// Creates the initial value for initialization.
    fn new_init_value(container: &mut InitAppContainer) -> Result<Self::Init, Self::Error>;

    /// Consumes the initialization value and constructs the runtime data.
    fn build_runtime_value(
        init: Self::Init,
        data: &mut Data,
        builder: &mut AppContainerBuilder,
    ) -> Result<Self::Runtime>;

    /// Retrieves the value from the [`AppContainer`] at runtime.
    ///
    /// This must match what was inserted during [`on_build`](Self::on_build).
    fn from_runtime_value(
        runtime: &'static Self::Runtime,
        container: AppContainer,
    ) -> Result<Self, Self::Error>;
}

/// A simple, initializable value
///
/// `Value<T>` is a lightweight [`Initializable`] implementation that allows modifying the `T`
/// during initialization and accessing (a reference to) the final value at runtime.
/// The initial value is the default of `T`.
#[derive(Debug)]
pub struct Value<T: 'static>(&'static T);

impl<T> Initializable for Value<T>
where
    T: DataItem + Default,
{
    type Error = Error;
    type Init = T;
    type Runtime = T;

    fn new_init_value(_container: &mut InitAppContainer) -> Result<Self::Init, Self::Error> {
        Ok(T::default())
    }

    fn build_runtime_value(
        init: Self::Init,
        _data: &mut Data,
        _builder: &mut AppContainerBuilder,
    ) -> Result<T> {
        Ok(init)
    }

    fn from_runtime_value(runtime: &'static T, _: AppContainer) -> Result<Self, Self::Error> {
        Ok(Self(runtime))
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
/// `StartValue<T>` allows modifying the `T` during initialization.
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
    type Runtime = ();

    fn new_init_value(_container: &mut InitAppContainer) -> Result<Self::Init, Self::Error> {
        Ok(T::default())
    }

    fn build_runtime_value(
        init: Self::Init,
        _data: &mut Data,
        builder: &mut AppContainerBuilder,
    ) -> Result<()> {
        builder.insert_start_data::<StartValueKey<T>>(init)?;
        Ok(())
    }

    fn from_runtime_value(
        _runtime: &'static (),
        _container: AppContainer,
    ) -> Result<Self, Self::Error> {
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
