use crate::container::AppContainerBuilder;
use crate::{AppContainer, BuildData, InitData, Result, hooks, init::InitInjectable};
use std::future;
use std::mem::take;
use std::pin::Pin;

/// Mutable application container used during initialization
///
/// `InitAppContainer` contains the _initialization state_ of the application.
/// It provides:
///
/// - mutable access to initialization state
/// - dependency injection via [`InitInjectable`]
/// - registration of build hooks that construct the final [`AppContainer`] during
///   [`build`](Self::build)
///
/// ## Data access
///
/// You can access the underlying [`InitData`] directly:
///
/// ```rust
/// # use dijavu::{InitData, InitAppContainer};
/// # let mut container = InitAppContainer::default();
/// let data: &mut InitData = container.data_mut();
/// ```
///
/// Or use [`InitInjectable`] for structured access:
///
/// ```rust,ignore
/// let config = init.get::<Config>()?;
/// ```
///
/// ## Build hooks
///
/// Build hooks define how initialization state is transformed into runtime data.
///
/// ```rust,ignore
/// init.on_build(|init, builder| {
///     let config = init.remove::<DbConfig>()?.ok_or_else(|| Error::msg("db config not set"))?;
///     let db = Db::connect(config)?;
///     builder.insert_app_data::<DbKey>(db)?;
///     Ok(())
/// });
/// ```
#[derive(Default)]
pub struct InitAppContainer {
    data: InitData,
    #[expect(clippy::type_complexity)]
    on_build: Vec<
        Box<
            dyn for<'a> FnOnce(
                &'a mut InitData,
                &'a mut AppContainerBuilder,
            )
                -> Pin<Box<dyn Future<Output = Result<()>> + 'a + Send>>,
        >,
    >,
}

impl InitAppContainer {
    /// Returns mutable access to the underlying initialization [`InitData`].
    ///
    /// This is primarily intended for low-level access in [`InitInjectable`] instances or to insert
    /// external data at application startup.
    pub fn data_mut(&mut self) -> &mut InitData {
        &mut self.data
    }

    /// Retrieves an [`InitInjectable`] from the container.
    ///
    /// This is the preferred way to access initialization data.
    pub fn get<I>(&mut self) -> Result<I::Init<'_>, I::InitError>
    where
        I: InitInjectable,
    {
        I::get_init(self)
    }

    /// Registers a build hook.
    ///
    /// Build hooks are executed during [`build`](Self::build) and are responsible for transforming
    /// initialization data into runtime data.
    pub fn on_build(
        &mut self,
        hook: impl FnOnce(&mut InitData, &mut AppContainerBuilder) -> Result<()> + 'static,
    ) {
        self.on_build_async(|data, container| {
            let result = hook(data, container);
            Box::pin(future::ready(result))
        })
    }

    /// Registers an asynchronous build hook.
    ///
    /// Build hooks are executed during [`build`](Self::build) and are responsible for transforming
    /// initialization data into runtime data.
    pub fn on_build_async(
        &mut self,
        hook: impl for<'a> FnOnce(
            &'a mut InitData,
            &'a mut AppContainerBuilder,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a + Send>>
        + 'static,
    ) {
        self.on_build.push(Box::new(hook));
    }

    /// Registers a start hook.
    ///
    /// Start hooks are executed during [`build`](Self::build) after the [`AppContainer`] was
    /// constructed and all start data was added by the build hooks.
    pub fn on_start(
        &mut self,
        hook: impl for<'a> FnOnce(AppContainer, &'a mut BuildData) -> Result<()> + Send + 'static,
    ) {
        self.on_build(move |_, builder| {
            builder.add_start_fn(hook);
            Ok(())
        });
    }

    /// Registers an asynchronous start hook.
    ///
    /// Start hooks are executed during [`build`](Self::build) after the [`AppContainer`] was
    /// constructed and all start data was added by the build hooks.
    pub fn on_start_async(
        &mut self,
        hook: impl for<'a> FnOnce(
            AppContainer,
            &'a mut BuildData,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
        + Send
        + 'static,
    ) {
        self.on_build(move |_, builder| {
            builder.add_async_start_fn(hook);
            Ok(())
        });
    }

    /// Finalizes initialization and constructs the [`AppContainer`] and initialization results.
    pub async fn build(mut self) -> Result<AppContainer> {
        let mut builder = AppContainerBuilder::default();
        hooks::run_global_before_build_hooks(&mut self)?;
        for hook in take(&mut self.on_build) {
            hook(&mut self.data, &mut builder).await?;
        }
        builder.build().await
    }
}
