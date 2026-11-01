use std::{
    any::{Any, TypeId},
    ptr::NonNull,
};

use rustc_hash::{FxBuildHasher, FxHashMap};
use std::collections::hash_map::Entry as MapEntry;

use crate::{Injectable, refs::RefVisitor};

/// Storage for the assembled injectables.
///
/// Values are boxed, so that their address stays the same, and only accessed via the raw pointers
/// obtained from [`Box::into_non_null`]. Pointers handed to references are derived from these as
/// well, so creating a temporary `&mut` for visiting does not invalidate them.
pub struct Injectables {
    entries: FxHashMap<TypeId, Entry>,
    /// Set if references might still be enabled while dropping, e.g. because visiting panicked. In
    /// this case, the values are leaked as dropping them could leave dangling references behind.
    leak: bool,
}

#[derive(Copy, Clone)]
struct Entry {
    /// Created from a `Box<T>`
    value: NonNull<dyn Any + Send + Sync>,
    /// Calls `T::visit_refs`.
    ///
    /// Must only be called while there are no other references to the value.
    visit_refs: unsafe fn(NonNull<dyn Any + Send + Sync>, &mut RefVisitor<'_>),
}

impl Injectables {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher),
            leak: false,
        }
    }

    /// Stores the given value.
    ///
    /// # Panics
    ///
    /// Panics if a value of type `T` has already been stored.
    pub(crate) fn set<T: Injectable>(&mut self, t: T) {
        unsafe fn visit_refs<T: Injectable>(
            value: NonNull<dyn Any + Send + Sync>,
            visitor: &mut RefVisitor<'_>,
        ) {
            // SAFETY: `value` was created from a `Box<T>` and the caller guarantees that there are
            // no other references to it
            unsafe { value.cast::<T>().as_mut() }.visit_refs(visitor)
        }

        // never replace a value, references might point to it
        let MapEntry::Vacant(vacant) = self.entries.entry(TypeId::of::<T>()) else {
            panic!("injectable `{}` should only be set once", T::name())
        };
        vacant.insert(Entry {
            value: Box::into_non_null(Box::new(t) as Box<dyn Any + Send + Sync>),
            visit_refs: visit_refs::<T>,
        });
    }

    /// Returns a pointer to the stored value of type `T`.
    pub(crate) fn get_ptr<T: Injectable>(&self) -> Option<NonNull<T>> {
        Some(self.entries.get(&TypeId::of::<T>())?.value.cast::<T>())
    }

    pub fn get<T: Injectable>(&self) -> Option<&T> {
        // SAFETY: values are never borrowed mutably except while visiting, which requires
        // `&mut self`
        self.get_ptr::<T>().map(|ptr| unsafe { ptr.as_ref() })
    }

    /// Enables all references contained in the stored values.
    ///
    /// Values stored afterwards will have their references disabled.
    pub fn enable_refs(&mut self) {
        // if visiting panics, some references may stay enabled, so never drop the values
        self.leak = true;
        // SAFETY: references are only disabled in `drop`, before any value is dropped. Values are
        // never moved or borrowed mutably, apart from visiting.
        let mut visitor = unsafe { RefVisitor::enable(self) };
        for entry in self.entries.values() {
            // SAFETY: there are no other references to the values, as we hold `&mut self` (the
            // visitor only accesses `entries`, not the values)
            unsafe { (entry.visit_refs)(entry.value, &mut visitor) };
        }
        self.leak = false;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Drop for Injectables {
    fn drop(&mut self) {
        if self.leak {
            return;
        }
        // disable all references first, so that dropping a value cannot leave behind dangling
        // references, regardless of the order in which values are dropped. If a value's `Drop`
        // dereferences another injectable, it panics instead.
        // If visiting panics, unwinding skips dropping the values below, i.e. they are leaked.
        let mut visitor = RefVisitor::disable();
        for entry in self.entries.values() {
            // SAFETY: there are no other references to the values, as we hold `&mut self`
            unsafe { (entry.visit_refs)(entry.value, &mut visitor) };
        }
        for (_, entry) in self.entries.drain() {
            // SAFETY: `value` was created via `Box::into_non_null` and is dropped only once
            unsafe { drop(Box::from_non_null(entry.value)) };
        }
    }
}

// SAFETY: all values are `Send + Sync` and only accessed as described above
unsafe impl Send for Injectables {}
unsafe impl Sync for Injectables {}
