use crate::{InitData, Injectable, Injector, InjectorBuilder, Restricted, Result, hooks};

pub mod initializable;
pub mod initializables;

/// Mutable container used during initialization
///
/// `InitInjector` contains the _initialization state_ of the application.
/// It provides mutable access this state, including structured access to the initialization state
/// of injectables.
///
/// ## Data access
///
/// You can access the underlying [`InitData`] directly using [`data_mut`](Self::data_mut).
/// This is typically only necessary if you implement a bespoke custom [`Injectable`].
/// In most cases, you should use [`get`](Self::get) for structured access to the initialization state
/// of other injectables:
///
/// ```
/// # use dijavu::{InitData, InitInjector};
/// # type Config = ();
/// # async fn f() -> dijavu::Result<()> {
/// # let mut init_injector = InitInjector::default();
/// let config = init_injector.get::<Config>().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct InitInjector {
    builder: InjectorBuilder,
}

impl InitInjector {
    /// Returns mutable access to the underlying [`InitData`].
    ///
    /// This is primarily intended for low-level access in [`Injectable`] implementations or to
    /// insert external data at application startup.
    pub fn data_mut(&mut self) -> &mut InitData {
        self.builder.init_data_mut()
    }

    /// Retrieves the initialization state of an [`Injectable`] from the container.
    /// This is the preferred way to access initialization data.
    ///
    /// # Errors
    ///
    /// This method performs a call to [`Injectable::init`] on `I` and returns any potential error
    /// of that call.
    pub async fn get<I>(&mut self) -> Result<I::Init<'_>, I::Error>
    where
        I: Injectable,
    {
        self.builder.enqueue::<I>();
        I::init(self, Restricted(())).await
    }

    /// Constructs an [`Injector`] from the current initialization state.
    ///
    /// This entails the following steps:
    /// - All global before build hooks are run
    /// - All [`Injectable`] types that were used during initialization are built, including those
    ///   that are used throughout the build process
    /// - The [`Injector`] is constructed from the injectables
    /// - The start hooks are executed
    ///
    /// # Errors
    ///
    /// This method returns an error if one occurs during any of the aforementioned steps.
    pub async fn build(mut self) -> Result<Injector> {
        hooks::run_global_before_build_hooks(&mut self).await?;
        self.builder.build().await
    }
}
