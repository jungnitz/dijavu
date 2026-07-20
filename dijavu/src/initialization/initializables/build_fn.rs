use crate::{Initializable, InjectorBuilder};
use futures::FutureExt;
use futures::future::BoxFuture;
use std::ops::{Deref, DerefMut};

/// An [`Initializable`] that is built using a [build function](BuildFn).
///
/// This type is typically used as `Option<FromBuildFn<T>`.
pub struct FromBuildFn<T>(pub T);

/// A dynamic function that is executed during build time. Typically used as the init value of
/// [`FromBuildFn`].
pub struct BuildFn<T>(
    #[expect(clippy::type_complexity)]
    Box<dyn FnOnce(&mut InjectorBuilder) -> BoxFuture<'_, crate::Result<T>> + Send + Sync>,
);

impl<T> BuildFn<T> {
    pub fn new(
        f: impl FnOnce(&mut InjectorBuilder) -> BoxFuture<'_, crate::Result<T>> + Send + Sync + 'static,
    ) -> Self {
        Self(Box::new(f))
    }

    /// Constructs `T` at build time by building `I` with the given init data and then converting it
    /// using its implementation of `Into<T>`.
    pub fn new_via_initializable<I: Initializable + Into<T>>(init: I::Init) -> Self {
        Self::new(|builder| async move { Ok(I::build(init, builder).await?.into()) }.boxed())
    }

    pub(crate) fn build(
        self,
        builder: &mut InjectorBuilder,
    ) -> impl Future<Output = crate::Result<T>> + Send {
        (self.0)(builder)
    }
}

impl<T: 'static> Initializable for FromBuildFn<T> {
    type Init = BuildFn<T>;

    #[expect(
        clippy::manual_async_fn,
        reason = "prevent higher ranked lifetime error"
    )]
    fn build(
        init: Self::Init,
        builder: &mut InjectorBuilder,
    ) -> impl Future<Output = crate::Result<Self>> + Send {
        async move { Ok(Self(init.build(builder).await?)) }
    }
}

impl<T> Deref for FromBuildFn<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for FromBuildFn<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
