//! The fields injectables are made of.

mod depend;
mod inject;
mod input;
mod value;

pub use depend::*;
pub use inject::*;
pub use input::*;
pub use value::*;

use crate::{Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, Visitable};

/// A field of an injectable, which takes part in every build step of the injectable.
///
/// Use `#[derive(InjectableField)]` to combine other fields into a reusable one.
pub trait InjectableField: Visitable + Sized + Send + Sync + 'static {
    /// The data of this field after the declare step.
    type Declaration: Send;
    /// The data of this field after the prepare step.
    type Preparation: Send;

    /// Runs when the injectable containing this field is declared.
    fn declare(ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error>;

    /// Runs when the injectable containing this field is prepared.
    fn prepare(
        declaration: Self::Declaration,
        ctx: FieldPrepareContext<'_>,
    ) -> impl Future<Output = Result<Self::Preparation, Error>> + Send;

    /// Creates this field when the injectable containing it is assembled.
    fn assemble(
        preparation: Self::Preparation,
        ctx: FieldAssembleContext<'_>,
    ) -> impl Future<Output = Result<Self, Error>> + Send;
}
