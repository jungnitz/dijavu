use std::{any::type_name, borrow::Cow};

use crate::{AssembleContext, DeclareContext, Error, PrepareContext, StartContext, Visitable};

/// A type that is created by a build, once per [`Injector`](crate::Injector).
///
/// Use `#[derive(Injectable)]` to implement this trait.
pub trait Injectable: Visitable + Sized + Send + Sync + 'static {
    /// The data of this injectable after the declare step.
    type Declaration: Send;
    /// The data of this injectable after the prepare step.
    type Preparation: Send;
    /// The state that other injectables access via
    /// [`FieldPrepareContext::get`](crate::FieldPrepareContext::get).
    type Preparing<'ctx>: Preparing<'ctx, Self>;
    /// The data created alongside this injectable by the assemble step, which is passed to
    /// [`start`](Self::start).
    type Start: Send;

    /// The name of this injectable, by default the name of the type.
    ///
    /// The name is typically only used for debugging purposes (e.g. in errors).
    fn name() -> Cow<'static, str> {
        Cow::Borrowed(type_name::<Self>())
    }

    /// Declares what this injectable references and provides.
    fn declare(ctx: DeclareContext<'_, Self>) -> Result<Self::Declaration, Error>;

    /// Prepares this injectable, after everything it depends on has been prepared.
    fn prepare(
        declaration: Self::Declaration,
        ctx: PrepareContext<'_, Self>,
    ) -> impl Future<Output = Result<Self::Preparation, Error>> + Send;

    /// Builds this injectable, along with the data passed to [`start`](Self::start).
    fn assemble(
        preparation: Self::Preparation,
        ctx: AssembleContext<'_, Self>,
    ) -> impl Future<Output = Result<(Self, Self::Start), Error>> + Send;

    /// Starts this injectable after all injectables have been assembled and their references
    /// enabled.
    ///
    /// Injectables are started after their dependencies have been started.
    fn start(
        &self,
        start: Self::Start,
        ctx: StartContext<'_, Self>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let _ = (start, ctx);
        async { Ok(()) }
    }
}

/// The state of an injectable while other injectables access it, see [`Injectable::Preparing`].
pub trait Preparing<'ctx, I: Injectable>: Send {
    /// Creates the state that other injectables access via
    /// [`FieldPrepareContext::get`](crate::FieldPrepareContext::get).
    fn make(preparing: I::Preparation, ctx: PrepareContext<'ctx, I>) -> Self;

    /// Returns the context and the possibly modified preparation.
    fn into_parts(self) -> (PrepareContext<'ctx, I>, I::Preparation);
}

impl Injectable for () {
    type Declaration = ();
    type Preparation = ();
    type Preparing<'ctx> = PrepareContext<'ctx, ()>;
    type Start = ();

    fn declare(_ctx: DeclareContext<'_, Self>) -> Result<Self::Declaration, Error> {
        Ok(())
    }

    async fn prepare(
        _declaration: Self::Declaration,
        _ctx: PrepareContext<'_, Self>,
    ) -> Result<Self::Preparation, Error> {
        Ok(())
    }

    async fn assemble(
        _preparation: Self::Preparation,
        _ctx: AssembleContext<'_, Self>,
    ) -> Result<(Self, Self::Start), Error> {
        Ok(((), ()))
    }
}

impl<'ctx> Preparing<'ctx, ()> for PrepareContext<'ctx, ()> {
    fn make(_preparing: <() as Injectable>::Preparation, ctx: PrepareContext<'ctx, ()>) -> Self {
        ctx
    }
    fn into_parts(self) -> (PrepareContext<'ctx, ()>, ()) {
        (self, ())
    }
}

impl<'ctx, I: Injectable> Preparing<'ctx, I> for (I::Preparation, PrepareContext<'ctx, I>) {
    fn make(preparing: <I as Injectable>::Preparation, ctx: PrepareContext<'ctx, I>) -> Self {
        (preparing, ctx)
    }

    fn into_parts(self) -> (PrepareContext<'ctx, I>, I::Preparation) {
        (self.1, self.0)
    }
}
