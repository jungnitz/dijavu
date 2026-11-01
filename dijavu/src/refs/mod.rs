//! References between injectables.
//!
//! This module bundles the unsafe code required for allowing circular references. Everything
//! outside of it only uses its safe API.

mod bindable;
mod injectables;
mod reference;
mod visitable;

pub use bindable::Bindable;
pub(crate) use injectables::Injectables;
pub(crate) use reference::Ref;
pub use reference::RefVisitor;
pub use visitable::Visitable;
