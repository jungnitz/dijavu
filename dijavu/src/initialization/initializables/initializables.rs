use crate::initializables::build_fn::BuildFn;
use crate::{InitInjector, Initializable, InjectorBuilder, NewInitValue};
use std::borrow::Borrow;
use std::collections::{HashMap, hash_map};
use std::convert::Infallible;
use std::hash::Hash;
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

pub struct InitializablesInit<T>(Vec<BuildFn<T>>);

impl<T> InitializablesInit<T> {
    pub fn add_with_init<I>(&mut self, init: I::Init)
    where
        I: Initializable + Into<T>,
    {
        self.0.push(BuildFn::new_via_initializable::<I>(init));
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
            values.push(init_builder.build(builder).await?);
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, T> IntoIterator for &'a Initializables<T> {
    type Item = &'a T;
    type IntoIter = InitializablesIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        InitializablesIter(self.0.iter())
    }
}

/// Map of initializables, usually used for collecting objects implementing a trait.
///
/// See [`Initializables`] for a similar initializable and examples of how it may be used.
pub struct InitializablesMap<K, T>(HashMap<K, T>);

impl<K, T> InitializablesMap<K, T> {
    pub fn get<Q>(&self, key: &Q) -> Option<&T>
    where
        Q: Hash + Eq,
        K: Eq + Hash + Borrow<Q>,
    {
        self.0.get(key)
    }

    pub fn iter(&self) -> InitializablesMapIter<'_, K, T> {
        InitializablesMapIter(self.0.iter())
    }
}

pub struct InitializablesMapInit<K, T>(HashMap<K, BuildFn<T>>);

impl<K, T> InitializablesMapInit<K, T> {
    pub fn insert_with_init<I>(&mut self, key: K, init: I::Init)
    where
        K: Eq + Hash,
        I: Initializable + Into<T>,
    {
        self.0
            .insert(key, BuildFn::new_via_initializable::<I>(init));
    }
}

impl<K, T> Initializable for InitializablesMap<K, T>
where
    K: Eq + Hash + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    type Init = InitializablesMapInit<K, T>;

    async fn build(init: Self::Init, builder: &mut InjectorBuilder) -> crate::Result<Self> {
        let mut initializables = HashMap::with_capacity(init.0.len());
        for (k, initializable_builder) in init.0 {
            initializables.insert(k, initializable_builder.build(builder).await?);
        }
        Ok(Self(initializables))
    }
}

impl<K, T> NewInitValue for InitializablesMap<K, T>
where
    K: Eq + Hash + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    type Error = Infallible;

    async fn new_init(_: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(InitializablesMapInit(HashMap::new()))
    }
}

/// Iterator over the items stored in [`InitializablesMap`]
pub struct InitializablesMapIter<'a, K, T>(hash_map::Iter<'a, K, T>);

impl<'a, K, T> Iterator for InitializablesMapIter<'a, K, T> {
    type Item = (&'a K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, K, T> IntoIterator for &'a InitializablesMap<K, T> {
    type Item = (&'a K, &'a T);
    type IntoIter = InitializablesMapIter<'a, K, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
