//! A collection of useful [`Initializable`](crate::Initializable) implementations.

mod drop_value;
pub use drop_value::*;

#[allow(
    clippy::module_inception,
    reason = "one refers to trait implementations, the other to the concrete implementation"
)]
mod initializables;
pub use initializables::*;

mod inject;
pub use inject::*;

mod value;
pub use value::*;
