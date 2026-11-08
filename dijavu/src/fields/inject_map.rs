use std::{
    borrow::Borrow,
    collections::{HashMap, hash_map},
    hash::Hash,
    iter::FusedIterator,
};

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, Injectable,
    InjectableField, RefVisitor, UnsizeFrom, Visitable, fields::InjectToken, refs::Ref,
};

/// References to any number of other injectables implementing `V`, each stored under a key of type
/// `K`, which are inserted during the prepare step via [`InjectMapState::insert`].
///
/// This works like [`InjectAll`](super::InjectAll), but looks up the implementations by key
/// instead of keeping them in order. Trait objects can be used here via
/// [`unsizable!`](crate::unsizable), or via [`bindable!`](crate::bindable) if they should be
/// bindable as well.
///
/// # Example
///
/// ```
/// use dijavu::{*, fields::{Inject, InjectMap}, state::Declare};
///
/// pub trait Handler: Send + Sync {
///     fn handle(&self) -> &str;
/// }
/// dijavu::unsizable!(Handler);
///
/// #[derive(Injectable)]
/// struct Logging;
///
/// impl Handler for Logging {
///     fn handle(&self) -> &str {
///         "logged"
///     }
/// }
///
/// #[derive(Injectable)]
/// struct Metrics;
///
/// impl Handler for Metrics {
///     fn handle(&self) -> &str {
///         "measured"
///     }
/// }
///
/// #[derive(Injectable)]
/// #[injectable(prepare = Self::add_handlers)]
/// struct Server {
///     handlers: InjectMap<&'static str, dyn Handler>,
///     logging: Inject<Logging>,
///     metrics: Inject<Metrics>,
/// }
///
/// impl Server {
///     async fn add_handlers(
///         state: &mut ServerState<Declare>,
///         _ctx: PrepareContext<'_, Self>,
///     ) -> Result<(), Error> {
///         state.handlers.insert("/log", state.logging);
///         state.handlers.insert("/metrics", state.metrics);
///         Ok(())
///     }
/// }
///
/// let mut builder = Builder::new();
/// builder.add::<Server>();
/// let injector = futures::executor::block_on(builder.build()).unwrap();
/// let handlers = &injector.get::<Server>().handlers;
/// assert_eq!(handlers.get("/metrics").unwrap().handle(), "measured");
/// assert!(handlers.get("/other").is_none());
/// ```
pub struct InjectMap<K: Eq + Hash + Send + Sync + 'static, V: ?Sized + Send + Sync + 'static>(
    HashMap<K, Ref<V>>,
);

/// The state of an [`InjectMap`] during the build.
pub struct InjectMapState<K: Eq + Hash + Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> {
    /// Create the references to the implementations.
    get_refs: HashMap<K, fn(&mut FieldAssembleContext<'_>) -> Ref<V>>,
}

impl<K: Eq + Hash + Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> InjectMapState<K, V> {
    /// Inserts the injectable `T` as the implementation for `key`.
    ///
    /// If an implementation was already inserted for `key`, it is replaced, and `true` is
    /// returned.
    ///
    /// The token is obtained by adding an [`Inject`](super::Inject) field for `T`, or for the
    /// injectable itself via its context, like
    /// [`PrepareContext::inject_token`](crate::PrepareContext::inject_token).
    ///
    /// # Panics
    ///
    /// The assemble step panics if `T` is not part of the build, which can only happen if the token
    /// was obtained during another build.
    pub fn insert<T: Injectable>(&mut self, key: K, _token: InjectToken<T>) -> bool
    where
        V: UnsizeFrom<T>,
    {
        self.get_refs
            .insert(key, |ctx| ctx.get_ref_as::<V, T>())
            .is_some()
    }

    /// Removes the implementation for `key`, returning whether there was one.
    pub fn remove<Q: Eq + Hash + ?Sized>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get_refs.remove(key).is_some()
    }

    /// Returns whether an implementation was inserted for `key` so far.
    pub fn contains_key<Q: Eq + Hash + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get_refs.contains_key(key)
    }

    /// Returns the number of implementations inserted so far.
    pub fn len(&self) -> usize {
        self.get_refs.len()
    }

    /// Returns whether no implementation has been inserted so far.
    pub fn is_empty(&self) -> bool {
        self.get_refs.is_empty()
    }
}

impl<K: Eq + Hash + Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> InjectMap<K, V> {
    /// Returns the implementation for `key`, or `None` if there is none.
    ///
    /// # Panics
    ///
    /// Panics if there is an implementation for `key` but the build has not finished yet.
    pub fn get<Q: Eq + Hash + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.0.get(key).map(|reference| &**reference)
    }

    /// Returns whether there is an implementation for `key`.
    pub fn contains_key<Q: Eq + Hash + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.0.contains_key(key)
    }

    /// Returns an iterator over the keys and their implementations, in arbitrary order.
    ///
    /// # Panics
    ///
    /// Advancing the iterator panics until the build has finished.
    pub fn iter(&self) -> InjectMapIter<'_, K, V> {
        InjectMapIter(self.0.iter())
    }

    /// Returns an iterator over the keys, in arbitrary order.
    pub fn keys(&self) -> InjectMapKeys<'_, K, V> {
        InjectMapKeys(self.0.keys())
    }

    /// Returns an iterator over the implementations, in arbitrary order.
    ///
    /// # Panics
    ///
    /// Advancing the iterator panics until the build has finished.
    pub fn values(&self) -> InjectMapValues<'_, K, V> {
        InjectMapValues(self.0.values())
    }

    /// Returns the number of implementations.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether there are no implementations.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<K: Eq + Hash + Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> InjectableField
    for InjectMap<K, V>
{
    type Declaration = InjectMapState<K, V>;
    type Preparation = InjectMapState<K, V>;
    type Start = ();

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(InjectMapState {
            get_refs: HashMap::new(),
        })
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration)
    }

    async fn assemble(
        preparation: Self::Preparation,
        mut ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        let refs = preparation
            .get_refs
            .into_iter()
            .map(|(key, get_ref)| (key, get_ref(&mut ctx)))
            .collect();
        Ok((Self(refs), ()))
    }
}

// SAFETY: the references can only be moved out of `InjectMap` through `&mut self`, and the map is
// never modified while visiting, so every call visits the same references
unsafe impl<K: Eq + Hash + Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> Visitable
    for InjectMap<K, V>
{
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
        for reference in self.0.values_mut() {
            visitor.visit_ref(reference);
        }
    }
}

impl<'a, K: Eq + Hash + Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> IntoIterator
    for &'a InjectMap<K, V>
{
    type Item = (&'a K, &'a V);
    type IntoIter = InjectMapIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the keys and implementations of an [`InjectMap`], see [`InjectMap::iter`].
pub struct InjectMapIter<'a, K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static>(
    hash_map::Iter<'a, K, Ref<V>>,
);

impl<'a, K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> Iterator
    for InjectMapIter<'a, K, V>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, reference)| (key, &**reference))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> ExactSizeIterator
    for InjectMapIter<'_, K, V>
{
}

impl<K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> FusedIterator
    for InjectMapIter<'_, K, V>
{
}

/// An iterator over the keys of an [`InjectMap`], see [`InjectMap::keys`].
pub struct InjectMapKeys<'a, K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static>(
    hash_map::Keys<'a, K, Ref<V>>,
);

impl<'a, K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> Iterator
    for InjectMapKeys<'a, K, V>
{
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> ExactSizeIterator
    for InjectMapKeys<'_, K, V>
{
}

impl<K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> FusedIterator
    for InjectMapKeys<'_, K, V>
{
}

/// An iterator over the implementations of an [`InjectMap`], see [`InjectMap::values`].
pub struct InjectMapValues<'a, K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static>(
    hash_map::Values<'a, K, Ref<V>>,
);

impl<'a, K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> Iterator
    for InjectMapValues<'a, K, V>
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|reference| &**reference)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> ExactSizeIterator
    for InjectMapValues<'_, K, V>
{
}

impl<K: Send + Sync + 'static, V: ?Sized + Send + Sync + 'static> FusedIterator
    for InjectMapValues<'_, K, V>
{
}
