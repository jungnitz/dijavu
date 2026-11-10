use std::ptr::NonNull;

use crate::{Injectable, Target};

/// An unsized type (e.g. a trait object) that a pointer to `T` can be converted to.
///
/// Implement this for trait objects via [`unsizable!`](crate::unsizable) or
/// [`bindable!`](crate::bindable), depending on the use-case.
///
/// # Safety
///
/// Each method must return a pointer, reference or box to the same value as its argument, only
/// adding metadata (i.e. it must perform an unsizing coercion like `ptr as NonNull<dyn Trait>`).
pub unsafe trait UnsizeFrom<T>: Send + Sync + 'static {
    /// Converts a pointer to `T` into a pointer to this type.
    fn coerce_ptr(ptr: NonNull<T>) -> NonNull<Self>;

    /// Converts a reference to `T` into a reference to this type.
    fn coerce_ref(t: &T) -> &Self;

    /// Converts a boxed `T` into a box of this type, keeping the same allocation.
    fn coerce_box(t: Box<T>) -> Box<Self>;
}

/// A [`Target`] that can be bound to the injectable `I`.
///
/// Implement this for trait objects via [`bindable!`](crate::bindable).
pub trait Bindable<I: Injectable>: Target + UnsizeFrom<I> {}

/// Implement [`UnsizeFrom`] for trait objects.
///
/// This implements [`UnsizeFrom`] for `dyn _`, so that `InjectAll<dyn _>` and `InjectVec<dyn _>`
/// can be used. As nothing can be bound to `dyn _`, `Inject<dyn _>` and `Depend<dyn _>` do not
/// compile, unless [`bindable!`](crate::bindable) is used instead. The trait object must be
/// `Send + Sync`, either via supertraits or by adding the bounds:
///
/// ```
/// trait Service: Send + Sync {}
/// dijavu::unsizable!(Service);
///
/// trait Other {}
/// dijavu::unsizable!(Other + Send + Sync);
/// ```
#[macro_export]
macro_rules! unsizable {
    ($($bounds:tt)+) => {
        // SAFETY: performs an unsizing coercion
        unsafe impl<T: 'static + $($bounds)+> $crate::UnsizeFrom<T> for dyn $($bounds)+ {
            fn coerce_ptr(ptr: ::core::ptr::NonNull<T>) -> ::core::ptr::NonNull<Self> {
                ptr
            }

            fn coerce_ref(t: &T) -> &Self {
                t
            }

            fn coerce_box(t: ::std::boxed::Box<T>) -> ::std::boxed::Box<Self> {
                t
            }
        }
    };
}

/// Allows binding a trait object to injectables implementing the trait.
///
/// This implements [`Target`] and [`Bindable`] for `dyn _`, so that `Inject<dyn _>` and
/// `Depend<dyn _>` can be used. It includes [`unsizable!`](crate::unsizable), so `InjectAll<dyn _>`
/// and `InjectVec<dyn _>` can be used as well. The trait object must be `Send + Sync`, either via
/// supertraits or by adding the bounds:
///
/// ```
/// trait Service: Send + Sync {}
/// dijavu::bindable!(Service);
///
/// trait Other {}
/// dijavu::bindable!(Other + Send + Sync);
/// ```
#[macro_export]
macro_rules! bindable {
    ($($bounds:tt)+) => {
        $crate::unsizable!($($bounds)+);

        impl $crate::Target for dyn $($bounds)+ {}

        impl<I: $crate::Injectable + $($bounds)+> $crate::Bindable<I> for dyn $($bounds)+ {}
    };
}
