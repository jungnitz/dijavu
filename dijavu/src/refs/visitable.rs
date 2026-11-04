use crate::refs::RefVisitor;

/// Gives access to references between injectables.
///
/// To enable circular references (e.g. using [`Inject`](crate::fields::Inject)), they are internally stored as pointers.
/// To make this safe, they are enabled by visiting them once the build has finished, and disabled
/// again by visiting them before any injectable is dropped. References that are not visited stay
/// disabled, i.e. they panic when dereferenced. This trait facilitates visiting.
///
/// Use `#[derive(Visitable)]` to implement this trait: it forwards the visitor to every field, so
/// all field types need to implement [`Visitable`] as well (fields can be excluded via
/// `#[visitable(skip)]`). The derive macros for injectables and injectable fields implement this
/// trait automatically.
///
/// # Safety
///
/// [`visit_refs`](Self::visit_refs) must forward the visitor to every reference it visits in a way
/// that ensures that
///
/// - each visited reference can neither be moved out of this value, nor replaced or dropped,
///   through a shared reference to this value (i.e. it must not be reachable through interior
///   mutability like a `Mutex`),
/// - every call visits exactly the same references, even if an earlier call panicked, and
/// - no reference is dereferenced while visiting.
///
/// An implementation that does not visit anything is always sound.
pub unsafe trait Visitable {
    /// Forwards the visitor to every reference in this value, see the safety section of this trait.
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>);
}

// SAFETY: does not visit anything
unsafe impl Visitable for () {
    fn visit_refs(&mut self, _visitor: &mut RefVisitor<'_>) {}
}

// SAFETY: the contained value can only be taken or replaced through `&mut self`, so every call
// visits the same references as long as `T` does
unsafe impl<T: Visitable> Visitable for Option<T> {
    fn visit_refs(&mut self, visitor: &mut RefVisitor<'_>) {
        if let Some(value) = self {
            value.visit_refs(visitor);
        }
    }
}
