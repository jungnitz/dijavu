use std::marker::PhantomData;

use crate::{
    Error, FieldAssembleContext, FieldDeclareContext, FieldPrepareContext, InjectableField,
    RefVisitor, Visitable,
};

/// A value that is passed to [`Injectable::start`](crate::Injectable::start) instead of being kept
/// in the injectable, e.g. the receiving end of a channel which is moved into a spawned task.
///
/// Like [`Value`](crate::fields::Value), it starts out as `T::default()` and the state of the
/// injectable holds the `T` directly while building, including in the
/// [`Start`](InjectableField::Start). The field itself holds nothing.
///
/// # Example
///
/// ```
/// use dijavu::{*, fields::{StartValue, Value}, state::{Declare, Start}};
/// use std::sync::Mutex;
///
/// #[derive(Injectable)]
/// #[injectable(prepare = Self::prepare, start = Self::start)]
/// struct Worker {
///     job: StartValue<String>,
///     done: Value<Mutex<Vec<String>>>,
/// }
///
/// impl Worker {
///     async fn prepare(
///         state: &mut WorkerState<Declare>,
///         _ctx: PrepareContext<'_, Self>,
///     ) -> Result<(), Error> {
///         state.job = "job".to_owned();
///         Ok(())
///     }
///
///     async fn start(
///         &self,
///         state: WorkerState<Start>,
///         _ctx: StartContext<'_, Self>,
///     ) -> Result<(), Error> {
///         self.done.lock().unwrap().push(state.job);
///         Ok(())
///     }
/// }
///
/// let mut builder = Builder::new();
/// builder.add::<Worker>();
/// let injector = futures::executor::block_on(builder.build()).unwrap();
/// assert_eq!(*injector.get::<Worker>().done.lock().unwrap(), ["job"]);
/// ```
pub struct StartValue<T>(PhantomData<fn() -> T>);

impl<T> std::fmt::Debug for StartValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StartValue")
    }
}

impl<T: Default + Send + 'static> InjectableField for StartValue<T> {
    type Declaration = T;
    type Preparation = T;
    type Start = T;

    fn declare(_ctx: FieldDeclareContext<'_>) -> Result<Self::Declaration, Error> {
        Ok(T::default())
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
    ) -> Result<(Self, Self::Start), Error> {
        Ok((Self(PhantomData), preparation))
    }
}

// SAFETY: does not visit anything
unsafe impl<T> Visitable for StartValue<T> {
    fn visit_refs(&mut self, _visitor: &mut RefVisitor<'_>) {}
}
