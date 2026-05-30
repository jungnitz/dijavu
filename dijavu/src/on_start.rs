use crate::Injector;
use futures::FutureExt;
use futures::future::BoxFuture;

pub trait OnStart: Send + Sync + 'static {
    fn on_start(self, injector: Injector) -> impl Future<Output = crate::Result<()>> + Send;
}

impl<F, Fut> OnStart for F
where
    F: FnOnce(Injector) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = crate::Result<()>> + Send,
{
    fn on_start(self, injector: Injector) -> impl Future<Output = crate::Result<()>> {
        self(injector)
    }
}

pub(crate) trait DynOnStart: Send + Sync + 'static {
    fn on_start(self: Box<Self>, injector: Injector) -> BoxFuture<'static, crate::Result<()>>;
}

impl<T: OnStart> DynOnStart for T {
    fn on_start(self: Box<Self>, injector: Injector) -> BoxFuture<'static, crate::Result<()>> {
        <T as OnStart>::on_start(*self, injector).boxed()
    }
}
