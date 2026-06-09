use crate::build::Slot;
use crate::{InitInjector, Initializable, Injectable, InjectorBuilder, NewInitValue};
use std::ops::Deref;

/// Injects an [`Injectable`].
///
/// Note that while this type is theoretically dereferenceable to `I` right after build, doing so
/// is wrong and will almost certainly lead to a panic.
/// Even if in your case it does not, you should never rely on the fact that it does not.
/// This behavior is necessary in order to allow circular references to exist between injectables.
pub struct Inject<I: 'static>(Slot<I>);

pub struct InjectInit<I: 'static>(Slot<I>);

impl<I> Initializable for Inject<I>
where
    I: Injectable,
{
    type Init = InjectInit<I>;

    async fn build(init: Self::Init, _builder: &mut InjectorBuilder) -> crate::Result<Self> {
        Ok(Self(init.0))
    }
}

impl<I> NewInitValue for Inject<I>
where
    I: Injectable,
{
    type Error = I::Error;

    async fn new_init(injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(InjectInit(injector.get_slot::<I>()))
    }
}

impl<I: 'static> Inject<I> {
    fn to_static_ref(self) -> &'static I {
        self.0
            .get()
            .expect("value is not yet injected; this is a bug: never use an Inject<_> right after building it")
    }
}

impl<I: 'static> Deref for Inject<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.to_static_ref()
    }
}

impl<I: 'static> Clone for Inject<I> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I: 'static> Copy for Inject<I> {}
