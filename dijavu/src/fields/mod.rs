//! The fields injectables are made of.

mod depend;
mod inject;
mod input;
mod value;

pub use depend::*;
pub use inject::*;
pub use input::*;
pub use value::*;

use crate::{AssembleContext, DeclareContext, Error, Injectable, PrepareContext, Visitable};

/// A field of the injectable `I`, which takes part in every build step of `I`.
///
/// Use `#[derive(InjectableField)]` to combine other fields into a reusable one.
pub trait InjectableField<I: Injectable>: Visitable + Sized + Send + Sync + 'static {
    /// The data of this field after the declare step.
    type Declaration: Send;
    /// The data of this field after the prepare step.
    type Preparation: Send;

    /// Runs when `I` is declared.
    fn declare(ctx: DeclareContext<'_, I>) -> Result<Self::Declaration, Error>;

    /// Runs when `I` is prepared.
    fn prepare(
        declaration: Self::Declaration,
        ctx: PrepareContext<'_, I>,
    ) -> impl Future<Output = Result<Self::Preparation, Error>> + Send;

    /// Creates this field when `I` is assembled.
    fn assemble(
        preparation: Self::Preparation,
        ctx: AssembleContext<'_, I>,
    ) -> impl Future<Output = Result<Self, Error>> + Send;
}
