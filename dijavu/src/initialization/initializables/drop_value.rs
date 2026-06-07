use crate::data::DataValue;
use crate::{InitInjector, Initializable, InjectorBuilder, NewInitValue};
use std::convert::Infallible;
use std::marker::PhantomData;

/// A value that is used only during initialization and is dropped on build.
///
/// `DropValue<T>` allows modifying a value of type `T` during initialization, which is dropped on
/// build.
/// It implements [`NewInitValue`] if `T` implements `Default` and uses `T`'s default value as the
/// initialization value.
#[derive(Debug, Copy, Clone)]
pub struct DropValue<T>(PhantomData<T>);

impl<T: DataValue> Initializable for DropValue<T> {
    type Init = T;

    async fn build(_: Self::Init, _: &mut InjectorBuilder) -> crate::Result<Self> {
        Ok(Self(PhantomData))
    }
}

impl<T: Default + DataValue> NewInitValue for DropValue<T> {
    type Error = Infallible;

    async fn new_init(_: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(T::default())
    }
}
