//! A collection of useful [`Initializable`](crate::Initializable) implementations.

mod drop_value;

pub use drop_value::*;
use std::convert::Infallible;

#[allow(
    clippy::module_inception,
    reason = "one refers to trait implementations, the other to the concrete implementation"
)]
mod initializables;
pub use initializables::*;

mod inject;
pub use inject::*;

mod depend;
pub use depend::*;

mod build_fn;
pub use build_fn::*;

mod value;
pub use value::*;

use crate::{InitInjector, Initializable, InjectorBuilder, NewInitValue};

impl<I: Initializable> Initializable for Option<I> {
    type Init = Option<I::Init>;

    async fn build(init: Self::Init, builder: &mut InjectorBuilder) -> crate::Result<Self> {
        match init {
            Some(init) => Ok(Some(I::build(init, builder).await?)),
            None => Ok(None),
        }
    }
}

impl<I: Initializable> NewInitValue for Option<I> {
    type Error = Infallible;

    async fn new_init(_injector: &mut InitInjector) -> Result<Self::Init, Self::Error> {
        Ok(None)
    }
}
