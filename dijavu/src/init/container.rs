use crate::container::AppContainerBuilder;
use crate::{AppContainer, Data, Result, hooks, init::InitInjectable};
use std::mem::take;

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
/// You can access the underlying [`Data`] directly:
///
/// ```rust
/// # use dijavu::{Data, InitAppContainer};
/// # let mut container = InitAppContainer::default();
/// let data: &mut Data = container.data_mut();
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
    data: Data,
    #[expect(clippy::type_complexity)]
    on_build: Vec<Box<dyn FnOnce(&mut Data, &mut AppContainerBuilder) -> Result<()>>>,
}

impl InitAppContainer {
    /// Returns mutable access to the underlying initialization [`Data`].
    ///
    /// This is primarily intended for low-level access in [`InitInjectable`] instances or to insert
    /// external data at application startup.
    pub fn data_mut(&mut self) -> &mut Data {
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
        hook: impl FnOnce(&mut Data, &mut AppContainerBuilder) -> Result<()> + 'static,
    ) {
        self.on_build.push(Box::new(hook));
    }

    /// Finalizes initialization and constructs the [`AppContainer`] and initialization results.
    pub async fn build(mut self) -> Result<AppContainer> {
        let mut builder = AppContainerBuilder::default();
        hooks::run_global_before_build_hooks(&mut self)?;
        for hook in take(&mut self.on_build) {
            hook(&mut self.data, &mut builder)?;
        }
        builder.build().await
    }
}
