use std::{ops::Deref, ptr::NonNull};

use crate::{
    Injectable,
    refs::{Injectables, UnsizeFrom},
};

/// References something within an [`Injector`](crate::Injector).
///
/// A reference starts out disabled, i.e. dereferencing it panics. It is enabled by the
/// [`RefVisitor`] once the build has finished and disabled again before the data within the
/// injector is dropped.
///
/// `T` may be unsized, e.g. a `dyn Trait` implemented by some injectable (see
/// [`with_resolver`](Self::with_resolver)).
pub(crate) struct Ref<T: ?Sized> {
    ptr: Option<NonNull<T>>,
    /// Looks up the referenced value when enabling.
    resolve: fn(&Injectables) -> Option<NonNull<T>>,
}

impl<T: Injectable> Ref<T> {
    /// Creates a new [`Ref`] to the injectable `T` that will panic when dereferenced.
    pub(crate) fn new() -> Self {
        // SAFETY: the pointer is obtained from `get_ptr`
        unsafe { Self::with_resolver(Injectables::get_ptr::<T>) }
    }
}

impl<T: ?Sized> Ref<T> {
    /// Creates a new [`Ref`] to the injectable `I`, referenced as `T` (e.g. a trait object), that
    /// will panic when dereferenced.
    pub(crate) fn unsized_from<I: Injectable>() -> Self
    where
        T: UnsizeFrom<I>,
    {
        fn resolve<U: ?Sized + UnsizeFrom<I>, I: Injectable>(
            injectables: &Injectables,
        ) -> Option<NonNull<U>> {
            injectables.get_ptr::<I>().map(U::coerce_ptr)
        }

        // SAFETY: the pointer is obtained from `get_ptr` and converted via
        // `UnsizeFrom::coerce_ptr`, whose safety requirements guarantee that it is an unsizing
        // coercion
        unsafe { Self::with_resolver(resolve::<T, I>) }
    }

    /// Creates a new [`Ref`] that will panic when dereferenced and, once enabled, points to the
    /// value returned by `resolve`. Returning `None` keeps the reference disabled.
    ///
    /// # Safety
    ///
    /// `resolve` must return a pointer obtained from [`Injectables::get_ptr`] on the given
    /// `injectables`, optionally converted via an unsizing coercion. It must not access any of the
    /// stored values (e.g. via [`Injectables::get`]), as one of them may be borrowed mutably while
    /// it is called.
    unsafe fn with_resolver(resolve: fn(&Injectables) -> Option<NonNull<T>>) -> Self {
        Self { ptr: None, resolve }
    }

    /// Makes dereferencing convert the given pointer to a reference.
    ///
    /// # Safety
    ///
    /// Until [`disable`](Self::disable) is called on this object, `ptr` must be safely convertible
    /// to a shared reference.
    unsafe fn enable(&mut self, ptr: NonNull<T>) {
        self.ptr = Some(ptr)
    }

    /// Makes dereferencing this object panic.
    fn disable(&mut self) {
        self.ptr = None
    }
}

impl<T: ?Sized> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let Some(ptr) = self.ptr else {
            panic!("reference is not valid yet or has been invalidated")
        };
        // SAFETY: guaranteed by the safety requirements of `enable`
        unsafe { ptr.as_ref() }
    }
}

// SAFETY: `Ref<T>` behaves like a `&T`
unsafe impl<T: ?Sized + Send + Sync> Send for Ref<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for Ref<T> {}

/// Enables or disables the references contained in an injectable.
///
/// There is no public way to create a visitor. It is passed to [`Visitable::visit_refs`](crate::Visitable::visit_refs), which
/// should forward it to the `visit_refs` implementation of each field.
pub struct RefVisitor<'a>(Mode<'a>);

enum Mode<'a> {
    Enable(&'a Injectables),
    Disable,
}

impl<'a> RefVisitor<'a> {
    /// Creates a visitor that points each visited reference to the corresponding value in
    /// `injectables`.
    ///
    /// # Safety
    ///
    /// Every reference enabled by this visitor must be disabled (via a visitor created by
    /// [`disable`](Self::disable)) before any value in `injectables` is moved or dropped. Until
    /// then, no mutable reference to these values may be created except for visiting.
    pub(crate) unsafe fn enable(injectables: &'a Injectables) -> Self {
        Self(Mode::Enable(injectables))
    }

    /// Creates a visitor that disables each visited reference.
    pub(crate) fn disable() -> Self {
        Self(Mode::Disable)
    }

    pub(crate) fn visit_ref<T: ?Sized>(&mut self, reference: &mut Ref<T>) {
        match self.0 {
            Mode::Enable(injectables) => {
                // references to injectables that do not exist stay disabled
                if let Some(ptr) = (reference.resolve)(injectables) {
                    // SAFETY: the safety requirements of `Ref::with_resolver` guarantee that `ptr`
                    // points to a value within `injectables`. The safety requirements of
                    // `RefVisitor::enable` guarantee that it remains valid and is not mutated
                    // until this reference is disabled again.
                    unsafe { reference.enable(ptr) }
                }
            }
            Mode::Disable => reference.disable(),
        }
    }
}
