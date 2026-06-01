//! A collection of useful [`Initializable`] implementations.

use crate::build::Slot;
use crate::data::DataValue;
use crate::{InitInjector, Initializable, Injectable, InjectorBuilder, NewInitValue};
use futures::future::BoxFuture;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::ops::Deref;

/// Injects an [`Injectable`].
///
/// Note that while this type is theoretically dereferenceable to `I` right after build, doing so
/// is wrong and will almost certainly lead to a panic.
/// Even if in your case it does not, you should never rely on the fact that it does not.
/// This behavior is necessary in order to allow circular references to exist between injectables.
pub struct Inject<I: 'static>(Slot<I>);

impl<I> Initializable for Inject<I>
where
    I: Injectable,
{
    type Init = ();

    async fn build(_init: Self::Init, builder: &mut InjectorBuilder) -> crate::Result<Self> {
        Ok(Self(builder.enqueue::<I>()))
    }
}

impl<I> NewInitValue for Inject<I>
where
    I: Injectable,
{
    type Error = I::Error;

    async fn new_init(injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        injector.get::<I>().await?;
        Ok(())
    }
}

impl<I: 'static> Inject<I> {
    fn to_static_ref(&self) -> &'static I {
        self.0
            .get()
            .expect("value is not yet injected; this is a bug: never use an Injected<_> right after building it")
    }
}

impl<I: 'static> Deref for Inject<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.to_static_ref()
    }
}

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

type InitializablesBuilder<T> = Box<
    dyn for<'a> FnOnce(&'a mut InjectorBuilder) -> BoxFuture<'a, crate::Result<T>> + Send + Sync,
>;

/// A value that is used only during initialization and is dropped on build.
///
/// `DropValue<T>` allows modifying a value of type `T` during initialization, which is dropped on
/// build.
/// It implements [`NewInitValue`] if `T` implements `Default` and uses `T`'s default value as the
/// initialization value.
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

/// A collection of initializables, usually used for collecting objects implementing a trait.
///
/// This is typically used for a collection of various objects implementing a dyn-compatible trait.
/// Unfortunately, because the `Unsize` trait is not yet stabilized, this still requires some
/// boilerplate:
/// To achieve the desired result, `T` should be a wrapper around a `Box<dyn SomeTrait>` that
/// implements `From<I>` where `I: SomeTrait` using implicit unsized coercion.
///
/// # Example
/// ```
/// # use dijavu::*;
/// # use dijavu::initializables::Initializables;
/// /// The trait that the objects should implement
/// trait MyTrait: Send + Sync + 'static {
///     fn do_something(&self);
/// }
///
/// /// The injectable that we want to register objects implementing `MyTrait` with
/// #[derive(Injectable)]
/// struct MyTraits(Initializables<BoxDynMyTrait>);
///
/// impl MyTraitsInit {
///     pub fn register<I>(&mut self, init: I::Init)
///     where
///         I: Initializable + MyTrait,
///     {
///         self.0.add::<I>(init)
///     }
/// }
///
/// impl MyTraits {
///     /// A function using the objects
///     pub fn do_something(&self) {
///         for my_trait in self.0.iter() {
///             my_trait.0.do_something()
///         }
///     }
/// }
///
/// /// Wrapper type
/// struct BoxDynMyTrait(Box<dyn MyTrait>);
///
/// impl<T> From<T> for BoxDynMyTrait
/// where
///     T: MyTrait,
/// {
///     fn from(value: T) -> Self {
///         Self(Box::new(value))
///     }
/// }
/// ```
pub struct Initializables<T: 'static>(Vec<T>);

/// Initialization state of [`Initializables`]
pub struct InitializablesInit<T>(Vec<InitializablesBuilder<T>>);

impl<T> InitializablesInit<T> {
    pub fn add<I>(&mut self, init: I::Init)
    where
        I: Initializable + Into<T>,
    {
        self.0
            .push(Box::new(|builder| -> BoxFuture<'_, crate::Result<T>> {
                Box::pin(async move { Ok(I::build(init, builder).await?.into()) })
            }));
    }
}

impl<T> Initializable for Initializables<T>
where
    T: Send,
{
    type Init = InitializablesInit<T>;

    async fn build(init: Self::Init, builder: &mut InjectorBuilder) -> crate::Result<Self> {
        let mut values = Vec::with_capacity(init.0.len());
        for init_builder in init.0 {
            values.push(init_builder(builder).await?);
        }
        Ok(Self(values))
    }
}

impl<T> NewInitValue for Initializables<T>
where
    T: Send,
{
    type Error = Infallible;

    async fn new_init(_injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(InitializablesInit(Vec::new()))
    }
}

impl<T> Deref for Initializables<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
