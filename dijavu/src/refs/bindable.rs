use std::ptr::NonNull;

use crate::{Injectable, Target};

/// A [`Target`] that can be bound to the injectable `I`.
///
/// Implement this for trait objects via [`bindable!`](crate::bindable).
///
/// # Safety
///
/// [`coerce`](Self::coerce) must return a pointer to the same value as `ptr`, only adding
/// metadata (i.e. it must perform an unsizing coercion like `ptr as NonNull<dyn Trait>`).
pub unsafe trait Bindable<I: Injectable>: Target {
    /// Converts a pointer to the injectable into a pointer to this target.
    fn coerce(ptr: NonNull<I>) -> NonNull<Self>;
}

/// Allows binding a trait object to injectables implementing the trait.
///
/// This implements [`Target`] and [`Bindable`] for `dyn _`, so that `Inject<dyn _>` and
/// `Depend<dyn _>` can be used. The trait object must be `Send + Sync`, either via supertraits or
/// by adding the bounds:
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
        impl $crate::Target for dyn $($bounds)+ {}

        // SAFETY: performs an unsizing coercion
        unsafe impl<I: $crate::Injectable + $($bounds)+> $crate::Bindable<I>
            for dyn $($bounds)+
        {
            fn coerce(ptr: ::core::ptr::NonNull<I>) -> ::core::ptr::NonNull<Self> {
                ptr
            }
        }
    };
}
