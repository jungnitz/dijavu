use std::{any::type_name, ops::Deref};

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, Visitable, error::UnsetLateError,
};

/// A value that is set during the build, e.g. because `T` has no sensible default or requires an
/// asynchronous call to be created.
///
/// While building, the state of the injectable holds an `Option<T>`, which starts out as `None`
/// and can be set by hooks of the injectable or, via its `Preparing` state, by other injectables.
/// The build fails in the assemble step if it has not been set, so afterwards, a reference to the
/// `T` can be accessed like for [`Value`](crate::fields::Value).
///
/// # Example
///
/// ```
/// use dijavu::{*, fields::{Input, Late}, state::Declare};
///
/// struct DatabaseUrl(String);
///
/// struct Connection {
///     url: String,
/// }
///
/// impl Connection {
///     async fn connect(url: &str) -> Result<Self, Error> {
///         Ok(Self { url: url.to_owned() })
///     }
/// }
///
/// #[derive(Injectable)]
/// #[injectable(prepare = Self::connect)]
/// struct Database {
///     url: Input<DatabaseUrl>,
///     connection: Late<Connection>,
/// }
///
/// impl Database {
///     async fn connect(
///         state: &mut DatabaseState<Declare>,
///         _ctx: PrepareContext<'_, Self>,
///     ) -> Result<(), Error> {
///         state.connection = Some(Connection::connect(&state.url.0).await?);
///         Ok(())
///     }
/// }
///
/// let mut builder = Builder::new();
/// builder
///     .set_input(DatabaseUrl("postgres://localhost".to_owned()))
///     .add::<Database>();
/// let injector = futures::executor::block_on(builder.build()).unwrap();
/// assert_eq!(injector.get::<Database>().connection.url, "postgres://localhost");
/// ```
#[derive(Debug)]
pub struct Late<T>(T);

impl<T> Late<T> {
    /// Returns the `T` held by this value.
    pub fn into_inner(this: Late<T>) -> T {
        this.0
    }
}

impl<T: Send + Sync + 'static> InjectableField for Late<T> {
    type Declaration = Option<T>;
    type Preparation = Option<T>;
    type Start = ();

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(None)
    }

    async fn prepare(
        declaration: Self::Declaration,
        _ctx: FieldPrepareContext<'_>,
    ) -> Result<Self::Preparation, Error> {
        Ok(declaration)
    }

    async fn assemble(
        preparation: Self::Preparation,
        ctx: FieldAssembleContext<'_>,
    ) -> Result<(Self, Self::Start), Error> {
        let Some(value) = preparation else {
            return Err(
                UnsetLateError::new(type_name::<T>(), ctx.injectable_name().clone()).into(),
            );
        };
        Ok((Self(value), ()))
    }
}

// SAFETY: does not visit anything
unsafe impl<T> Visitable for Late<T> {
    fn visit_refs(&mut self, _visitor: &mut RefVisitor<'_>) {}
}

impl<T> Deref for Late<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
