use crate::data::DataValue;
use crate::{InitInjector, Initializable, InjectorBuilder, NewInitValue};
use std::convert::Infallible;
use std::ops::Deref;

/// A simple, initializable value
///
/// `Value<T>` is a slim wrapper around `T` that allows modifying the `T` during initialization and
/// accessing a reference to the final value at runtime.
/// It implements [`NewInitValue`] if `T` implements `Default` and uses `T`'s default value as the
/// initialization value.
#[derive(Debug)]
pub struct Value<T>(T);

impl<T> Initializable for Value<T>
where
    T: DataValue,
{
    type Init = T;

    async fn build(init: Self::Init, _builder: &mut InjectorBuilder) -> crate::Result<Self> {
        Ok(Self(init))
    }
}

impl<T> NewInitValue for Value<T>
where
    T: DataValue + Default,
{
    type Error = Infallible;

    async fn new_init(_injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(T::default())
    }
}

impl<T> Deref for Value<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
