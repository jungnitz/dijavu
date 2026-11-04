//! The fields injectables are made of.

mod depend;
#[expect(clippy::module_inception, reason = "different concepts")]
mod fields;
mod inject;
mod inject_all;
mod input;
mod late;
mod option;
mod registered_values;
mod start_value;
mod value;

pub use depend::*;
pub use fields::*;
pub use inject::*;
pub use inject_all::*;
pub use input::*;
pub use late::*;
pub use start_value::*;
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
    /// The data created alongside this field by the assemble step, which becomes part of the
    /// [`Injectable::Start`](crate::Injectable::Start) of the injectable containing it.
    type Start: Send;

    /// Runs when the injectable containing this field is declared.
    fn declare(ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error>;

    /// Runs when the injectable containing this field is prepared.
    fn prepare(
        declaration: Self::Declaration,
        ctx: FieldPrepareContext<'_>,
    ) -> impl Future<Output = Result<Self::Preparation, Error>> + Send;

    /// Creates this field when the injectable containing it is assembled, along with its
    /// [`Start`](Self::Start).
    fn assemble(
        preparation: Self::Preparation,
        ctx: FieldAssembleContext<'_>,
    ) -> impl Future<Output = Result<(Self, Self::Start), Error>> + Send;
}
