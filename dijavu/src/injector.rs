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
    pub fn get_handle<I: Injectable>(&self) -> InjectableHandle<I> {
        let Some(injectable) = self.get_handle_optional::<I>() else {
            panic!("injectable `{}` is not part of this injector", I::name())
        };
        injectable
    }

    /// Returns an owned handle to injectable `I`, or `None` if it is not part of this injector.
    pub fn get_handle_optional<I: Injectable>(&self) -> Option<InjectableHandle<I>> {
        Yoke::try_attach_to_cart(self.injectables.clone(), |injectables| {
            injectables.get::<I>().ok_or(())
        })
        .ok()
        .map(InjectableHandle)
    }
}

pub struct InjectableHandle<I: Injectable>(Yoke<&'static I, Arc<Injectables>>);

impl<I: Injectable> Deref for InjectableHandle<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.0.get()
    }
}
