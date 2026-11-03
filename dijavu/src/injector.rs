use std::sync::Arc;

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

    /// Returns the injectable `T`.
    ///
    /// # Panics
    ///
    /// Panics if `T` is not part of this injector.
    #[track_caller]
    pub fn get<T: Injectable>(&self) -> &T {
        let Some(injectable) = self.get_optional::<T>() else {
            panic!("injectable `{}` is not part of this injector", T::name())
        };
        injectable
    }

    /// Returns the injectable `T`, or `None` if it is not part of this injector.
    pub fn get_optional<T: Injectable>(&self) -> Option<&T> {
        self.injectables.get::<T>()
    }
}
