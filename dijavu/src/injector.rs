use std::{ops::Deref, sync::Arc};

use yoke::Yoke;

use crate::{Injectable, refs::Injectables};

/// The result of a build, which owns all injectables.
///
/// Clones share the same injectables, which are dropped together with the last clone.
#[derive(Clone)]
pub struct Injector {
    injectables: Arc<Injectables>,
}

impl Injector {
    pub(crate) fn new(injectables: Injectables) -> Self {
        Self {
            injectables: Arc::new(injectables),
        }
    }

    /// Returns the injectable `I`.
    ///
    /// # Panics
    ///
    /// Panics if `I` is not part of this injector.
    #[track_caller]
    pub fn get<I: Injectable>(&self) -> &I {
        let Some(injectable) = self.get_optional::<I>() else {
            panic!("injectable `{}` is not part of this injector", I::name())
        };
        injectable
    }

    /// Returns the injectable `I`, or `None` if it is not part of this injector.
    pub fn get_optional<I: Injectable>(&self) -> Option<&I> {
        self.injectables.get::<I>()
    }

    /// Returns an owned handle to injectable `I`.
    ///
    /// # Panics
    ///
    /// Panics if `I` is not part of this injector.
    #[track_caller]
    pub fn get_handle<I: Injectable>(&self) -> InjectorHandle<I> {
        let Some(injectable) = self.get_handle_optional::<I>() else {
            panic!("injectable `{}` is not part of this injector", I::name())
        };
        injectable
    }

    /// Returns an owned handle to injectable `I`, or `None` if it is not part of this injector.
    pub fn get_handle_optional<I: Injectable>(&self) -> Option<InjectorHandle<I>> {
        Yoke::try_attach_to_cart(self.injectables.clone(), |injectables| {
            injectables.get::<I>().ok_or(())
        })
        .ok()
        .map(Yoke::wrap_cart_in_option)
        .map(InjectorHandle)
    }
}

/// A handle to a value (injectable, field, ...) that is stored within an [`Injector`].
pub struct InjectorHandle<T: 'static>(Yoke<&'static T, Option<Arc<Injectables>>>);

impl InjectorHandle<()> {
    #[expect(clippy::new_without_default, reason = "makes no sense semantically")]
    pub fn new() -> Self {
        Self(Yoke::new_owned(&()))
    }
}

impl<T: 'static> InjectorHandle<T> {
    pub fn map<New>(self, f: impl FnOnce(&T) -> &New) -> InjectorHandle<New> {
        InjectorHandle(Yoke::map_project(self.0, |value, _| f(value)))
    }
}

impl<T: 'static> Deref for InjectorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.get()
    }
}

impl<T: 'static> Clone for InjectorHandle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
