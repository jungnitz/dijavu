use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::Arc,
};

use rustc_hash::FxHashMap;

use crate::Injectable;

/// A type whose values can be registered via [`FieldDeclareContext::register`] and read via
/// [`FieldPrepareContext::registered`] and [`FieldAssembleContext::registered`].
///
/// [`FieldDeclareContext::register`]: crate::FieldDeclareContext::register
/// [`FieldPrepareContext::registered`]: crate::FieldPrepareContext::registered
/// [`FieldAssembleContext::registered`]: crate::FieldAssembleContext::registered
pub trait Registered: Send + Sync + 'static {
    /// The injectable that consumes values of this type, which is added to the build whenever one
    /// is registered.
    ///
    /// While this type reads as if there were some sort of single-consumer guarantees, there are
    /// not. Any injectable can read the values registered for this type.
    /// If no concrete consumer exists, you may use `()` as the consumer here.
    type Consumer: Injectable;
}

/// The values registered during the declare step, grouped by their type.
#[derive(Default)]
pub(super) struct Registry {
    /// Maps the `TypeId` of `R` to a `RegisteredValues<R>`.
    entries: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Registry {
    pub(super) fn register<R: Registered>(&mut self, value: R) {
        let values = self
            .entries
            .entry(TypeId::of::<R>())
            .or_insert_with(|| Box::new(RegisteredValues::<R>::new()))
            .downcast_mut::<RegisteredValues<R>>()
            .unwrap();
        Arc::get_mut(&mut values.0)
            .expect("values should only be read after the declare step")
            .push(value);
    }

    pub(super) fn get<R: Registered>(&self) -> RegisteredValues<R> {
        match self.entries.get(&TypeId::of::<R>()) {
            Some(values) => values
                .downcast_ref::<RegisteredValues<R>>()
                .unwrap()
                .clone(),
            None => RegisteredValues::new(),
        }
    }
}

/// The values of type `T` that were registered during the declare step, in the order of their
/// registration.
#[derive(Debug)]
pub struct RegisteredValues<T>(Arc<Vec<T>>);

impl<T> RegisteredValues<T> {
    fn new() -> Self {
        Self(Arc::default())
    }
}

impl<T> Clone for RegisteredValues<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Deref for RegisteredValues<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T> IntoIterator for &'a RegisteredValues<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
