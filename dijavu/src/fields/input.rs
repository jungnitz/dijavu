use std::{ops::Deref, sync::Arc};

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, Visitable,
};

/// A value from outside of the build, which was set via
/// [`Builder::set_input`](crate::Builder::set_input).
///
/// There is one value per type `T`, which is shared by all of its `Input<T>`, so prefer newtypes
/// over types like `String`.
///
/// As a field, the build fails in the declare step if the input has not been set. While building,
/// the state of the injectable holds the `Input<T>` as well.
///
/// # Example
///
/// ```
/// use dijavu::{*, fields::{Input, Value}, state::Declare};
///
/// struct ConfigPath(String);
///
/// #[derive(Injectable)]
/// #[injectable(prepare = Self::load)]
/// struct Config {
///     path: Input<ConfigPath>,
///     contents: Value<String>,
/// }
///
/// impl Config {
///     async fn load(
///         state: &mut ConfigState<Declare>,
///         _ctx: PrepareContext<'_, Self>,
///     ) -> Result<(), Error> {
///         state.contents = format!("contents of {}", state.path.0);
///         Ok(())
///     }
/// }
///
/// let mut builder = Builder::new();
/// builder.set_input(ConfigPath("app.toml".to_owned())).add::<Config>();
/// let injector = futures::executor::block_on(builder.build()).unwrap();
/// assert_eq!(*injector.get::<Config>().contents, "contents of app.toml");
/// ```
#[derive(Debug)]
pub struct Input<T>(Arc<T>);

impl<T> Input<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> Clone for Input<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Send + Sync + 'static> InjectableField for Input<T> {
    type Declaration = Input<T>;
    type Preparation = Input<T>;

    fn declare(ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(ctx.input()?)
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration)
    }

    async fn assemble(
        preparation: Self::Preparation,
        _ctx: FieldAssembleContext<'_>,
    ) -> Result<Self, Error> {
        Ok(preparation)
    }
}

// SAFETY: does not visit anything
unsafe impl<T> Visitable for Input<T> {
    fn visit_refs(&mut self, _visitor: &mut RefVisitor<'_>) {}
}

impl<T> Deref for Input<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
