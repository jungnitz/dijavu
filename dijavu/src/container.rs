use std::pin::Pin;

use crate::data::DataEntry;
use crate::injectable::{Injectable, ScopeInjectable};
use crate::{BuildData, DataKey, Error, Result, RuntimeData, ScopeData};

/// Immutable handle to application-wide runtime data.
///
/// `AppContainer` is the central access point for dependency injection during the runtime phase.
/// It is a thin wrapper around a [`&'static Data`](RuntimeData) instance constructed during
/// initialization.
///
/// ## Usage
///
/// The primary way to access dependencies is via [`AppContainer::get`]:
///
/// ```rust,ignore
/// # fn example(container: AppContainer) -> Result<(), Error> {
/// let service = container.get::<MyService>()?;
/// # Ok(())
/// # }
/// ```
///
/// You can also access the underlying [`RuntimeData`] directly if needed, usually in the
/// implementations of [`Injectable`]:
///
/// ```rust
/// # use dijavu::{Data, AppContainer};
/// # let container = AppContainer::empty();
/// let data: &'static Data = container.data();
/// ```
///
/// ## Construction
///
/// See [`InitAppContainer`](crate::InitAppContainer) and [`InitInjectable`](crate::InitInjectable)
/// module for details on how to construct an `AppContainer`.
#[derive(Copy, Clone)]
pub struct AppContainer(&'static RuntimeData);

impl AppContainer {
    /// Creates an [`AppContainer`] with no data.
    pub fn empty() -> Self {
        Self(Box::leak(Box::new(RuntimeData::default())))
    }

    /// Returns the underlying [`RuntimeData`] storage.
    ///
    /// This is primarily intended for low-level access.
    pub fn data(self) -> &'static RuntimeData {
        self.0
    }

    /// Retrieves an [`Injectable`] from the container.
    ///
    /// This is the primary entry point for dependency injection.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # fn example(app: AppContainer) -> Result<(), Error> {
    /// let service = app.get::<MyService>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<I>(self) -> Result<I, I::Error>
    where
        I: Injectable,
    {
        I::get(self)
    }
}

/// Utility for constructing the final [`AppContainer`] during initialization
///
/// `AppContainerBuilder` is used during the _build phase_ to assemble the final runtime container
/// from initialization data.
/// It is typically accessed from within build hooks registered on the initialization container.
///
/// The builder separates data into two categories: _App data_ becomes part of the final
/// [`AppContainer`], while _start data_ is passed to the registered _start functions_ on final
/// assembly (i.e. in [`build`](Self::build)).
/// This allows initialization code to:
///
/// - construct runtime dependencies,
/// - extract and transform initialization state and
/// - use one-time artifacts in the start functions (e.g. routers, startup handles)
#[derive(Default)]
pub struct AppContainerBuilder {
    app_data: RuntimeData,
    start_data: BuildData,
    #[expect(clippy::type_complexity)]
    start_fns: Vec<
        Box<
            dyn for<'a> FnOnce(
                    AppContainer,
                    &'a mut BuildData,
                )
                    -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
                + Send,
        >,
    >,
}

impl AppContainerBuilder {
    /// Inserts a value into the final [`AppContainer`].
    ///
    /// Returns an error if a value for this key has already been inserted.
    /// You should **really** aim to guarantee that no entry is inserted twice (e.g. using private
    /// key types).
    /// If you run into an error here, this usually means bad design on your end.
    pub fn insert_app_data<K>(&mut self, value: K::Item) -> Result<()>
    where
        K: DataKey,
    {
        let DataEntry::Vacant(entry) = self.app_data.entry::<K>() else {
            return Err(Error::msg("app data entry already present"));
        };
        entry.insert(value);
        Ok(())
    }

    /// Inserts a value into the initialization results storage.
    ///
    /// These values are returned separately after build and are intended for  one-time use during
    /// application startup.
    ///
    /// Returns an error if a value for this key has already been inserted.
    /// You should **really** aim to guarantee that no entry is inserted twice (e.g. using private
    /// key types).
    /// If you run into an error here, this usually means bad design on your end.
    pub fn insert_start_data<K>(&mut self, value: K::Item) -> Result<()>
    where
        K: DataKey,
    {
        let DataEntry::Vacant(entry) = self.start_data.entry::<K>() else {
            return Err(Error::msg("init result entry already present"));
        };
        entry.insert(value);
        Ok(())
    }

    /// Adds a start function to be executed in [`build`](Self::build).
    pub fn add_start_fn(
        &mut self,
        func: impl FnOnce(AppContainer, &mut BuildData) -> Result<()> + Send + 'static,
    ) {
        self.add_async_start_fn(move |container, data| {
            let result = func(container, data);
            Box::pin(async { result })
        });
    }

    /// Adds a start function to be executed in [`build`](Self::build).
    pub fn add_async_start_fn(
        &mut self,
        func: impl for<'a> FnOnce(
            AppContainer,
            &'a mut BuildData,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
        + Send
        + 'static,
    ) {
        self.start_fns.push(Box::new(func));
    }

    /// Finalizes the build process.
    ///
    /// Runs the start functions with the collected start data and then returns the constructed
    /// [`AppContainer`] with runtime data.
    /// The runtime data is leaked to `'static`, making the resulting [`AppContainer`] globally
    /// usable for the lifetime of the application.
    pub async fn build(mut self) -> Result<AppContainer> {
        let container = AppContainer(Box::leak(Box::new(self.app_data)));
        for start_fn in self.start_fns {
            start_fn(container, &mut self.start_data).await?
        }
        Ok(container)
    }
}

/// Container supplementing an [`AppContainer`] with additional, short-lived data.
///
/// `ScopeContainer` extends [`AppContainer`] with a mutable, scope-local
/// [`ScopeData`] store. It is intended for per-request / per-task state while still
/// providing access to global application data.
pub struct ScopeContainer {
    app: AppContainer,
    data: ScopeData,
}

impl ScopeContainer {
    /// Creates a new scope from the given [`AppContainer`].
    pub fn new(container: AppContainer) -> Self {
        Self {
            app: container,
            data: ScopeData::default(),
        }
    }

    /// Returns the underlying [`AppContainer`].
    pub fn app(&self) -> AppContainer {
        self.app
    }

    /// Returns mutable access to the [`ScopeData`].
    pub fn scope_data_mut(&mut self) -> &mut ScopeData {
        &mut self.data
    }

    /// Returns shared access to the [`ScopeData`].
    pub fn scope_data(&self) -> &ScopeData {
        &self.data
    }

    /// Retrieves a [`ScopeInjectable`] from this scope.
    ///
    /// This is the primary way to access scoped dependencies.
    pub fn get<'a, I>(&'a mut self) -> Result<I, I::Error>
    where
        I: ScopeInjectable<'a>,
    {
        I::get(self)
    }
}
