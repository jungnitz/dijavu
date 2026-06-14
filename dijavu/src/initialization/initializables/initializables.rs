use crate::{InitInjector, Initializable, InjectorBuilder, NewInitValue};
use futures::future::BoxFuture;
use std::convert::Infallible;
use std::slice;

/// Collection of initializables, usually used for collecting objects implementing a trait.
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
/// impl MyTraitsInit<'_> {
///     pub fn register<I>(&mut self, init: I::Init)
///     where
///         I: Initializable + MyTrait,
///     {
///         self.0.data_mut().0.add_with_init::<I>(init)
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
pub struct Initializables<T>(Vec<T>);

impl<T> Initializables<T> {
    pub fn iter(&self) -> InitializablesIter<'_, T> {
        self.into_iter()
    }
}

type InitializablesBuilder<T> = Box<
    dyn for<'a> FnOnce(&'a mut InjectorBuilder) -> BoxFuture<'a, crate::Result<T>> + Send + Sync,
>;

pub struct InitializablesInit<T>(Vec<InitializablesBuilder<T>>);

impl<T> InitializablesInit<T> {
    pub fn add_with_init<I>(&mut self, init: I::Init)
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
    T: Send + 'static,
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
    T: Send + 'static,
{
    type Error = Infallible;

    async fn new_init(_injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(InitializablesInit(Vec::new()))
    }
}

/// Iterator over the items stored in [`Initializables`]
pub struct InitializablesIter<'a, T>(slice::Iter<'a, T>);

impl<'a, T> Iterator for InitializablesIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<'a, T> IntoIterator for &'a Initializables<T> {
    type Item = &'a T;
    type IntoIter = InitializablesIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        InitializablesIter(self.0.iter())
    }
}
